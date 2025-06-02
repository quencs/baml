use functions::render_functions;

mod r#type;
mod types;
mod functions;
mod utils;

pub struct GoInputs {
    go_mod_name: String,
    functions: Vec<functions::FunctionGo>,
}


pub fn generate_go_client(inputs: GoInputs) -> Result<String, askama::Error> {
    let pkg = r#type::Package::new("baml_client");
    let rendered = render_functions(&inputs.functions, &pkg, &inputs.go_mod_name)?;
    Ok(rendered)
}
