// Test for reading a text file as a string input in tests
// This allows you to keep large test inputs in separate files for better organization

function EchoString(text: string) -> string {
  client GPT4o
  prompt #"
    Echo back the following text exactly:
    {{ text }}
  "#
}

// Test using file syntax to load string from external file
test EchoStringFromFile {
  functions [EchoString]
  args {
    text { file "sample-text.txt" }
  }
}

// Test with inline string for comparison
test EchoStringInline {
  functions [EchoString]
  args {
    text "Hello, this is inline text!"
  }
}
