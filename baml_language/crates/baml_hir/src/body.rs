//! Function bodies - either LLM prompts or expression IR.
//!
//! The CST already distinguishes `LLM_FUNCTION_BODY` from `EXPR_FUNCTION_BODY`,
//! so we just need to lower each type appropriately.

use crate::Name;
use la_arena::{Arena, Idx};
use rowan::ast::AstNode;
use std::sync::Arc;

/// The body of a function - determined by CST node type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FunctionBody {
    /// LLM function: has `LLM_FUNCTION_BODY` in CST
    Llm(LlmBody),

    /// Expression function: has `EXPR_FUNCTION_BODY` in CST
    Expr(ExprBody),

    /// Function has no body (error recovery)
    Missing,
}

/// Body of an LLM function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmBody {
    /// The client to use (e.g., "GPT4")
    pub client: Option<Name>,

    /// The prompt template
    pub prompt: Option<PromptTemplate>,
}

/// A prompt template with interpolations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptTemplate {
    /// The raw prompt string (may contain {{ }} interpolations)
    pub text: String,

    /// Parsed interpolation expressions
    pub interpolations: Vec<Interpolation>,
}

/// A {{ var }} interpolation in a prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Interpolation {
    /// Variable name referenced
    pub var_name: Name,

    /// Source offset in the prompt string
    pub offset: u32,
}

/// Body of an expression function (turing-complete).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExprBody {
    /// Expression arena
    pub exprs: Arena<Expr>,

    /// Statement arena
    pub stmts: Arena<Stmt>,

    /// Root expression of the function body (usually a `BLOCK_EXPR`)
    pub root_expr: Option<ExprId>,
}

// IDs for arena indices
pub type ExprId = Idx<Expr>;
pub type StmtId = Idx<Stmt>;
pub type PatId = Idx<Pattern>;

/// Expressions in BAML function bodies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    /// Literal values
    Literal(Literal),

    /// Variable/path reference (e.g., `x`, `GPT4`)
    Path(Name),

    /// If expression
    If {
        condition: ExprId,
        then_branch: ExprId,
        else_branch: Option<ExprId>,
    },

    /// Match expression
    Match {
        scrutinee: ExprId,
        arms: Vec<MatchArm>,
    },

    /// Binary operation
    Binary {
        op: BinaryOp,
        lhs: ExprId,
        rhs: ExprId,
    },

    /// Unary operation
    Unary { op: UnaryOp, expr: ExprId },

    /// Function call: `call_f1()`, `transform(user)`
    Call { callee: ExprId, args: Vec<ExprId> },

    /// Object constructor: `Point { x: 1, y: 2 }`
    Object {
        type_name: Option<Name>,
        fields: Vec<(Name, ExprId)>,
    },

    /// Array constructor: `[1, 2, 3]`
    Array { elements: Vec<ExprId> },

    /// Block expression: `{ stmt1; stmt2; expr }`
    Block {
        stmts: Vec<StmtId>,
        tail_expr: Option<ExprId>,
    },

    /// Missing/error expression
    Missing,
}

/// Statements in BAML function bodies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stmt {
    /// Expression statement: `call_f1();`
    Expr(ExprId),

    /// Let binding: `let x = call_f3();`
    Let {
        pattern: PatId,
        type_annotation: Option<crate::type_ref::TypeRef>,
        initializer: Option<ExprId>,
    },

    /// Return statement: `return "minor";`
    Return(Option<ExprId>),

    /// Missing/error statement
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchArm {
    pub pattern: PatId,
    pub expr: ExprId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pattern {
    /// Literal pattern: `42`, `"hello"`
    Literal(Literal),

    /// Path pattern: `SomeVariant`
    Path(Name),

    /// Binding pattern: `x`, `user`
    Binding(Name),

    /// Wildcard: `_`
    Wildcard,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Literal {
    String(String),
    Int(i64),
    Float(String),
    Bool(bool),
    Null,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Mod,

    // Comparison
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,

    // Logical
    And,
    Or,

    // Bitwise
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Not,
    Neg,
}

impl FunctionBody {
    /// Lower a function body from CST to HIR.
    ///
    /// The CST already tells us if it's LLM or Expr via node type!
    pub fn lower(func_node: &baml_syntax::ast::FunctionDef) -> Arc<FunctionBody> {
        // Check which body type we have
        if let Some(llm_body) = func_node.llm_body() {
            Arc::new(FunctionBody::Llm(Self::lower_llm_body(&llm_body)))
        } else if let Some(expr_body) = func_node.expr_body() {
            Arc::new(FunctionBody::Expr(Self::lower_expr_body(&expr_body)))
        } else {
            Arc::new(FunctionBody::Missing)
        }
    }

    fn lower_llm_body(llm_body: &baml_syntax::ast::LlmFunctionBody) -> LlmBody {
        let mut client = None;
        let mut prompt = None;

        // Extract client from CLIENT_FIELD
        for child in llm_body.syntax().children() {
            if child.kind() == baml_syntax::SyntaxKind::CLIENT_FIELD {
                // CLIENT_FIELD has: KW_CLIENT "client" WORD "GPT4"
                if let Some(client_name) = child
                    .children_with_tokens()
                    .filter_map(baml_syntax::NodeOrToken::into_token)
                    .filter(|t| t.kind() == baml_syntax::SyntaxKind::WORD)
                    .nth(0)
                {
                    client = Some(Name::new(client_name.text()));
                }
            } else if child.kind() == baml_syntax::SyntaxKind::PROMPT_FIELD {
                // PROMPT_FIELD has: WORD "prompt" RAW_STRING_LITERAL (node, not token!)
                // The RAW_STRING_LITERAL node contains the full text including delimiters
                if let Some(prompt_node) = child
                    .children()
                    .find(|n| n.kind() == baml_syntax::SyntaxKind::RAW_STRING_LITERAL)
                {
                    let text = prompt_node.text().to_string();
                    prompt = Some(Self::parse_prompt(&text));
                }
            }
        }

        LlmBody { client, prompt }
    }

    fn parse_prompt(prompt_text: &str) -> PromptTemplate {
        // Strip #"..."# or "..." delimiters
        let prompt_text = prompt_text.trim();
        let content = if prompt_text.starts_with("#\"") && prompt_text.ends_with("\"#") {
            &prompt_text[2..prompt_text.len() - 2]
        } else if prompt_text.starts_with('"') && prompt_text.ends_with('"') {
            &prompt_text[1..prompt_text.len() - 1]
        } else {
            prompt_text
        };

        // Parse {{ var }} interpolations
        let interpolations = Self::parse_interpolations(content);

        PromptTemplate {
            text: content.to_string(),
            interpolations,
        }
    }

    fn parse_interpolations(prompt: &str) -> Vec<Interpolation> {
        let mut interpolations = Vec::new();
        let mut offset = 0;

        while let Some(start) = prompt[offset..].find("{{") {
            let abs_start = offset + start;
            if let Some(end) = prompt[abs_start..].find("}}") {
                let abs_end = abs_start + end;
                let var_text = prompt[abs_start + 2..abs_end].trim();

                #[allow(clippy::cast_possible_truncation)]
                interpolations.push(Interpolation {
                    var_name: Name::new(var_text),
                    offset: abs_start as u32, // Prompt strings are unlikely to exceed 4GB
                });

                offset = abs_end + 2;
            } else {
                break;
            }
        }

        interpolations
    }

    fn lower_expr_body(expr_body: &baml_syntax::ast::ExprFunctionBody) -> ExprBody {
        let mut ctx = LoweringContext::new();

        // The EXPR_FUNCTION_BODY contains a BLOCK_EXPR as its child
        // which contains all the statements and expressions
        let root_expr = expr_body
            .syntax()
            .children()
            .find_map(baml_syntax::ast::BlockExpr::cast)
            .map(|block| ctx.lower_block_expr(&block));

        ExprBody {
            exprs: ctx.exprs,
            stmts: ctx.stmts,
            root_expr,
        }
    }
}

struct LoweringContext {
    exprs: Arena<Expr>,
    stmts: Arena<Stmt>,
    #[allow(dead_code)] // Will be used when pattern lowering is implemented
    patterns: Arena<Pattern>,
}

impl LoweringContext {
    fn new() -> Self {
        Self {
            exprs: Arena::new(),
            stmts: Arena::new(),
            patterns: Arena::new(),
        }
    }

    fn lower_block_expr(&mut self, _block: &baml_syntax::ast::BlockExpr) -> ExprId {
        // TODO: Implement proper block expression lowering
        // For now, just return a placeholder
        self.exprs.alloc(Expr::Missing)
    }
}
