//! From the typed program to Rust source.
//!
//! Every expression is emitted with a *representation*: owned value, place (a field or an
//! element), reference, string slice… and coerced to what its context wants (`&`, `&mut`,
//! `.clone()`, `.to_string()`, `as usize`). Variables that are read for the last time are
//! moved rather than cloned.

mod calls;
mod exprs;
mod items;
mod runtime;
mod stmts;

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use crate::ast::*;
use crate::error::{Error, Span};
use crate::infer::{Program, Role};
use crate::types::{Flavor, Ty};
pub use runtime::Helper;

/// Options of the generator.
#[derive(Clone, Debug, Default)]
pub struct Options {
    /// The source file name, mentioned in the header.
    pub file_name: Option<String>,
    /// Omit the header comment.
    pub no_header: bool,
}

/// How an emitted expression holds its value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Repr {
    /// A value we own (a temporary, a local by value).
    Owned,
    /// A place we do not own: a field, an element.
    Place,
    /// A shared reference (`&T`).
    Ref,
    /// A mutable reference (`&mut T`).
    RefMut,
    /// A `&str`.
    Str,
    /// A `char` standing for a one-character string.
    Char,
    /// A `usize` standing for an integer number (a length, an index).
    Usize,
}

/// What a context needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Want {
    Owned,
    Ref,
    RefMut,
    /// A `&str`.
    StrRef,
    /// A `&[T]`.
    Slice,
    /// Anything that prints (format arguments).
    Display,
    /// A `usize`.
    Index,
}

/// An emitted expression.
#[derive(Clone, Debug)]
pub(super) struct E {
    pub code: String,
    /// Rust precedence of the outermost operator (higher binds tighter).
    pub prec: u8,
    pub repr: Repr,
    /// The variable read, and the id of this read (for moves on last use).
    pub var: Option<(NodeId, NodeId)>,
}

impl E {
    pub fn new(code: impl Into<String>, prec: u8, repr: Repr) -> E {
        E {
            code: code.into(),
            prec,
            repr,
            var: None,
        }
    }

    pub fn atom(code: impl Into<String>, repr: Repr) -> E {
        E::new(code, PREC_ATOM, repr)
    }

    pub fn owned(code: impl Into<String>, prec: u8) -> E {
        E::new(code, prec, Repr::Owned)
    }
}

pub(super) const PREC_ASSIGN: u8 = 0;
pub(super) const PREC_OR: u8 = 2;
pub(super) const PREC_AND: u8 = 3;
pub(super) const PREC_CMP: u8 = 4;
pub(super) const PREC_BITOR: u8 = 5;
pub(super) const PREC_BITXOR: u8 = 6;
pub(super) const PREC_BITAND: u8 = 7;
pub(super) const PREC_SHIFT: u8 = 8;
pub(super) const PREC_ADD: u8 = 9;
pub(super) const PREC_MUL: u8 = 10;
pub(super) const PREC_CAST: u8 = 11;
pub(super) const PREC_UNARY: u8 = 12;
pub(super) const PREC_POSTFIX: u8 = 13;
pub(super) const PREC_ATOM: u8 = 14;

/// A loop or a `switch`, for `break` and `continue`.
pub(super) enum Target {
    Loop {
        label: Option<String>,
        rust_label: Option<String>,
        /// A `continue` must run the update of a `for`: it breaks out of a labeled block.
        continue_label: Option<String>,
    },
    Switch {
        rust_label: String,
        used: bool,
    },
}

/// Inside the closure made for a `try` block.
pub(super) struct TryCtx {
    /// `return v` inside the block returns `Ok(Some(v))`.
    returns: bool,
}

pub(super) struct FnGen {
    #[allow(dead_code)]
    pub id: NodeId,
    pub returns_result: bool,
    pub ret_ty: Ty,
    pub targets: Vec<Target>,
    pub try_ctx: Vec<TryCtx>,
    pub in_ctor: bool,
    pub this_name: &'static str,
    pub type_params: Vec<String>,
    pub bounds: BTreeMap<String, BTreeSet<&'static str>>,
    pub class: Option<String>,
    /// Closure counter for unique labels.
    pub labels: usize,
}

pub(super) struct Gen<'a> {
    pub m: &'a Module,
    pub p: &'a Program,
    pub out: String,
    pub indent: usize,
    pub helpers: BTreeSet<Helper>,
    pub uses_hashmap: bool,
    pub uses_hashset: bool,
    /// Binding → Rust name.
    pub names: HashMap<NodeId, String>,
    /// Binding → how the variable holds its value.
    pub reprs: HashMap<NodeId, Repr>,
    pub fn_asts: HashMap<NodeId, &'a Function>,
    pub class_asts: HashMap<String, &'a Class>,
    pub fns: Vec<FnGen>,
    /// Rust names declared in each scope, to avoid clashes between different variables.
    pub scopes: Vec<HashSet<String>>,
    /// Names bound by narrowing, innermost last.
    pub narrowed: Vec<(String, Repr)>,
    pub opts: &'a Options,
}

pub(super) type GResult<T> = Result<T, Error>;

/// Generates the Rust source of a typed program.
pub fn generate(module: &Module, prog: &Program, opts: &Options) -> GResult<String> {
    let mut g = Gen {
        m: module,
        p: prog,
        out: String::new(),
        indent: 0,
        helpers: BTreeSet::new(),
        uses_hashmap: false,
        uses_hashset: false,
        names: HashMap::new(),
        reprs: HashMap::new(),
        fn_asts: HashMap::new(),
        class_asts: HashMap::new(),
        fns: Vec::new(),
        scopes: vec![HashSet::new()],
        narrowed: Vec::new(),
        opts,
    };
    collect_asts(&module.body, &mut g.fn_asts, &mut g.class_asts);
    g.module()
}

fn collect_asts<'a>(
    stmts: &'a [Stmt],
    fns: &mut HashMap<NodeId, &'a Function>,
    classes: &mut HashMap<String, &'a Class>,
) {
    for s in stmts {
        collect_stmt(s, fns, classes);
    }
}

fn collect_stmt<'a>(
    s: &'a Stmt,
    fns: &mut HashMap<NodeId, &'a Function>,
    classes: &mut HashMap<String, &'a Class>,
) {
    match &s.kind {
        StmtKind::Func(f) => collect_fn(f, fns, classes),
        StmtKind::Class(c) => {
            classes.insert(c.name.clone(), c);
            for m in &c.members {
                match m {
                    ClassMember::Method(m) => collect_fn(&m.func, fns, classes),
                    ClassMember::Field(f) => {
                        if let Some(i) = &f.init {
                            collect_expr(i, fns, classes);
                        }
                    }
                    ClassMember::StaticBlock(b) => collect_asts(&b.stmts, fns, classes),
                }
            }
        }
        StmtKind::Var(v) => {
            for d in &v.decls {
                if let Some(i) = &d.init {
                    collect_expr(i, fns, classes);
                }
            }
        }
        StmtKind::Return(Some(e))
        | StmtKind::Expr(e)
        | StmtKind::Throw(e)
        | StmtKind::ExportDefault(e) => collect_expr(e, fns, classes),
        StmtKind::If {
            test,
            then,
            otherwise,
        } => {
            collect_expr(test, fns, classes);
            collect_stmt(then, fns, classes);
            if let Some(o) = otherwise {
                collect_stmt(o, fns, classes);
            }
        }
        StmtKind::While { test, body } | StmtKind::DoWhile { body, test } => {
            collect_expr(test, fns, classes);
            collect_stmt(body, fns, classes);
        }
        StmtKind::For {
            init,
            test,
            update,
            body,
        } => {
            if let Some(i) = init {
                collect_stmt(i, fns, classes);
            }
            if let Some(t) = test {
                collect_expr(t, fns, classes);
            }
            if let Some(u) = update {
                collect_expr(u, fns, classes);
            }
            collect_stmt(body, fns, classes);
        }
        StmtKind::ForOf { iter, body, .. } => {
            collect_expr(iter, fns, classes);
            collect_stmt(body, fns, classes);
        }
        StmtKind::Switch { disc, cases } => {
            collect_expr(disc, fns, classes);
            for c in cases {
                if let Some(t) = &c.test {
                    collect_expr(t, fns, classes);
                }
                collect_asts(&c.body, fns, classes);
            }
        }
        StmtKind::Try {
            block,
            catch,
            finally,
        } => {
            collect_asts(&block.stmts, fns, classes);
            if let Some(c) = catch {
                collect_asts(&c.body.stmts, fns, classes);
            }
            if let Some(f) = finally {
                collect_asts(&f.stmts, fns, classes);
            }
        }
        StmtKind::Block(b) => collect_asts(&b.stmts, fns, classes),
        StmtKind::Labeled { body, .. } => collect_stmt(body, fns, classes),
        _ => {}
    }
}

fn collect_fn<'a>(
    f: &'a Function,
    fns: &mut HashMap<NodeId, &'a Function>,
    classes: &mut HashMap<String, &'a Class>,
) {
    fns.insert(f.id, f);
    for p in &f.params {
        if let Some(d) = &p.default {
            collect_expr(d, fns, classes);
        }
    }
    match &f.body {
        FnBody::Block(b) => collect_asts(&b.stmts, fns, classes),
        FnBody::Expr(e) => collect_expr(e, fns, classes),
    }
}

fn collect_expr<'a>(
    e: &'a Expr,
    fns: &mut HashMap<NodeId, &'a Function>,
    classes: &mut HashMap<String, &'a Class>,
) {
    macro_rules! visit {
        ($x:expr) => {
            collect_expr($x, fns, classes)
        };
    }
    match &e.kind {
        ExprKind::Func(f) | ExprKind::Arrow(f) => collect_fn(f, fns, classes),
        ExprKind::Template { exprs, .. } | ExprKind::Seq(exprs) => {
            exprs.iter().for_each(|x| visit!(x))
        }
        ExprKind::Array(elems) => {
            for el in elems {
                match el {
                    ArrayElem::Expr(x) | ArrayElem::Spread(x) => visit!(x),
                    ArrayElem::Hole => {}
                }
            }
        }
        ExprKind::Object(props) => {
            for p in props {
                match &p.value {
                    PropValue::Expr(x) | PropValue::Spread(x) => visit!(x),
                    PropValue::Method(f) | PropValue::Get(f) | PropValue::Set(f) => {
                        collect_fn(f, fns, classes)
                    }
                    PropValue::Shorthand(_) => {}
                }
            }
        }
        ExprKind::Unary { arg, .. } | ExprKind::Update { arg, .. } => visit!(arg),
        ExprKind::Binary { left, right, .. } | ExprKind::Logical { left, right, .. } => {
            visit!(left);
            visit!(right);
        }
        ExprKind::Assign { target, value, .. } => {
            if let AssignTarget::Expr(t) = target {
                visit!(t);
            }
            visit!(value);
        }
        ExprKind::Cond { test, cons, alt } => {
            visit!(test);
            visit!(cons);
            visit!(alt);
        }
        ExprKind::Call { callee, args, .. } | ExprKind::New { callee, args, .. } => {
            visit!(callee);
            for a in args {
                visit!(&a.expr);
            }
        }
        ExprKind::Member { obj, .. } => visit!(obj),
        ExprKind::Index { obj, index, .. } => {
            visit!(obj);
            visit!(index);
        }
        ExprKind::As(x, _)
        | ExprKind::NonNull(x)
        | ExprKind::TypeOf(x)
        | ExprKind::Void(x)
        | ExprKind::Delete(x)
        | ExprKind::Await(x) => visit!(x),
        ExprKind::Yield(Some(x)) => visit!(x),
        ExprKind::Tagged { tag, exprs, .. } => {
            visit!(tag);
            exprs.iter().for_each(|x| visit!(x));
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------------------

const RUST_KEYWORDS: &[&str] = &[
    "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn", "for",
    "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
    "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where",
    "while", "async", "await", "dyn", "abstract", "become", "box", "do", "final", "macro",
    "override", "priv", "typeof", "unsized", "virtual", "yield", "try", "gen",
];

/// `userName` → `user_name`; keywords get a `r#` prefix.
pub fn snake(name: &str) -> String {
    let mut out = String::new();
    let chars: Vec<char> = name.chars().filter(|c| *c != '#').collect();
    for (i, &c) in chars.iter().enumerate() {
        if c == '$' {
            out.push_str("dollar");
            continue;
        }
        if c.is_uppercase() {
            let prev_lower =
                i > 0 && (chars[i - 1].is_lowercase() || chars[i - 1].is_ascii_digit());
            let next_lower = chars.get(i + 1).is_some_and(|n| n.is_lowercase());
            let prev_upper = i > 0 && chars[i - 1].is_uppercase();
            if i > 0 && (prev_lower || (prev_upper && next_lower)) && !out.ends_with('_') {
                out.push('_');
            }
            out.extend(c.to_lowercase());
        } else {
            out.push(c);
        }
    }
    if out.is_empty() {
        out.push('x');
    }
    if out.starts_with(|c: char| c.is_ascii_digit()) {
        out.insert(0, '_');
    }
    escape_keyword(out)
}

/// `user_name` → `USER_NAME`.
pub fn screaming(name: &str) -> String {
    snake(name).trim_start_matches("r#").to_uppercase()
}

/// `point` → `Point`.
pub fn pascal(name: &str) -> String {
    let p = crate::infer::pascal(name);
    if p.is_empty() { "X".into() } else { p }
}

fn escape_keyword(name: String) -> String {
    if name == "self" || name == "Self" || name == "super" || name == "crate" {
        return format!("{name}_");
    }
    if RUST_KEYWORDS.contains(&name.as_str()) {
        format!("r#{name}")
    } else {
        name
    }
}

/// A Rust string literal.
pub fn string_literal(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\0' => out.push_str("\\0"),
            c if (c as u32) < 0x20 || c == '\u{7f}' => {
                out.push_str(&format!("\\u{{{:x}}}", c as u32))
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Text inside a `format!` string: braces doubled.
pub fn format_text(s: &str) -> String {
    let lit = string_literal(s);
    lit[1..lit.len() - 1].replace('{', "{{").replace('}', "}}")
}

impl<'a> Gen<'a> {
    // ----- output -----

    pub(super) fn line(&mut self, s: &str) {
        for _ in 0..self.indent {
            self.out.push_str("    ");
        }
        self.out.push_str(s);
        self.out.push('\n');
    }

    pub(super) fn blank(&mut self) {
        if !self.out.ends_with("\n\n") && !self.out.is_empty() {
            self.out.push('\n');
        }
    }

    /// Runs `f` with a fresh buffer and returns what it wrote.
    pub(super) fn capture(&mut self, f: impl FnOnce(&mut Self) -> GResult<()>) -> GResult<String> {
        let saved = std::mem::take(&mut self.out);
        let result = f(self);
        let text = std::mem::replace(&mut self.out, saved);
        result?;
        Ok(text)
    }

    pub(super) fn helper(&mut self, h: Helper) -> &'static str {
        self.helpers.insert(h);
        match h {
            Helper::StrSlice => "js_slice",
            Helper::StrSubstring => "js_substring",
            Helper::StrCharAt => "js_char_at",
            Helper::StrIndexOf => "js_index_of",
            Helper::StrLastIndexOf => "js_last_index_of",
            Helper::StrCharCodeAt => "js_char_code_at",
            Helper::StrPadStart => "js_pad_start",
            Helper::StrPadEnd => "js_pad_end",
            Helper::StrSplit => "js_split",
            Helper::VecSlice => "js_slice_vec",
            Helper::VecSplice => "js_splice",
            Helper::VecIndexOf => "js_index_of_item",
            Helper::VecLastIndexOf => "js_last_index_of_item",
            Helper::VecAt => "js_at",
            Helper::VecShift => "js_shift",
            Helper::Compare => "js_compare",
            Helper::Ordering => "js_ordering",
            Helper::ParseInt => "js_parse_int",
            Helper::ParseFloat => "js_parse_float",
            Helper::ToNumber => "js_number",
            Helper::Random => "js_random",
            Helper::NumToString => "js_num_to_string",
            Helper::BlockOn => "block_on",
        }
    }

    pub(super) fn err<T>(&self, msg: impl Into<String>, span: Span) -> GResult<T> {
        Err(Error::new(msg, span))
    }

    pub(super) fn fn_ctx(&self) -> &FnGen {
        self.fns.last().expect("function")
    }

    pub(super) fn fn_ctx_mut(&mut self) -> &mut FnGen {
        self.fns.last_mut().expect("function")
    }

    // ----- types -----

    pub(super) fn resolve(&self, ty: &Ty) -> Ty {
        self.p.types.resolve(ty)
    }

    pub(super) fn ty_of(&self, e: &Expr) -> Ty {
        self.resolve(&self.p.ty(e.id))
    }

    pub(super) fn is_copy(&self, ty: &Ty) -> bool {
        self.p.is_copy(ty)
    }

    pub(super) fn is_int(&self, ty: &Ty) -> bool {
        matches!(self.resolve(ty), Ty::Num(f) if self.p.types.flavor(f) != Flavor::Float)
    }

    pub(super) fn is_float(&self, ty: &Ty) -> bool {
        matches!(self.resolve(ty), Ty::Num(f) if self.p.types.flavor(f) == Flavor::Float)
    }

    pub(super) fn is_str(&self, ty: &Ty) -> bool {
        matches!(self.resolve(ty), Ty::Str)
    }

    /// A struct that contains itself: held in a `Box` inside `Option`.
    pub(super) fn is_boxed(&self, ty: &Ty) -> bool {
        matches!(self.resolve(ty), Ty::Named(n) if self.p.boxed.contains(&n))
    }

    pub(super) fn is_type_param(&self, name: &str) -> bool {
        self.fns
            .iter()
            .rev()
            .any(|f| f.type_params.iter().any(|t| t == name))
    }

    /// The Rust type of an owned value.
    pub(super) fn rust_ty(&mut self, ty: &Ty, span: Span) -> GResult<String> {
        Ok(match self.resolve(ty) {
            Ty::Var(_) => {
                return self.err("a type could not be inferred here: add an annotation", span);
            }
            Ty::Num(f) => match self.p.types.flavor(f) {
                Flavor::Float => "f64".into(),
                _ => "i64".into(),
            },
            Ty::Str => "String".into(),
            Ty::Bool => "bool".into(),
            Ty::Void | Ty::Null => "()".into(),
            Ty::Never => "()".into(),
            Ty::Option(t) => {
                let inner = self.rust_ty(&t, span)?;
                if self.is_boxed(&t) {
                    format!("Option<Box<{inner}>>")
                } else {
                    format!("Option<{inner}>")
                }
            }
            Ty::Array(t) => format!("Vec<{}>", self.rust_ty(&t, span)?),
            Ty::Tuple(ts) => {
                let mut parts = Vec::new();
                for t in &ts {
                    parts.push(self.rust_ty(t, span)?);
                }
                if parts.len() == 1 {
                    format!("({},)", parts[0])
                } else {
                    format!("({})", parts.join(", "))
                }
            }
            Ty::Map(k, v) => {
                self.uses_hashmap = true;
                format!(
                    "HashMap<{}, {}>",
                    self.rust_ty(&k, span)?,
                    self.rust_ty(&v, span)?
                )
            }
            Ty::Set(t) => {
                self.uses_hashset = true;
                format!("HashSet<{}>", self.rust_ty(&t, span)?)
            }
            Ty::Named(n) => {
                if n == "Error" {
                    "String".into()
                } else if self.is_type_param(&n) {
                    n
                } else {
                    pascal(&n)
                }
            }
            Ty::Func(f) => {
                let mut params = Vec::new();
                for t in &f.params {
                    params.push(self.rust_ty(t, span)?);
                }
                let ret = self.rust_ty(&f.ret, span)?;
                let ret = if ret == "()" {
                    String::new()
                } else {
                    format!(" -> {ret}")
                };
                format!("Box<dyn Fn({}){ret}>", params.join(", "))
            }
            Ty::Any => return self.err("`any` cannot be translated: give this value a type", span),
        })
    }

    /// The type of a returned closure.
    pub(super) fn fn_return_ty(&mut self, ty: &Ty, span: Span) -> GResult<String> {
        if let Ty::Func(f) = self.resolve(ty) {
            let mut params = Vec::new();
            for t in &f.params {
                params.push(self.rust_ty(t, span)?);
            }
            let ret = self.rust_ty(&f.ret, span)?;
            let ret = if ret == "()" {
                String::new()
            } else {
                format!(" -> {ret}")
            };
            return Ok(format!("impl FnMut({}){ret}", params.join(", ")));
        }
        self.rust_ty(ty, span)
    }

    /// The type of a closure parameter or of a `impl Fn` parameter.
    pub(super) fn fn_param_ty(&mut self, ty: &Ty, span: Span) -> GResult<String> {
        if let Ty::Func(f) = self.resolve(ty) {
            let mut params = Vec::new();
            for t in &f.params {
                params.push(self.rust_ty(t, span)?);
            }
            let ret = self.rust_ty(&f.ret, span)?;
            let ret = if ret == "()" {
                String::new()
            } else {
                format!(" -> {ret}")
            };
            return Ok(format!("impl Fn({}){ret}", params.join(", ")));
        }
        self.rust_ty(ty, span)
    }

    /// How a parameter of this type and usage is passed.
    pub(super) fn param_mode(
        &self,
        ty: &Ty,
        stored: bool,
        reassigned: bool,
        mutated: bool,
        type_params: &[String],
    ) -> Mode {
        let ty = self.resolve(ty);
        if let Ty::Func(_) = ty {
            return if stored { Mode::Value } else { Mode::FnImpl };
        }
        if self.is_copy(&ty) {
            return Mode::Value;
        }
        if stored || reassigned {
            return Mode::Value;
        }
        if let Ty::Named(n) = &ty
            && (self.is_type_param(n) || type_params.contains(n))
        {
            return Mode::Value;
        }
        if mutated {
            return Mode::RefMut;
        }
        match ty {
            Ty::Str => Mode::StrRef,
            Ty::Array(_) => Mode::Slice,
            Ty::Option(inner) if !self.is_copy(&inner) => Mode::OptRef,
            _ => Mode::Ref,
        }
    }

    /// The Rust type of a parameter passed in `mode`.
    pub(super) fn mode_ty(&mut self, ty: &Ty, mode: Mode, span: Span) -> GResult<String> {
        Ok(match mode {
            Mode::Value => self.rust_ty(ty, span)?,
            Mode::FnImpl => self.fn_param_ty(ty, span)?,
            Mode::Ref => format!("&{}", self.rust_ty(ty, span)?),
            Mode::RefMut => format!("&mut {}", self.rust_ty(ty, span)?),
            Mode::StrRef => "&str".into(),
            Mode::Slice => match self.resolve(ty) {
                Ty::Array(t) => format!("&[{}]", self.rust_ty(&t, span)?),
                _ => format!("&{}", self.rust_ty(ty, span)?),
            },
            Mode::OptRef => match self.resolve(ty) {
                Ty::Option(inner) if self.is_str(&inner) => "Option<&str>".into(),
                Ty::Option(inner) => format!("Option<&{}>", self.rust_ty(&inner, span)?),
                _ => self.rust_ty(ty, span)?,
            },
        })
    }

    /// The representation of a variable bound as a parameter in `mode`.
    pub(super) fn mode_repr(mode: Mode) -> Repr {
        match mode {
            Mode::Value | Mode::FnImpl | Mode::OptRef => Repr::Owned,
            Mode::Ref | Mode::Slice => Repr::Ref,
            Mode::RefMut => Repr::RefMut,
            Mode::StrRef => Repr::Str,
        }
    }

    /// `{}` or `{:?}` for a value of this type in a format string.
    pub(super) fn fmt_spec(&self, ty: &Ty) -> &'static str {
        match self.resolve(ty) {
            Ty::Num(_) | Ty::Str | Ty::Bool => "{}",
            Ty::Named(n) => {
                let prints = n == "Error"
                    || self.p.is_enum(&n)
                    || self
                        .p
                        .struct_info(&n)
                        .is_some_and(|s| s.method("toString", MethodKind::Method).is_some());
                if prints { "{}" } else { "{:?}" }
            }
            _ => "{:?}",
        }
    }

    // ----- names -----

    /// Declares the Rust name of a binding, avoiding clashes in the current scope.
    pub(super) fn bind(&mut self, id: NodeId, repr: Repr) -> String {
        let var = self.p.var(id);
        let base = match var.role {
            Role::TopLevel if var.const_literal => screaming(&var.name),
            _ => snake(&var.name),
        };
        let mut name = base.clone();
        let mut n = 2;
        while self.scopes.last().is_some_and(|s| s.contains(&name)) && !self.same_binding(id, &name)
        {
            name = format!("{base}_{n}");
            n += 1;
        }
        if let Some(s) = self.scopes.last_mut() {
            s.insert(name.clone());
        }
        self.names.insert(id, name.clone());
        self.reprs.insert(id, repr);
        name
    }

    fn same_binding(&self, id: NodeId, name: &str) -> bool {
        self.names.get(&id).is_some_and(|n| n == name)
    }

    pub(super) fn name(&self, id: NodeId) -> String {
        match self.names.get(&id) {
            Some(n) => n.clone(),
            None => snake(&self.p.var(id).name),
        }
    }

    pub(super) fn repr_of(&self, id: NodeId) -> Repr {
        self.reprs.get(&id).copied().unwrap_or(Repr::Owned)
    }

    pub(super) fn push_scope(&mut self) {
        self.scopes.push(HashSet::new());
    }

    pub(super) fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    /// A unique label for loops and blocks.
    pub(super) fn fresh_label(&mut self, base: &str) -> String {
        let ctx = self.fn_ctx_mut();
        ctx.labels += 1;
        if ctx.labels == 1 {
            format!("'{base}")
        } else {
            format!("'{base}{}", ctx.labels)
        }
    }

    // ----- coercions -----

    pub(super) fn paren(e: &E, min_prec: u8) -> String {
        if e.prec < min_prec {
            format!("({})", e.code)
        } else {
            e.code.clone()
        }
    }

    /// Whether this read of a variable is its last: the value can be moved.
    pub(super) fn can_move(&self, e: &E) -> bool {
        match e.var {
            Some((binding, use_id)) => {
                let v = self.p.var(binding);
                self.p.last_use.contains(&use_id)
                    && !v.captured
                    && (matches!(v.role, Role::Local | Role::Param | Role::Loop | Role::Catch)
                        || (v.role == Role::TopLevel && !v.const_literal))
                    && self.repr_of(binding) == Repr::Owned
            }
            None => false,
        }
    }

    /// `e` as the context wants it, for a value of type `ty`.
    pub(super) fn coerce(&mut self, e: E, ty: &Ty, want: Want) -> String {
        let ty = self.resolve(ty);
        let copy = self.is_copy(&ty);
        match want {
            Want::Owned => match e.repr {
                Repr::Owned => e.code,
                Repr::Place => {
                    if copy {
                        e.code
                    } else {
                        self.bound_needed(&ty, "Clone");
                        format!("{}.clone()", Self::paren(&e, PREC_POSTFIX))
                    }
                }
                Repr::Ref | Repr::RefMut => {
                    if copy {
                        format!("*{}", Self::paren(&e, PREC_UNARY))
                    } else {
                        self.bound_needed(&ty, "Clone");
                        format!("{}.clone()", Self::paren(&e, PREC_POSTFIX))
                    }
                }
                Repr::Str | Repr::Char => format!("{}.to_string()", Self::paren(&e, PREC_POSTFIX)),
                Repr::Usize => {
                    if self.is_float(&ty) {
                        format!("{} as f64", Self::paren(&e, PREC_CAST))
                    } else {
                        format!("{} as i64", Self::paren(&e, PREC_CAST))
                    }
                }
            },
            Want::Ref => match e.repr {
                Repr::Owned | Repr::Place | Repr::Usize => {
                    format!("&{}", Self::paren(&e, PREC_UNARY))
                }
                Repr::Ref | Repr::RefMut => e.code,
                Repr::Str => e.code,
                Repr::Char => format!("&{}.to_string()", Self::paren(&e, PREC_POSTFIX)),
            },
            Want::RefMut => match e.repr {
                Repr::Owned | Repr::Place => format!("&mut {}", Self::paren(&e, PREC_UNARY)),
                Repr::RefMut => e.code,
                Repr::Ref => format!("&mut {}", Self::paren(&e, PREC_UNARY)),
                Repr::Str | Repr::Char | Repr::Usize => {
                    format!("&mut {}", Self::paren(&e, PREC_UNARY))
                }
            },
            Want::StrRef => match e.repr {
                Repr::Owned | Repr::Place => {
                    if e.prec >= PREC_POSTFIX {
                        format!("&{}", e.code)
                    } else {
                        format!("&({})", e.code)
                    }
                }
                Repr::Ref | Repr::RefMut => {
                    if self.is_copy(&ty) {
                        e.code
                    } else {
                        format!("{}.as_str()", Self::paren(&e, PREC_POSTFIX))
                    }
                }
                Repr::Str => e.code,
                Repr::Char => format!("&{}.to_string()", Self::paren(&e, PREC_POSTFIX)),
                Repr::Usize => e.code,
            },
            Want::Slice => match e.repr {
                Repr::Owned | Repr::Place => format!("&{}", Self::paren(&e, PREC_UNARY)),
                Repr::Ref | Repr::RefMut => e.code,
                _ => e.code,
            },
            Want::Display => match e.repr {
                Repr::Ref | Repr::RefMut => {
                    if copy && e.prec < PREC_POSTFIX {
                        format!("*{}", Self::paren(&e, PREC_UNARY))
                    } else {
                        e.code
                    }
                }
                _ => e.code,
            },
            Want::Index => match e.repr {
                Repr::Usize => e.code,
                Repr::Ref | Repr::RefMut => format!("*{} as usize", Self::paren(&e, PREC_UNARY)),
                _ => {
                    if is_int_literal(&e.code) {
                        e.code
                    } else {
                        format!("{} as usize", Self::paren(&e, PREC_CAST))
                    }
                }
            },
        }
    }

    /// Whether a binding holds a reference that is rebound by assignments.
    pub(super) fn is_alias(&self, id: NodeId) -> bool {
        matches!(self.repr_of(id), Repr::Ref | Repr::RefMut) && self.p.var(id).role != Role::Param
    }

    /// `e`, moved when this is its last use, cloned otherwise.
    pub(super) fn owned(&mut self, e: E, ty: &Ty) -> String {
        if e.repr == Repr::Owned && e.var.is_some() && !self.is_copy(ty) && !self.can_move(&e) {
            self.bound_needed(ty, "Clone");
            return format!("{}.clone()", Self::paren(&e, PREC_POSTFIX));
        }
        self.coerce(e, ty, Want::Owned)
    }

    /// An owned value of type `from`, wrapped in `Some` when `to` is an `Option` and the
    /// value is not.
    pub(super) fn owned_as(&mut self, e: E, from: &Ty, to: &Ty) -> String {
        let (from_r, to_r) = (self.resolve(from), self.resolve(to));
        if matches!(from_r, Ty::Null) {
            return "None".into();
        }
        let code = self.owned(e, &from_r);
        if let Ty::Option(inner) = &to_r
            && !matches!(from_r, Ty::Option(_))
        {
            if self.is_boxed(inner) {
                return format!("Some(Box::new({code}))");
            }
            return format!("Some({code})");
        }
        code
    }

    /// Records that a type parameter needs a trait.
    pub(super) fn bound_needed(&mut self, ty: &Ty, bound: &'static str) {
        if let Ty::Named(n) = self.resolve(ty)
            && self.is_type_param(&n)
        {
            self.fn_ctx_mut().bounds.entry(n).or_default().insert(bound);
        }
    }
}

/// How a parameter is passed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Mode {
    Value,
    Ref,
    RefMut,
    StrRef,
    Slice,
    OptRef,
    FnImpl,
}

pub(super) fn is_int_literal(code: &str) -> bool {
    !code.is_empty() && code.chars().all(|c| c.is_ascii_digit())
}
