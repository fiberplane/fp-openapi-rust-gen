use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use anyhow::{Context, Result};
use okapi::Map;
use okapi::openapi3::{Components, OpenApi, PathItem};
use crate::models::generate_models;

pub(crate) fn generate_crate(document: OpenApi, path: &Path) -> Result<()> {
    // Required dependencies: serde, serde_json, anyhow
    // TODO WHAT HTTP CLIENT`i'd say isacc https://www.arewewebyet.org/topics/http-clients/

    if let Some(components) = document.components {
        generate_models(&components)?;
    }

    generate_routes(&document.paths)
}

pub(crate) fn generate_routes(paths: &Map<String, PathItem>) -> Result<()> {
    unimplemented!()
}
