use anyhow::Result;
use std::{
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
};

use super::{repr::Node, IntermediateRepr, Walker};

mod class;
mod client;
mod r#enum;
mod field_type;
mod function;
mod retry_policy;
mod type_alias;

trait ShallowSignature {
    fn shallow_hash_prefix(&self) -> &'static str;
    fn shallow_interface_hash(&self) -> impl std::hash::Hash;
    fn unsorted_interface_dependencies(&self) -> HashSet<String>;
    fn interface_dependencies(&self) -> Vec<String> {
        let mut deps: Vec<String> = self.unsorted_interface_dependencies().into_iter().collect();
        deps.sort();
        deps
    }
    fn shallow_implementation_hash(&self) -> Option<impl std::hash::Hash>;
    fn unsorted_implementation_dependencies(&self) -> HashSet<String> {
        self.unsorted_interface_dependencies()
    }
    fn implementation_dependencies(&self) -> Vec<String> {
        let mut deps: Vec<String> = self
            .unsorted_implementation_dependencies()
            .into_iter()
            .collect();
        deps.sort();
        deps
    }
}

enum SignatureType {
    AST,
    Function,
    Class,
    Enum,
    TypeAlias,
    Client,
    RetryPolicy,
}

pub struct Signature {
    r#type: SignatureType,
    display_name: String,
    interface_hash: u64,
    implementation_hash: Option<u64>,
    dependencies: Vec<String>,
}

fn recursively_collect_dependencies<'a>(
    name: &str,
    shallow_hash: &'a HashMap<&str, ShallowHash>,
    find_dependencies: fn(&str, &'a HashMap<&str, ShallowHash>) -> Option<&'a Vec<String>>,
) -> Result<Vec<&'a String>> {
    // Recursively collect all dependencies
    let mut seen = HashSet::new();
    let mut queue = vec![name];
    while let Some(name) = queue.pop() {
        let dep_hash = find_dependencies(name, shallow_hash)
            .ok_or(anyhow::anyhow!("Dependency: {} not found", name))?;
        for dep in dep_hash {
            // For recursive dependencies, we want to insert self back into the queue
            // This is why seen starts empty, so we actually insert the dependency
            // back into the queue, when/if we see it again
            if seen.insert(dep) {
                queue.push(dep);
            }
        }
    }

    // Sort dependencies so hasher is deterministic
    let mut dependencies = seen.into_iter().collect::<Vec<_>>();
    dependencies.sort();
    Ok(dependencies)
}

impl Signature {
    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    pub fn dependency_names(&self) -> &Vec<String> {
        &self.dependencies
    }

    pub fn interface_hash(&self) -> u64 {
        self.interface_hash
    }

    pub fn implementation_hash(&self) -> Option<u64> {
        self.implementation_hash
    }

    fn new_function(name: &str, shallow_hash: &HashMap<&str, ShallowHash>) -> Result<Self> {
        Self::new(SignatureType::Function, name, shallow_hash)
    }

    fn new_class(name: &str, shallow_hash: &HashMap<&str, ShallowHash>) -> Result<Self> {
        Self::new(SignatureType::Class, name, shallow_hash)
    }

    fn new_enum(name: &str, shallow_hash: &HashMap<&str, ShallowHash>) -> Result<Self> {
        Self::new(SignatureType::Enum, name, shallow_hash)
    }

    fn new_type_alias(name: &str, shallow_hash: &HashMap<&str, ShallowHash>) -> Result<Self> {
        Self::new(SignatureType::TypeAlias, name, shallow_hash)
    }

    fn new_client(name: &str, shallow_hash: &HashMap<&str, ShallowHash>) -> Result<Self> {
        Self::new(SignatureType::Client, name, shallow_hash)
    }

    fn new_retry_policy(name: &str, shallow_hash: &HashMap<&str, ShallowHash>) -> Result<Self> {
        Self::new(SignatureType::RetryPolicy, name, shallow_hash)
    }

    fn new(
        r#type: SignatureType,
        name: &str,
        shallow_hash: &HashMap<&str, ShallowHash>,
    ) -> Result<Self> {
        let item = shallow_hash
            .get(name)
            .ok_or(anyhow::anyhow!("Item: {} not found", name))?;

        let mut all_dependencies = HashSet::new();

        let interface_hash = {
            let dependencies =
                recursively_collect_dependencies(name, shallow_hash, |name, shallow_hash| {
                    shallow_hash.get(name).map(|h| &h.interface_dependencies)
                })?;

            all_dependencies.extend(dependencies.iter().cloned());

            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            item.interface_hash.hash(&mut hasher);
            for dep in dependencies {
                let dep_hash = shallow_hash
                    .get(dep.as_str())
                    .ok_or(anyhow::anyhow!("Dependency: {} not found", dep))?;
                dep.as_str().hash(&mut hasher);
                dep_hash.interface_hash.hash(&mut hasher);
            }
            hasher.finish()
        };

        let implementation_hash = {
            let dependencies =
                recursively_collect_dependencies(name, shallow_hash, |name, shallow_hash| {
                    shallow_hash
                        .get(name)
                        .map(|h| &h.implementation_dependencies)
                })?;
            all_dependencies.extend(dependencies.iter().cloned());
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            let mut has_implementation_hash = item.implementation_hash.is_some();
            item.implementation_hash.map(|h| h.hash(&mut hasher));
            for dep in dependencies {
                let dep_hash = shallow_hash
                    .get(dep.as_str())
                    .ok_or(anyhow::anyhow!("Dependency: {} not found", dep))?;
                dep_hash.implementation_hash.map(|h| {
                    has_implementation_hash = true;
                    dep.as_str().hash(&mut hasher);
                    h.hash(&mut hasher);
                });
            }
            has_implementation_hash.then_some(hasher.finish())
        };

        Ok(Self {
            r#type,
            display_name: name.to_string(),
            interface_hash,
            implementation_hash,
            dependencies: all_dependencies.into_iter().cloned().collect(),
        })
    }
}

impl<'a, T: ShallowSignature> ShallowSignature for Walker<'a, &'a T> {
    fn shallow_hash_prefix(&self) -> &'static str {
        self.item.shallow_hash_prefix()
    }

    fn shallow_interface_hash(&self) -> impl std::hash::Hash {
        self.item.shallow_interface_hash()
    }

    fn shallow_implementation_hash(&self) -> Option<impl std::hash::Hash> {
        self.item.shallow_implementation_hash()
    }

    fn unsorted_interface_dependencies(&self) -> HashSet<String> {
        self.item.unsorted_interface_dependencies()
    }

    fn unsorted_implementation_dependencies(&self) -> HashSet<String> {
        self.item.unsorted_implementation_dependencies()
    }

    fn interface_dependencies(&self) -> Vec<String> {
        self.item.interface_dependencies()
    }

    fn implementation_dependencies(&self) -> Vec<String> {
        self.item.implementation_dependencies()
    }
}

struct ShallowHash {
    interface_hash: u64,
    implementation_hash: Option<u64>,
    interface_dependencies: Vec<String>,
    implementation_dependencies: Vec<String>,
}

impl ShallowHash {
    fn from_signature(signature: impl ShallowSignature) -> Self {
        Self {
            interface_hash: {
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                signature.shallow_interface_hash().hash(&mut hasher);
                hasher.finish()
            },
            implementation_hash: signature.shallow_implementation_hash().map(|h| {
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                h.hash(&mut hasher);
                hasher.finish()
            }),
            interface_dependencies: signature.interface_dependencies(),
            implementation_dependencies: signature.implementation_dependencies(),
        }
    }
}

pub struct IRSignature {
    pub functions: HashMap<String, Signature>,
    pub classes: HashMap<String, Signature>,
    pub enums: HashMap<String, Signature>,
    pub type_aliases: HashMap<String, Signature>,
    pub clients: HashMap<String, Signature>,

    // Aggregate signature for the AST
    pub ast_signature: Signature,
}

impl IRSignature {
    pub fn new_from_ir(ir: &IntermediateRepr) -> Result<Self> {
        let functions = ir
            .walk_functions()
            .map(|f| (f.name(), ShallowHash::from_signature(f)));
        let classes = ir
            .walk_classes()
            .map(|c| (c.name(), ShallowHash::from_signature(c)));
        let enums = ir
            .walk_enums()
            .map(|e| (e.name(), ShallowHash::from_signature(e)));
        let type_aliases = ir
            .walk_type_aliases()
            .map(|t| (t.name(), ShallowHash::from_signature(t)));
        let clients = ir
            .walk_clients()
            .map(|c| (c.name(), ShallowHash::from_signature(c)));
        let retry_policies = ir
            .walk_retry_policies()
            .map(|r| (r.name(), ShallowHash::from_signature(r)));

        let shallow_hashes: HashMap<_, _> = functions
            .into_iter()
            .chain(classes.into_iter())
            .chain(enums.into_iter())
            .chain(type_aliases.into_iter())
            .chain(clients.into_iter())
            .chain(retry_policies.into_iter())
            .collect();

        let mut functions = ir
            .walk_functions()
            .map(|f| {
                (
                    f.name().to_string(),
                    Signature::new_function(f.name(), &shallow_hashes),
                )
            })
            .collect::<Vec<_>>();
        let mut classes = ir
            .walk_classes()
            .map(|c| {
                (
                    c.name().to_string(),
                    Signature::new_class(c.name(), &shallow_hashes),
                )
            })
            .collect::<Vec<_>>();
        let mut enums = ir
            .walk_enums()
            .map(|e| {
                (
                    e.name().to_string(),
                    Signature::new_enum(e.name(), &shallow_hashes),
                )
            })
            .collect::<Vec<_>>();
        let mut type_aliases = ir
            .walk_type_aliases()
            .map(|t| {
                (
                    t.name().to_string(),
                    Signature::new_type_alias(t.name(), &shallow_hashes),
                )
            })
            .collect::<Vec<_>>();
        let mut clients = ir
            .walk_clients()
            .map(|c| {
                (
                    c.name().to_string(),
                    Signature::new_client(c.name(), &shallow_hashes),
                )
            })
            .collect::<Vec<_>>();
        let mut retry_policies = ir
            .walk_retry_policies()
            .map(|r| {
                (
                    r.name().to_string(),
                    Signature::new_retry_policy(r.name(), &shallow_hashes),
                )
            })
            .collect::<Vec<_>>();

        let functions = {
            functions.sort_by_key(|f| f.0.clone());
            functions
                .into_iter()
                .map(|(name, signature)| signature.map(|s| (name, s)))
                .collect::<Result<HashMap<_, _>>>()?
        };

        let classes = {
            classes.sort_by_key(|c| c.0.clone());
            classes
                .into_iter()
                .map(|(name, signature)| signature.map(|s| (name, s)))
                .collect::<Result<HashMap<_, _>>>()?
        };

        let enums = {
            enums.sort_by_key(|e| e.0.clone());
            enums
                .into_iter()
                .map(|(name, signature)| signature.map(|s| (name, s)))
                .collect::<Result<HashMap<_, _>>>()?
        };

        let type_aliases = {
            type_aliases.sort_by_key(|t| t.0.clone());
            type_aliases
                .into_iter()
                .map(|(name, signature)| signature.map(|s| (name, s)))
                .collect::<Result<HashMap<_, _>>>()?
        };

        let clients = {
            clients.sort_by_key(|c| c.0.clone());
            clients
                .into_iter()
                .map(|(name, signature)| signature.map(|s| (name, s)))
                .collect::<Result<HashMap<_, _>>>()?
        };

        let retry_policies = {
            retry_policies.sort_by_key(|r| r.0.clone());
            retry_policies
                .into_iter()
                .map(|(name, signature)| signature.map(|s| (name, s)))
                .collect::<Result<HashMap<_, _>>>()?
        };

        let ast_signature = {
            let mut has_implementation_hash = false;
            let mut implementation_hash = std::collections::hash_map::DefaultHasher::new();
            let mut interface_hash = std::collections::hash_map::DefaultHasher::new();
            for (name, signature) in &functions {
                name.hash(&mut interface_hash);
                signature.interface_hash.hash(&mut interface_hash);
                signature.implementation_hash.map(|h| {
                    name.hash(&mut implementation_hash);
                    h.hash(&mut implementation_hash);
                });
                has_implementation_hash |= signature.implementation_hash.is_some();
            }
            for (name, signature) in &classes {
                name.hash(&mut interface_hash);
                signature.interface_hash.hash(&mut interface_hash);
                signature.implementation_hash.map(|h| {
                    name.hash(&mut implementation_hash);
                    h.hash(&mut implementation_hash);
                });
                has_implementation_hash |= signature.implementation_hash.is_some();
            }
            for (name, signature) in &enums {
                name.hash(&mut interface_hash);
                signature.interface_hash.hash(&mut interface_hash);
                signature.implementation_hash.map(|h| {
                    name.hash(&mut implementation_hash);
                    h.hash(&mut implementation_hash);
                });
                has_implementation_hash |= signature.implementation_hash.is_some();
            }
            for (name, signature) in &type_aliases {
                name.hash(&mut interface_hash);
                signature.interface_hash.hash(&mut interface_hash);
                signature.implementation_hash.map(|h| {
                    name.hash(&mut implementation_hash);
                    h.hash(&mut implementation_hash);
                });
                has_implementation_hash |= signature.implementation_hash.is_some();
            }
            for (name, signature) in &clients {
                name.hash(&mut interface_hash);
                signature.interface_hash.hash(&mut interface_hash);
                signature.implementation_hash.map(|h| {
                    name.hash(&mut implementation_hash);
                    h.hash(&mut implementation_hash);
                });
                has_implementation_hash |= signature.implementation_hash.is_some();
            }
            for (name, signature) in &retry_policies {
                name.hash(&mut interface_hash);
                signature.interface_hash.hash(&mut interface_hash);
            }

            Signature {
                r#type: SignatureType::AST,
                display_name: "baml_src".into(),
                interface_hash: interface_hash.finish(),
                implementation_hash: has_implementation_hash
                    .then_some(implementation_hash.finish()),
                dependencies: vec![],
            }
        };

        Ok(Self {
            functions,
            classes,
            enums,
            type_aliases,
            clients,
            ast_signature,
        })
    }
}
