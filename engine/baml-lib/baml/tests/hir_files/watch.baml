function Foo() -> int {
  watch let x = 5;
  watch let y = 10;
  watch let z: int = 20;
  10
}


function MyFunction(prev: int, next: int) -> bool {
  true
}

// function Foo() {
//   let x = 5 @watch(name=x);
//   let y = 10 @watch(name=y);
//   let z: int = 20 @watch(name=z);
//
//   10
// }
//
// function MyFunction(prev, next) {
//   true
// }
//
// class std::Request {
//   base_url: string
//   headers: map<string, string>
//   query_params: map<string, string>
// }
//
// class baml::WatchOptions {
//   name: string
//   when: ("never" | "manual" | (ANY, ANY) -> bool)
// }
//
// enum std::HttpMethod {
//   Get
// }
