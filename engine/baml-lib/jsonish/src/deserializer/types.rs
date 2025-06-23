use std::collections::HashSet;

use anyhow::Result;
use baml_types::ir_type::TypeGeneric;
use baml_types::type_meta::{base::TypeMeta, stream::TypeMetaStreaming};
use baml_types::{
    BamlMap, BamlMedia, BamlValue, BamlValueWithMeta, Constraint, FieldType, JinjaExpression,
};
use either::Either;
use serde_json::json;
use strsim::jaro;

use super::{
    coercer::ParsingError,
    deserialize_flags::{DeserializerConditions, Flag},
    score::WithScore,
};

/// Type Metadata useful during parsing.
/// We store either regular TypeMeta or TypeMetaStreaming, depending on whether
/// we are performing a final or partial parse.
///
/// The `ParseMeta` type is used temporarily, during parsing, before
/// the resulting BamlValue has its original type restored.
///
/// Going through this monomorphic intermediare makes it easier to write jsonish
/// deserializors without becoming too generic.
#[derive(Clone, Debug)]
pub struct ParseMeta {
    meta: Either<TypeMeta, TypeMetaStreaming>,
}

impl ParseMeta {
    pub fn constraints(&self) -> &Vec<Constraint> {
        match &self.meta {
            Either::Left(TypeMeta { constraints, .. }) => &constraints,
            Either::Right(TypeMetaStreaming { constraints, .. }) => &constraints,
        }
    }
}

/// Representation of BAML values used during parsing.
#[derive(Clone, Debug)]
pub struct BamlValueWithFlags(
    pub BamlValueWithMeta<(TypeGeneric<ParseMeta>, DeserializerConditions)>,
);

// /// Representation of BAML values after parsing in final (non-streaming) mode.
// pub type BamlValueWithFlags = BamlValueWithMeta<(TypeGeneric<TypeMeta>, DeserializerConditions)>;
//
// /// Representation of BAML values after parsing in streaming mode.
// pub type BamlValueWithFlagsStreaming =
//     BamlValueWithMeta<(TypeGeneric<TypeMetaStreaming>, DeserializerConditions)>;

impl BamlValueWithFlags {
    #[cfg(test)]
    pub fn as_list(&self) -> Option<Vec<BamlValueWithFlags>> {
        match &self.0 {
            BamlValueWithMeta::List(v, _) => {
                Some(v.iter().map(|v| BamlValueWithFlags(v.clone())).collect())
            }
            _ => None,
        }
    }

    pub fn r#type(&self) -> &TypeGeneric<ParseMeta> {
        &self.0.meta().0
    }

    pub fn with_target(self, target: TypeGeneric<ParseMeta>) -> Self {
        let mut r = self;
        r.0.meta_mut().0 = target;
        r
    }

    pub fn is_composite(&self) -> bool {
        match self.0 {
            BamlValueWithMeta::Bool(..)
            | BamlValueWithMeta::Enum(..)
            | BamlValueWithMeta::Int(..)
            | BamlValueWithMeta::Float(..)
            | BamlValueWithMeta::String(..)
            | BamlValueWithMeta::Null(..) => false,
            BamlValueWithMeta::List(..)
            | BamlValueWithMeta::Map(..)
            | BamlValueWithMeta::Class(..)
            | BamlValueWithMeta::Media(..) => true,
        }
    }

    pub fn score(&self) -> i32 {
        todo!()
    }

    pub fn conditions(&self) -> &DeserializerConditions {
        &self.0.meta().1
    }

    pub fn add_flag(&mut self, flag: Flag) {
        self.0.meta_mut().1.add_flag(flag);
    }
}

pub trait ParsingErrorToUiJson {
    fn to_ui_json(&self) -> serde_json::Value;
}

impl ParsingErrorToUiJson for ParsingError {
    fn to_ui_json(&self) -> serde_json::Value {
        json!({
            if self.scope.is_empty() {
                "<root>".to_string()
            } else {
                self.scope.join(".")
            }: self.reason,
            "causes": self.causes.iter().map(|c| c.to_ui_json()).collect::<Vec<_>>(),
        })
    }
}

pub trait HasFlags {
    fn flags(&self) -> &DeserializerConditions;

    fn explanation_impl(
        value: &BamlValueWithMeta<Self>,
        scope: Vec<String>,
        expls: &mut Vec<ParsingError>,
    ) where
        Self: Sized,
    {
        let causes = value.flags().explanation();
        let type_name = value.r#type().to_string();
        if !causes.is_empty() {
            expls.push(ParsingError {
                scope: scope.clone(),
                reason: format!("error while parsing {type_name}"),
                causes,
            });
        }
        match value {
            BamlValueWithMeta::String(..) => {}
            BamlValueWithMeta::Int(..) => {}
            BamlValueWithMeta::Float(..) => {}
            BamlValueWithMeta::Bool(..) => {}
            BamlValueWithMeta::Media(..) => {}
            BamlValueWithMeta::List(items, _) => {
                for (i, item) in items.iter().enumerate() {
                    let mut scope = scope.clone();
                    scope.push(format!("parsed:{}", i));
                    Self::explanation_impl(item, scope, expls);
                }
            }
            BamlValueWithMeta::Map(items, _) => {
                for (k, v) in items.iter() {
                    // TODO: Do we need kv-level flags? That would be awkward to support
                    //       in BamlValueWithMeta.
                    // let causes = v_flags.explanation();
                    // if !causes.is_empty() {
                    //     expls.push(ParsingError {
                    //         scope: scope.clone(),
                    //         reason: format!("error while parsing value for map key '{}'", k),
                    //         causes,
                    //     });
                    // }
                    let mut scope = scope.clone();
                    scope.push(format!("parsed:{}", k));
                    Self::explanation_impl(v, scope, expls);
                }
            }
            BamlValueWithMeta::Enum(v, _, f) => {}
            BamlValueWithMeta::Class(class_name, fields, _) => {
                for (k, v) in fields.iter() {
                    let mut scope = scope.clone();
                    scope.push(k.to_string());
                    Self::explanation_impl(v, scope, expls);
                }
            }
            BamlValueWithMeta::Null(..) => {}
        }
    }

    fn explanation_json(value: &BamlValueWithMeta<Self>) -> Vec<serde_json::Value>
    where
        Self: Sized,
    {
        let mut expl = vec![];
        Self::explanation_impl(value, vec!["<root>".to_string()], &mut expl);
        expl.into_iter().map(|e| e.to_ui_json()).collect::<Vec<_>>()
    }
}

impl HasFlags for BamlValueWithFlags {
    fn flags(&self) -> &DeserializerConditions {
        &self.0.meta().1
    }
}

impl<T> HasFlags for (T, DeserializerConditions) {
    fn flags(&self) -> &DeserializerConditions {
        &self.1
    }
}

impl<T: HasFlags> HasFlags for BamlValueWithMeta<T> {
    fn flags(&self) -> &DeserializerConditions {
        &self.meta().flags()
    }
}

impl std::fmt::Display for BamlValueWithFlags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // write!(f, "{} (Score: {}): ", self.r#type(), self.score())?;
        // match self {
        //     BamlValueWithFlags::String(v) => {
        //         write!(f, "{}", v.value)?;
        //         if !v.flags.flags.is_empty() {
        //             write!(f, "\n  {}", v.flags.to_string().replace("\n", "\n  "))?;
        //         }
        //     }
        //     BamlValueWithFlags::Int(v) => {
        //         write!(f, "{}", v.value)?;
        //         if !v.flags.flags.is_empty() {
        //             write!(f, "\n  {}", v.flags.to_string().replace("\n", "\n  "))?;
        //         }
        //     }
        //     BamlValueWithFlags::Float(v) => {
        //         write!(f, "{}", v.value)?;
        //         if !v.flags.flags.is_empty() {
        //             write!(f, "\n  {}", v.flags.to_string().replace("\n", "\n  "))?;
        //         }
        //     }
        //     BamlValueWithFlags::Bool(v) => {
        //         write!(f, "{}", v.value)?;
        //         if !v.flags.flags.is_empty() {
        //             write!(f, "\n  {}", v.flags.to_string().replace("\n", "\n  "))?;
        //         }
        //     }
        //     BamlValueWithFlags::List(flags, _, v) => {
        //         writeln!(f)?;
        //         for (idx, item) in v.iter().enumerate() {
        //             writeln!(f, "  {idx}: {}", item.to_string().replace("\n", "  \n"))?;
        //         }
        //         if !flags.flags.is_empty() {
        //             writeln!(f, "  {}", flags.to_string().replace("\n", "\n  "))?;
        //         }
        //     }
        //     BamlValueWithFlags::Map(_, _, v) => {
        //         writeln!(f)?;
        //         for (key, value) in v {
        //             writeln!(f, "{}: {}", key, value.1)?;
        //         }
        //     }
        //     BamlValueWithFlags::Enum(_, _, v) => {
        //         write!(f, "{}", v.value)?;
        //         if !v.flags.flags.is_empty() {
        //             write!(f, "\n  {}", v.flags.to_string().replace("\n", "\n  "))?;
        //         }
        //     }
        //     BamlValueWithFlags::Class(_, flags, _, v) => {
        //         writeln!(f)?;
        //         for (k, v) in v.iter() {
        //             writeln!(f, "  KV {}", k.to_string().replace("\n", "\n  "))?;
        //             writeln!(f, "  {}", v.to_string().replace("\n", "\n  "))?;
        //         }
        //         if !flags.flags.is_empty() {
        //             writeln!(f, "  {}", flags.to_string().replace("\n", "\n  "))?;
        //         }
        //     }
        //     BamlValueWithFlags::Null(_, flags) => {
        //         write!(f, "null")?;
        //         if !flags.flags.is_empty() {
        //             write!(f, "\n  {}", flags.to_string().replace("\n", "\n  "))?;
        //         }
        //     }
        //     BamlValueWithFlags::Media(_, v) => {
        //         write!(f, "{:#?}", v.value)?;
        //         if !v.flags.flags.is_empty() {
        //             write!(f, "\n  {}", v.flags.to_string().replace("\n", "\n  "))?;
        //         }
        //     }
        // };

        todo!("Display on BamlValueWithFlags needed?");
        Ok(())
    }
}
