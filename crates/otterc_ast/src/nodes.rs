use std::convert::{AsMut, AsRef};
use std::fmt::{Debug, Display};
use std::hash::{Hash, Hasher};

use otterc_ident::Identifier;
use otterc_span::Span;
use otterc_symbol::{Symbol, Visibility};
use otterc_ty::{GenericArg, TyId, TyRef};

/// A node in the AST with an associated span.
#[derive(Debug, Clone)]
pub struct Node<T> {
    value: T,
    span: Span,
}

impl<T> Node<T> {
    pub fn new(value: T, span: impl Into<Span>) -> Self {
        Self {
            value,
            span: span.into(),
        }
    }

    pub fn into_inner(self) -> T {
        self.value
    }

    pub fn into_parts(self) -> (T, Span) {
        (self.value, self.span)
    }

    pub fn parts(&self) -> (&T, Span) {
        (&self.value, self.span)
    }

    pub fn span(&self) -> Span {
        self.span
    }

    pub fn map<U, F>(self, f: F) -> Node<U>
    where
        F: FnOnce(T) -> U,
    {
        Node {
            value: f(self.value),
            span: self.span,
        }
    }
}

impl<T> AsRef<T> for Node<T> {
    fn as_ref(&self) -> &T {
        &self.value
    }
}

impl<T> AsMut<T> for Node<T> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.value
    }
}

impl<T> PartialEq for Node<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T> Eq for Node<T> where T: Eq {}

impl<T> Hash for Node<T>
where
    T: Hash,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

impl<T> Display for Node<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

#[derive(Debug, Clone)]
pub struct Program {
    pub statements: Vec<Node<Stmt>>,
}

impl Program {
    pub fn new(statements: Vec<Node<Stmt>>) -> Self {
        Self { statements }
    }

    /// Get all function definitions in the program
    pub fn functions(&self) -> impl Iterator<Item = &Node<Function>> {
        self.statements.iter().filter_map(|stmt| {
            if let Stmt::Function { func, .. } = stmt.as_ref() {
                Some(func)
            } else {
                None
            }
        })
    }

    /// Count the total number of statements recursively
    pub fn statement_count(&self) -> usize {
        self.statements
            .iter()
            .map(|s| s.as_ref().recursive_count())
            .sum()
    }
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: Identifier,
    pub params: Vec<Node<Param>>,
    pub ret_ty: Option<Node<TyRef>>,
    pub body: Node<Block>,
    pub visibility: Visibility,
    pub ty_id: Option<TyId>,
}

impl Function {
    pub fn new(
        name: Identifier,
        params: Vec<Node<Param>>,
        ret_ty: Option<Node<TyRef>>,
        body: Node<Block>,
    ) -> Self {
        Self {
            name,
            params,
            ret_ty,
            body,
            visibility: Visibility::Private,
            ty_id: None,
        }
    }

    pub fn new_public(
        name: Identifier,
        params: Vec<Node<Param>>,
        ret_ty: Option<Node<TyRef>>,
        body: Node<Block>,
    ) -> Self {
        Self {
            name,
            params,
            ret_ty,
            body,
            visibility: Visibility::Public,
            ty_id: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: Node<Identifier>,
    pub ty: Node<TyRef>,
    pub default: Option<Node<Expr>>,
}

impl Param {
    pub fn new(name: Node<Identifier>, ty: Node<TyRef>, default: Option<Node<Expr>>) -> Self {
        Self { name, ty, default }
    }
}

#[derive(Debug, Clone)]
pub struct Block {
    pub statements: Vec<Node<Stmt>>,
}

impl Block {
    pub fn new(statements: Vec<Node<Stmt>>) -> Self {
        Self { statements }
    }
}

#[derive(Debug, Clone)]
pub struct UseImport {
    pub module: Identifier,
    pub alias: Option<Identifier>,
}

impl UseImport {
    pub fn new(module: Identifier, alias: Option<Identifier>) -> Self {
        Self { module, alias }
    }
}

#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub name: Identifier,
    pub fields: Vec<Node<TyRef>>,
}

impl EnumVariant {
    pub fn new(name: Identifier, fields: Vec<Node<TyRef>>) -> Self {
        Self { name, fields }
    }
}

#[derive(Debug, Clone)]
pub enum Stmt {
    // Variable declarations and assignments
    Let {
        name: Node<Identifier>,
        expr: Node<Expr>,
        ty: Option<Node<TyRef>>,
        visibility: Visibility,
        sym: Option<Symbol>, // Filled in during type resolution
    },
    Assignment {
        name: Node<Identifier>,
        expr: Node<Expr>,
    },

    // Control flow
    If {
        cond: Node<Expr>,
        then_block: Node<Block>,
        elif_blocks: Vec<(Node<Expr>, Node<Block>)>, // Vec<(condition, block)>
        else_block: Option<Node<Block>>,
    },
    For {
        var: Node<Identifier>,
        iterable: Node<Expr>,
        body: Node<Block>,
    },
    While {
        cond: Node<Expr>,
        body: Node<Block>,
    },
    Break,
    Continue,
    Pass,
    Return(Option<Node<Expr>>),

    // Function definitions
    Function {
        func: Node<Function>,
        sym: Option<Symbol>, // Filled in during type resolution
    },

    // Type definitions
    Struct {
        name: Identifier,
        fields: Vec<(Identifier, Node<TyRef>)>,
        methods: Vec<Node<Function>>, // Methods (functions with self parameter)
        visibility: Visibility,
        generics: Vec<GenericArg>, // Generic type parameters
        sym: Option<Symbol>,       // Filled in during type resolution
    },
    Enum {
        name: Identifier,
        variants: Vec<Node<EnumVariant>>,
        visibility: Visibility,
        generics: Vec<GenericArg>,
        sym: Option<Symbol>, // Filled in during type resolution
    },
    TypeAlias {
        name: Identifier,
        target: Node<TyRef>,
        visibility: Visibility,
        generics: Vec<GenericArg>, // Generic type parameters
        sym: Option<Symbol>,       // Filled in during type resolution
    },

    // Expressions as statements
    Expr(Node<Expr>),

    // Module imports
    Use {
        imports: Vec<Node<UseImport>>,
    },

    // Re-exports
    PubUse {
        module: Identifier,
        item: Option<Identifier>,  // None means re-export all public items
        alias: Option<Identifier>, // Optional rename
    },

    // Blocks (for grouping)
    Block(Node<Block>),
}

impl Stmt {
    /// Recursively count statements
    pub fn recursive_count(&self) -> usize {
        match self {
            Stmt::Let { .. }
            | Stmt::Assignment { .. }
            | Stmt::Break
            | Stmt::Continue
            | Stmt::Pass
            | Stmt::Return(_)
            | Stmt::Expr(_)
            | Stmt::Use { .. }
            | Stmt::PubUse { .. }
            | Stmt::Struct { .. }
            | Stmt::Enum { .. }
            | Stmt::TypeAlias { .. } => 1,

            Stmt::If {
                then_block,
                elif_blocks,
                else_block,
                ..
            } => {
                let mut count = 1;
                count += then_block.as_ref().recursive_count();
                for (_, block) in elif_blocks {
                    count += block.as_ref().recursive_count();
                }
                if let Some(block) = else_block {
                    count += block.as_ref().recursive_count();
                }
                count
            }
            Stmt::For { body, .. } | Stmt::While { body, .. } => {
                1 + body.as_ref().recursive_count()
            }
            Stmt::Function { func, .. } => 1 + func.as_ref().body.as_ref().recursive_count(),
            Stmt::Block(block) => block.as_ref().recursive_count(),
        }
    }

    /// Check if statement is pure (has no side effects)
    pub fn is_pure(&self) -> bool {
        matches!(
            self,
            Stmt::Let { .. } | Stmt::Break | Stmt::Continue | Stmt::Pass
        )
    }
}

impl Block {
    /// Recursively count statements
    pub fn recursive_count(&self) -> usize {
        self.statements
            .iter()
            .map(|s| s.as_ref().recursive_count())
            .sum()
    }

    /// Check if block is empty
    pub fn is_empty(&self) -> bool {
        self.statements.is_empty()
    }
}

#[derive(Debug, Clone)]
pub enum Expr {
    // Literals
    Literal(Node<Literal>),

    // Variables and access
    Identifier(Identifier),
    Member {
        object: Box<Node<Expr>>,
        field: Identifier,
    },

    // Function calls
    Call {
        func: Box<Node<Expr>>,
        args: Vec<Node<Expr>>,
    },

    // Binary operations
    Binary {
        op: BinaryOp,
        left: Box<Node<Expr>>,
        right: Box<Node<Expr>>,
    },

    // Unary operations
    Unary {
        op: UnaryOp,
        expr: Box<Node<Expr>>,
    },

    // Control flow expressions
    // NOTE: Not implemented yet
    If {
        cond: Box<Node<Expr>>,
        then_branch: Box<Node<Expr>>,
        else_branch: Box<Node<Expr>>,
    },

    // Match expressions (pattern matching)
    Match {
        value: Box<Node<Expr>>,
        arms: Vec<Node<MatchArm>>,
    },

    // Range expressions
    Range {
        start: Box<Node<Expr>>,
        end: Box<Node<Expr>>,
    },

    // Collection literals
    Array(Vec<Node<Expr>>),
    Dict(Vec<(Node<Expr>, Node<Expr>)>), // Key-value pairs
    ListComprehension {
        element: Box<Node<Expr>>,
        var: Identifier,
        iterable: Box<Node<Expr>>,
        condition: Option<Box<Node<Expr>>>,
    },
    DictComprehension {
        key: Box<Node<Expr>>,
        value: Box<Node<Expr>>,
        var: Identifier,
        iterable: Box<Node<Expr>>,
        condition: Option<Box<Node<Expr>>>,
    },

    // String interpolation
    FString {
        parts: Vec<Node<FStringPart>>,
    },

    // Async operations
    Await(Box<Node<Expr>>),
    Spawn(Box<Node<Expr>>),

    // Struct instantiation
    Struct {
        name: Identifier,
        fields: Vec<(Identifier, Node<Expr>)>, // field name -> value
    },
}

/// Match arm for pattern matching
#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Node<Pattern>,
    pub guard: Option<Node<Expr>>,
    pub body: Node<Block>,
}

/// Pattern for match expressions
#[derive(Debug, Clone)]
pub enum Pattern {
    /// Wildcard pattern (_)
    Wildcard,
    /// Literal pattern (1, true, "hello")
    Literal(Node<Literal>),
    /// Identifier pattern (binds to variable)
    Identifier(Identifier),
    /// Enum variant pattern (Enum.Variant(...))
    EnumVariant {
        enum_name: Identifier,
        variant: Identifier,
        fields: Vec<Node<Pattern>>,
    },
    /// Tuple/struct pattern (Point { x, y })
    Struct {
        name: Identifier,
        fields: Vec<(Identifier, Option<Node<Pattern>>)>, // field name and optional nested pattern
    },
    /// Array/list pattern ([a, b, ..rest])
    Array {
        patterns: Vec<Node<Pattern>>,
        rest: Option<Identifier>, // Variable name for rest pattern
    },
}

#[derive(Debug, Clone)]
pub enum FStringPart {
    Text(String),
    Expr(Node<Expr>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    Gt,
    LtEq,
    GtEq,
    Is,
    IsNot,

    // Logical
    And,
    Or,
}

impl Display for BinaryOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let op_str = match self {
            BinaryOp::Add => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Mul => "*",
            BinaryOp::Div => "/",
            BinaryOp::Mod => "%",
            BinaryOp::Eq => "==",
            BinaryOp::Ne => "!=",
            BinaryOp::Lt => "<",
            BinaryOp::Gt => ">",
            BinaryOp::LtEq => "<=",
            BinaryOp::GtEq => ">=",
            BinaryOp::Is => "is",
            BinaryOp::IsNot => "is not",
            BinaryOp::And => "and",
            BinaryOp::Or => "or",
        };
        write!(f, "{}", op_str)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, Copy)]
pub struct NumberLiteral {
    pub value: f64,
    pub is_float_literal: bool,
}

impl NumberLiteral {
    pub fn new(value: f64, is_float_literal: bool) -> Self {
        Self {
            value,
            is_float_literal,
        }
    }
}

impl PartialEq for NumberLiteral {
    fn eq(&self, other: &Self) -> bool {
        self.is_float_literal == other.is_float_literal
            && self.value.to_bits() == other.value.to_bits()
    }
}

impl Eq for NumberLiteral {}

impl Hash for NumberLiteral {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.value.to_bits().hash(state);
        self.is_float_literal.hash(state);
    }
}

#[derive(Debug, Clone)]
pub enum Literal {
    String(String),
    Number(NumberLiteral),
    Bool(bool),
    None,
    Unit, // Unit literal ()
}

impl PartialEq for Literal {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Literal::String(a), Literal::String(b)) => a == b,
            (Literal::Bool(a), Literal::Bool(b)) => a == b,
            (Literal::Number(a), Literal::Number(b)) => a == b,
            (Literal::None, Literal::None) | (Literal::Unit, Literal::Unit) => true,
            _ => false,
        }
    }
}

impl Eq for Literal {}

impl Hash for Literal {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Literal::String(s) => {
                0u8.hash(state);
                s.hash(state);
            }
            Literal::Number(n) => {
                1u8.hash(state);
                n.hash(state);
            }
            Literal::Bool(b) => {
                2u8.hash(state);
                b.hash(state);
            }
            Literal::None => {
                3u8.hash(state);
            }
            Literal::Unit => {
                4u8.hash(state);
            }
        }
    }
}
