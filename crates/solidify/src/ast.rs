//! The syntax tree of a TypeScript module. JavaScript is TypeScript without annotations.

use crate::error::Span;

/// Identifies an expression, a binding or a function: the inference results are keyed by it.
pub type NodeId = usize;

#[derive(Clone, Debug)]
pub struct Module {
    pub body: Vec<Stmt>,
    /// One more than the largest [`NodeId`] in the tree.
    pub node_count: usize,
}

#[derive(Clone, Debug)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
    /// The comments written just before the statement.
    pub comments: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VarKind {
    Var,
    Let,
    Const,
}

#[derive(Clone, Debug)]
pub enum StmtKind {
    Var(VarDecl),
    Func(Function),
    Class(Class),
    Interface(Interface),
    TypeAlias(TypeAlias),
    Enum(EnumDecl),
    Return(Option<Expr>),
    If {
        test: Expr,
        then: Box<Stmt>,
        otherwise: Option<Box<Stmt>>,
    },
    While {
        test: Expr,
        body: Box<Stmt>,
    },
    DoWhile {
        body: Box<Stmt>,
        test: Expr,
    },
    For {
        init: Option<Box<Stmt>>,
        test: Option<Expr>,
        update: Option<Expr>,
        body: Box<Stmt>,
    },
    ForOf {
        kind: VarKind,
        pat: Pattern,
        iter: Expr,
        body: Box<Stmt>,
        /// `for … in` (keys) instead of `for … of` (values).
        is_in: bool,
    },
    Switch {
        disc: Expr,
        cases: Vec<Case>,
    },
    Break(Option<String>),
    Continue(Option<String>),
    Throw(Expr),
    Try {
        block: Block,
        catch: Option<Catch>,
        finally: Option<Block>,
    },
    Block(Block),
    Expr(Expr),
    Empty,
    Import(Import),
    /// `export { a, b as c }`.
    ExportNames(Vec<(String, Option<String>)>),
    ExportDefault(Expr),
    Labeled {
        label: String,
        body: Box<Stmt>,
    },
}

#[derive(Clone, Debug)]
pub struct VarDecl {
    pub kind: VarKind,
    pub decls: Vec<Declarator>,
    pub exported: bool,
}

#[derive(Clone, Debug)]
pub struct Declarator {
    pub pat: Pattern,
    pub ty: Option<TypeAnn>,
    pub init: Option<Expr>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Case {
    /// `None` for `default`.
    pub test: Option<Expr>,
    pub body: Vec<Stmt>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Catch {
    pub param: Option<Pattern>,
    pub body: Block,
}

#[derive(Clone, Debug)]
pub enum Pattern {
    Ident {
        name: String,
        span: Span,
        id: NodeId,
    },
    Array {
        elems: Vec<Option<PatElem>>,
        rest: Option<Box<Pattern>>,
        span: Span,
    },
    Object {
        props: Vec<PatProp>,
        rest: Option<Box<Pattern>>,
        span: Span,
    },
}

impl Pattern {
    pub fn span(&self) -> Span {
        match self {
            Pattern::Ident { span, .. }
            | Pattern::Array { span, .. }
            | Pattern::Object { span, .. } => *span,
        }
    }

    /// The names bound by the pattern, in order.
    pub fn names(&self) -> Vec<(&str, NodeId)> {
        let mut out = Vec::new();
        self.collect_names(&mut out);
        out
    }

    fn collect_names<'a>(&'a self, out: &mut Vec<(&'a str, NodeId)>) {
        match self {
            Pattern::Ident { name, id, .. } => out.push((name, *id)),
            Pattern::Array { elems, rest, .. } => {
                for e in elems.iter().flatten() {
                    e.pat.collect_names(out);
                }
                if let Some(r) = rest {
                    r.collect_names(out);
                }
            }
            Pattern::Object { props, rest, .. } => {
                for p in props {
                    p.value.collect_names(out);
                }
                if let Some(r) = rest {
                    r.collect_names(out);
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct PatElem {
    pub pat: Pattern,
    pub default: Option<Expr>,
}

#[derive(Clone, Debug)]
pub struct PatProp {
    pub key: String,
    pub value: Pattern,
    pub default: Option<Expr>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Function {
    pub name: Option<String>,
    pub type_params: Vec<TypeParam>,
    pub params: Vec<Param>,
    pub ret: Option<TypeAnn>,
    pub body: FnBody,
    pub is_async: bool,
    pub is_arrow: bool,
    pub is_generator: bool,
    pub exported: bool,
    pub span: Span,
    pub id: NodeId,
}

#[derive(Clone, Debug)]
pub enum FnBody {
    Block(Block),
    Expr(Box<Expr>),
}

#[derive(Clone, Debug)]
pub struct Param {
    pub pat: Pattern,
    pub ty: Option<TypeAnn>,
    pub optional: bool,
    pub default: Option<Expr>,
    pub rest: bool,
    /// A parameter property of a constructor (`constructor(private x: number)`).
    pub access: Option<Access>,
    pub readonly: bool,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Access {
    Public,
    Private,
    Protected,
}

#[derive(Clone, Debug)]
pub struct TypeParam {
    pub name: String,
    pub constraint: Option<TypeAnn>,
    pub default: Option<TypeAnn>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Class {
    pub name: String,
    pub type_params: Vec<TypeParam>,
    pub extends: Option<Expr>,
    pub implements: Vec<TypeAnn>,
    pub members: Vec<ClassMember>,
    pub exported: bool,
    pub is_abstract: bool,
    pub span: Span,
    pub id: NodeId,
}

#[derive(Clone, Debug)]
pub enum ClassMember {
    Field(Field),
    Method(Method),
    StaticBlock(Block),
}

#[derive(Clone, Debug)]
pub struct Field {
    pub name: String,
    pub ty: Option<TypeAnn>,
    pub init: Option<Expr>,
    pub is_static: bool,
    pub access: Option<Access>,
    pub readonly: bool,
    pub optional: bool,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Method {
    pub name: String,
    pub kind: MethodKind,
    pub func: Function,
    pub is_static: bool,
    pub access: Option<Access>,
    /// `abstract m(): T;`: a signature the subclasses implement.
    pub is_abstract: bool,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MethodKind {
    Method,
    Get,
    Set,
    Constructor,
}

#[derive(Clone, Debug)]
pub struct Interface {
    pub name: String,
    pub type_params: Vec<TypeParam>,
    pub extends: Vec<TypeAnn>,
    pub members: Vec<TypeMember>,
    pub exported: bool,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum TypeMember {
    Prop {
        name: String,
        ty: TypeAnn,
        optional: bool,
        readonly: bool,
        span: Span,
    },
    Method {
        name: String,
        type_params: Vec<TypeParam>,
        params: Vec<Param>,
        ret: Option<TypeAnn>,
        optional: bool,
        span: Span,
    },
    Index {
        key: TypeAnn,
        value: TypeAnn,
        span: Span,
    },
    Call {
        params: Vec<Param>,
        ret: Option<TypeAnn>,
        span: Span,
    },
}

#[derive(Clone, Debug)]
pub struct TypeAlias {
    pub name: String,
    pub type_params: Vec<TypeParam>,
    pub ty: TypeAnn,
    pub exported: bool,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct EnumDecl {
    pub name: String,
    pub members: Vec<EnumMember>,
    pub exported: bool,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct EnumMember {
    pub name: String,
    pub init: Option<Expr>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Import {
    pub default: Option<String>,
    pub namespace: Option<String>,
    pub names: Vec<(String, Option<String>)>,
    pub from: String,
    pub type_only: bool,
}

#[derive(Clone, Debug)]
pub enum TypeAnn {
    /// `number`, `Point`, `Array<T>`, `A.B`…
    Named {
        name: String,
        args: Vec<TypeAnn>,
        span: Span,
    },
    Array(Box<TypeAnn>, Span),
    Tuple(Vec<TypeAnn>, Span),
    Union(Vec<TypeAnn>, Span),
    Intersection(Vec<TypeAnn>, Span),
    Object(Vec<TypeMember>, Span),
    Func {
        type_params: Vec<TypeParam>,
        params: Vec<Param>,
        ret: Box<TypeAnn>,
        span: Span,
    },
    Lit(Lit, Span),
    TypeOf(String, Span),
    KeyOf(Box<TypeAnn>, Span),
    Indexed(Box<TypeAnn>, Box<TypeAnn>, Span),
    Readonly(Box<TypeAnn>, Span),
}

impl TypeAnn {
    pub fn span(&self) -> Span {
        match self {
            TypeAnn::Named { span, .. }
            | TypeAnn::Array(_, span)
            | TypeAnn::Tuple(_, span)
            | TypeAnn::Union(_, span)
            | TypeAnn::Intersection(_, span)
            | TypeAnn::Object(_, span)
            | TypeAnn::Func { span, .. }
            | TypeAnn::Lit(_, span)
            | TypeAnn::TypeOf(_, span)
            | TypeAnn::KeyOf(_, span)
            | TypeAnn::Indexed(_, _, span)
            | TypeAnn::Readonly(_, span) => *span,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Lit {
    Str(String),
    Num(f64),
    Bool(bool),
    Null,
    Undefined,
}

#[derive(Clone, Debug)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
    pub id: NodeId,
}

#[derive(Clone, Debug)]
pub enum ExprKind {
    Num {
        value: f64,
        raw: String,
        int: bool,
    },
    Str(String),
    Template {
        quasis: Vec<String>,
        exprs: Vec<Expr>,
    },
    Tagged {
        tag: Box<Expr>,
        quasis: Vec<String>,
        exprs: Vec<Expr>,
    },
    Bool(bool),
    Null,
    Undefined,
    Ident(String),
    This,
    Super,
    Array(Vec<ArrayElem>),
    Object(Vec<Prop>),
    Func(Box<Function>),
    Arrow(Box<Function>),
    Class(Box<Class>),
    Unary {
        op: UnaryOp,
        arg: Box<Expr>,
    },
    Update {
        inc: bool,
        prefix: bool,
        arg: Box<Expr>,
    },
    Binary {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Logical {
        op: LogicalOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Assign {
        op: AssignOp,
        target: AssignTarget,
        value: Box<Expr>,
    },
    Cond {
        test: Box<Expr>,
        cons: Box<Expr>,
        alt: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Arg>,
        optional: bool,
        type_args: Vec<TypeAnn>,
    },
    New {
        callee: Box<Expr>,
        args: Vec<Arg>,
        type_args: Vec<TypeAnn>,
    },
    Member {
        obj: Box<Expr>,
        prop: String,
        optional: bool,
    },
    Index {
        obj: Box<Expr>,
        index: Box<Expr>,
        optional: bool,
    },
    As(Box<Expr>, TypeAnn),
    NonNull(Box<Expr>),
    TypeOf(Box<Expr>),
    Void(Box<Expr>),
    Delete(Box<Expr>),
    Await(Box<Expr>),
    Yield(Option<Box<Expr>>),
    Seq(Vec<Expr>),
    Regex {
        pattern: String,
        flags: String,
    },
}

#[derive(Clone, Debug)]
pub enum ArrayElem {
    Expr(Expr),
    Spread(Expr),
    Hole,
}

#[derive(Clone, Debug)]
pub struct Arg {
    pub expr: Expr,
    pub spread: bool,
}

#[derive(Clone, Debug)]
pub struct Prop {
    pub key: PropKey,
    pub value: PropValue,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum PropKey {
    Ident(String),
    Str(String),
    Num(f64),
    Computed(Expr),
}

impl PropKey {
    /// The name of a non-computed key.
    pub fn name(&self) -> Option<String> {
        match self {
            PropKey::Ident(s) | PropKey::Str(s) => Some(s.clone()),
            PropKey::Num(n) => Some(n.to_string()),
            PropKey::Computed(_) => None,
        }
    }
}

#[derive(Clone, Debug)]
pub enum PropValue {
    Expr(Expr),
    /// `{ x }`: the value is the variable named like the key.
    Shorthand(NodeId),
    Spread(Expr),
    Method(Function),
    Get(Function),
    Set(Function),
}

#[derive(Clone, Debug)]
pub enum AssignTarget {
    Expr(Box<Expr>),
    Pat(Pattern),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Pos,
    Not,
    BitNot,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Eq,
    NotEq,
    StrictEq,
    StrictNotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    Shl,
    Shr,
    UShr,
    BitAnd,
    BitOr,
    BitXor,
    In,
    InstanceOf,
}

impl BinOp {
    pub fn is_comparison(self) -> bool {
        matches!(
            self,
            BinOp::Eq
                | BinOp::NotEq
                | BinOp::StrictEq
                | BinOp::StrictNotEq
                | BinOp::Lt
                | BinOp::Gt
                | BinOp::LtEq
                | BinOp::GtEq
        )
    }

    pub fn is_equality(self) -> bool {
        matches!(
            self,
            BinOp::Eq | BinOp::NotEq | BinOp::StrictEq | BinOp::StrictNotEq
        )
    }

    pub fn is_bitwise(self) -> bool {
        matches!(
            self,
            BinOp::Shl | BinOp::Shr | BinOp::UShr | BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor
        )
    }

    pub fn symbol(self) -> &'static str {
        match self {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
            BinOp::Mod => "%",
            BinOp::Pow => "**",
            BinOp::Eq => "==",
            BinOp::NotEq => "!=",
            BinOp::StrictEq => "===",
            BinOp::StrictNotEq => "!==",
            BinOp::Lt => "<",
            BinOp::Gt => ">",
            BinOp::LtEq => "<=",
            BinOp::GtEq => ">=",
            BinOp::Shl => "<<",
            BinOp::Shr => ">>",
            BinOp::UShr => ">>>",
            BinOp::BitAnd => "&",
            BinOp::BitOr => "|",
            BinOp::BitXor => "^",
            BinOp::In => "in",
            BinOp::InstanceOf => "instanceof",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogicalOp {
    And,
    Or,
    Nullish,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssignOp {
    Assign,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Shl,
    Shr,
    UShr,
    BitAnd,
    BitOr,
    BitXor,
    And,
    Or,
    Nullish,
}

impl AssignOp {
    /// The binary operator of a compound assignment.
    pub fn binary(self) -> Option<BinOp> {
        Some(match self {
            AssignOp::Assign | AssignOp::And | AssignOp::Or | AssignOp::Nullish => return None,
            AssignOp::Add => BinOp::Add,
            AssignOp::Sub => BinOp::Sub,
            AssignOp::Mul => BinOp::Mul,
            AssignOp::Div => BinOp::Div,
            AssignOp::Mod => BinOp::Mod,
            AssignOp::Pow => BinOp::Pow,
            AssignOp::Shl => BinOp::Shl,
            AssignOp::Shr => BinOp::Shr,
            AssignOp::UShr => BinOp::UShr,
            AssignOp::BitAnd => BinOp::BitAnd,
            AssignOp::BitOr => BinOp::BitOr,
            AssignOp::BitXor => BinOp::BitXor,
        })
    }
}
