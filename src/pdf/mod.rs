mod context;

use context::CompileContext;
use typst::{diag::SourceDiagnostic, ecow::EcoVec};
use typst_pdf::{PdfOptions, pdf};

use crate::config::Config;

pub fn compile(
    template: String,
    input: String,
    config: &Config
) -> Result<Vec<u8>, EcoVec<SourceDiagnostic>> {
    let context = CompileContext::new(
        template,
        config.rootdir.clone(),
        config.cachedir.clone(),
    );

    context.insert_file("/input.json".to_string(), input);

    typst::compile(&context)
        .output
        .and_then(|d|
            // TODO: allow setting PDF standard
            pdf(&d, &PdfOptions::default())
        )
}
