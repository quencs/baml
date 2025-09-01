# BAML Generator Test Data Flow

```mermaid
graph TD
    %% Input Files
    A[BAML Source: engine/generators/data/classes/baml_src/main.baml] --> B[Test Harness]
    C[Go Template Files] --> B
    D[Test Data: engine/generators/data/classes/go/] --> B
    
    %% Test Harness Processing
    B --> E[TestStructure::new]
    E --> F[Copy Language Sources]
    E --> G[Create Symlink to baml_src]
    E --> H[Parse BAML → IR]
    
    %% Code Generation
    H --> I[GoLanguageFeatures::generate_sdk_files]
    I --> J[Generate Go Code]
    
    %% Generated Files
    J --> K[baml_client/types/classes.go]
    J --> L[baml_client/functions.go]
    J --> M[baml_client/functions_stream.go]
    J --> N[baml_client/runtime.go]
    J --> O[baml_client/type_map.go]
    
    %% Post-Generation Commands
    J --> P[Post-Generation Commands]
    P --> Q[gofmt -w .]
    P --> R[goimports -w .]
    P --> S[go mod tidy]
    
    %% Test Execution Decision
    J --> T{RUN_GENERATOR_TESTS=1?}
    
    %% Test Execution
    T -->|Yes| U[Run Go Tests]
    T -->|No| V[Print: Not running! Set RUN_GENERATOR_TESTS=1 to run tests]
    
    %% Go Test Execution
    U --> W[go test -v]
    W --> X[Set BAML_LIBRARY_PATH]
    W --> Y[Execute main_test.go]
    
    %% Test Results
    Y --> Z[TestConsumeSimpleClass]
    Y --> AA[TestMakeSimpleClassStream]
    
    %% LLM Integration
    Z --> BB[Call ConsumeSimpleClass with LLM]
    AA --> CC[Call MakeSimpleClass with LLM]
    
    %% Output
    BB --> DD[Test Results]
    CC --> DD
    V --> DD
    
    %% Styling
    classDef inputFile fill:#e1f5fe
    classDef generatedFile fill:#f3e5f5
    classDef process fill:#e8f5e8
    classDef test fill:#fff3e0
    classDef output fill:#fce4ec
    
    class A,C,D inputFile
    class K,L,M,N,O generatedFile
    class B,E,F,G,H,I,J,P,Q,R,S process
    class T,U,W,X,Y,Z,AA,BB,CC test
    class DD,V output
```

## Key Components

### Input Files
- **BAML Source**: The main BAML file defining classes, functions, and tests
- **Go Templates**: Language-specific templates for code generation
- **Test Data**: Pre-existing Go test files and configuration

### Processing Pipeline
1. **Test Harness**: Orchestrates the entire test process
2. **IR Generation**: Converts BAML to Intermediate Representation
3. **Code Generation**: Uses templates to generate language-specific code
4. **Post-Processing**: Formats and tidies the generated code

### Generated Output
- **Type Definitions**: Go structs corresponding to BAML classes
- **Function Implementations**: Go functions for BAML functions
- **Streaming Support**: Streaming versions of functions
- **Runtime Integration**: Core BAML runtime code

### Test Execution
- **Conditional**: Only runs when `RUN_GENERATOR_TESTS=1`
- **Integration**: Tests generated code against actual LLM providers
- **Validation**: Ensures generated code works end-to-end
