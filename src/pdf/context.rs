use typst::{
    Library,
    diag::FileResult,
    foundations::{Bytes, Datetime},
    syntax::{FileId, Source, VirtualPath},
    text::{Font, FontBook},
    utils::LazyHash,
};

use crate::pdf::world::World;

pub(crate) struct Context<'a> {
    pub world: &'a World,
    pub template: String,
    pub input: String,
}

impl typst::World for Context<'_> {
    // the standard library
    fn library(&self) -> &LazyHash<Library> {
        &self.world.std_lib
    }

    // metadata about all known fonts
    fn book(&self) -> &LazyHash<FontBook> {
        &self.world.font_book
    }

    // try to access the font with the given index in the font book
    fn font(&self, index: usize) -> Option<Font> {
        self.world.font_metadata[index].get()
    }

    // the FileId of the main source file to compile
    fn main(&self) -> FileId {
        FileId::new(None, VirtualPath::new(&self.template))
    }

    // try to access the specified source file
    fn source(&self, id: FileId) -> FileResult<Source> {
        self.world.get_file(id)?.source(id)
    }

    // try to access the specified file
    fn file(&self, id: FileId) -> FileResult<Bytes> {
        if id.vpath().as_rooted_path().to_str().unwrap() == "/input.json" {
            Ok(Bytes::from_string(self.input.clone()))
        } else {
            self.world.get_file(id).map(|file| file.bytes.clone())
        }
    }

    // the current date for Typst's `#datetime`;
    // if no offset from UTC is given, return local time;
    fn today(&self, offset: Option<i64>) -> Option<Datetime> {
        let offset = offset.unwrap_or(0);
        let offset = time::UtcOffset::from_hms(offset.try_into().ok()?, 0, 0).ok()?;
        let time = time::UtcDateTime::now().checked_to_offset(offset)?;
        Some(Datetime::Date(time.date()))
    }
}
