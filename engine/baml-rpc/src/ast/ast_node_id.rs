use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Hash, Deserialize, Serialize, Clone)]
#[serde(into = "String", from = "String")]
pub struct AstNodeId {
    pub type_name: String,
    pub name: String,
    pub interface_hash: u64,
    pub impl_hash: Option<u64>,
}

impl std::fmt::Display for AstNodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}##{}##{}##{}",
            self.type_name,
            self.name,
            self.interface_hash,
            self.impl_hash.unwrap_or(0)
        )
    }
}

impl std::str::FromStr for AstNodeId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts = s.split("##").collect::<Vec<_>>();
        if parts.len() != 4 {
            return Err(anyhow::anyhow!("Invalid unique id: {}", s));
        }
        Ok(AstNodeId {
            type_name: parts[0].to_string(),
            name: parts[1].to_string(),
            interface_hash: match parts[2].parse() {
                Ok(interface_hash) => interface_hash,
                Err(_) => return Err(anyhow::anyhow!("Invalid unique id: {}", s)),
            },
            impl_hash: match parts[3].parse() {
                Ok(0) => None,
                Ok(impl_hash) => Some(impl_hash),
                Err(_) => return Err(anyhow::anyhow!("Invalid unique id: {}", s)),
            },
        })
    }
}

impl From<AstNodeId> for String {
    fn from(value: AstNodeId) -> Self {
        value.to_string()
    }
}

impl From<String> for AstNodeId {
    fn from(value: String) -> Self {
        value
            .parse()
            .expect("Failed to parse AstNodeId from string")
    }
}
