use baml_types::UnresolvedValue;
use internal_baml_diagnostics::Diagnostics;

use crate::ast::{
    ArgumentsList, BlockArgs, Expression, ExpressionBlock,
    FieldType, Identifier, Stmt
};

#[derive(Clone, Debug)]
pub struct Agent {
    pub state: State,
    pub tools: Tools,
    pub prompt: AgentPrompt
}

#[derive(Clone, Debug)]
pub struct State {
    pub r#type: FieldType,
    pub value: Expression,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Tools {
    pub inner: Vec<Tool>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Tool {
    pub identifier: Identifier,
    pub arguments: ArgumentsList,
    pub return_value: ArgumentsList,
    pub body: ExpressionBlock,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Prompt {
    inner: Expression,
}