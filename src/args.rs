use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    /// The version string to be included in the crate.
    /// If `local` is `true`, the local dependencies will be referenced with the same version.
    /// Ignored if `workspace` is `true`.
    #[clap(short, long)]
    pub crate_version: Option<String>,
    /// Path to input openapi file
    #[clap(parse(from_os_str), required = true)]
    pub file: PathBuf,
    /// Force overwriting of crate path if it exists
    #[clap(short, long)]
    pub force: bool,
    /// Whenever fiberplane-rs dependencies are located locally relative to the output crate.
    /// Ignored if `workspace` is `true`.
    #[clap(short, long)]
    pub local: bool,
    /// Path to the crate that will be generated
    #[clap(short, long, parse(from_os_str), required = true)]
    pub output: PathBuf,
    /// Set to indicate fiberplane-rs dependencies should be loaded from the workspace
    #[clap(short, long)]
    pub workspace: bool,
}
