use crate::client_config::generate_client_configs;
use crate::routes::generate_routes;
use anyhow::{anyhow, bail, Context, Result};
use cargo_toml::{Dependency, DependencyDetail, DepsSet, Manifest};
use okapi::openapi3::OpenApi;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::process::Command;

pub(crate) fn generate_crate(
    document: OpenApi,
    path: &Path,
    local_dependencies: bool,
) -> Result<()> {
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

    edit_cargo_toml(path, local_dependencies)?;

    let src_directory = path.join("src");

    generate_client_configs(&document.servers, &src_directory)?;

    if let Some(components) = document.components {
        //generate_models(&components, &src_directory)?;
        generate_routes(&document.paths, &src_directory, &components)?;
    }

    Ok(())
}

fn open_manifest(path: &Path) -> Result<Manifest> {
    Ok(Manifest::from_path(path).context("Failed to parse `Cargo.toml`")?)
}

fn edit_cargo_toml(path: &Path, local_dependencies: bool) -> Result<()> {
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
    add_dependencies(&mut manifest.dependencies, local_dependencies)?;

    let serialized = toml::to_string(&manifest).context("Failed to serialize `Cargo.toml`")?;
    file.write_all(serialized.as_bytes())
        .context("Failed to write `Cargo.toml`")?;

    Ok(())
}

fn add_dependencies(dependencies: &mut DepsSet, local_dependencies: bool) -> Result<()> {
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

    // secrecy
    dependencies.insert("secrecy".to_string(), Dependency::Simple("0".to_string()));

    // reqwest
    {
        let mut dependency = DependencyDetail::default();
        dependency.version = Some("0.11".to_string());
        dependency.features.push("json".to_string());
        dependency.features.push("multipart".to_string());
        dependency.features.push("gzip".to_string());
        dependency.features.push("rustls-tls".to_string());
        dependency.default_features = Some(false);

        dependencies.insert("reqwest".to_string(), Dependency::Detailed(dependency));
    }

    // base64uuid
    dependencies.insert(
        "base64uuid".to_string(),
        fp_dependency("base64uuid", local_dependencies, vec![]),
    );

    // fiberplane-models
    dependencies.insert(
        "fiberplane-models".to_string(),
        fp_dependency("fiberplane-models", local_dependencies, vec![]),
    );

    // time
    {
        let mut dependency = DependencyDetail::default();
        dependency.version = Some("0.3".to_string());
        dependency.features.push("parsing".to_string());
        dependency.features.push("formatting".to_string());
        dependency.features.push("serde-human-readable".to_string());
        dependency.features.push("serde-well-known".to_string());

        dependencies.insert("time".to_string(), Dependency::Detailed(dependency));
    }

    dependencies.insert("bytes".to_string(), Dependency::Simple("1".to_string()));

    Ok(())
}

/// declare a dependency which lives within the fiberplane-rs repository
fn fp_dependency(name: &str, local_dependency: bool, features: Vec<&str>) -> Dependency {
    let mut dependency = DependencyDetail::default();
    dependency.features = features.into_iter().map(str::to_string).collect();

    if local_dependency {
        dependency.path = Some(format!("../{name}"));
    } else {
        dependency.git = Some("ssh://git@github.com/fiberplane/fiberplane-rs.git".to_string());
        dependency.branch = Some("main".to_string());
    }

    Dependency::Detailed(dependency)
}
