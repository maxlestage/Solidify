//! From the typed program to Rust source.
//!
//! Every expression is emitted with a *representation*: owned value, place (a field or an
//! element), reference, string slice… and coerced to what its context wants (`&`, `&mut`,
//! `.clone()`, `.to_string()`, `as usize`). Variables that are read for the last time are
//! moved rather than cloned.

mod calls;
mod dynamic;
mod exprs;
mod items;
mod runtime;
mod stmts;

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use crate::ast::*;
use crate::error::{Error, Span};
use crate::infer::{Callee, FnInfo, Program, Role};
use crate::types::{Flavor, Ty};
pub use runtime::Helper;
pub use runtime::regex_rt;

/// Options of the generator.
#[derive(Clone, Debug, Default)]
pub struct Options {
    /// The source file name, mentioned in the header.
    pub file_name: Option<String>,
    /// Omit the header comment.
    pub no_header: bool,
    /// The files of the program: the imported ones become modules.
    pub files: Vec<crate::project::FileInfo>,
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
    /// An `Option<&T>` (or `Option<&str>`): a nullable value borrowed from elsewhere.
    OptRef,
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
    /// Inside a generator: the type of the values yielded.
    pub yield_ty: Option<Ty>,
    pub targets: Vec<Target>,
    pub try_ctx: Vec<TryCtx>,
    pub in_ctor: bool,
    pub this_name: &'static str,
    pub type_params: Vec<String>,
    pub bounds: BTreeMap<String, BTreeSet<&'static str>>,
    pub class: Option<String>,
    /// Closure counter for unique labels.
    pub labels: usize,
    /// `this` is the handle of a shared object (`Shared<Self>`).
    pub handle_this: bool,
}

pub(super) struct Gen<'a> {
    pub m: &'a Module,
    pub p: &'a Program,
    pub out: String,
    pub indent: usize,
    pub helpers: BTreeSet<Helper>,
    pub uses_hashmap: bool,
    pub uses_hashset: bool,
    /// Maps and sets are iterated or shown: they keep the insertion order (`JsMap`, `JsSet`).
    pub ordered: bool,
    /// Structs and enums shown by `console.log` or converted to strings: they get a
    /// `JsInspect` implementation.
    pub inspected: BTreeSet<String>,
    /// The structs, enums and unions that `JSON.parse` builds: they get a `JsFromJson`.
    pub json_types: BTreeSet<String>,
    /// The expressions being written before their conversion to or from a `JsValue`.
    pub casting: HashSet<NodeId>,
    /// Shared classes whose constructor returns the handle.
    pub ctor_gives_handle: HashSet<String>,
    /// Fields read (or written: `true`) on union values: accessors on the enum.
    pub union_accessors: BTreeSet<(String, String, bool)>,
    /// Unions whose `typeof` is taken.
    pub union_typeof: BTreeSet<String>,
    /// The class whose `impl` is being written (methods inherited from a base class are
    /// written again for each subclass).
    pub current_class: Option<String>,
    /// Fields written through a `dyn` value: (class, field).
    pub dyn_setters: BTreeSet<(String, String)>,
    /// `x instanceof Sub` on a `dyn Base` value: (base, sub, mutable).
    pub instance_tests: BTreeSet<(String, String, bool)>,
    /// `super.m()` helpers to write: class → (declaring class, method).
    pub super_helpers: BTreeMap<String, BTreeSet<(String, String)>>,
    /// Variables held in an `Rc<RefCell<T>>` (shared with closures that outlive the scope).
    pub shared: HashSet<NodeId>,
    pub uses_rc: bool,
    /// Top-level variables used inside functions: statics (`true`: changed, in a `Mutex`).
    pub statics: HashMap<NodeId, bool>,
    pub uses_lazy: bool,
    pub uses_mutex: bool,
    /// The next variable read is written through: borrow a shared one mutably.
    pub want_mut_root: bool,
    /// Parameters to wrap in `Rc<RefCell<T>>` at the start of the body.
    pub param_rebinds: Vec<String>,
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
    /// Writing the items of an imported file, inside its module.
    pub in_module: bool,
    /// The call that is the whole current statement: its result is not used.
    pub stmt_call: Option<NodeId>,
    /// The type parameters of the generic struct whose items are being written.
    pub class_params: Vec<String>,
    /// The traits its methods need of them.
    pub class_bounds: BTreeMap<String, BTreeSet<&'static str>>,
    /// A type parameter is shown (`JsInspect`): every type of the program may be its
    /// argument, and is given the trait.
    pub generic_inspect: bool,
    /// Writing the methods a class inherits from a generic base (`extends Stack<number>`):
    /// the base's parameters as the class gives them.
    pub class_subst: HashMap<String, Ty>,
    /// The array being shown has `undefined` (not `null`) for its missing elements.
    pub elems_undefined: bool,
    /// Plain variables may be written inside a format string (`{x}`): no other value of
    /// the same call changes them.
    pub inline_names: bool,
    /// Fields written through a constrained type parameter: (constraint, field).
    pub bound_setters: BTreeSet<(String, String)>,
    /// The bounds of the generic items found by the previous pass (`fn:<id>`,
    /// `struct:<name>`), and those found by this one.
    pub known_bounds: HashMap<String, Bounds>,
    pub found_bounds: HashMap<String, Bounds>,
}

pub(super) type GResult<T> = Result<T, Error>;

/// Generates the Rust source of a typed program.
pub fn generate(module: &Module, prog: &Program, opts: &Options) -> GResult<String> {
    // What generic functions and structs need of their type parameters depends on what
    // the generic items they use need: written again until nothing changes.
    let mut known: HashMap<String, Bounds> = HashMap::new();
    let mut text = String::new();
    for _ in 0..8 {
        let (out, found) = generate_with(module, prog, opts, &known)?;
        text = out;
        if found == known {
            break;
        }
        known = found;
    }
    Ok(text)
}

/// The traits each type parameter of an item needs.
pub(super) type Bounds = BTreeMap<String, BTreeSet<&'static str>>;

fn generate_with(
    module: &Module,
    prog: &Program,
    opts: &Options,
    known: &HashMap<String, Bounds>,
) -> GResult<(String, HashMap<String, Bounds>)> {
    let mut g = Gen {
        known_bounds: known.clone(),
        bound_setters: BTreeSet::new(),
        elems_undefined: false,
        inline_names: true,
        class_subst: HashMap::new(),
        found_bounds: HashMap::new(),
        m: module,
        p: prog,
        out: String::new(),
        indent: 0,
        helpers: BTreeSet::new(),
        uses_hashmap: false,
        uses_hashset: false,
        ordered: false,
        in_module: false,
        stmt_call: None,
        class_params: Vec::new(),
        class_bounds: BTreeMap::new(),
        generic_inspect: false,
        inspected: BTreeSet::new(),
        json_types: BTreeSet::new(),
        casting: HashSet::new(),
        ctor_gives_handle: HashSet::new(),
        union_accessors: BTreeSet::new(),
        union_typeof: BTreeSet::new(),
        current_class: None,
        dyn_setters: BTreeSet::new(),
        instance_tests: BTreeSet::new(),
        super_helpers: BTreeMap::new(),
        shared: HashSet::new(),
        uses_rc: false,
        statics: HashMap::new(),
        uses_lazy: false,
        uses_mutex: false,
        want_mut_root: false,
        param_rebinds: Vec::new(),
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
    g.ordered = g.needs_ordered();
    let text = g.module()?;
    Ok((text, g.found_bounds))
}

/// Visits every statement and every expression of a list of statements, nested
/// functions included.
pub(super) fn visit_all<'e>(
    stmts: &'e [Stmt],
    on_stmt: &mut dyn FnMut(&'e Stmt),
    on_expr: &mut dyn FnMut(&'e Expr),
) {
    for s in stmts {
        visit_stmt(s, on_stmt, on_expr);
    }
}

fn visit_stmt<'e>(
    s: &'e Stmt,
    on_stmt: &mut dyn FnMut(&'e Stmt),
    on_expr: &mut dyn FnMut(&'e Expr),
) {
    on_stmt(s);
    macro_rules! ex {
        ($x:expr) => {
            visit_expr_deep($x, on_stmt, on_expr)
        };
    }
    match &s.kind {
        StmtKind::Func(f) => visit_fn(f, on_stmt, on_expr),
        StmtKind::Class(c) => {
            for m in &c.members {
                match m {
                    ClassMember::Method(m) => visit_fn(&m.func, on_stmt, on_expr),
                    ClassMember::Field(f) => {
                        if let Some(i) = &f.init {
                            visit_expr_deep(i, on_stmt, on_expr);
                        }
                    }
                    ClassMember::StaticBlock(b) => visit_all(&b.stmts, on_stmt, on_expr),
                }
            }
        }
        StmtKind::Var(v) => {
            for d in &v.decls {
                if let Some(i) = &d.init {
                    ex!(i);
                }
            }
        }
        StmtKind::Return(Some(e))
        | StmtKind::Expr(e)
        | StmtKind::Throw(e)
        | StmtKind::ExportDefault(e) => ex!(e),
        StmtKind::If {
            test,
            then,
            otherwise,
        } => {
            ex!(test);
            visit_stmt(then, on_stmt, on_expr);
            if let Some(o) = otherwise {
                visit_stmt(o, on_stmt, on_expr);
            }
        }
        StmtKind::While { test, body } | StmtKind::DoWhile { body, test } => {
            ex!(test);
            visit_stmt(body, on_stmt, on_expr);
        }
        StmtKind::For {
            init,
            test,
            update,
            body,
        } => {
            if let Some(i) = init {
                visit_stmt(i, on_stmt, on_expr);
            }
            if let Some(t) = test {
                ex!(t);
            }
            if let Some(u) = update {
                ex!(u);
            }
            visit_stmt(body, on_stmt, on_expr);
        }
        StmtKind::ForOf { iter, body, .. } => {
            ex!(iter);
            visit_stmt(body, on_stmt, on_expr);
        }
        StmtKind::Switch { disc, cases } => {
            ex!(disc);
            for c in cases {
                if let Some(t) = &c.test {
                    ex!(t);
                }
                visit_all(&c.body, on_stmt, on_expr);
            }
        }
        StmtKind::Try {
            block,
            catch,
            finally,
        } => {
            visit_all(&block.stmts, on_stmt, on_expr);
            if let Some(c) = catch {
                visit_all(&c.body.stmts, on_stmt, on_expr);
            }
            if let Some(f) = finally {
                visit_all(&f.stmts, on_stmt, on_expr);
            }
        }
        StmtKind::Block(b) => visit_all(&b.stmts, on_stmt, on_expr),
        StmtKind::Labeled { body, .. } => visit_stmt(body, on_stmt, on_expr),
        _ => {}
    }
}

fn visit_fn<'e>(
    f: &'e Function,
    on_stmt: &mut dyn FnMut(&'e Stmt),
    on_expr: &mut dyn FnMut(&'e Expr),
) {
    for p in &f.params {
        if let Some(d) = &p.default {
            visit_expr_deep(d, on_stmt, on_expr);
        }
    }
    match &f.body {
        FnBody::Block(b) => visit_all(&b.stmts, on_stmt, on_expr),
        FnBody::Expr(e) => visit_expr_deep(e, on_stmt, on_expr),
    }
}

/// Visits an expression and everything inside it, function bodies included.
pub(super) fn visit_expr_deep<'e>(
    e: &'e Expr,
    on_stmt: &mut dyn FnMut(&'e Stmt),
    on_expr: &mut dyn FnMut(&'e Expr),
) {
    on_expr(e);
    macro_rules! visit {
        ($x:expr) => {
            visit_expr_deep($x, on_stmt, on_expr)
        };
    }
    match &e.kind {
        ExprKind::Func(f) | ExprKind::Arrow(f) => visit_fn(f, on_stmt, on_expr),
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
                if let PropKey::Computed(k) = &p.key {
                    visit!(k);
                }
                match &p.value {
                    PropValue::Expr(x) | PropValue::Spread(x) => visit!(x),
                    PropValue::Method(f) | PropValue::Get(f) | PropValue::Set(f) => {
                        visit_fn(f, on_stmt, on_expr)
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
        ExprKind::Yield { value: Some(x), .. } => visit!(x),
        ExprKind::Tagged { tag, exprs, .. } => {
            visit!(tag);
            exprs.iter().for_each(|x| visit!(x));
        }
        ExprKind::Class(c) => {
            for m in &c.members {
                if let ClassMember::Method(m) = m {
                    visit_fn(&m.func, on_stmt, on_expr);
                }
            }
        }
        _ => {}
    }
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
        ExprKind::Yield { value: Some(x), .. } => visit!(x),
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
    // `[Symbol.iterator]()` is `js_iterator()`.
    if let Some(rest) = name.strip_prefix("@@") {
        return format!("js_{}", snake(rest));
    }
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
    if p.is_empty() {
        return "X".into();
    }
    // A type of the program named like one the generated code uses.
    const TAKEN: &[&str] = &[
        "Box",
        "Vec",
        "Option",
        "Result",
        "Some",
        "None",
        "Ok",
        "Err",
        "String",
        "Rc",
        "RefCell",
        "HashMap",
        "HashSet",
        "Self",
        "LazyLock",
        "Mutex",
        "JsStatic",
        "Fn",
        "FnMut",
        "Iterator",
        "Display",
        "Debug",
        "Clone",
        "PartialEq",
        "Default",
        "JsMap",
        "JsSet",
        "JsValue",
        "JsInspect",
        "JsFromJson",
        "JsonValue",
        "JsRegex",
        "JsMatch",
        "JsDate",
        "JsGenerator",
        "JsPromise",
    ];
    if TAKEN.contains(&p.as_str()) {
        format!("{p}_")
    } else {
        p
    }
}

fn escape_keyword(name: String) -> String {
    if name == "self" || name == "Self" || name == "super" || name == "crate" || name == "main" {
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
        if h == Helper::BlockOn {
            self.helpers.insert(Helper::Timers);
        }
        if h == Helper::Json {
            self.helpers.insert(Helper::Inspect);
        }
        if h == Helper::Eval {
            self.helper(Helper::Dynamic);
        }
        if h == Helper::BigInt {
            self.helpers.insert(Helper::Inspect);
            self.helpers.insert(Helper::NumText);
        }
        if h == Helper::Shared {
            self.helpers.insert(Helper::Inspect);
            self.helpers.insert(Helper::NumText);
        }
        if h == Helper::Dynamic {
            self.helpers.extend([
                Helper::Inspect,
                Helper::Json,
                Helper::NumFormat,
                Helper::ToInt32,
            ]);
        }
        if matches!(
            h,
            Helper::Json | Helper::Inspect | Helper::NumFormat | Helper::Dynamic | Helper::Eval
        ) {
            self.helpers.insert(Helper::NumText);
        }
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
            Helper::ParseIntRadix => "js_parse_int_radix",
            Helper::ParseFloat => "js_parse_float",
            Helper::ToInt32 => "js_to_int32",
            Helper::NumText => "js_number_text",
            Helper::Symbol => "JsSymbol",
            Helper::NumFormat => "js_to_fixed",
            Helper::ToNumber => "js_number",
            Helper::Random => "js_random",
            Helper::NumToString => "js_num_to_string",
            Helper::BlockOn => "block_on",
            Helper::Inspect => "js_inspect",
            Helper::JsMap => "JsMap",
            Helper::JsSet => "JsSet",
            Helper::Generator => "JsGenerator",
            Helper::Timers => "js_run_timers",
            Helper::Promise => "JsPromise",
            Helper::Regex => "JsRegex",
            Helper::Json => "js_json_stringify",
            Helper::Date => "JsDate",
            Helper::Dynamic => "JsValue",
            Helper::Eval => "js_eval",
            Helper::Shared => "Shared",
            Helper::Node => "js_node_os_info",
            Helper::BigInt => "JsBigInt",
        }
    }

    /// The Rust type of a `Map` (`JsMap` keeps the insertion order when it matters).
    pub(super) fn map_name(&mut self) -> &'static str {
        if self.ordered {
            self.helper(Helper::JsMap)
        } else {
            self.uses_hashmap = true;
            "HashMap"
        }
    }

    pub(super) fn set_name(&mut self) -> &'static str {
        if self.ordered {
            self.helper(Helper::JsSet)
        } else {
            self.uses_hashset = true;
            "HashSet"
        }
    }

    /// Whether some map or set of the program is iterated or shown: then every map and set
    /// keeps its insertion order, like in JavaScript.
    fn needs_ordered(&self) -> bool {
        let mut found = false;
        let is_collection = |g: &Self, e: &Expr| matches!(g.ty_of(e), Ty::Map(..) | Ty::Set(_));
        let mut on_stmt = |s: &Stmt| {
            if let StmtKind::ForOf { iter, .. } = &s.kind
                && is_collection(self, iter)
            {
                found = true;
            }
        };
        let mut found2 = false;
        let mut on_expr = |e: &Expr| match &e.kind {
            ExprKind::Call { args, .. } => {
                if let Some(Callee::Builtin(name)) = self.p.calls.get(&e.id)
                    && (matches!(
                        *name,
                        "map.keys"
                            | "map.values"
                            | "map.entries"
                            | "map.forEach"
                            | "set.values"
                            | "set.keys"
                            | "set.forEach"
                            | "Object.keys"
                            | "Object.values"
                            | "Object.entries"
                            | "JSON.stringify"
                    ) || (matches!(
                        *name,
                        "Array.from"
                            | "Array.from.map"
                            | "console.log"
                            | "console.error"
                            | "console.warn"
                            | "console.info"
                            | "console.debug"
                            | "global.String"
                    ) && args.iter().any(|a| is_collection(self, &a.expr))))
                {
                    found2 = true;
                }
            }
            ExprKind::Array(elems) => {
                if elems
                    .iter()
                    .any(|el| matches!(el, ArrayElem::Spread(x) if is_collection(self, x)))
                {
                    found2 = true;
                }
            }
            ExprKind::Template { exprs, .. } => {
                if exprs.iter().any(|x| is_collection(self, x)) {
                    found2 = true;
                }
            }
            ExprKind::Binary {
                op: BinOp::Add,
                left,
                right,
            } if is_collection(self, left) || is_collection(self, right) => {
                found2 = true;
            }
            _ => {}
        };
        visit_all(&self.m.body, &mut on_stmt, &mut on_expr);
        found || found2
    }

    /// Whether values of this type print through `JsInspect` rather than `Display`.
    pub(super) fn is_inspect_ty(&self, ty: &Ty) -> bool {
        match self.resolve(ty) {
            Ty::Array(_) | Ty::Tuple(_) | Ty::Map(..) | Ty::Set(_) => true,
            Ty::Option(inner) => self.is_inspect_ty(&inner),
            Ty::Named(n, ..) => {
                n != "Error"
                    && n != "RegExp"
                    && ((self.is_type_param(&n) && !self.plain_param(ty))
                        || n == "RegExpMatch"
                        || n == "Date"
                        || self.p.struct_info(&n).is_some()
                        || self.p.union_info(&n).is_some())
            }
            _ => false,
        }
    }

    /// The union a type names, if any.
    pub(super) fn union_of(&self, ty: &Ty) -> Option<&crate::infer::UnionInfo> {
        self.p.union_of(ty)
    }

    /// The variant of union `to` that a value of type `from` fills, if `to` is a union
    /// and `from` one of its members.
    pub(super) fn union_variant(&self, from: &Ty, to: &Ty) -> Option<(String, String)> {
        let to = match self.resolve(to) {
            Ty::Option(inner) => self.resolve(&inner),
            other => other,
        };
        let u = self.union_of(&to)?;
        let from = self.resolve(from);
        if matches!(&from, Ty::Named(n, ..) if *n == u.name) {
            return None;
        }
        let zf = self.p.types.zonk(&from);
        let m = u
            .members
            .iter()
            .find(|m| crate::infer::same_shape_ty(&self.p.types.zonk(&m.ty), &zf))?;
        Some((pascal(&u.name), m.name.clone()))
    }

    /// Records that the structs and enums reachable from this type are shown.
    pub(super) fn mark_inspected(&mut self, ty: &Ty) {
        self.helpers.insert(Helper::Inspect);
        if let Ty::Named(_, args) = self.resolve(ty) {
            for a in &args {
                self.mark_inspected(a);
            }
        }
        match self.resolve(ty) {
            Ty::Option(t) | Ty::Array(t) | Ty::Set(t) => self.mark_inspected(&t),
            Ty::Map(k, v, _) => {
                self.mark_inspected(&k);
                self.mark_inspected(&v);
            }
            Ty::Tuple(ts) => {
                for t in &ts {
                    self.mark_inspected(t);
                }
            }
            Ty::Named(n, ..)
                if (self.p.struct_info(&n).is_some()
                    || self.p.is_enum(&n)
                    || self.p.union_info(&n).is_some())
                    && !self.abstract_iface(&n)
                    && self.inspected.insert(n.clone()) =>
            {
                let mut inner: Vec<Ty> = self
                    .p
                    .struct_info(&n)
                    .map(|s| s.fields.iter().map(|f| f.ty.clone()).collect())
                    .unwrap_or_default();
                if let Some(u) = self.p.union_info(&n) {
                    inner.extend(u.members.iter().map(|m| m.ty.clone()));
                }
                for t in &inner {
                    self.mark_inspected(t);
                }
            }
            _ => {}
        }
    }

    /// `js_inspect(&x.js_value())` or `js_to_string(&x.js_value())` for a value of type `ty`.
    /// Registers the types `JSON.parse` must build for `ty`, refusing those JSON cannot
    /// describe.
    pub(super) fn mark_json(&mut self, ty: &Ty, span: Span) -> GResult<()> {
        match self.resolve(ty) {
            Ty::Num(_) | Ty::Str | Ty::Bool | Ty::Null | Ty::Void | Ty::Never => Ok(()),
            Ty::Var(_) | Ty::Any => self.err(
                "the type of the parsed JSON is not known: write `const x: T = JSON.parse(…)` or `JSON.parse(…) as T`",
                span,
            ),
            Ty::Option(t) | Ty::Array(t) => self.mark_json(&t, span),
            Ty::Tuple(ts) => {
                for t in &ts {
                    self.mark_json(t, span)?;
                }
                Ok(())
            }
            Ty::Map(k, v, _) => {
                if !self.is_str(&k) {
                    return self.err(
                        "JSON object keys are strings: a map read from JSON needs string keys",
                        span,
                    );
                }
                self.mark_json(&v, span)
            }
            Ty::Set(_) => self.err(
                "a `Set` cannot be read from JSON, which only has arrays and objects",
                span,
            ),
            Ty::Func(_) => self.err("a function cannot be read from JSON", span),
            Ty::Gen(_) => self.err("a generator cannot be read from JSON", span),
            Ty::Named(n, args) => {
                if n == "Error" || n == "RegExp" || n == "RegExpMatch" || n == "Date" {
                    return self.err(
                        format!("a `{n}` cannot be read from JSON: read a string and parse it"),
                        span,
                    );
                }
                // A parameter of the generic struct being registered: its argument is.
                if self.class_params.contains(&n) {
                    return Ok(());
                }
                for a in &args {
                    self.mark_json(a, span)?;
                }
                if self.is_type_param(&n) {
                    return self.err(
                        "`JSON.parse` needs a concrete type here, not a type parameter",
                        span,
                    );
                }
                if self.p.is_dyn(&n) {
                    return self.err(
                        format!(
                            "`{n}` has subclasses: `JSON.parse` cannot tell which one to build; parse the exact class"
                        ),
                        span,
                    );
                }
                if !self.json_types.insert(n.clone()) {
                    return Ok(());
                }
                if self.p.is_enum(&n) {
                    return Ok(());
                }
                if let Some(u) = self.p.union_info(&n).cloned() {
                    for m in &u.members {
                        self.mark_json(&m.ty, span)?;
                    }
                    return Ok(());
                }
                let Some(info) = self.p.struct_info(&n).cloned() else {
                    return self.err(format!("`{n}` cannot be read from JSON"), span);
                };
                let prev = std::mem::replace(&mut self.class_params, info.type_params.clone());
                let mut r = Ok(());
                for f in &info.fields {
                    if matches!(self.resolve(&f.ty), Ty::Func(_)) {
                        r = self.err(
                            format!("`{n}.{}` is a function: it cannot be read from JSON", f.name),
                            span,
                        );
                        break;
                    }
                    r = self.mark_json(&f.ty, span);
                    if r.is_err() {
                        break;
                    }
                }
                self.class_params = prev;
                r
            }
        }
    }

    pub(super) fn inspect_call(&mut self, code: &str, ty: &Ty, inspect: bool) -> String {
        let f = if inspect {
            "js_inspect"
        } else {
            "js_to_string"
        };
        let value = self.js_value_of(code, ty);
        format!("{f}(&{value})")
    }

    /// Whether a record (an object used as a dictionary) is somewhere in this type.
    pub(super) fn contains_record(&self, ty: &Ty) -> bool {
        match self.resolve(ty) {
            Ty::Map(k, v, record) => record || self.contains_record(&k) || self.contains_record(&v),
            Ty::Option(t) | Ty::Array(t) | Ty::Set(t) => self.contains_record(&t),
            Ty::Tuple(ts) => ts.iter().any(|t| self.contains_record(t)),
            _ => false,
        }
    }

    /// The `JsValue` of a Rust expression of type `ty`: `.js_value()`, except that records
    /// are shown as objects, not as maps.
    pub(super) fn js_value_of(&mut self, code: &str, ty: &Ty) -> String {
        self.mark_inspected(ty);
        self.bound_needed(ty, "JsInspect");
        let recv = if code.contains(' ') && !(code.starts_with('(') && code.ends_with(')')) {
            format!("({code})")
        } else {
            code.to_string()
        };
        if self.elems_undefined
            && let Ty::Array(t) = self.resolve(ty)
            && let Ty::Option(inner) = self.resolve(&t)
        {
            self.elems_undefined = false;
            let shown = self.js_value_of("v", &inner);
            return format!(
                "JsValue::Array({recv}.iter().map(|x| x.as_ref().map_or(JsValue::Undefined, |v| {shown})).collect())"
            );
        }
        if !self.contains_record(ty) {
            return format!("{recv}.js_value()");
        }
        match self.resolve(ty) {
            Ty::Map(k, v, record) => {
                let inner = self.js_value_of("v", &v);
                if record {
                    format!(
                        "JsValue::Object(None, {recv}.iter().map(|(k, v)| (k.to_string(), {inner})).collect())"
                    )
                } else {
                    let key = self.js_value_of("k", &k);
                    format!("JsValue::Map({recv}.iter().map(|(k, v)| ({key}, {inner})).collect())")
                }
            }
            Ty::Array(t) => {
                let inner = self.js_value_of("x", &t);
                format!("JsValue::Array({recv}.iter().map(|x| {inner}).collect())")
            }
            Ty::Set(t) => {
                let inner = self.js_value_of("x", &t);
                format!("JsValue::Set({recv}.iter().map(|x| {inner}).collect())")
            }
            Ty::Option(t) => {
                let inner = self.js_value_of("x", &t);
                format!("{recv}.as_ref().map_or(JsValue::Null, |x| {inner})")
            }
            Ty::Tuple(ts) => {
                let mut parts = Vec::new();
                for (i, t) in ts.iter().enumerate() {
                    parts.push(self.js_value_of(&format!("{recv}.{i}"), t));
                }
                format!("JsValue::Array(vec![{}])", parts.join(", "))
            }
            _ => format!("{recv}.js_value()"),
        }
    }

    /// An interface with methods: it exists as a trait only, never as a struct.
    pub(super) fn abstract_iface(&self, name: &str) -> bool {
        self.p
            .struct_info(name)
            .is_some_and(|s| s.kind == crate::infer::StructKind::Interface && !s.methods.is_empty())
    }

    /// Visibility of an item: inside a module, what is not exported stays in the crate.
    pub(super) fn vis(&self, exported: bool) -> &'static str {
        if exported {
            "pub "
        } else if self.in_module {
            "pub(crate) "
        } else {
            ""
        }
    }

    /// The file a source offset is in.
    pub(super) fn file_of(&self, offset: usize) -> usize {
        self.opts
            .files
            .iter()
            .position(|f| f.contains(offset))
            .unwrap_or(0)
    }

    /// Whether an item at `span` belongs to the file being written (`None`: every file).
    pub(super) fn owned_by(&self, span: Span, file: Option<usize>) -> bool {
        file.is_none_or(|i| self.file_of(span.start) == i)
    }

    /// The file declaring a type.
    pub(super) fn type_file(&self, name: &str) -> usize {
        let span = self
            .p
            .struct_info(name)
            .map(|s| s.span)
            .or_else(|| self.p.enum_info(name).map(|e| e.span))
            .or_else(|| self.p.union_info(name).map(|u| u.span))
            .unwrap_or_default();
        self.file_of(span.start)
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
        let t = self.p.types.resolve(ty);
        if !self.class_subst.is_empty()
            && let Ty::Named(n, args) = &t
            && args.is_empty()
            && let Some(given) = self.class_subst.get(n)
            && !matches!(given, Ty::Named(g, a) if g == n && a.is_empty())
        {
            return self.resolve(given);
        }
        t
    }

    /// The parameters of `class`'s generic ancestors, as each class gives them to its
    /// base (`class NumberStack extends Stack<number>`: `T → number`).
    pub(super) fn ancestor_subst(&self, class: &str) -> HashMap<String, Ty> {
        let mut out: HashMap<String, Ty> = HashMap::new();
        let own: Vec<String> = self
            .p
            .struct_info(class)
            .map(|s| s.type_params.clone())
            .unwrap_or_default();
        let mut current = class.to_string();
        let mut seen = HashSet::new();
        while seen.insert(current.clone()) {
            let Some(s) = self.p.struct_info(&current) else {
                break;
            };
            let Some(parent) = s.parent.clone() else {
                break;
            };
            if let Some(p) = self.p.struct_info(&parent) {
                for (name, t) in p.type_params.iter().zip(&s.parent_args) {
                    if own.contains(name) {
                        continue;
                    }
                    let t = crate::infer::substitute(t, &out, &self.p.types);
                    out.entry(name.clone()).or_insert(t);
                }
            }
            current = parent;
        }
        out
    }

    pub(super) fn ty_of(&self, e: &Expr) -> Ty {
        // Converted for its place: the type it has there.
        if !self.casting.contains(&e.id)
            && let Some((_, to)) = self.p.dyn_casts.get(&e.id)
        {
            return self.resolve(to);
        }
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
        matches!(self.resolve(ty), Ty::Named(n, ..) if self.p.boxed.contains(&n))
    }

    pub(super) fn is_type_param(&self, name: &str) -> bool {
        self.fns
            .iter()
            .rev()
            .any(|f| f.type_params.iter().any(|t| t == name))
            || self.class_params.iter().any(|t| t == name)
    }

    /// A method returning `this` that returns `&mut Self` (or `&Self`): not for classes
    /// held as `dyn`, whose traits cannot name `Self`.
    pub(super) fn fluent(&self, info: &FnInfo) -> bool {
        info.returns_this
            && info.method_of.as_ref().is_some_and(|c| {
                !self.p.is_shared(c)
                    && !self.p.ancestry(c).iter().any(|a| {
                        self.p.is_dyn(a)
                            || self
                                .p
                                .struct_info(a)
                                .is_some_and(|s| s.implements.iter().any(|i| self.p.is_dyn(i)))
                    })
            })
    }

    /// What `for … of` gives for an object with `[Symbol.iterator]()`, and whether that
    /// method is a generator (else it returns an object with `next()`).
    pub(super) fn struct_iteration(&self, ty: &Ty) -> Option<(Ty, bool)> {
        let Ty::Named(n, _) = self.resolve(ty) else {
            return None;
        };
        let (_, m) = self.p.find_method(&n, "@@iterator", MethodKind::Method)?;
        match self.resolve(&self.p.funcs[&m.fn_id].sig.ret) {
            Ty::Gen(t) => Some((*t, true)),
            Ty::Named(it, _) => {
                let next = self.p.struct_info(&it)?.field("next")?.ty.clone();
                let Ty::Func(f) = self.resolve(&next) else {
                    return None;
                };
                let Ty::Named(r, _) = self.resolve(&f.ret) else {
                    return None;
                };
                Some((self.p.struct_info(&r)?.field("value")?.ty.clone(), false))
            }
            _ => None,
        }
    }

    /// An iterator over an object with `[Symbol.iterator]()`.
    pub(super) fn iteration_source(&self, recv: &str, generator: bool) -> String {
        if generator {
            format!("{recv}.js_iterator()")
        } else {
            format!(
                "{{ let it = {recv}.js_iterator(); std::iter::from_fn(move || {{ let r = (it.next)(); if r.done {{ None }} else {{ Some(r.value) }} }}) }}"
            )
        }
    }

    /// `T extends HasId`: the struct a type parameter in scope is constrained to.
    pub(super) fn constraint_of(&self, param: &str) -> Option<String> {
        for ctx in self.fns.iter().rev() {
            if let Some(f) = self.p.funcs.get(&ctx.id)
                && let Some((_, c)) = f.type_bounds.iter().find(|(p, _)| p == param)
            {
                return Some(c.clone());
            }
            if ctx.type_params.iter().any(|t| t == param) {
                return None;
            }
        }
        if !self.class_params.iter().any(|t| t == param) {
            return None;
        }
        let s = self.p.struct_info(self.current_class.as_ref()?)?;
        s.type_bounds
            .iter()
            .find(|(p, _)| p == param)
            .map(|(_, c)| c.clone())
    }

    /// The constrained type parameter a type is, with its constraint.
    pub(super) fn constrained(&self, ty: &Ty) -> Option<(String, String)> {
        match self.resolve(ty) {
            Ty::Named(n, args) if args.is_empty() && self.is_type_param(&n) => {
                let c = self.constraint_of(&n)?;
                Some((n, c))
            }
            _ => None,
        }
    }

    /// The trait standing for a constraint: `HasIdLike`.
    pub(super) fn bound_trait(c: &str) -> &'static str {
        Box::leak(format!("{}Like", pascal(c)).into_boxed_str())
    }

    /// The type parameters a function sees: its own, and its class's.
    pub(super) fn all_type_params(&self, info: &FnInfo) -> Vec<String> {
        let mut out = info.type_params.clone();
        if let Some(s) = info.method_of.as_ref().and_then(|c| self.p.struct_info(c)) {
            out.extend(s.type_params.iter().cloned());
        }
        out
    }

    /// A value of a generic struct (`Page<String>`): its `let` names the type, which the
    /// value alone may not tell (`Page { items: Vec::new() }`).
    pub(super) fn generic_instance(&self, ty: &Ty) -> bool {
        matches!(self.resolve(ty), Ty::Named(n, args) if !args.is_empty() && !self.p.is_dyn(&n))
    }

    /// Notes the parameters of a generic item given something else than a number, a
    /// string or a boolean: shown through `JsInspect` rather than `Display`.
    pub(super) fn note_instances(&mut self, key: &str, subst: &HashMap<String, Ty>) {
        for (p, t) in subst {
            let plain = matches!(self.resolve(t), Ty::Num(_) | Ty::Str | Ty::Bool)
                || matches!(self.resolve(t), Ty::Named(ref n, ..) if self.p.is_enum(n));
            if !plain {
                self.found_bounds
                    .entry(format!("inst:{key}"))
                    .or_default()
                    .entry(p.clone())
                    .or_default()
                    .insert("other");
            }
        }
    }

    /// A type parameter only ever given numbers, strings or booleans: shown with
    /// `Display`, like them.
    pub(super) fn plain_param(&self, ty: &Ty) -> bool {
        let Ty::Named(n, args) = self.resolve(ty) else {
            return false;
        };
        if !args.is_empty() || !self.is_type_param(&n) {
            return false;
        }
        let key = self
            .fns
            .iter()
            .rev()
            .find(|f| f.type_params.contains(&n))
            .map(|f| format!("fn:{}", f.id))
            .or_else(|| {
                self.class_params
                    .contains(&n)
                    .then(|| format!("struct:{}", self.current_class.clone().unwrap_or_default()))
            });
        let Some(key) = key else {
            return false;
        };
        !self
            .known_bounds
            .get(&format!("inst:{key}"))
            .is_some_and(|b| b.contains_key(&n))
    }

    /// What a generic item (`fn:<id>`, `struct:<name>`) needs of its type parameters,
    /// required of the types `subst` gives them.
    pub(super) fn propagate_bounds(&mut self, key: &str, subst: &HashMap<String, Ty>) {
        self.note_instances(key, subst);
        let Some(bounds) = self.known_bounds.get(key).cloned() else {
            return;
        };
        for (p, traits) in bounds {
            if let Some(t) = subst.get(&p) {
                for tr in traits {
                    self.bound_needed(t, tr);
                }
            }
        }
    }

    /// The types a call gives to type parameters: `params` found in `pattern` (a
    /// parameter's type) matched against `actual` (the argument's type).
    pub(super) fn match_params(
        &self,
        pattern: &Ty,
        actual: &Ty,
        params: &[String],
        out: &mut HashMap<String, Ty>,
    ) {
        match (self.resolve(pattern), self.resolve(actual)) {
            (Ty::Named(n, args), a) if args.is_empty() && params.contains(&n) => {
                out.entry(n).or_insert(a);
            }
            (Ty::Named(_, xs), Ty::Named(_, ys)) | (Ty::Tuple(xs), Ty::Tuple(ys)) => {
                for (x, y) in xs.iter().zip(&ys) {
                    self.match_params(x, y, params, out);
                }
            }
            (Ty::Option(x), Ty::Option(y))
            | (Ty::Array(x), Ty::Array(y))
            | (Ty::Set(x), Ty::Set(y))
            | (Ty::Gen(x), Ty::Gen(y)) => self.match_params(&x, &y, params, out),
            (Ty::Option(x), y) => self.match_params(&x, &y, params, out),
            (Ty::Map(k1, v1, _), Ty::Map(k2, v2, _)) => {
                self.match_params(&k1, &k2, params, out);
                self.match_params(&v1, &v2, params, out);
            }
            (Ty::Func(f), Ty::Func(g)) => {
                for (x, y) in f.params.iter().zip(&g.params) {
                    self.match_params(x, y, params, out);
                }
                self.match_params(&f.ret, &g.ret, params, out);
            }
            _ => {}
        }
    }

    /// The type arguments of a generic struct type, by parameter.
    pub(super) fn instance_subst(&self, ty: &Ty) -> HashMap<String, Ty> {
        let mut out = HashMap::new();
        if let Ty::Named(n, args) = self.resolve(ty)
            && let Some(s) = self.p.struct_info(&n)
            && s.type_params.len() == args.len()
        {
            for (p, a) in s.type_params.iter().zip(args) {
                out.insert(p.clone(), a);
            }
        }
        out
    }

    /// `<T, U>` after a generic struct's name: its parameters, or the given arguments.
    pub(super) fn type_args(&mut self, name: &str, args: &[Ty], span: Span) -> GResult<String> {
        let params = self
            .p
            .struct_info(name)
            .map(|s| s.type_params.clone())
            .unwrap_or_default();
        if params.is_empty() {
            return Ok(String::new());
        }
        let mut parts = Vec::new();
        if args.len() == params.len() {
            for a in args {
                parts.push(self.rust_ty(a, span)?);
            }
        } else {
            parts = params;
        }
        Ok(format!("<{}>", parts.join(", ")))
    }

    /// `impl<T: Clone + …>`: the parameters of a generic struct with what its methods
    /// need of them (and `extra` for every one).
    pub(super) fn generic_header(&self, params: &[String], extra: &[&'static str]) -> String {
        if params.is_empty() {
            return String::new();
        }
        let parts: Vec<String> = params
            .iter()
            .map(|p| {
                let mut b: BTreeSet<&'static str> = extra.iter().copied().collect();
                if let Some(more) = self.class_bounds.get(p) {
                    b.extend(more.iter().copied());
                }
                if b.is_empty() {
                    p.clone()
                } else {
                    format!("{p}: {}", b.into_iter().collect::<Vec<_>>().join(" + "))
                }
            })
            .collect();
        format!("<{}>", parts.join(", "))
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
            Ty::Map(k, v, _) => {
                let name = self.map_name();
                format!(
                    "{name}<{}, {}>",
                    self.rust_ty(&k, span)?,
                    self.rust_ty(&v, span)?
                )
            }
            Ty::Set(t) => {
                let name = self.set_name();
                format!("{name}<{}>", self.rust_ty(&t, span)?)
            }
            Ty::Gen(t) => {
                let name = self.helper(Helper::Generator);
                format!("{name}<{}>", self.rust_ty(&t, span)?)
            }
            Ty::Named(n, ..) => {
                if n == "Error" {
                    "String".into()
                } else if n == "RegExp" {
                    self.helper(Helper::Regex).into()
                } else if n == "RegExpMatch" {
                    self.helper(Helper::Regex);
                    "JsMatch".into()
                } else if n == "Date" {
                    self.helper(Helper::Date).into()
                } else if n == "Symbol" {
                    self.helper(Helper::Symbol).into()
                } else if n == "bigint" {
                    self.helper(Helper::BigInt).into()
                } else if n == crate::types::DYNAMIC {
                    self.helper(Helper::Dynamic).into()
                } else if self.is_type_param(&n) {
                    n
                } else if self.p.is_dyn(&n) {
                    let args = match self.resolve(ty) {
                        Ty::Named(_, args) => args,
                        _ => Vec::new(),
                    };
                    let g = self.dyn_generics(&n, &args, false, span)?;
                    if self.p.is_shared(&n) {
                        let h = self.helper(Helper::Shared);
                        format!("{h}<dyn {}Like{g}>", pascal(&n))
                    } else {
                        format!("Box<dyn {}Like{g}>", pascal(&n))
                    }
                } else {
                    let args = match self.resolve(ty) {
                        Ty::Named(_, args) => args,
                        _ => Vec::new(),
                    };
                    let generics = self.type_args(&n, &args, span)?;
                    if self.p.is_shared(&n) {
                        let h = self.helper(Helper::Shared);
                        format!("{h}<{}{generics}>", pascal(&n))
                    } else {
                        format!("{}{generics}", pascal(&n))
                    }
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
                self.uses_rc = true;
                format!("Rc<dyn Fn({}){ret}>", params.join(", "))
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
        // A type parameter, even when this class gives it a type: by value.
        if let Ty::Named(n, a) = self.p.types.resolve(ty)
            && a.is_empty()
            && type_params.contains(&n)
        {
            return Mode::Value;
        }
        let ty = self.resolve(ty);
        if let Ty::Func(_) = ty {
            return if stored { Mode::Value } else { Mode::FnImpl };
        }
        // A generator is consumed by whoever iterates it.
        if self.is_copy(&ty) || matches!(ty, Ty::Gen(_)) {
            return Mode::Value;
        }
        if stored || reassigned {
            return Mode::Value;
        }
        if let Ty::Named(n, ..) = &ty
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

    /// `<i64>` after the trait of a generic class or interface held dynamically: the
    /// type's arguments, else (`cast` set) placeholders Rust infers, else the parameters.
    pub(super) fn dyn_generics(
        &mut self,
        n: &str,
        args: &[Ty],
        cast: bool,
        span: Span,
    ) -> GResult<String> {
        let params = self
            .p
            .struct_info(n)
            .map(|s| s.type_params.clone())
            .unwrap_or_default();
        if params.is_empty() {
            return Ok(String::new());
        }
        let parts: Vec<String> = if cast {
            // Rust tells them from where the value goes.
            params.iter().map(|_| "_".to_string()).collect()
        } else if args.len() == params.len() {
            let mut out = Vec::new();
            for a in args {
                out.push(self.rust_ty(a, span)?);
            }
            out
        } else {
            params
        };
        Ok(format!("<{}>", parts.join(", ")))
    }

    /// `dyn CLike<…>` for a type held dynamically, with its type arguments.
    pub(super) fn dyn_ty(&mut self, ty: &Ty, span: Span) -> GResult<Option<String>> {
        match self.resolve(ty) {
            Ty::Named(n, args) if self.p.is_dyn(&n) && !self.p.is_shared(&n) => {
                let g = self.dyn_generics(&n, &args, false, span)?;
                Ok(Some(format!("dyn {}Like{g}", pascal(&n))))
            }
            _ => Ok(None),
        }
    }

    /// `dyn CLike` for a class held dynamically, else `None`.
    pub(super) fn dyn_name(&self, ty: &Ty) -> Option<String> {
        match self.resolve(ty) {
            Ty::Named(n, ..) if self.p.is_dyn(&n) => Some(format!("dyn {}Like", pascal(&n))),
            _ => None,
        }
    }

    /// Whether a value of this type holds a `dyn` class somewhere (no `PartialEq` then).
    pub(super) fn contains_dyn(&self, ty: &Ty) -> bool {
        match self.resolve(ty) {
            Ty::Named(n, ..) => self.p.is_dyn(&n),
            Ty::Option(t) | Ty::Array(t) | Ty::Set(t) => self.contains_dyn(&t),
            Ty::Map(k, v, _) => self.contains_dyn(&k) || self.contains_dyn(&v),
            Ty::Tuple(ts) => ts.iter().any(|t| self.contains_dyn(t)),
            _ => false,
        }
    }

    /// The Rust type of a parameter passed in `mode`.
    pub(super) fn mode_ty(&mut self, ty: &Ty, mode: Mode, span: Span) -> GResult<String> {
        Ok(match mode {
            Mode::Value => self.rust_ty(ty, span)?,
            Mode::FnImpl => self.fn_param_ty(ty, span)?,
            Mode::Ref => match self.dyn_ty(ty, span)? {
                Some(d) => format!("&{d}"),
                None => format!("&{}", self.rust_ty(ty, span)?),
            },
            Mode::RefMut => match self.dyn_ty(ty, span)? {
                Some(d) => format!("&mut {d}"),
                None => format!("&mut {}", self.rust_ty(ty, span)?),
            },
            Mode::StrRef => "&str".into(),
            Mode::Slice => match self.resolve(ty) {
                Ty::Array(t) => format!("&[{}]", self.rust_ty(&t, span)?),
                _ => format!("&{}", self.rust_ty(ty, span)?),
            },
            Mode::OptRef => match self.resolve(ty) {
                Ty::Option(inner) if self.is_str(&inner) => "Option<&str>".into(),
                Ty::Option(inner) => match self.dyn_ty(&inner, span)? {
                    Some(d) => format!("Option<&{d}>"),
                    None => format!("Option<&{}>", self.rust_ty(&inner, span)?),
                },
                _ => self.rust_ty(ty, span)?,
            },
        })
    }

    /// The representation of a variable bound as a parameter in `mode`.
    pub(super) fn mode_repr(mode: Mode) -> Repr {
        match mode {
            Mode::Value | Mode::FnImpl => Repr::Owned,
            Mode::OptRef => Repr::OptRef,
            Mode::Ref | Mode::Slice => Repr::Ref,
            Mode::RefMut => Repr::RefMut,
            Mode::StrRef => Repr::Str,
        }
    }

    /// `{}` or `{:?}` for a value of this type in a format string.
    pub(super) fn fmt_spec(&self, ty: &Ty) -> &'static str {
        match self.resolve(ty) {
            Ty::Num(_) | Ty::Str | Ty::Bool => "{}",
            Ty::Named(n, ..) => {
                let prints = n == "Error"
                    || n == crate::types::DYNAMIC
                    || n == "bigint"
                    || n == "Symbol"
                    || n == "RegExp"
                    || self.is_type_param(&n)
                    || self.p.is_enum(&n)
                    || self
                        .p
                        .find_method(&n, "toString", MethodKind::Method)
                        .is_some();
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
        if e.repr == Repr::OptRef {
            return match want {
                Want::Owned => {
                    let inner = match &ty {
                        Ty::Option(inner) => self.resolve(inner),
                        other => other.clone(),
                    };
                    if self.is_boxed(&inner) {
                        format!(
                            "{}.map(|v| Box::new(v.clone()))",
                            Self::paren(&e, PREC_POSTFIX)
                        )
                    } else if self.is_copy(&inner) {
                        format!("{}.copied()", Self::paren(&e, PREC_POSTFIX))
                    } else if self.is_str(&inner) {
                        format!("{}.map(|v| v.to_string())", Self::paren(&e, PREC_POSTFIX))
                    } else {
                        format!("{}.cloned()", Self::paren(&e, PREC_POSTFIX))
                    }
                }
                _ => e.code,
            };
        }
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
                Repr::OptRef => e.code,
            },
            // A boxed trait object lent as `&dyn T`: `&Box<dyn T>` does not coerce by itself.
            Want::Ref
                if self.dyn_name(&ty).is_some()
                    && e.repr != Repr::OptRef
                    && !matches!(self.resolve(&ty), Ty::Named(ref n, ..) if self.p.is_shared(n)) =>
            {
                format!("{}.as_ref()", Self::paren(&e, PREC_POSTFIX))
            }
            Want::Ref => match e.repr {
                Repr::Owned | Repr::Place | Repr::Usize => {
                    format!("&{}", Self::paren(&e, PREC_UNARY))
                }
                Repr::Ref | Repr::RefMut | Repr::OptRef => e.code,
                Repr::Str => e.code,
                Repr::Char => format!("&{}.to_string()", Self::paren(&e, PREC_POSTFIX)),
            },
            Want::RefMut => match e.repr {
                Repr::Owned | Repr::Place => format!("&mut {}", Self::paren(&e, PREC_UNARY)),
                Repr::RefMut => e.code,
                Repr::Ref => format!("&mut {}", Self::paren(&e, PREC_UNARY)),
                Repr::Str | Repr::Char | Repr::Usize | Repr::OptRef => {
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
                Repr::Usize | Repr::OptRef => e.code,
            },
            Want::Slice => match e.repr {
                Repr::Owned if e.code.starts_with("vec![") => format!("&{}", &e.code[4..]),
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
        matches!(self.repr_of(id), Repr::Ref | Repr::RefMut | Repr::OptRef)
            && self.p.var(id).role != Role::Param
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
        // An integer going where a double is wanted (`fib(i)` returning a double).
        let to_num = match &to_r {
            Ty::Option(inner) if !matches!(from_r, Ty::Option(_)) => self.resolve(inner),
            other => other.clone(),
        };
        if matches!(from_r, Ty::Num(_))
            && !self.is_float(&from_r)
            && matches!(to_num, Ty::Num(_))
            && self.is_float(&to_num)
        {
            let code = if is_int_literal_code(&e.code) {
                format!("{}.0", e.code.trim_end_matches("_i64"))
            } else {
                let prec = e.prec;
                let owned = self.owned(e, &from_r);
                let x = E::owned(owned, prec);
                format!("{} as f64", Self::paren(&x, exprs::PREC_CAST_PLUS))
            };
            return if matches!(to_r, Ty::Option(_)) {
                format!("Some({code})")
            } else {
                code
            };
        }
        // A function kept in a field or a collection is shared: behind an `Rc`.
        let fresh_fn = e.code.starts_with('|')
            || e.code.starts_with("move |")
            // A closure after the copies of what it captures: `{ let k = k.clone(); move || … }`.
            || (e.code.starts_with("{ let ") && e.code.contains("; move |"))
            || e.var.is_some_and(|(b, _)| {
                let v = self.p.var(b);
                v.role == Role::Func || v.closure_fn.is_some()
            });
        let stored_fn = match &to_r {
            Ty::Option(inner) => matches!(self.resolve(inner), Ty::Func(_)),
            other => matches!(other, Ty::Func(_)),
        };
        let mut code = self.owned(e, &from_r);
        if stored_fn && fresh_fn {
            self.uses_rc = true;
            code = format!("Rc::new({code})");
            if let Ty::Option(inner) = &to_r
                && !matches!(from_r, Ty::Option(_))
            {
                let t = self.rust_ty(inner, Span::default()).unwrap_or_default();
                return format!("Some({code} as {t})");
            }
        }
        // A member of a union goes into its variant.
        let target = match &to_r {
            Ty::Option(inner) if !matches!(from_r, Ty::Option(_)) => self.resolve(inner),
            other => other.clone(),
        };
        if let Some((en, variant)) = self.union_variant(&from_r, &target) {
            code = format!("{en}::{variant}({code})");
        }
        // An instance of a subclass where the base class is held dynamically.
        if let (Ty::Named(from_class, ..), Ty::Named(base, base_args)) = (&from_r, &target)
            && from_class != base
            && self.p.is_dyn(base)
            && self.p.is_subclass(from_class, base)
        {
            let g = self
                .dyn_generics(base, base_args, true, Span::default())
                .unwrap_or_default();
            let trait_name = format!("Box<dyn {}Like{g}>", pascal(base));
            code = if self.p.is_shared(base) {
                // The same object, held as its base class.
                format!(
                    "Shared({}.0 as std::rc::Rc<std::cell::RefCell<dyn {}Like{g}>>)",
                    paren_unless_simple_owned(&code),
                    pascal(base)
                )
            } else if self.p.is_dyn(from_class) {
                format!("{code} as {trait_name}")
            } else {
                format!("Box::new({code}) as {trait_name}")
            };
        }
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
        // `Vec<T>: Clone` needs `T: Clone`.
        match self.resolve(ty) {
            Ty::Option(t) | Ty::Array(t) | Ty::Set(t) | Ty::Gen(t) => {
                return self.bound_needed(&t, bound);
            }
            Ty::Map(k, v, _) => {
                self.bound_needed(&k, bound);
                return self.bound_needed(&v, bound);
            }
            Ty::Tuple(ts) => {
                for t in &ts {
                    self.bound_needed(t, bound);
                }
                return;
            }
            Ty::Named(_, args) if !args.is_empty() => {
                for t in &args {
                    self.bound_needed(t, bound);
                }
                return;
            }
            _ => {}
        }
        if let Ty::Named(n, ..) = self.resolve(ty)
            && self.is_type_param(&n)
        {
            if bound == "JsInspect" {
                self.generic_inspect = true;
            }
            let own = self.fns.last().is_some_and(|f| f.type_params.contains(&n));
            if !own && self.class_params.contains(&n) {
                self.class_bounds.entry(n).or_default().insert(bound);
            } else {
                self.fn_ctx_mut().bounds.entry(n).or_default().insert(bound);
            }
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

/// A variable, a field or an element: a value that lives on after the expression.
pub(super) fn is_place_kind(e: &Expr) -> bool {
    matches!(
        e.kind,
        ExprKind::Ident(_) | ExprKind::Member { .. } | ExprKind::Index { .. } | ExprKind::This
    )
}

pub(super) fn is_int_literal(code: &str) -> bool {
    !code.is_empty() && code.chars().all(|c| c.is_ascii_digit())
}

/// An integer literal: `3`, `-3`, `3_i64`.
fn is_int_literal_code(code: &str) -> bool {
    let c = code.trim_end_matches("_i64");
    let c = c.strip_prefix('-').unwrap_or(c);
    !c.is_empty() && c.bytes().all(|b| b.is_ascii_digit())
}

/// Whether an expression changes a variable (`i++`, `x = 1`): the values around it must
/// be read in order.
pub(super) fn changes_something(e: &Expr) -> bool {
    let mut found = false;
    let mut copy = e.clone();
    crate::desugar::expr_mut(&mut copy, &mut |x| {
        if matches!(x.kind, ExprKind::Update { .. } | ExprKind::Assign { .. }) {
            found = true;
        }
        !found
    });
    found
}

/// `code` as the receiver of `.0`: parenthesized unless a plain path.
fn paren_unless_simple_owned(code: &str) -> String {
    if code
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '.')
    {
        code.to_string()
    } else {
        format!("({code})")
    }
}
