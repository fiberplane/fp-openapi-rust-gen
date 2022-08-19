use crate::types;
use crate::types::{resolve, ResolveTarget, ResolvedReference};
use anyhow::{anyhow, bail, Context, Result};
use convert_case::{Case, Casing};
use okapi::openapi3::{
    Components, MediaType, Operation, Parameter, ParameterValue, PathItem, RefOr, RequestBody,
    Response,
};
use okapi::Map;
use regex::Regex;
use std::borrow::Borrow;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::PathBuf;

pub(crate) fn generate_routes(
    paths: &Map<String, PathItem>,
    src_path: &PathBuf,
    components: &Components,
) -> Result<()> {
    let path = src_path.join("lib.rs");

    // https://stackoverflow.com/a/50691004/11494565
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .append(false)
        .open(path)
        .context("Failed to open or create lib.rs file")?;

    let mut writer = BufWriter::new(file);

    write!(writer, "#[forbid(unsafe_code)]\n\n")?;

    write!(writer, "use anyhow::{{Context as _, Result}};\n")?;
    write!(writer, "use crate::clients::ApiClient;\n")?;
    write!(writer, "use reqwest::Method;\n\n")?;

    write!(writer, "pub mod clients;\n")?;
    write!(writer, "pub mod models;\n\n")?;

    for (endpoint, item) in paths {
        // this is so ugly omg 😭
        if let Some(operation) = &item.get {
            generate_route(endpoint, "GET", operation, &mut writer, components)?;
        }
        if let Some(operation) = &item.put {
            generate_route(endpoint, "PUT", operation, &mut writer, components)?;
        }
        if let Some(operation) = &item.post {
            generate_route(endpoint, "POST", operation, &mut writer, components)?;
        }
        if let Some(operation) = &item.delete {
            generate_route(endpoint, "DELETE", operation, &mut writer, components)?;
        }
        if let Some(operation) = &item.patch {
            generate_route(endpoint, "PATCH", operation, &mut writer, components)?;
        }

        // options, head, trace not yet supported

        write!(writer, "\n")?;
    }

    writer.flush().context("Failed to flush api.rs")?;

    Ok(())
}

fn generate_route(
    endpoint: &str,
    method: &str,
    operation: &Operation,
    writer: &mut BufWriter<File>,
    components: &Components,
) -> Result<()> {
    if let Some(description) = &operation.description {
        write!(writer, "/// {}\n", description)?;
    }

    let method_name = operation
        .operation_id
        .as_ref()
        .ok_or_else(|| anyhow!("\"{} {}\" does not have operation_id", method, endpoint))?;

    write!(writer, "pub async fn {}(\n", method_name)?;
    write!(writer, "    client: ApiClient,\n")?;

    for raw_param in &operation.parameters {
        match resolve(ResolveTarget::Parameter(&Some(raw_param)), components)? {
            Some(ResolvedReference::Parameter(parameter)) => {
                write!(writer, "    {}: ", &parameter.name.to_case(Case::Snake))?;

                if !parameter.required {
                    write!(writer, "Option<")?;
                }

                match &parameter.value {
                    ParameterValue::Schema { schema, .. } => {
                        let type_ = types::map_type(
                            schema.format.as_deref(),
                            schema.instance_type.as_ref(),
                            schema.reference.as_deref(),
                        )
                        .with_context(|| {
                            format!(
                                "Failed to map type for parameter {}. Schema: {:?}",
                                &parameter.name, schema
                            )
                        })?;

                        let string: &str = type_.borrow();
                        write!(writer, "{}", string)?;
                    }
                    ParameterValue::Content { .. } => {}
                }

                if !parameter.required {
                    write!(writer, ">")?;
                }

                write!(writer, ",\n")?;
            }
            Some(resolved) => bail!(
                "resolved to unexpected type {:?}, expected `Parameter`",
                resolved
            ),
            None => {}
        }
    }

    match &operation.request_body {
        Some(RefOr::Ref(_)) => unimplemented!("ref not implemented for request body"),
        Some(RefOr::Object(body)) => {
            let media_types: Vec<&MediaType> = body
                .content
                .iter()
                .filter_map(|(content_type, media_type)| {
                    if content_type == "application/json" || content_type == "multipart/form-data" {
                        Some(media_type)
                    } else {
                        None
                    }
                })
                .collect();
            let json_type = media_types
                .first()
                .ok_or_else(|| anyhow!("only json/form-data supported"))?;
            let schema = json_type
                .schema
                .as_ref()
                .ok_or_else(|| anyhow!("need a schema"))?;

            if let Some(reference) = &schema.reference {
                write!(writer, "    payload: ")?;

                if let Some((_, reference_name)) = reference.rsplit_once('/') {
                    write!(writer, "models::{},", reference_name.to_case(Case::Pascal))?;
                } else {
                    write!(writer, "models::{},", reference.to_case(Case::Pascal))?;
                }

                write!(writer, "\n")?;
            }
        }
        None => {}
    }

    write!(writer, ") -> Result<")?;

    match resolve(
        ResolveTarget::Response(&operation.responses.responses.get("200")),
        components,
    )? {
        Some(ResolvedReference::Responses(response)) => {
            let media_types: Vec<&MediaType> = response
                .content
                .iter()
                .filter_map(|(content_type, media_type)| {
                    if content_type == "application/json" {
                        Some(media_type)
                    } else {
                        None
                    }
                })
                .collect();

            if let Some(json_type) = media_types.first() {
                let schema = json_type
                    .schema
                    .as_ref()
                    .ok_or_else(|| anyhow!("need a schema"))?;

                if let Some(reference) = &schema.reference {
                    if let Some((_, reference_name)) = reference.rsplit_once('/') {
                        write!(writer, "models::{}", reference_name.to_case(Case::Pascal))?;
                    } else {
                        write!(writer, "models::{}", reference.to_case(Case::Pascal))?;
                    }
                }
            } else {
                write!(writer, "()")?;
            }
        }
        Some(resolved) => bail!(
            "resolved to unexpected type {:?}, expected `Response`",
            resolved
        ),
        None => write!(writer, "()")?,
    }

    // RETURN TYPE

    write!(writer, "> {{\n")?;

    generate_function_body(endpoint, method, operation, writer)?;

    write!(writer, "\n}}\n\n")?;

    Ok(())
}

fn generate_function_body(
    endpoint: &str,
    method: &str,
    operation: &Operation,
    writer: &mut BufWriter<File>,
) -> Result<()> {
    write!(writer, "    let response = client.request(\n", )?;
    write!(writer, "        Method::{},\n", method)?;

    // https://stackoverflow.com/a/413077/11494565
    let regex = Regex::new(r#"\{(.*?)\}"#).context("Failed to build regex")?;
    let matched_captures = regex.captures(endpoint);

    if let Some(captures) = matched_captures {
        write!(writer, "        &format!(\"{}\", ", endpoint)?;

        // The first argument of the captures iter is the overall match
        for option in captures.iter().skip(1) {
            if let Some(matched) = option {
                let matched_str = matched.as_str();
                let arg = matched_str.replacen("{", "", 1).replacen("}", "", 1);

                write!(writer, "{} = {}, ", arg, arg)?;
            }
        }

        write!(writer, ")")?;
    } else {
        write!(writer, "        \"{}\"", endpoint)?;
    }

    write!(writer, "\n    )\n")?;

    write!(writer, "        .send()\n")?;
    write!(writer, "        .await?;\n")?;

    Ok(())
}
