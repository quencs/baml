use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// #[derive(Serialize, Deserialize, Debug)]
// pub struct Span {
//     pub file_path: String,
//     pub start_line: u32,
//     pub start: u32,
//     pub end_line: u32,
//     pub end: u32,
// }

// #[derive(Serialize, Deserialize, Debug)]
// pub struct CursorPosition {
//     pub file_name: String,
//     pub file_text: String,
//     pub line: u32,
//     pub column: u32,
// }

// Note: the name add_project should match exactly to the
// EventListener.tsx command definitions due to how serde serializes these into json
#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "command", content = "content")]
pub enum FrontendMessage {
    add_project {
        root_path: String,
        files: HashMap<String, String>,
    },
    remove_project {
        root_path: String,
    },
    // set_flashing_regions {
    //     spans: Vec<Span>,
    // },
    select_function {
        root_path: String,
        function_name: String,
    },
    // update_cursor {
    //     cursor: CursorPosition,
    // },
    baml_settings_updated {
        settings: HashMap<String, String>,
    },
    run_test {
        test_name: String,
    },
}

pub struct BamlState {
    pub files: HashMap<String, String>,
    pub tx: tokio::sync::broadcast::Sender<String>,
}

impl BamlState {
    pub fn new() -> Self {
        let (tx, _) = tokio::sync::broadcast::channel(100);
        Self {
            files: HashMap::new(),
            tx,
        }
    }
}
