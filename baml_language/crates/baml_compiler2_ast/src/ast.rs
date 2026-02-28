//! Concrete AST structs for BAML — full structural data in memory.
//!
//! Every node carries all its content as owned Rust data (names, type trees,
//! expression trees) with `TextRange` alongside for source mapping. A single
//! `lower_file` function converts the CST to `Vec<Item>`. This isolates all
//! CST `Option` handling in one layer so everything downstream gets clean
//! typed data and can be constructed directly in tests without parsing.

use baml_base::Name;
use la_arena::{Arena, Idx};
use text_size::TextRange;

// ── Attributes ──────────────────────────────────────────────────

/// Raw attribute from CST — not yet validated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawAttribute {
    pub name: Name,
    pub args: Vec<RawAttributeArg>,
    pub span: TextRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawAttributeArg {
    pub key: Option<Name>,
    pub value: String,
    pub span: TextRange,
}

// ── Type Expressions ────────────────────────────────────────────

/// Full recursive type expression — all structural content in memory.
///
/// Corresponds to `TypeRef` in `baml_compiler_hir/src/type_ref.rs` but lives
/// in the AST layer (before any name resolution). CST → TypeExpr conversion
/// happens once during `lower_file` and is never repeated.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeExpr {
    /// Named type path: `User`, `baml.http.Request`
    Path(Vec<Name>),
    /// Primitive types
    Int,
    Float,
    String,
    Bool,
    Null,
    /// Media types
    Media(baml_base::MediaKind),
    /// T?
    Optional(Box<TypeExpr>),
    /// T[]
    List(Box<TypeExpr>),
    /// map<K, V>
    Map {
        key: Box<TypeExpr>,
        value: Box<TypeExpr>,
    },
    /// A | B | C
    Union(Vec<TypeExpr>),
    /// Literal types in unions: "user", 200, true
    StringLiteral(std::string::String),
    IntLiteral(i64),
    FloatLiteral(std::string::String),
    BoolLiteral(bool),
    /// Function type: (params) -> return
    Function {
        params: Vec<FunctionTypeParam>,
        ret: Box<TypeExpr>,
    },
    /// The `unknown` keyword type
    BuiltinUnknown,
    /// The `type` meta-type keyword
    Type,
    /// Error recovery sentinel
    Error,
    /// Unknown/missing type
    Unknown,
}

/// A parameter in a function type expression.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionTypeParam {
    pub name: Option<Name>,
    pub ty: TypeExpr,
}

/// A type expression with its source span — used in item definitions
/// where we need both the type data and the source location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpannedTypeExpr {
    pub expr: TypeExpr,
    pub span: TextRange,
}

// ── Expression Bodies ───────────────────────────────────────────
//
// Full expression/statement arena — modeled after the existing
// `ExprBody` in `body.rs`. All structural content is owned;
// spans are stored in a parallel `AstSourceMap`.

pub type ExprId = Idx<Expr>;
pub type StmtId = Idx<Stmt>;
pub type PatId = Idx<Pattern>;
pub type MatchArmId = Idx<MatchArm>;
pub type TypeAnnotId = Idx<TypeExpr>;

/// Full expression body — owned arena of expressions, statements,
/// and patterns. Modeled after `ExprBody` in `body.rs`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExprBody {
    pub exprs: Arena<Expr>,
    pub stmts: Arena<Stmt>,
    pub patterns: Arena<Pattern>,
    pub match_arms: Arena<MatchArm>,
    /// Type annotations on let bindings etc.
    pub type_annotations: Arena<TypeExpr>,
    /// Root expression of the function body.
    pub root_expr: Option<ExprId>,
}

/// Parallel span storage for an ExprBody — maps arena IDs to source ranges.
/// Separated so semantic queries (type checking) can ignore spans and get
/// Salsa early-cutoff on whitespace changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AstSourceMap {
    pub expr_spans: Arena<TextRange>,
    pub stmt_spans: Arena<TextRange>,
    pub pattern_spans: Arena<TextRange>,
    pub match_arm_spans: Arena<TextRange>,
}

impl AstSourceMap {
    pub fn new() -> Self {
        Self {
            expr_spans: Arena::new(),
            stmt_spans: Arena::new(),
            pattern_spans: Arena::new(),
            match_arm_spans: Arena::new(),
        }
    }
}

impl Default for AstSourceMap {
    fn default() -> Self {
        Self::new()
    }
}

/// Expressions — modeled after `Expr` in `body.rs`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    Literal(Literal),
    /// Path expression: `x`, `user.name`, `Status.Active`
    Path(Vec<Name>),
    If {
        condition: ExprId,
        then_branch: ExprId,
        else_branch: Option<ExprId>,
    },
    Match {
        scrutinee: ExprId,
        scrutinee_type: Option<TypeAnnotId>,
        arms: Vec<MatchArmId>,
    },
    Binary {
        op: BinaryOp,
        lhs: ExprId,
        rhs: ExprId,
    },
    Unary {
        op: UnaryOp,
        expr: ExprId,
    },
    Call {
        callee: ExprId,
        args: Vec<ExprId>,
    },
    Object {
        type_name: Option<Name>,
        fields: Vec<(Name, ExprId)>,
        spreads: Vec<SpreadField>,
    },
    Array {
        elements: Vec<ExprId>,
    },
    Map {
        entries: Vec<(ExprId, ExprId)>,
    },
    Block {
        stmts: Vec<StmtId>,
        tail_expr: Option<ExprId>,
    },
    FieldAccess {
        base: ExprId,
        field: Name,
    },
    Index {
        base: ExprId,
        index: ExprId,
    },
    Missing,
}

/// Statements — modeled after `Stmt` in `body.rs`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stmt {
    Expr(ExprId),
    Let {
        pattern: PatId,
        type_annotation: Option<TypeAnnotId>,
        initializer: Option<ExprId>,
        is_watched: bool,
        origin: LetOrigin,
    },
    While {
        condition: ExprId,
        body: ExprId,
        after: Option<StmtId>,
        origin: LoopOrigin,
    },
    Return(Option<ExprId>),
    Break,
    Continue,
    Assign {
        target: ExprId,
        value: ExprId,
    },
    AssignOp {
        target: ExprId,
        op: AssignOp,
        value: ExprId,
    },
    Assert {
        condition: ExprId,
    },
    Missing,
    HeaderComment {
        name: Name,
        level: usize,
    },
}

/// Patterns — modeled after `Pattern` in `body.rs`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pattern {
    Binding(Name),
    TypedBinding { name: Name, ty: TypeExpr },
    Literal(Literal),
    EnumVariant { enum_name: Name, variant: Name },
    Union(Vec<PatId>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchArm {
    pub pattern: PatId,
    pub guard: Option<ExprId>,
    pub body: ExprId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpreadField {
    pub expr: ExprId,
    pub position: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Literal {
    String(std::string::String),
    Int(i64),
    Float(std::string::String),
    Bool(bool),
    Null,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LetOrigin {
    Source,
    Compiler,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopOrigin {
    While,
    For,
}

/// Binary operators — matches those supported in `body.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    Instanceof,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Not,
    Neg,
}

/// Compound assignment operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
}

// ── Top-Level Items ─────────────────────────────────────────────

/// Top-level item — the output unit of CST → AST lowering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    Function(FunctionDef),
    Class(ClassDef),
    Enum(EnumDef),
    TypeAlias(TypeAliasDef),
    Client(ClientDef),
    Test(TestDef),
    Generator(GeneratorDef),
    TemplateString(TemplateStringDef),
    RetryPolicy(RetryPolicyDef),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionDef {
    pub name: Name,
    pub params: Vec<Param>,
    pub return_type: Option<SpannedTypeExpr>,
    pub body: Option<FunctionBodyDef>,
    pub attributes: Vec<RawAttribute>,
    pub span: TextRange,
    pub name_span: TextRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FunctionBodyDef {
    Llm(LlmBodyDef),
    Expr(ExprBody, AstSourceMap),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmBodyDef {
    pub client: Option<Name>,
    pub prompt: Option<RawPrompt>,
    pub span: TextRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawPrompt {
    pub text: std::string::String,
    /// Interpolation locations within the template.
    pub interpolations: Vec<Interpolation>,
    pub span: TextRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Interpolation {
    pub content: std::string::String,
    pub span: TextRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    pub name: Name,
    pub type_expr: Option<SpannedTypeExpr>,
    pub span: TextRange,
    pub name_span: TextRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassDef {
    pub name: Name,
    pub fields: Vec<FieldDef>,
    pub methods: Vec<FunctionDef>,
    pub attributes: Vec<RawAttribute>,
    pub span: TextRange,
    pub name_span: TextRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldDef {
    pub name: Name,
    pub type_expr: Option<SpannedTypeExpr>,
    pub attributes: Vec<RawAttribute>,
    pub span: TextRange,
    pub name_span: TextRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumDef {
    pub name: Name,
    pub variants: Vec<VariantDef>,
    pub attributes: Vec<RawAttribute>,
    pub span: TextRange,
    pub name_span: TextRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariantDef {
    pub name: Name,
    pub attributes: Vec<RawAttribute>,
    pub span: TextRange,
    pub name_span: TextRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeAliasDef {
    pub name: Name,
    pub type_expr: Option<SpannedTypeExpr>,
    pub span: TextRange,
    pub name_span: TextRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientDef {
    pub name: Name,
    pub config_items: Vec<ConfigItemDef>,
    pub span: TextRange,
    pub name_span: TextRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigItemDef {
    pub key: Name,
    pub value: std::string::String,
    pub span: TextRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestDef {
    pub name: Name,
    pub config_items: Vec<ConfigItemDef>,
    pub span: TextRange,
    pub name_span: TextRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratorDef {
    pub name: Name,
    pub config_items: Vec<ConfigItemDef>,
    pub span: TextRange,
    pub name_span: TextRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateStringDef {
    pub name: Name,
    pub params: Vec<Param>,
    pub body: Option<RawPrompt>,
    pub span: TextRange,
    pub name_span: TextRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetryPolicyDef {
    pub name: Name,
    pub config_items: Vec<ConfigItemDef>,
    pub span: TextRange,
    pub name_span: TextRange,
}
