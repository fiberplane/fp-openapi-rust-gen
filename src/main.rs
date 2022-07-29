use std::fs::File;
use std::io::BufReader;
use anyhow::{bail, Context, Result};
use clap::Parser;
use std::path::PathBuf;
use okapi::openapi3::OpenApi;
use crate::parsing::OpenApiDocument;

mod generator;

fn main() -> Result<()> {
    let args: Args = Args::parse();

    let path = args.file.as_path();

    if !path.is_file() {
        bail!("Open api file not found");
    }

    let extension = path.extension().unwrap_or_default();

    if extension != "yml" && extension != "yaml" {
        bail!("Input needs to be a YAML file (extension: .yml or .yaml)");
    }

    let file = File::open(path).context("Failed to read OpenAPI document")?;
    let reader = BufReader::new(file);

    let document: OpenApi = serde_yaml::from_reader(reader).context("Failed to parse OpenAPI document")?;

    let output = args.output.as_path();

    if output.exists() && !args.force {
        bail!("Output crate already exists. Supply --force to allow overwriting");
    }

    generator::generate_crate(document, output)
}

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Path to input openapi file
    #[clap(parse(from_os_str), required = true)]
    file: PathBuf,
    /// Path to the crate that will be generated
    #[clap(short, long, parse(from_os_str), required = true)]
    output: PathBuf,
    /// Force overwriting of crate path if it exists
    #[clap(short, long)]
    force: bool,
}
