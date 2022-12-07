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
    write!(
        writer,
        "use reqwest::{{Client, header, Method, RequestBuilder, Url}};\n"
    )?;
    write!(writer, "use std::time::Duration;\n\n")?;

    generate_config_method(&mut writer)?;

    for server in servers {
        generate_client_method(server, &mut writer)?;
    }

    generate_client_type(&mut writer)?;

    writer
        .flush()
        .context("Failed to flush output for `clients.rs`")
}

fn generate_config_method(writer: &mut BufWriter<File>) -> Result<()> {
    write!(writer, "pub fn default_config(\n")?;
    write!(writer, "    timeout: Option<Duration>,\n")?;
    write!(writer, "    user_agent: Option<&str>,\n")?;
    write!(writer, "    default_headers: Option<header::HeaderMap>,\n")?;
    write!(writer, ") -> Result<Client> {{\n")?;

    write!(
        writer,
        "    let mut headers = default_headers.unwrap_or_default();\n"
    )?;
    write!(writer, "    headers.insert(header::USER_AGENT, header::HeaderValue::from_str(user_agent.unwrap_or(\"Fiberplane Rust API client\"))?);\n\n")?;

    write!(writer, "    Ok(Client::builder()\n")?;
    write!(
        writer,
        "        .connect_timeout(timeout.unwrap_or_else(|| Duration::from_secs(10)))\n"
    )?;
    write!(writer, "        .default_headers(headers)\n")?;
    write!(writer, "        .build()?)\n")?;

    write!(writer, "}}\n\n")?;

    Ok(())
}

fn generate_client_method(server: &Server, writer: &mut BufWriter<File>) -> Result<()> {
    let description = server
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

    write!(writer, ") -> Result<ApiClient> {{\n")?;

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
    write!(writer, "        Some(Duration::from_secs(5)),\n")?;
    write!(writer, "        None,\n")?;
    write!(writer, "        None,\n")?;
    write!(writer, "    )?;\n\n")?;

    write!(writer, "    Ok(ApiClient {{\n")?;
    write!(writer, "        client: config,\n")?;
    write!(writer, "        server: Url::parse(url).context(\"Failed to parse base url from Open API document\")?,\n")?;
    write!(writer, "    }})\n")?;

    write!(writer, "}}\n\n")?;

    Ok(())
}

fn generate_client_type(writer: &mut BufWriter<File>) -> Result<()> {
    write!(writer, "#[derive(Debug)]\n")?;
    write!(writer, "pub struct ApiClient {{\n")?;
    write!(writer, "    pub client: Client,\n")?;
    write!(writer, "    pub server: Url,\n")?;
    write!(writer, "}}\n\n")?;

    write!(writer, "impl ApiClient {{\n")?;

    write!(
        writer,
        "    pub fn request(&self, method: Method, endpoint: &str) -> RequestBuilder {{\n"
    )?;
    write!(
        writer,
        "        let url = format!(\"{{}}{{}}\", &self.server, endpoint);\n\n"
    )?;

    write!(writer, "        self.client.request(method, url)\n")?;
    write!(writer, "    }}\n")?;

    write!(writer, "}}\n")?;

    Ok(())
}
