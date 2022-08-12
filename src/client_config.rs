use anyhow::{anyhow, Context, Result};
use convert_case::{Case, Casing};
use okapi::openapi3::Server;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::PathBuf;

pub(crate) fn generate_client_configs(servers: &[Server], src_path: &PathBuf) -> Result<()> {
    let path = src_path.join("clients.rs");

    // https://stackoverflow.com/a/50691004/11494565
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .append(false)
        .open(path)
        .context("Failed to open or create clients.rs file")?;

    let mut writer = BufWriter::new(file);

    write!(writer, "use anyhow::{{Context as _, Result}};\n")?;
    write!(writer, "use surf::{{Client, Config, Url}};\n")?;
    write!(writer, "use std::time::Duration;\n\n")?;

    generate_config_method(&mut writer)?;

    for server in servers {
        generate_client_method(server, &mut writer)?;
    }

    writer
        .flush()
        .context("Failed to flush output for `clients.rs`")
}

fn generate_config_method(writer: &mut BufWriter<File>) -> Result<()> {
    write!(writer, "pub fn default_config(\n")?;
    write!(writer, "    url: Url,\n")?;
    write!(writer, "    timeout: Option<u64>,\n")?;
    write!(writer, "    user_agent: Option<&str>,\n")?;
    write!(writer, ") -> Result<Config> {{\n")?;

    write!(writer, "    Ok(Config::new()\n")?;
    write!(writer, "        .set_base_url(url)\n")?;
    write!(
        writer,
        "        .set_timeout(timeout.map(|seconds| Duration::from_secs(seconds)))\n"
    )?;
    write!(writer, "        .add_header(\"User-Agent\", user_agent.unwrap_or(\"Fiberplane Rust API client\"))?)\n")?;

    write!(writer, "}}\n\n")?;

    Ok(())
}

fn generate_client_method(server: &Server, writer: &mut BufWriter<File>) -> Result<()> {
    let mut description = server
        .description
        .as_ref()
        .ok_or_else(|| anyhow!("Server {:?} does not have `description`", server))?;
    let description = description.replacen("servers", "", 1);

    write!(
        writer,
        "pub fn {}_client(",
        description.to_case(Case::Snake)
    )?;

    let mut peekable = server.variables.iter().peekable();

    while let Some((name, _)) = peekable.next() {
        write!(writer, "\n    {}: Option<&str>,", name.to_case(Case::Snake))?;

        if peekable.peek().is_none() {
            write!(writer, "\n")?;
        }
    }

    write!(writer, ") -> Result<Client> {{\n")?;

    let variables: Vec<String> = server
        .variables
        .iter()
        .map(|(name, server)| {
            let snake_name = name.to_case(Case::Snake);
            format!(
                "let {} = {}.unwrap_or(\"{}\");",
                snake_name, snake_name, server.default
            )
        })
        .collect();

    let variables = variables.join("\n    ");

    if !server.variables.is_empty() {
        write!(writer, "    {}\n", variables)?;

        let format_args: Vec<String> = server
            .variables
            .iter()
            .map(|(name, _)| {
                let snake_name = name.to_case(Case::Snake);
                format!("{} = {}", snake_name, snake_name)
            })
            .collect();

        write!(
            writer,
            "    let url = &format!(\"{}\", {});",
            server.url,
            format_args.join(", ")
        )?;
    } else {
        write!(writer, "    let url = \"{}\";", server.url)?;
    }

    write!(writer, "\n\n")?;

    write!(writer, "    let config = default_config(\n")?;
    write!(
        writer,
        "        Url::parse(url).context(\"Failed to parse base url from Open API document\")?,\n"
    )?;
    write!(writer, "        Some(5),\n")?;
    write!(writer, "        None,\n")?;
    write!(writer, "    )?;\n\n")?;

    write!(writer, "    Ok(config.try_into()?)\n")?;

    write!(writer, "}}\n\n")?;

    Ok(())
}
