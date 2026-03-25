use std::{
    collections::HashMap, io::Read, path::PathBuf, sync::{Arc, Mutex}
};

use typst::{
    Library, LibraryExt, diag::{FileError, FileResult, PackageError, PackageResult}, ecow::eco_format, foundations::Bytes, syntax::{FileId, Source, package::PackageSpec}, text::FontBook, utils::LazyHash
};
use typst_kit::fonts::{FontSearcher, FontSlot};

/// `World` holds the context of the server which is used in Typst compilations
/// and which remains constant between compilations. It is only instantiated
/// once, and it is updated when the server context changes, such as when
/// `Context`s request that new files be inserted into the `file_map`.
pub(crate) struct World {
    pub root_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub std_lib: LazyHash<Library>,
    pub font_book: LazyHash<FontBook>,
    pub font_metadata: Vec<FontSlot>,
    pub file_map: Arc<Mutex<HashMap<FileId, FileEntry>>>,
    pub http_agent: ureq::Agent,
}

impl World {
    pub fn new(root_dir: PathBuf, cache_dir: PathBuf) -> Self {
        let fonts = FontSearcher::new()
            .include_embedded_fonts(true)
            .search_with([&root_dir]);

        let library = Library::builder().build();

        Self {
            root_dir,
            cache_dir,
            std_lib: LazyHash::new(library),
            font_book: LazyHash::new(fonts.book),
            font_metadata: fonts.fonts,
            file_map: Arc::new(Mutex::new(HashMap::new())),
            http_agent: ureq::Agent::new_with_defaults(),
        }
    }

    pub fn get_file(&self, id: FileId) -> FileResult<FileEntry> {
        let mut files = self.file_map.lock().map_err(|_| FileError::AccessDenied)?;
        if let Some(entry) = files.get(&id) {
            return Ok(entry.clone());
        }

        let path = if let Some(package) = id.package() {
            let package_dir = self.download_package(package)?;
            id.vpath().resolve(&package_dir)
        } else {
            id.vpath().resolve(&self.root_dir)
        }
        .ok_or(FileError::AccessDenied)?;

        let content = std::fs::read(&path).map_err(|error| FileError::from_io(error, &path))?;

        Ok(files
            .entry(id)
            .or_insert(FileEntry::new(content, None))
            .clone())
    }

    fn download_package(&self, package: &PackageSpec) -> PackageResult<PathBuf> {
        let package_subdir = format!("{}/{}/{}", package.namespace, package.name, package.version);
        let path = self.cache_dir.join(package_subdir);

        if path.exists() {
            return Ok(path);
        }

        log::info!("Downloading {package}.");

        let url = format!(
            "https://packages.typst.org/{}/{}-{}.tar.gz",
            package.namespace, package.name, package.version,
        );

        let response = retry(|| {
            let response = self
                .http_agent
                .get(&url)
                .call()
                .map_err(|error| eco_format!("{error}"))?;

            let status = response.status().as_u16();
            if status / 100 != 2 {
                return Err(eco_format!(
                    "Response returned unsuccessful status code {status}"
                ));
            }

            Ok(response)
        })
        .map_err(|error| PackageError::NetworkFailed(Some(error)))?;

        let mut compressed_archive = Vec::new();

        response
            .into_body()
            .into_reader()
            .read_to_end(&mut compressed_archive)
            .map_err(|error| PackageError::NetworkFailed(Some(eco_format!("{error}"))))?;

        let raw_archive = zune_inflate::DeflateDecoder::new(&compressed_archive)
            .decode_gzip()
            .map_err(|error| PackageError::MalformedArchive(Some(eco_format!("{error}"))))?;

        let mut archive = tar::Archive::new(raw_archive.as_slice());

        archive.unpack(&path).map_err(|error| {
            _ = std::fs::remove_dir_all(&path);
            PackageError::MalformedArchive(Some(eco_format!("{error}")))
        })?;

        Ok(path)
    }
}

#[derive(Clone, Debug)]
pub struct FileEntry {
    pub bytes: Bytes,
    pub source: Option<Source>,
}

impl FileEntry {
    fn new(bytes: Vec<u8>, source: Option<Source>) -> Self {
        Self {
            bytes: Bytes::new(bytes),
            source,
        }
    }

    pub fn source(&mut self, id: FileId) -> FileResult<Source> {
        let source = if let Some(source) = &self.source {
            source
        } else {
            let contents = std::str::from_utf8(&self.bytes).map_err(|_| FileError::InvalidUtf8)?;
            let contents = contents.trim_start_matches('\u{feff}');
            let source = Source::new(id, contents.into());
            self.source.insert(source)
        };
        Ok(source.clone())
    }
}

fn retry<T, E: std::fmt::Debug>(mut f: impl FnMut() -> Result<T, E>) -> Result<T, E> {
    match f() {
        Ok(ok) => Ok(ok),
        Err(err) => {
            log::warn!("Failed to download package: {err:?}. Retrying.");
            f()
        },
    }
}
