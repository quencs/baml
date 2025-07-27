use internal_baml_diagnostics::{DatamodelError, Diagnostics};

use super::{
    helpers::{parsing_catch_all, Pair},
    parse_identifier::parse_identifier,
    parse_named_args_list::Parse_named_argument_list,
};
use crate::{
    assert_correct_parser,
    ast::*,
    parser::{parse_field::parse_field_type_with_attr, parse_types::parse_field_type},
};

pub (crate) fn parse_agent(pair: Pair<'_>, diagnostics: &mut Diagnostics) -> Agent {
    assert_correct_parser!(pair, Rule::agent);

    Agent {

    }
}