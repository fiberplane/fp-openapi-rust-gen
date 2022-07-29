use std::path::Path;
use anyhow::Result;
use okapi::openapi3::OpenApi;
use crate::OpenApiDocument;

pub(crate) fn generate_crate(document: OpenApi, path: &Path) -> Result<()> {
    unimplemented!()
}

pub(crate) fn generate_models() -> Result<()> {
    unimplemented!()
}

pub(crate) fn generate_routes() -> Result<()> {
    unimplemented!()
}
