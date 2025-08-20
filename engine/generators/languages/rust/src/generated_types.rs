use crate::package::CurrentRenderPackage;

#[derive(Debug, Clone)]
pub struct RustClass {
    pub name: String,
    pub fields: Vec<RustField>,
}

#[derive(Debug, Clone)]
pub struct RustField {
    pub name: String,
    pub rust_type: String,
    pub optional: bool,
}

#[derive(Debug, Clone)]
pub struct RustEnum {
    pub name: String,
    pub values: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RustUnion {
    pub name: String,
    pub variants: Vec<String>,
}

pub fn render_rust_types(
    classes: &[RustClass],
    enums: &[RustEnum],
    unions: &[RustUnion],
    _pkg: &CurrentRenderPackage,
) -> Result<String, anyhow::Error> {
    let mut output = String::new();
    
    output.push_str("use serde::{Deserialize, Serialize};\n");
    output.push_str("use std::collections::HashMap;\n\n");

    // Generate enums
    for enum_type in enums {
        output.push_str(&format!(
            "#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]\npub enum {} {{\n",
            enum_type.name
        ));
        for value in &enum_type.values {
            output.push_str(&format!("    {},\n", value));
        }
        output.push_str("}\n\n");
    }

    // Generate classes (structs)
    for class in classes {
        output.push_str(&format!(
            "#[derive(Debug, Clone, Serialize, Deserialize)]\npub struct {} {{\n",
            class.name
        ));
        for field in &class.fields {
            let field_type = if field.optional {
                format!("Option<{}>", field.rust_type)
            } else {
                field.rust_type.clone()
            };
            output.push_str(&format!("    pub {}: {},\n", field.name, field_type));
        }
        output.push_str("}\n\n");
    }

    // Generate unions (for now as enums)
    for union in unions {
        output.push_str(&format!(
            "#[derive(Debug, Clone, Serialize, Deserialize)]\n#[serde(untagged)]\npub enum {} {{\n",
            union.name
        ));
        for (i, variant) in union.variants.iter().enumerate() {
            output.push_str(&format!("    Variant{}({}),\n", i, variant));
        }
        output.push_str("}\n\n");
    }

    Ok(output)
}