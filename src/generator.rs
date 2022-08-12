use crate::client_config::generate_client_configs;
use crate::models::generate_models;
use crate::routes::generate_routes;
use anyhow::{anyhow, bail, Context, Result};
use cargo_toml::{Dependency, DependencyDetail, DepsSet, Manifest};
use okapi::openapi3::OpenApi;
use std::fs;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::process::Command;

pub(crate) fn generate_crate(document: OpenApi, path: &Path) -> Result<()> {
    let status = Command::new("cargo")
        .arg("new")
        .arg("--quiet")
        .args(["--vcs", "none"])
        .arg("--lib")
        .args(["--edition", "2021"])
        .arg(
            path.to_str()
                .ok_or_else(|| anyhow!("Failed to convert output OsStr into &str"))?,
        )
        .status()
        .context("Failed to run Cargo")?;

    if !status.success() {
        if let Some(code) = status.code() {
            bail!("Cargo exited with non-zero status code: {}", code);
        } else {
            bail!("Cargo subprocess was killed by another process or system");
        }
    }

    edit_cargo_toml(path)?;

    let src_directory = path.join("src");

    generate_client_configs(&document.servers, &src_directory)?;

    if let Some(components) = document.components {
        generate_models(&components, &src_directory)?;
    }

    generate_routes(&document.paths, &src_directory)
}

fn open_manifest(path: &Path) -> Result<Manifest> {
    Ok(Manifest::from_path(path).context("Failed to parse `Cargo.toml`")?)
}

fn edit_cargo_toml(path: &Path) -> Result<()> {
    let path = path.join("Cargo.toml");

    // https://stackoverflow.com/a/50691004/11494565
    let mut file = OpenOptions::new()
        .read(false)
        .write(true)
        .create(true)
        .append(false)
        .open(&path)
        .context("Failed to open or create `Cargo.toml`")?;

    let mut manifest = open_manifest(&path)?;
    add_dependencies(&mut manifest.dependencies)?;

    let serialized = toml::to_string(&manifest).context("Failed to serialize `Cargo.toml`")?;
    file.write_all(serialized.as_bytes())
        .context("Failed to write `Cargo.toml`")?;

    Ok(())
}

fn add_dependencies(dependencies: &mut DepsSet) -> Result<()> {
    // serde
    {
        let mut dependency = DependencyDetail::default();
        dependency.version = Some("1".to_string());
        dependency.features.push("derive".to_string());

        dependencies.insert("serde".to_string(), Dependency::Detailed(dependency));
    }

    // serde_json
    dependencies.insert(
        "serde_json".to_string(),
        Dependency::Simple("1".to_string()),
    );

    // anyhow
    dependencies.insert("anyhow".to_string(), Dependency::Simple("1".to_string()));

    // surf
    {
        let mut dependency = DependencyDetail::default();
        dependency.version = Some("2".to_string());
        dependency.features.push("encoding".to_string());
        dependency.features.push("hyper-client".to_string());
        dependency.default_features = Some(false);

        dependencies.insert("surf".to_string(), Dependency::Detailed(dependency));
    }

    // base64uuid
    {
        let mut dependency = DependencyDetail::default();
        dependency.git = Some("ssh://git@github.com/fiberplane/fiberplane-rs.git".to_string());
        dependency.branch = Some("main".to_string());

        dependencies.insert("base64uuid".to_string(), Dependency::Detailed(dependency));
    }

    Ok(())
}
