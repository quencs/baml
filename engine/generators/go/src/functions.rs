use askama::Template;

use crate::r#type::{Package, TypeGo, SerializeType};

pub struct FunctionGo {
    documentation: Option<String>,
    name: String,
    args: Vec<(String, TypeGo)>,
    return_type: TypeGo,
    stream_return_type: TypeGo,
}

fn render_function(function: &FunctionGo, pkg: &Package) -> Result<String, askama::Error> {
    let template = FunctionTemplate {
        r#fn: function,
        pkg,
    };

    let stream_template = FunctionStreamTemplate {
        r#fn: function,
        pkg,
    };

    Ok(format!("{}\n\n{}", template.render()?, stream_template.render()?))
}

const GO_TEMPLATE: &str = include_str!("./_templates/client.go.j2");

/// We use doc comments to render the functions.
///
/// ```askama
/// package baml_client
/// 
/// import "{{ go_mod_name }}/baml_client/types"
/// {{ GO_TEMPLATE }}
/// 
/// {% for function in functions %}
/// {{ crate::functions::render_function(function, pkg)? }}
/// {% endfor %}
/// ```
#[derive(askama::Template)]
#[template(in_doc = true, ext = "txt", escape = "none")]
struct FunctionsTemplate<'a> {
    functions: &'a [FunctionGo],
    pkg: &'a Package,
    go_mod_name: &'a str,
}

pub fn render_functions(functions: &[FunctionGo], pkg: &Package, go_mod_name: &str) -> Result<String, askama::Error> {
    FunctionsTemplate {
        functions,
        pkg,
        go_mod_name,
    }.render()
}

#[derive(askama::Template)]
#[template(path = "function.go.j2", escape = "none")]
struct FunctionTemplate<'a> {
    r#fn: &'a FunctionGo,
    pkg: &'a Package,
}


#[derive(askama::Template)]
#[template(path = "function.stream.go.j2", escape = "none")]
struct FunctionStreamTemplate<'a> {
    r#fn: &'a FunctionGo,
    pkg: &'a Package,
}


#[cfg(test)]
mod tests {
    use askama::Template;

    use crate::r#type::TypeMetaGo;

    use super::*;

    #[test]
    fn test_function_template() {
        let function = FunctionGo {
            documentation: Some("hello".to_string()),
            name: "test".to_string(),
            args: vec![],
            return_type: TypeGo::String(TypeMetaGo::default()),
            stream_return_type: TypeGo::String(TypeMetaGo::default()),
        };

        let template = FunctionTemplate {
            r#fn: &function,
            pkg: &Package::new("baml_client"),
        };

        let rendered = template.render().unwrap();
        let expected = r#"
/// hello
func test(ctx context.Context, opts ...CallOptionFunc) (string, error) {

    var callOpts callOption
    for _, opt := range opts {
        opt(&callOpts)
    }

    args := baml.BamlFunctionArguments{
        Kwargs: map[string]any{  },
        Env: getEnvVars(callOpts.env),
    }

    if callOpts.clientRegistry != nil {
        args.ClientRegistry = callOpts.clientRegistry
    }

    encoded, err := baml.EncodeRoot(args)
    if err != nil {
        panic(err)
    }

    result, err := bamlRuntime.CallFunction(ctx, "test", encoded)
    if err != nil {
        return nil, err
    }

    if result.Error != nil {
        return nil, result.Error
    }

    castResult := func (result any) string {
        return string
    }

    casted := castResult(*result.Data)

    return casted, nil
}
        "#.trim();

        if expected != rendered {
            // Pretty diff the rendered and expected
            let diff = prettydiff::diff_words(&expected, &rendered);
            println!("{}", diff.set_highlight_whitespace(true));
            panic!("Expected and rendered are different");
        }
    }

    #[test]
    fn test_function_stream_template() {
        let function = FunctionGo {
            documentation: Some("hello".to_string()),
            name: "test".to_string(),
            args: vec![],
            return_type: TypeGo::String(TypeMetaGo::default()),
            stream_return_type: TypeGo::String(TypeMetaGo::default()),
        };

        let template = FunctionStreamTemplate {
            r#fn: &function,
            pkg: &Package::new("baml_client"),
        };

        let rendered = template.render().unwrap();
        let expected = r#"
/// Streaming version of test
/// hello
func (*stream) test(ctx context.Context, opts ...CallOptionFunc) <-chan StreamResult[string, string] {

    var callOpts callOption
    for _, opt := range opts {
        opt(&callOpts)
    }

    args := baml.BamlFunctionArguments{
        Kwargs: map[string]any{  },
        Env: getEnvVars(callOpts.env),
    }

    if callOpts.clientRegistry != nil {
        args.ClientRegistry = callOpts.clientRegistry
    }

    encoded, err := baml.EncodeRoot(args)
    if err != nil {
        panic(err)
    }

    channel := make(chan StreamResult[string, string])
    raw, err := bamlRuntime.CallFunctionStream(ctx, "test", encoded)
    if err != nil {
        close(channel)
        return channel
    }

    go func() {
        for {
            select {
            case <-ctx.Done():
                close(channel)
                return
            case result, ok := <-raw:
                if !ok {
                    close(channel)
                    return
                }
                if result.Error != nil {
                    close(channel)
                    return
                }
                channel <- (*result.Data).(string)
            }
        }
    }()
    return channel
}
        "#.trim();

        if expected != rendered {
            // Pretty diff the rendered and expected
            let diff = prettydiff::diff_words(&expected, &rendered);
            println!("{}", diff.set_highlight_whitespace(true));
            panic!("Expected and rendered are different");
        }
    }
}
