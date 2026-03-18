// based on https://github.com/tfachmann/typst-as-library/blob/main/src/lib.rs

use std::{
    collections::HashMap,
    io::Read,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use typst::{
    Library, LibraryExt,
    diag::{FileError, FileResult, PackageError, PackageResult},
    ecow::eco_format,
    foundations::{Bytes, Datetime},
    syntax::{FileId, Source, VirtualPath, package::PackageSpec},
    text::{Font, FontBook},
    utils::LazyHash,
};

use typst_kit::fonts::{FontSearcher, FontSlot};

pub struct CompileContext {
    root_dir: PathBuf,
    // source_content: Source,
    template: String,
    std_lib: LazyHash<Library>,
    font_book: LazyHash<FontBook>,
    font_metadata: Vec<FontSlot>,
    file_map: Arc<Mutex<HashMap<FileId, FileEntry>>>,
    cache_dir: PathBuf,
    http_agent: ureq::Agent,
    datetime: time::OffsetDateTime,
}

impl CompileContext {
    pub fn new(
        template: String,
        root_dir: PathBuf,
        cache_dir: PathBuf,
    ) -> Self {
        let fonts = FontSearcher::new()
            .include_embedded_fonts(true)
            .include_system_fonts(true)
            .search_with([PathBuf::from(&root_dir)]);

        let library = Library::builder().build();

        Self {
            template,
            root_dir,
            cache_dir,
            std_lib: LazyHash::new(library),
            font_book: LazyHash::new(fonts.book),
            font_metadata: fonts.fonts,
            file_map: Arc::new(Mutex::new(HashMap::new())),
            http_agent: ureq::Agent::new_with_defaults(),
            datetime: time::OffsetDateTime::now_utc(),
        }
    }
}

#[derive(Clone, Debug)]
struct FileEntry {
    bytes: Bytes,
    source: Option<Source>,
}

impl FileEntry {
    fn new(bytes: Vec<u8>, source: Option<Source>) -> Self {
        Self {
            bytes: Bytes::new(bytes),
            source,
        }
    }

    fn source(&mut self, id: FileId) -> FileResult<Source> {
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

impl CompileContext {
    pub fn insert_file(&self, path: String, content: String) {
        let mut files = self.file_map
            .lock()
            .map_err(|_| FileError::AccessDenied)
            .unwrap();
        let id = FileId::new(None, VirtualPath::new(path));
        let bytes = typst::foundations::Bytes::from_string(content);
        files.insert(
            id,
            FileEntry {
                bytes,
                source: None,
            },
        );
    }
    
    fn get_file(&self, id: FileId) -> FileResult<FileEntry> {
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

impl typst::World for CompileContext {
    fn library(&self) -> &LazyHash<Library> {
        &self.std_lib
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.font_book
    }

    fn main(&self) -> FileId {
        // self.source_content.id()
        FileId::new(None, VirtualPath::new(self.template.clone()))
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        // if id == self.source_content.id() {
        //     Ok(self.source_content.clone())
        // } else {
            self.get_file(id)?.source(id)
        // }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.get_file(id).map(|file| file.bytes.clone())
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.font_metadata[index].get()
    }

    fn today(&self, offset: Option<i64>) -> Option<Datetime> {
        let offset = offset.unwrap_or(0);
        let offset = time::UtcOffset::from_hms(offset.try_into().ok()?, 0, 0).ok()?;
        let time = self.datetime.checked_to_offset(offset)?;
        Some(Datetime::Date(time.date()))
    }
}

fn retry<T, E>(mut f: impl FnMut() -> Result<T, E>) -> Result<T, E> {
    if let Ok(ok) = f() { Ok(ok) } else { f() }
}
