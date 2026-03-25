pub mod context;
pub mod world;

use context::Context;
use typst::{diag::SourceDiagnostic, ecow::EcoVec};
use typst_pdf::{PdfOptions, pdf};

use crate::pdf::world::World;

pub fn compile(
    world: &World,
    template: String,
    input: String,
) -> Result<Vec<u8>, EcoVec<SourceDiagnostic>> {
    let context = Context {
        world,
        template,
        input,
    };

    typst::compile(&context).output.and_then(|d|
        // TODO: allow setting PDF standard
        pdf(&d, &PdfOptions::default()))
}
