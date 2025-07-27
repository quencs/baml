function Foo() -> int {
    10
}

agent Basic() {
    state: int = 0;
    tools: [];
    prompt: #"You are a helpful assistant."#;
}

agent Simple() {
    state: int = 0;
    tools: [
      Foo(): (res) => {
        state = res;
      }
    ];
    prompt: #"You are a helpful assistant."#;
}