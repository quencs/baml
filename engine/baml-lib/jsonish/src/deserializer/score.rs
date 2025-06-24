use baml_types::{Constraint, ConstraintLevel};

use super::{
    deserialize_flags::{DeserializerConditions, Flag},
    types::BamlValueWithFlags,
};

// Lower is better
pub trait WithScore {
    fn score(&self) -> i32;
}

impl WithScore for BamlValueWithFlags {
    fn score(&self) -> i32 {
        // The full BamlValueWithFlags score is the sum of the
        // node-level scores during a BamlValue traversal.
        self.0.iter().map(|v| v.meta().1.score()).sum()
    }
}

impl WithScore for Flag {
    fn score(&self) -> i32 {
        match self {
            Flag::InferedObject(_) => 0, // Dont penalize for this but instead handle it at the top level
            Flag::OptionalDefaultFromNoValue => 1,
            Flag::DefaultFromNoValue => 100,
            Flag::DefaultButHadValue(_) => 110,
            Flag::ObjectFromFixedJson(_) => 0,
            Flag::ObjectFromMarkdown(s) => *s,
            Flag::DefaultButHadUnparseableValue(_) => 2,
            Flag::ObjectToMap(_) => 1,
            Flag::ObjectToString(_) => 2,
            Flag::ObjectToPrimitive(_) => 2,
            Flag::ExtraKey(_, _) => 1,
            Flag::StrippedNonAlphaNumeric(_) => 3,
            Flag::SubstringMatch(_) => 2,
            Flag::ImpliedKey(_) => 2,
            Flag::JsonToString(_) => 2,
            Flag::SingleToArray => 1,
            // Parsing errors are bad.
            Flag::ArrayItemParseError(x, _) => 1 + (*x as i32),
            Flag::MapKeyParseError(x, _) => 1,
            Flag::MapValueParseError(x, _) => 1,
            // Harmless to drop additional matches
            Flag::FirstMatch(_, _) => 1,
            // No penalty for picking an option from a union
            Flag::UnionMatch(_, _) => 0,
            Flag::StrMatchOneFromMany(values) => {
                values.iter().map(|(_, count)| *count as i32).sum::<i32>()
            }
            Flag::StringToBool(_) => 1,
            Flag::StringToNull(_) => 1,
            Flag::StringToChar(_) => 1,
            Flag::StringToFloat(_) => 1,
            Flag::FloatToInt(_) => 1,
            Flag::NoFields(_) => 1,
            // No scores for contraints
            Flag::ConstraintResults(_) => 0,
            // No scores for incompleteness.
            Flag::Incomplete => 0,
            Flag::Pending => 0,
        }
    }
}

impl WithScore for DeserializerConditions {
    fn score(&self) -> i32 {
        self.flags.iter().map(WithScore::score).sum()
    }
}

impl WithScore for Vec<Flag> {
    fn score(&self) -> i32 {
        self.iter().map(WithScore::score).sum()
    }
}
