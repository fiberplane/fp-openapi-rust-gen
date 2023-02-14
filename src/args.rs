use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    /// Path to input openapi file
    #[clap(parse(from_os_str), required = true)]
    pub file: PathBuf,
    /// Path to the crate that will be generated
    #[clap(short, long, parse(from_os_str), required = true)]
    pub output: PathBuf,

    /// Force overwriting of crate path if it exists
    #[clap(short, long)]
    pub force: bool,

    /// The version string to be included in the crate.
    /// If `local` is `true`, the local dependencies will be referenced with the same version.
    /// Ignored if `workspace` is `true`.
    #[clap(short, long, conflicts_with = "workspace")]
    pub crate_version: Option<String>,
    /// Optional license string to include in the generated Cargo file.
    /// Will inherit from workspace if `workspace` is `true`.
    #[clap(long)]
    pub license: Option<String>,
    /// Optional description string to include in the generated Cargo file.
    #[clap(long)]
    pub description: Option<String>,
    /// Optional readme path string to include in the generated Cargo file.
    /// Will inherit from workspace if `workspace` is `true`.
    #[clap(long)]
    pub readme: Option<String>,
    /// Optional documentation url to include in the generated Cargo file.
    /// Will inherit from workspace if `workspace` is `true`.
    #[clap(long)]
    pub documentation: Option<String>,
    /// Optional repository url to include in the generated Cargo file.
    /// Will inherit from workspace if `workspace` is `true`.
    #[clap(long)]
    pub repository: Option<String>,

    /// Whenever fiberplane-rs dependencies are located locally relative to the output crate.
    /// Ignored if `workspace` is `true`.
    #[clap(short, long)]
    pub local: bool,
    /// Set to indicate fiberplane-rs dependencies should be loaded from the workspace
    #[clap(short, long)]
    pub workspace: bool,
}
