client<llm> BaseClient {
  provider openai
  options {
    model "gpt-4"
  }
}

client<llm> InvalidCompositeFields {
  provider fallback
  options {
    strategy [BaseClient]
    http {
      invalid_field_name 5000  // Unrecognized field
    }
  }
}

// error: Unrecognized field 'invalid_field_name' in http configuration block. Composite clients (fallback/round-robin) support: connect_timeout_ms, request_timeout_ms, time_to_first_token_timeout_ms, idle_timeout_ms, total_timeout_ms
//   -->  client/http_config_composite_wrong_field.baml:12
//    | 
// 11 |     strategy [BaseClient]
// 12 |     http {
// 13 |       invalid_field_name 5000  // Unrecognized field
// 14 |     }
//    | 
