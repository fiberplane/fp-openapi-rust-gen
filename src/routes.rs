use crate::types;
use crate::types::{map_type, resolve, ResolveTarget, ResolvedReference};
use anyhow::{anyhow, bail, Context, Result};
use convert_case::{Case, Casing};
use okapi::openapi3::{
    Components, MediaType, Operation, Parameter, ParameterValue, PathItem, RefOr, RequestBody,
    Response,
};
use okapi::Map;
use regex::Regex;
use schemars::schema::{Schema, SingleOrVec};
use std::borrow::Borrow;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::ops::Deref;
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

    match resolve(
        ResolveTarget::RequestBody(&operation.request_body.as_ref()),
        components,
    )? {
        Some(ResolvedReference::RequestBody(body)) => {
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
        Some(resolved) => bail!(
            "resolved to unexpected type {:?}, expected `RequestBody`",
            resolved
        ),
        None => {}
    }

    write!(writer, ") -> Result<")?;

    match resolve(
        ResolveTarget::Response(&operation.responses.responses.get("200")),
        components,
    )? {
        Some(ResolvedReference::Responses(response)) => {
            if response.content.is_empty() {
                write!(writer, "()")?;
            } else {
                if let Some(json_media) = response.content.get("application/json") {
                    let schema = json_media
                        .schema
                        .as_ref()
                        .ok_or_else(|| anyhow!("need a schema"))?;

                    if let Some(reference) = &schema.reference {
                        if let Some((_, reference_name)) = reference.rsplit_once('/') {
                            write!(writer, "models::{}", reference_name.to_case(Case::Pascal))?;
                        } else {
                            write!(writer, "models::{}", reference.to_case(Case::Pascal))?;
                        }
                    } else if let Some(array) = &schema.array {
                        match &array.items {
                            Some(SingleOrVec::Single(single)) => match single.deref() {
                                Schema::Bool(_) => eprintln!("unsupported bool for array items"),
                                Schema::Object(schema) => {
                                    let type_ = map_type(
                                        schema.format.as_deref(),
                                        schema.instance_type.as_ref(),
                                        schema.reference.as_deref(),
                                    )?;
                                    write!(writer, "Vec<{}>", type_)?;
                                }
                            },
                            Some(SingleOrVec::Vec(vec)) => {
                                eprintln!("unsupported multiple items vec {:?}", vec)
                            }
                            None => eprintln!("type is array but has no items? {:?}", schema),
                        }
                    } else {
                        let type_ = map_type(
                            schema.format.as_deref(),
                            schema.instance_type.as_ref(),
                            schema.reference.as_deref(),
                        )?;
                        write!(writer, "{}", type_)?;
                    }
                } else if response.content.get("text/plain").is_some() {
                    write!(writer, "String")?;
                } else {
                    eprintln!("unknown response mime type: {:?}", response.content);
                    write!(writer, "Vec<u8>")?;
                }
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

    generate_function_body(endpoint, method, operation, writer, components)?;

    write!(writer, "\n}}\n\n")?;

    Ok(())
}

fn generate_function_body(
    endpoint: &str,
    method: &str,
    operation: &Operation,
    writer: &mut BufWriter<File>,
    components: &Components,
) -> Result<()> {
    write!(writer, "    let response = client.request(\n",)?;
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

    // Query strings as parameters
    for ref_or in &operation.parameters {
        let option = Some(ref_or);
        let option = resolve(ResolveTarget::Parameter(&option), components)?;

        if let Some(resolved_reference) = option {
            if let ResolvedReference::Parameter(parameter) = resolved_reference {
                match parameter.location.as_str() {
                    "path" => continue,
                    "query" => {
                        let parameter_name = parameter.name.to_case(Case::Snake);

                        write!(writer, "        .query(")?;

                        if !parameter.required {
                            write!(
                                writer,
                                "if let Some({}) = {} {{\n            ",
                                parameter_name, parameter_name
                            )?;
                        }

                        write!(writer, "&[(\"{}\", {})]", parameter.name, parameter_name)?;

                        if !parameter.required {
                            write!(writer, "\n        }} else {{\n            &[]\n        }}")?;
                        }

                        write!(writer, ")\n")?;
                    }
                    location => eprintln!("unknown `in`: {}", location),
                }
            }
        }
    }

    // Request body
    if let Some(request_body) = &operation.request_body {
        match resolve(ResolveTarget::RequestBody(&Some(request_body)), components)? {
            Some(ResolvedReference::RequestBody(body)) => {
                if body.content.get("application/json").is_some() {
                    write!(writer, "        .json(&payload)\n")?;
                } else if body.content.get("multipart/form-data").is_some() {
                    write!(writer, "        .form(&payload)\n")?;
                } else {
                    eprintln!("Unsupported type(s): {:?}", body.content);
                }
            }
            Some(resolved) => bail!(
                "resolved to unexpected type {:?}, expected `RequestBody`",
                resolved
            ),
            None => write!(writer, "()")?,
        }
    }

    write!(writer, "        .send()\n")?;
    write!(writer, "        .await?;\n")?;

    Ok(())
}
