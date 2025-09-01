# Function Call Sequence: RUN_GENERATOR_TESTS=1 cargo test --package generators-go

## Complete Function Call Stack

### 1. **Cargo Test Entry Point**
```
cargo test --package generators-go
```

### 2. **Build Script Execution** (build.rs)
```
build.rs::main()
├── fs::read_dir(data_dir)  // Read engine/generators/data/
├── collect test_dirs       // ["classes", "enums", "unions", ...]
├── generate macro_code     // create_code_gen_test_suites! macro
└── fs::write(generated_macro.rs)
```

### 3. **Generated Test Functions** (from macro)
For each test directory (e.g., "classes"), two functions are generated:

#### 3a. **Consistency Test**
```rust
fn test_classes_consistent() -> anyhow::Result<()> {
    let test_harness = test_harness::TestHarness::load_test("classes", <GoLanguageFeatures>::default(), false)?;
    test_harness.ensure_consistent_codegen()
}
```

#### 3b. **Evaluation Test** (when RUN_GENERATOR_TESTS=1)
```rust
fn test_classes_evaluate() -> anyhow::Result<()> {
    let test_harness = test_harness::TestHarness::load_test("classes", <GoLanguageFeatures>::default(), true)?;
    test_harness.run()
}
```

### 4. **Test Harness Setup** (TestHarness::load_test)
```
TestHarness::load_test("classes", GoLanguageFeatures::default(), true)
├── get_cargo_root()
├── test_data_dir = cargo_root.join("generators/data/classes")
└── TestStructure::new(test_data_dir, generator, true)
```

### 5. **Test Structure Initialization** (TestStructure::new)
```
TestStructure::new(test_data_dir, generator, persist)
├── project_name = dir.iter().next_back()
├── base_test_dir = cargo_root.join("generators/languages/go/generated_tests")
├── test_dir = utils::unique_dir(base_test_dir, project_name, persist)
├── std::fs::create_dir_all(&test_dir)
├── utils::copy_dir_flat(&dir.join("go"), &test_dir)  // Copy Go sources
├── utils::create_symlink(&dir.join("baml_src"), &test_dir.join("baml_src"))
└── ir = make_test_ir_from_dir(&dir.join("baml_src"))
```

### 6. **Test Execution** (TestStructure::run)
```
test_harness.run()
├── std::env::var("RUN_GENERATOR_TESTS")  // Check environment variable
├── glob::glob("**/*.baml")               // Find BAML files
├── read BAML files into baml_files map
└── create GeneratorArgs
```

### 7. **Generator Arguments Creation**
```rust
GeneratorArgs {
    output_dir_relative_to_baml_src: src_dir.join("baml_client"),
    baml_src_dir: src_dir.join("baml_src"),
    inlined_file_map: baml_files,
    version: env!("CARGO_PKG_VERSION"),
    no_version_check: true,
    default_client_mode: Async,
    on_generate: vec!["gofmt -w . && goimports -w . && go mod tidy && BAML_LIBRARY_PATH=... go test -run NEVER_MATCH"],
    client_type: GeneratorOutputType::Go,
    client_package_name: Some("classes"),
    module_format: None,
    is_pydantic_2: None,
}
```

### 8. **Code Generation** (GoLanguageFeatures::generate_sdk)
```
generator.generate_sdk(ir, &args)
├── GoLanguageFeatures::generate_sdk_files(collector, ir, args)
    ├── package::CurrentRenderPackage::new("baml_client", ir)
    ├── collector.add_file("baml_source_map.go", render_source_files(file_map))
    ├── collector.add_file("runtime.go", render_runtime_code(&pkg))
    ├── ir.functions.iter().map(|f| ir_to_go::functions::ir_function_to_go(f, &pkg))
    ├── collector.add_file("functions.go", render_functions(&functions, &pkg, go_mod_name))
    ├── collector.add_file("functions_stream.go", render_functions_stream(&functions, &pkg, go_mod_name))
    ├── collector.add_file("functions_parse.go", render_functions_parse(&functions, &pkg, go_mod_name))
    ├── collector.add_file("functions_parse_stream.go", render_functions_parse_stream(&functions, &pkg, go_mod_name))
    ├── ir.walk_classes().map(|c| ir_to_go::classes::ir_class_to_go(c.item, &pkg))
    ├── collector.add_file("types/classes.go", render_go_types(&go_classes, &pkg))
    ├── ir.walk_enums().map(|e| ir_to_go::enums::ir_enum_to_go(e.item, &pkg))
    ├── collector.add_file("types/enums.go", render_go_types(&enums, &pkg))
    ├── ir.walk_all_non_streaming_unions().filter_map(|t| ir_to_go::unions::ir_union_to_go(&t, &pkg))
    ├── collector.add_file("types/unions.go", render_go_types(&unions, &pkg))
    ├── collector.add_file("type_map.go", render_type_map(...))
    ├── collector.add_file("types/utils.go", render_go_types_utils(&pkg))
    ├── collector.add_file("type_builder/type_builder.go", render_type_builder_common(...))
    ├── collector.add_file("type_builder/enums.go", render_type_builder_enums(&enums, &pkg))
    ├── collector.add_file("type_builder/classes.go", render_type_builder_classes(&go_classes, &pkg))
    ├── ir.walk_classes().map(|c| ir_to_go::classes::ir_class_to_go_stream(c.item, &pkg))
    ├── collector.add_file("stream_types/classes.go", render_go_stream_types(&go_classes, &pkg, go_mod_name))
    ├── ir.walk_all_streaming_unions().filter_map(|t| ir_to_go::unions::ir_union_to_go_stream(&t, &pkg))
    └── collector.add_file("stream_types/unions.go", render_go_stream_types(&stream_unions, &pkg, go_mod_name))
```

### 9. **Post-Generation Commands** (if RUN_GENERATOR_TESTS=1)
```
for cmd_str in args.on_generate {
    Command::new("sh")
    ├── cmd.args(["-c", &cmd_str])
    ├── cmd.current_dir(&self.src_dir)
    └── cmd.output()
        ├── "gofmt -w ."
        ├── "goimports -w ."
        ├── "go mod tidy"
        └── "BAML_LIBRARY_PATH=... go test -run NEVER_MATCH"
}
```

### 10. **Test Execution** (if RUN_GENERATOR_TESTS=1)
```
if also_run_tests && client_type == Go {
    Command::new("go")
    ├── cmd.args(vec!["test", "-v"])
    ├── cmd.current_dir(&self.src_dir)
    ├── cmd.env("BAML_LIBRARY_PATH", cargo_target_dir)
    └── run_and_stream(&mut cmd)
        ├── child.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()
        ├── thread::spawn(|| { /* stdout forwarding */ })
        ├── thread::spawn(|| { /* stderr forwarding */ })
        └── child.wait()
}
```

### 11. **Go Test Execution** (main_test.go)
```
go test -v
├── TestConsumeSimpleClass()
│   ├── types.SimpleClass{Digits: 10, Words: "hello"}
│   ├── b.ConsumeSimpleClass(ctx, cls)
│   └── LLM API call via BAML runtime
└── TestMakeSimpleClassStream()
    ├── b.Stream.MakeSimpleClass(ctx)
    ├── for result := range channel
    └── LLM streaming API call via BAML runtime
```

## Key Template Rendering Functions

### Class Generation
- `render_go_types(&go_classes, &pkg)` → `types/classes.go`
- Template: `class.go.j2`

### Function Generation
- `render_functions(&functions, &pkg, go_mod_name)` → `functions.go`
- Template: `function.go.j2`

### Streaming Functions
- `render_functions_stream(&functions, &pkg, go_mod_name)` → `functions_stream.go`
- Template: `function.stream.go.j2`

### Runtime Integration
- `render_runtime_code(&pkg)` → `runtime.go`
- Template: `runtime.go.j2`

## Environment Variables Used
- `RUN_GENERATOR_TESTS=1` - Controls test execution
- `BAML_LIBRARY_PATH` - Path to BAML CFFI library
- `CARGO_MANIFEST_DIR` - Cargo project root
- `CARGO_PKG_VERSION` - Package version

## Generated Files Structure
```
generated_tests/classes/
├── baml_client/
│   ├── types/
│   │   ├── classes.go
│   │   ├── enums.go
│   │   ├── unions.go
│   │   └── utils.go
│   ├── stream_types/
│   │   ├── classes.go
│   │   └── unions.go
│   ├── type_builder/
│   │   ├── classes.go
│   │   ├── enums.go
│   │   └── type_builder.go
│   ├── functions.go
│   ├── functions_stream.go
│   ├── functions_parse.go
│   ├── functions_parse_stream.go
│   ├── runtime.go
│   ├── type_map.go
│   └── baml_source_map.go
├── baml_src/ (symlink)
├── main.go
├── main_test.go
├── go.mod
└── go.sum
```
