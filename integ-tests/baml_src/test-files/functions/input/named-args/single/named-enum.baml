enum NamedArgsSingleEnum {
  ONE
  TWO
}

function FnTestNamedArgsSingleEnum(myArg: NamedArgsSingleEnum) -> string {
  client GPT35
  prompt #"
    Print these values back to me:
    {{myArg}}
  "#
}

test FnTestNamedArgsSingleEnum {
  fuctns [FnTestNamedArgsSingleEnum]
  args {
    myArg ONE
  }
}