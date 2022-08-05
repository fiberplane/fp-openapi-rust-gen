use std::fs::File;
use std::io::{BufWriter, Write};
use okapi::openapi3::{Components, SchemaObject};
use anyhow::{anyhow, bail, Context, Result};
use check_keyword::CheckKeyword;
use convert_case::{Case, Casing};
use schemars::schema::{InstanceType, Schema, SingleOrVec};
use schemars::Set;

pub(crate) fn generate_models(components: &Components) -> Result<()> {
    for (name, object) in &components.schemas {
        let file_name = name.to_case(Case::Snake);

        let file = File::create(format!("models/{}.rs", file_name)).with_context(|| format!("Failed to create file for {}", name))?;
        let mut writer = BufWriter::new(file);

        generate_model(name, object, &mut writer)?;

        writer.flush().with_context(|| format!("Failed to flush output for {}", name))?;
    }

    Ok(())
}

fn generate_model(name: &str, object: &SchemaObject, writer: &mut BufWriter<File>) -> Result<()> {
    writer.write(b"use serde::{Deserialize, Serialize};\n")?;
    writer.write(b"\n")?;

    writer.write(b"#[derive(Clone, Debug, Serialize, Deserialize)]\n")?;
    writer.write_fmt(format_args!("pub struct {} {{\n", name.to_case(Case::Pascal)))?;

    if let Some(object_validation) = &object.object {
        for (id, schema) in &object_validation.properties {
            match schema {
                Schema::Bool(_) => unimplemented!("bool is not implemented for schema"),
                Schema::Object(schema_object) => {
                    generate_normal_field(id, schema_object, &object_validation.required, writer)?
                }
            }
        }
    } else {
        eprintln!("warn: {} had no object. probably a enum?", name);
    }

    writer.write(b"}\n\n")?;

    Ok(())
}

fn generate_normal_field(name: &str, schema: &SchemaObject, required_list: &Set<String>, writer: &mut BufWriter<File>) -> Result<()> {
    write!(writer, "    #[serde(rename = \"{}\")]\n", name)?;

    let mut snake_name = name.to_case(Case::Snake);

    if snake_name.is_keyword() {
        snake_name.push('_');
    }

    write!(writer, "    pub {}: ", snake_name)?;

    let required = required_list.contains(name);

    if !required {
        write!(writer, "Option<")?;
    }

    match schema.format.as_deref() {
        Some("base64uuid") => write!(writer, "base64uuid::Base64Uuid")?,
        Some("int32") => write!(writer, "i32")?,
        Some("int64") => write!(writer, "i64")?,
        Some("float") => write!(writer, "f32")?,
        Some("double") => write!(writer, "f64")?,
        Some("byte") => write!(writer, "Vec<u8>")?, // TODO: Deserialize from Base64
        Some("binary") => write!(writer, "Vec<u8>")?,
        Some("date") | Some("date-time") => write!(writer, "time::OffsetDateTime")?,
        Some("password") => write!(writer, "SecureString")?,
        Some(_) | None => if let Some(SingleOrVec::Single(instance_type)) = &schema.instance_type {
            match **instance_type {
                InstanceType::Null => write!(writer, "()")?,
                InstanceType::Boolean => write!(writer, "bool")?,
                InstanceType::Object => write!(writer, "std::collections::HashMap<String, String>")?,
                InstanceType::Array => write!(writer, "Vec<serde_json::Value>")?,
                InstanceType::Number => write!(writer, "i64")?,
                InstanceType::String => write!(writer, "String")?,
                InstanceType::Integer => write!(writer, "i32")?,
            }
        } else if let Some(reference) = &schema.reference {
            if let Some((_, reference_name)) = reference.rsplit_once('/') {
                write!(writer, "crate::models::{}", reference_name.to_case(Case::Pascal))?;
            } else {
                write!(writer, "crate::models::{}", reference.to_case(Case::Pascal))?;
            }
        } else {
            bail!("Failed to write field {}. Schema: {:?}", name, schema);
        }
    }

    if !required {
        write!(writer, ">")?;
    }

    write!(writer, ",\n")?;

    Ok(())
}
