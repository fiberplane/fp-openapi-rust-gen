use std::fs;
use std::fs::File;
use std::io::BufReader;
use anyhow::{bail, Context, Result};
use clap::Parser;
use std::path::{Path, PathBuf};
use okapi::openapi3::OpenApi;

mod generator;
mod models;
mod routes;

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

    if output.exists() {
        if args.force {
            // Windows does not delete the directory until the last handle to it is closed: https://docs.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-removedirectorya#remarks
            // A handle gets created by `clap` to it however (as its passed as a PathBuf in CLI args)
            // To work around this, rename the old directory and delete that. I hate Windows.
            let new_name = Path::new("__tmp_old_crate");

            fs::rename(output, new_name).context("Failed to rename previous output crate")?;
            fs::remove_dir_all(new_name).context("Failed to delete previous output crate")?;
        } else {
            bail!("Output crate already exists. Supply --force to allow overwriting");
        }
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
