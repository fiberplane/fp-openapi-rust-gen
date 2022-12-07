use crate::types::map_type;
use anyhow::{Context, Result};
use check_keyword::CheckKeyword;
use convert_case::{Case, Casing};
use okapi::openapi3::{Components, SchemaObject};
use schemars::schema::Schema;
use schemars::Set;
use std::fs;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

pub(crate) fn generate_models(components: &Components, src_path: &PathBuf) -> Result<()> {
    let models_path = src_path.join("models");
    fs::create_dir_all(&models_path).context("Failed to create models directory")?;

    let mut mod_rs_content = String::new();

    for (name, object) in &components.schemas {
        let file_name = name.to_case(Case::Snake);
        let path = models_path.join(format!("{}.rs", file_name));

        let file =
            File::create(path).with_context(|| format!("Failed to create file for {}", name))?;
        let mut writer = BufWriter::new(file);

        generate_model(name, object, &mut writer)?;

        writer
            .flush()
            .with_context(|| format!("Failed to flush output for {}", name))?;

        mod_rs_content.push_str(&format!("pub mod {};\n", file_name));
        mod_rs_content.push_str(&format!("pub use {}::*;\n\n", file_name));
    }

    let path = src_path.join("models").join("mod.rs");
    let mut file = File::create(path).context("Failed to create models/mod.rs")?;

    write!(file, "{}", mod_rs_content)?;

    Ok(())
}

fn generate_model(name: &str, object: &SchemaObject, writer: &mut BufWriter<File>) -> Result<()> {
    writer.write_all(b"use serde::{Deserialize, Serialize};\n")?;
    writer.write_all(b"use crate::models;\n")?;
    writer.write_all(b"\n")?;

    writer.write_all(b"#[derive(Clone, Debug, Serialize, Deserialize)]\n")?;
    writer.write_fmt(format_args!(
        "pub struct {} {{\n",
        name.to_case(Case::Pascal)
    ))?;

    if let Some(object_validation) = &object.object {
        for (id, schema) in &object_validation.properties {
            match schema {
                Schema::Bool(_) => unimplemented!("bool is not implemented for schema"),
                Schema::Object(schema_object) => {
                    generate_normal_field(id, schema_object, &object_validation.required, writer)?;
                }
            }
        }
    } else {
        eprintln!("warn: {} had no object. probably a enum?", name);
    }

    writer.write_all(b"}\n\n")?;

    Ok(())
}

fn generate_normal_field(
    name: &str,
    schema: &SchemaObject,
    required_list: &Set<String>,
    writer: &mut BufWriter<File>,
) -> Result<()> {
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

    let type_ = map_type(
        schema.format.as_deref(),
        schema.instance_type.as_ref(),
        schema.reference.as_deref(),
    )?;
    write!(writer, "{}", type_)?;

    if !required {
        write!(writer, ">")?;
    }

    write!(writer, ",\n")?;

    Ok(())
}
