//! Type inference and the analyses the code generator relies on: which variables are
//! reassigned, mutated in place or stored away, which functions throw, which `if` tests
//! narrow a nullable value, where a value is used for the last time.
//!
//! Types come from annotations, literals and unification: a variable without annotation
//! gets the type of what it is assigned. Numbers are integers until a float literal or a
//! division says otherwise (see [`crate::types`]).

mod annotations;
mod builtins;
mod decls;
mod exprs;
mod finish;
mod stmts;
mod unions;

use std::collections::{HashMap, HashSet};

use crate::ast::*;
use crate::error::{Error, Span};
use crate::types::{FuncTy, Ty, Types};

pub use finish::{pascal, same_shape_ty};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Local,
    Param,
    TopLevel,
    Func,
    Class,
    Enum,
    Catch,
    /// The variable of a `for … of` loop.
    Loop,
}

#[derive(Clone, Debug)]
pub struct VarInfo {
    pub name: String,
    pub ty: Ty,
    pub role: Role,
    /// Assigned again after its declaration (`x = …`, `x++`).
    pub reassigned: bool,
    /// Changed in place: a field or an element assigned, a mutating method called.
    pub mutated: bool,
    /// Kept somewhere by value: pushed into a list, put in a field, returned.
    pub stored: bool,
    /// Read inside a nested function.
    pub captured: bool,
    /// Reassigned or mutated inside a nested function.
    pub mutated_in_closure: bool,
    /// The function declaring it (`None` at the top level).
    pub func: Option<NodeId>,
    /// Parameter `index` of function `id`.
    pub param_of: Option<(NodeId, usize)>,
    /// A top-level `const` holding a literal: becomes a Rust `const`.
    pub const_literal: bool,
    /// Read from inside a function (top-level variables only).
    pub used_in_fn: bool,
    /// Every value it ever holds is a field or an element of something else: it can be a
    /// reference.
    pub alias: bool,
    /// Initialized with an arrow function: its function id.
    pub closure_fn: Option<NodeId>,
    /// Holds the result of a generator's `next()`: `.value` and `.done` read it.
    pub iter_result: bool,
    /// Holds a regular expression made from a literal: its flags.
    pub regex_flags: Option<String>,
    /// Changed, and captured by a closure that outlives the scope: the closure and the
    /// scope share it (`Rc<RefCell<T>>`).
    pub shared: bool,
    /// The loop depth where it was declared (last-use analysis).
    pub loop_depth: usize,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct FnInfo {
    pub id: NodeId,
    pub name: Option<String>,
    /// The binding of each parameter.
    pub params: Vec<NodeId>,
    /// Parameters declared optional (`x?: T`): their type is already an `Option`.
    pub optional: Vec<bool>,
    /// Parameters with a default value.
    pub defaults: Vec<bool>,
    pub sig: FuncTy,
    pub type_params: Vec<String>,
    /// Returns `Result`: throws, or calls a function that throws outside a `try`.
    pub throws: bool,
    pub throw_direct: bool,
    pub is_async: bool,
    pub is_arrow: bool,
    pub method_of: Option<String>,
    pub method_kind: Option<MethodKind>,
    pub is_static: bool,
    /// `abstract m(): T;`: no body; the subclasses implement it.
    pub is_abstract: bool,
    pub mutates_self: bool,
    /// Outer bindings read by this closure.
    pub captures: Vec<NodeId>,
    /// A closure that is returned or stored: captures by value (`move`).
    pub escapes: bool,
    /// Declared inside another function and using its variables: a closure, not an item.
    pub is_closure: bool,
    pub parent: Option<NodeId>,
    /// Functions called outside a `try`, with the id of the call.
    pub calls: Vec<(NodeId, NodeId)>,
    /// Uses `this`.
    pub uses_this: bool,
    pub exported: bool,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StructKind {
    Class,
    Interface,
    /// From an object literal or an inline object type.
    Synth,
}

#[derive(Clone, Debug)]
pub struct FieldInfo {
    pub name: String,
    pub ty: Ty,
    pub optional: bool,
    pub access: Option<Access>,
    pub readonly: bool,
    pub has_init: bool,
    /// Declared with a string literal type (`kind: "circle"`): the tag of a union member.
    pub literal: Option<String>,
    pub span: Span,
}

/// One member of a union of different types: a variant of the Rust enum.
#[derive(Clone, Debug)]
pub struct UnionMember {
    /// The variant name.
    pub name: String,
    pub ty: Ty,
    /// The value of the discriminant field for this member.
    pub tag: Option<String>,
}

/// `A | B`: an enum with one variant per member type.
#[derive(Clone, Debug)]
pub struct UnionInfo {
    pub name: String,
    pub members: Vec<UnionMember>,
    /// A field every member has, with a different string literal type each: the tag.
    pub discriminant: Option<String>,
    pub exported: bool,
    pub span: Span,
}

/// The other side of a union member test.
#[derive(Clone, Debug)]
pub enum Complement {
    /// The only other member: its variant and type.
    Variant(String, Ty),
    /// The union of the remaining members.
    Union(Ty),
}

impl Complement {
    pub fn ty(&self) -> &Ty {
        match self {
            Complement::Variant(_, t) | Complement::Union(t) => t,
        }
    }
}

impl UnionInfo {
    pub fn member_with_tag(&self, tag: &str) -> Option<&UnionMember> {
        self.members.iter().find(|m| m.tag.as_deref() == Some(tag))
    }

    pub fn variant(&self, name: &str) -> Option<&UnionMember> {
        self.members.iter().find(|m| m.name == name)
    }
}

#[derive(Clone, Debug)]
pub struct MethodInfo {
    pub name: String,
    pub fn_id: NodeId,
    pub kind: MethodKind,
    pub is_static: bool,
    pub is_abstract: bool,
    pub access: Option<Access>,
}

#[derive(Clone, Debug)]
pub struct StructInfo {
    pub name: String,
    pub kind: StructKind,
    /// The fields, inherited ones first.
    pub fields: Vec<FieldInfo>,
    /// The methods declared in the class itself.
    pub methods: Vec<MethodInfo>,
    pub ctor: Option<NodeId>,
    pub exported: bool,
    pub span: Span,
    /// Static fields, with the id of their initializer.
    pub statics: Vec<(String, Ty, Option<NodeId>)>,
    /// `class C extends P`: the base class.
    pub parent: Option<String>,
    pub is_abstract: bool,
}

impl StructInfo {
    pub fn field(&self, name: &str) -> Option<&FieldInfo> {
        self.fields.iter().find(|f| f.name == name)
    }

    pub fn method(&self, name: &str, kind: MethodKind) -> Option<&MethodInfo> {
        self.methods
            .iter()
            .find(|m| m.name == name && m.kind == kind)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum EnumValue {
    Num(f64),
    Str(String),
}

#[derive(Clone, Debug)]
pub struct Variant {
    /// The Rust name.
    pub name: String,
    /// The TypeScript name, or the string literal for a union of literals.
    pub source: String,
    pub value: Option<EnumValue>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnumKind {
    /// `enum E { A, B = 5 }`.
    Numeric,
    /// `enum E { A = "a" }`.
    Strings,
    /// `type E = "a" | "b"`.
    Literals,
}

#[derive(Clone, Debug)]
pub struct EnumInfo {
    pub name: String,
    pub variants: Vec<Variant>,
    pub kind: EnumKind,
    pub exported: bool,
    pub span: Span,
}

impl EnumInfo {
    pub fn variant_for_literal(&self, lit: &str) -> Option<&Variant> {
        self.variants.iter().find(|v| v.source == lit)
    }
}

/// What a call resolved to.
#[derive(Clone, Debug, PartialEq)]
pub enum Callee {
    /// A function declared with `function`: its binding.
    Fn(NodeId),
    /// A variable holding a closure.
    Closure(NodeId),
    Method {
        class: String,
        name: String,
        fn_id: NodeId,
        is_static: bool,
    },
    /// `new Class(…)`.
    Ctor(String),
    /// `"Math.floor"`, `"array.push"`, `"string.split"`, `"map.get"`…
    Builtin(&'static str),
    /// `super(…)` in a constructor: the class whose constructor runs.
    SuperCtor(String),
    /// `super.m(…)`: the class declaring the method.
    SuperMethod {
        class: String,
        name: String,
        fn_id: NodeId,
    },
    /// Any other callee of function type.
    Dynamic,
}

/// Where a nullable value is tested, what it binds.
#[derive(Clone, Debug)]
pub struct NarrowTest {
    pub root: Root,
    pub path: Vec<String>,
    /// The name bound to the content.
    pub name: String,
    /// `x !== null` / `x`: the content exists when the test is true.
    pub positive: bool,
    /// A test of a union member (`typeof x === "string"`, `x.kind === "circle"`): the enum
    /// and the variant bound; `None` for a nullable value (`Some`).
    pub union: Option<(String, String)>,
    /// What the value is on the other side of the test: the other variant of a union of
    /// two, or the union of the remaining members.
    pub complement: Option<Complement>,
    /// `x instanceof C` on a value of a base class: the subclass bound.
    pub instance_of: Option<String>,
    /// The content is changed in the narrowed region: bind it mutably.
    pub mutated: bool,
    pub inner: Ty,
    /// `if (x === null) return;`: the rest of the block is narrowed (`let … else`).
    pub let_else: bool,
    /// Narrowed inside a `while`: the content is read with `unwrap`.
    pub unwrap: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Root {
    Var(NodeId),
    This,
}

/// The whole program, typed.
pub struct Program {
    pub types: Types,
    pub node_count: usize,
    pub expr_ty: HashMap<NodeId, Ty>,
    pub vars: HashMap<NodeId, VarInfo>,
    /// Identifier expression → binding.
    pub refs: HashMap<NodeId, NodeId>,
    pub funcs: HashMap<NodeId, FnInfo>,
    /// Declared function binding → function id.
    pub fn_bindings: HashMap<NodeId, NodeId>,
    pub structs: Vec<StructInfo>,
    pub enums: Vec<EnumInfo>,
    pub unions: Vec<UnionInfo>,
    pub calls: HashMap<NodeId, Callee>,
    /// Object literal → struct it builds.
    pub literal_struct: HashMap<NodeId, String>,
    /// String literal typed as a union enum → (enum, variant).
    pub enum_literal: HashMap<NodeId, (String, String)>,
    /// Test expression → what it narrows.
    pub narrow_tests: HashMap<NodeId, NarrowTest>,
    /// Expression inside a narrowed region → the bound name to use instead, and the
    /// test that bound it.
    pub narrowed_uses: HashMap<NodeId, (String, NodeId)>,
    /// Identifier uses that are the last use of their variable.
    pub last_use: HashSet<NodeId>,
    /// Calls that happen inside a `try` block.
    pub calls_in_try: HashSet<NodeId>,
    /// `try` blocks (by the start offset of the block) whose body contains a `return`.
    pub try_returns: HashSet<usize>,
    /// The function holding the top-level statements.
    pub main_id: NodeId,
    pub has_main: bool,
    /// Structs that contain themselves: held through a `Box` inside `Option`.
    pub boxed: HashSet<String>,
    /// Base classes whose values may be instances of a subclass: a trait and `dyn`.
    pub polymorphic: HashSet<String>,
    /// Base method → the methods overriding it in subclasses.
    pub overrides: HashMap<NodeId, Vec<NodeId>>,
    /// The program sets timers: they run once `main` is done.
    pub uses_timers: bool,
}

impl Program {
    /// The class and its ancestors, nearest first.
    pub fn ancestry(&self, class: &str) -> Vec<String> {
        let mut out = vec![class.to_string()];
        let mut current = class.to_string();
        while let Some(p) = self.struct_info(&current).and_then(|s| s.parent.clone()) {
            if out.contains(&p) {
                break;
            }
            out.push(p.clone());
            current = p;
        }
        out
    }

    /// Whether `class` is `base` or descends from it.
    pub fn is_subclass(&self, class: &str, base: &str) -> bool {
        self.ancestry(class).iter().any(|c| c == base)
    }

    /// The classes descending from `base` (not `base` itself).
    pub fn descendants(&self, base: &str) -> Vec<String> {
        self.structs
            .iter()
            .filter(|s| s.name != base && self.is_subclass(&s.name, base))
            .map(|s| s.name.clone())
            .collect()
    }

    /// A method of the class or of an ancestor, with the class declaring it.
    pub fn find_method(
        &self,
        class: &str,
        name: &str,
        kind: MethodKind,
    ) -> Option<(String, MethodInfo)> {
        for c in self.ancestry(class) {
            if let Some(m) = self.struct_info(&c).and_then(|s| s.method(name, kind)) {
                return Some((c, m.clone()));
            }
        }
        None
    }

    /// Whether a method, or any override of it in a subclass, changes `this`.
    pub fn method_mutates(&self, fn_id: NodeId) -> bool {
        self.funcs.get(&fn_id).is_some_and(|f| f.mutates_self)
            || self
                .overrides
                .get(&fn_id)
                .is_some_and(|os| os.iter().any(|o| self.method_mutates(*o)))
    }

    /// Whether values of the class are held as `dyn`.
    pub fn is_dyn(&self, name: &str) -> bool {
        self.polymorphic.contains(name)
    }
    pub fn ty(&self, id: NodeId) -> Ty {
        self.expr_ty.get(&id).cloned().unwrap_or(Ty::Any)
    }

    pub fn var(&self, id: NodeId) -> &VarInfo {
        &self.vars[&id]
    }

    pub fn struct_info(&self, name: &str) -> Option<&StructInfo> {
        self.structs.iter().find(|s| s.name == name)
    }

    pub fn enum_info(&self, name: &str) -> Option<&EnumInfo> {
        self.enums.iter().find(|e| e.name == name)
    }

    pub fn is_enum(&self, name: &str) -> bool {
        self.enum_info(name).is_some()
    }

    pub fn union_info(&self, name: &str) -> Option<&UnionInfo> {
        self.unions.iter().find(|u| u.name == name)
    }

    /// The union a type names, if any.
    pub fn union_of(&self, ty: &Ty) -> Option<&UnionInfo> {
        match self.types.resolve(ty) {
            Ty::Named(n) => self.union_info(&n),
            _ => None,
        }
    }

    /// Whether values of the type copy freely in Rust.
    pub fn is_copy(&self, ty: &Ty) -> bool {
        self.types.is_copy(ty, &|name| self.is_enum(name))
    }
}

// ---------------------------------------------------------------------------------------

#[derive(Clone)]
pub(super) struct Scope {
    pub(super) vars: HashMap<String, NodeId>,
}

pub(super) struct FnCtx {
    pub(super) id: NodeId,
    pub(super) ret: Ty,
    pub(super) this: Option<String>,
    pub(super) in_ctor: bool,
    pub(super) is_static: bool,
    pub(super) try_depth: usize,
    pub(super) loop_depth: usize,
    pub(super) type_params: Vec<String>,
    pub(super) captures: Vec<NodeId>,
    /// The function (not arrow) this one takes `this` from.
    pub(super) this_fn: NodeId,
    pub(super) has_return_value: bool,
    /// Inside a generator: the type of the values it yields.
    pub(super) yield_ty: Option<Ty>,
}

pub(super) struct ActiveNarrow {
    pub(super) root: Root,
    pub(super) path: Vec<String>,
    pub(super) name: String,
    pub(super) inner: Ty,
    pub(super) test_id: NodeId,
}

impl ActiveNarrow {
    pub(super) fn duplicate(&self) -> ActiveNarrow {
        ActiveNarrow {
            root: self.root.clone(),
            path: self.path.clone(),
            name: self.name.clone(),
            inner: self.inner.clone(),
            test_id: self.test_id,
        }
    }
}

/// A simple path: a variable or `this`, then fields.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct Path {
    pub(super) root: Root,
    pub(super) fields: Vec<String>,
}

pub(super) struct Infer<'m> {
    pub(super) src: &'m str,
    pub(super) types: Types,
    pub(super) node_count: usize,
    pub(super) next_synth: usize,
    pub(super) expr_ty: HashMap<NodeId, Ty>,
    pub(super) vars: HashMap<NodeId, VarInfo>,
    pub(super) refs: HashMap<NodeId, NodeId>,
    pub(super) funcs: HashMap<NodeId, FnInfo>,
    pub(super) structs: Vec<StructInfo>,
    pub(super) enums: Vec<EnumInfo>,
    pub(super) unions: Vec<UnionInfo>,
    pub(super) aliases: HashMap<String, Ty>,
    pub(super) calls: HashMap<NodeId, Callee>,
    pub(super) literal_struct: HashMap<NodeId, String>,
    pub(super) enum_literal: HashMap<NodeId, (String, String)>,
    pub(super) narrow_tests: HashMap<NodeId, NarrowTest>,
    pub(super) narrowed_uses: HashMap<NodeId, (String, NodeId)>,
    pub(super) calls_in_try: HashSet<NodeId>,
    /// The types `JSON.parse` fills: their unconstrained numbers are floats, as in JSON.
    pub(super) json_targets: Vec<Ty>,
    pub(super) try_returns: HashSet<usize>,
    pub(super) scopes: Vec<Scope>,
    pub(super) fns: Vec<FnCtx>,
    pub(super) narrows: Vec<ActiveNarrow>,
    /// Uses of each binding in order.
    pub(super) uses: HashMap<NodeId, Vec<Use>>,
    /// The branches (test, side) the current statement is in: uses in different sides of
    /// the same test never both run.
    pub(super) branch: Vec<(NodeId, u8)>,
    /// Parameters mutated in place / stored, by (function, index).
    pub(super) param_mutated: HashSet<(NodeId, usize)>,
    pub(super) param_stored: HashSet<(NodeId, usize)>,
    /// Arguments that are plain variables: (callee fn id, index, binding).
    pub(super) arg_bindings: Vec<(NodeId, usize, NodeId)>,
    /// Method calls on a variable: (method fn id, receiver binding).
    pub(super) method_receivers: Vec<(NodeId, NodeId)>,
    /// Method calls on `this`: (method fn id, calling method fn id).
    pub(super) this_calls: Vec<(NodeId, NodeId)>,
    /// `for (const x of xs)`: (loop variable, collection): a changed element changes the
    /// collection.
    pub(super) loop_sources: Vec<(NodeId, NodeId)>,
    pub(super) synth_names: HashSet<String>,
    pub(super) main_id: NodeId,
    pub(super) has_main: bool,
    /// Functions declared with `function`: binding → fn id.
    pub(super) fn_bindings: HashMap<NodeId, NodeId>,
    pub(super) this_class_stack: Vec<String>,
    /// Names suggested for the structs of object literals, by expression id.
    pub(super) hints: HashMap<NodeId, String>,
    /// Variables initialized from a narrowed value: (binding, test).
    pub(super) alias_of_narrow: Vec<(NodeId, NodeId)>,
    /// Base classes whose values may be instances of a subclass.
    pub(super) polymorphic: HashSet<String>,
    /// Base method → overriding methods.
    pub(super) overrides: HashMap<NodeId, Vec<NodeId>>,
    /// The class declarations, by name (for base classes collected on demand).
    pub(super) class_asts: HashMap<String, &'m Class>,
    pub(super) collected_classes: HashSet<String>,
    pub(super) uses_timers: bool,
}

/// A use of a variable: the expression, the loop depth, the function and the branch path.
pub(super) type Use = (NodeId, usize, NodeId, Vec<(NodeId, u8)>);

pub(super) type IResult<T> = Result<T, Error>;

pub(super) const GLOBALS: &[&str] = &[
    "console",
    "Math",
    "JSON",
    "Object",
    "Array",
    "Number",
    "String",
    "Boolean",
    "Error",
    "Map",
    "Set",
    "Promise",
    "Date",
    "process",
    "performance",
    "parseInt",
    "parseFloat",
    "isNaN",
    "isFinite",
    "Infinity",
    "NaN",
    "structuredClone",
    "globalThis",
    "window",
    "document",
    "require",
    "module",
    "setTimeout",
    "setInterval",
    "clearTimeout",
    "clearInterval",
    "alert",
    "RegExp",
    "Symbol",
    "BigInt",
];

pub fn infer(module: &Module, src: &str) -> IResult<Program> {
    let main_id = module.node_count;
    let mut inf = Infer {
        src,
        types: Types::default(),
        node_count: module.node_count,
        next_synth: module.node_count + 1,
        expr_ty: HashMap::new(),
        vars: HashMap::new(),
        refs: HashMap::new(),
        funcs: HashMap::new(),
        structs: Vec::new(),
        enums: Vec::new(),
        unions: Vec::new(),
        aliases: HashMap::new(),
        calls: HashMap::new(),
        literal_struct: HashMap::new(),
        enum_literal: HashMap::new(),
        narrow_tests: HashMap::new(),
        narrowed_uses: HashMap::new(),
        calls_in_try: HashSet::new(),
        json_targets: Vec::new(),
        try_returns: HashSet::new(),
        scopes: vec![Scope {
            vars: HashMap::new(),
        }],
        fns: Vec::new(),
        narrows: Vec::new(),
        uses: HashMap::new(),
        branch: Vec::new(),
        param_mutated: HashSet::new(),
        param_stored: HashSet::new(),
        arg_bindings: Vec::new(),
        method_receivers: Vec::new(),
        this_calls: Vec::new(),
        loop_sources: Vec::new(),
        synth_names: HashSet::new(),
        main_id,
        has_main: false,
        fn_bindings: HashMap::new(),
        this_class_stack: Vec::new(),
        hints: HashMap::new(),
        alias_of_narrow: Vec::new(),
        polymorphic: HashSet::new(),
        overrides: HashMap::new(),
        class_asts: HashMap::new(),
        collected_classes: HashSet::new(),
        uses_timers: false,
    };
    inf.run(module)?;
    inf.finish()
}

impl<'m> Infer<'m> {
    pub(super) fn err<T>(&self, msg: impl Into<String>, span: Span) -> IResult<T> {
        Err(Error::new(msg, span))
    }

    pub(super) fn synth_id(&mut self) -> NodeId {
        let id = self.next_synth;
        self.next_synth += 1;
        id
    }

    pub(super) fn ctx(&self) -> &FnCtx {
        self.fns.last().expect("function context")
    }

    pub(super) fn ctx_mut(&mut self) -> &mut FnCtx {
        self.fns.last_mut().expect("function context")
    }

    pub(super) fn describe(&self, ty: &Ty) -> String {
        self.types.describe(ty)
    }

    pub(super) fn unify(&mut self, a: &Ty, b: &Ty, span: Span) -> IResult<()> {
        self.types.unify(a, b).map_err(|msg| Error::new(msg, span))
    }

    /// Unifies the type of a value with what is expected of it. A value of a member type
    /// is accepted where a union is expected (the generator wraps it in the variant), and
    /// an instance of a subclass where its base class is expected.
    pub(super) fn expect(&mut self, found: &Ty, expected: &Ty, span: Span) -> IResult<()> {
        if self.types.unify(found, expected).is_err() {
            if self.union_accepts(expected, found) || self.class_accepts(expected, found) {
                return Ok(());
            }
            return self.err(
                format!(
                    "expected {}, found {}",
                    self.describe(expected),
                    self.describe(found)
                ),
                span,
            );
        }
        Ok(())
    }

    pub(super) fn set_ty(&mut self, id: NodeId, ty: Ty) -> Ty {
        self.expr_ty.insert(id, ty.clone());
        ty
    }

    pub(super) fn text(&self, e: &Expr) -> &str {
        &self.src[e.span.start..e.span.end.min(self.src.len())]
    }

    // ----- scopes and bindings -----

    pub(super) fn push_scope(&mut self) {
        self.scopes.push(Scope {
            vars: HashMap::new(),
        });
    }

    pub(super) fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    pub(super) fn lookup(&self, name: &str) -> Option<NodeId> {
        self.scopes
            .iter()
            .rev()
            .find_map(|s| s.vars.get(name).copied())
    }

    pub(super) fn declare(&mut self, name: &str, id: NodeId, ty: Ty, role: Role, span: Span) {
        let at_top = self.fns.len() <= 1 && self.scopes.len() == 1;
        let func = if at_top {
            None
        } else {
            self.fns.last().map(|f| f.id)
        };
        let depth = self.fns.last().map_or(0, |f| f.loop_depth);
        self.vars.insert(
            id,
            VarInfo {
                name: name.to_string(),
                ty,
                role,
                reassigned: false,
                mutated: false,
                stored: false,
                captured: false,
                mutated_in_closure: false,
                func,
                param_of: None,
                const_literal: false,
                used_in_fn: false,
                alias: false,
                closure_fn: None,
                iter_result: false,
                regex_flags: None,
                shared: false,
                loop_depth: depth,
                span,
            },
        );
        self.scopes
            .last_mut()
            .expect("scope")
            .vars
            .insert(name.to_string(), id);
    }

    /// Binds the names of a pattern to the parts of `ty`.
    pub(super) fn bind_pattern(&mut self, pat: &'m Pattern, ty: &Ty, role: Role) -> IResult<()> {
        match pat {
            Pattern::Ident { name, span, id } => {
                self.declare(name, *id, ty.clone(), role, *span);
                Ok(())
            }
            Pattern::Array { elems, rest, span } => {
                let ty = self.types.resolve(ty);
                for (i, el) in elems.iter().enumerate() {
                    let Some(el) = el else { continue };
                    let item = match &ty {
                        Ty::Tuple(ts) => ts.get(i).cloned().ok_or_else(|| {
                            Error::new(format!("the tuple has no element {i}"), el.pat.span())
                        })?,
                        Ty::Array(t) => (**t).clone(),
                        Ty::Var(_) => {
                            let t = self.types.fresh();
                            self.unify(&ty, &Ty::Array(Box::new(t.clone())), *span)?;
                            t
                        }
                        other => {
                            let d = self.describe(other);
                            return self.err(format!("cannot destructure {d} as an array"), *span);
                        }
                    };
                    if let Some(d) = &el.default {
                        let dt = self.expr(d, Some(&item))?;
                        self.expect(&dt, &item, d.span)?;
                    }
                    self.bind_pattern(&el.pat, &item, role)?;
                }
                if let Some(rest) = rest {
                    let elem = match &ty {
                        Ty::Array(t) => (**t).clone(),
                        _ => return self.err("rest elements need an array", rest.span()),
                    };
                    self.bind_pattern(rest, &Ty::Array(Box::new(elem)), role)?;
                }
                Ok(())
            }
            Pattern::Object { props, rest, span } => {
                let ty = self.types.resolve(ty);
                let fields: Vec<(String, Ty)> = match &ty {
                    Ty::Named(n) => match self.struct_by_name(n) {
                        Some(s) => s
                            .fields
                            .iter()
                            .map(|f| {
                                let t = if f.optional {
                                    Ty::Option(Box::new(f.ty.clone()))
                                } else {
                                    f.ty.clone()
                                };
                                (f.name.clone(), t)
                            })
                            .collect(),
                        None => return self.err(format!("cannot destructure {n}"), *span),
                    },
                    Ty::Map(_, v, _) => props
                        .iter()
                        .map(|p| (p.key.clone(), (**v).clone()))
                        .collect(),
                    Ty::Var(_) => {
                        return self.err(
                            "the type of this object is not known yet: add an annotation",
                            *span,
                        );
                    }
                    other => {
                        let d = self.describe(other);
                        return self.err(format!("cannot destructure {d} as an object"), *span);
                    }
                };
                for p in props {
                    let Some((_, fty)) = fields.iter().find(|(n, _)| *n == p.key) else {
                        let d = self.describe(&ty);
                        return self.err(format!("{d} has no property `{}`", p.key), p.span);
                    };
                    let mut fty = fty.clone();
                    if let Some(d) = &p.default {
                        let inner = match self.types.resolve(&fty) {
                            Ty::Option(t) => *t,
                            t => t,
                        };
                        let dt = self.expr(d, Some(&inner))?;
                        self.expect(&dt, &inner, d.span)?;
                        fty = inner;
                    }
                    self.bind_pattern(&p.value, &fty, role)?;
                }
                if rest.is_some() {
                    return self.err("rest properties are not supported", *span);
                }
                Ok(())
            }
        }
    }

    pub(super) fn struct_by_name(&self, name: &str) -> Option<&StructInfo> {
        self.structs.iter().find(|s| s.name == name)
    }

    pub(super) fn struct_mut(&mut self, name: &str) -> Option<&mut StructInfo> {
        self.structs.iter_mut().find(|s| s.name == name)
    }

    pub(super) fn enum_by_name(&self, name: &str) -> Option<&EnumInfo> {
        self.enums.iter().find(|e| e.name == name)
    }

    pub(super) fn type_exists(&self, name: &str) -> bool {
        self.struct_by_name(name).is_some()
            || self.enum_by_name(name).is_some()
            || self.union_by_name(name).is_some()
            || self.aliases.contains_key(name)
    }

    pub(super) fn union_by_name(&self, name: &str) -> Option<&UnionInfo> {
        self.unions.iter().find(|u| u.name == name)
    }

    /// The class and its ancestors, nearest first.
    pub(super) fn ancestry(&self, class: &str) -> Vec<String> {
        let mut out = vec![class.to_string()];
        let mut current = class.to_string();
        while let Some(p) = self.struct_by_name(&current).and_then(|s| s.parent.clone()) {
            if out.contains(&p) {
                break;
            }
            out.push(p.clone());
            current = p;
        }
        out
    }

    pub(super) fn is_subclass(&self, class: &str, base: &str) -> bool {
        self.ancestry(class).iter().any(|c| c == base)
    }

    /// A method of the class or of an ancestor, with the class declaring it.
    pub(super) fn find_method(
        &self,
        class: &str,
        name: &str,
        kind: MethodKind,
    ) -> Option<(String, MethodInfo)> {
        for c in self.ancestry(class) {
            if let Some(m) = self.struct_by_name(&c).and_then(|s| s.method(name, kind)) {
                return Some((c, m.clone()));
            }
        }
        None
    }

    /// A static field of the class or of an ancestor.
    pub(super) fn find_static(&self, class: &str, name: &str) -> Option<(String, Ty)> {
        for c in self.ancestry(class) {
            if let Some((_, t, _)) = self
                .struct_by_name(&c)
                .and_then(|s| s.statics.iter().find(|(n, _, _)| n == name))
            {
                return Some((c, t.clone()));
            }
        }
        None
    }

    /// The constructor a class runs: its own, or the nearest inherited one.
    pub(super) fn nearest_ctor(&self, class: &str) -> Option<(String, NodeId)> {
        for c in self.ancestry(class) {
            if let Some(ctor) = self.struct_by_name(&c).and_then(|s| s.ctor) {
                return Some((c, ctor));
            }
        }
        None
    }

    /// Whether a value of class `found` can go where `expected` (a base class of it,
    /// possibly nullable) is wanted; the base class then needs dynamic dispatch.
    pub(super) fn class_accepts(&mut self, expected: &Ty, found: &Ty) -> bool {
        let target = match self.types.resolve(expected) {
            Ty::Option(inner) => self.types.resolve(&inner),
            other => other,
        };
        let found = match self.types.resolve(found) {
            Ty::Option(inner) => self.types.resolve(&inner),
            other => other,
        };
        let (Ty::Named(base), Ty::Named(class)) = (target, found) else {
            return false;
        };
        if base == class
            || self
                .struct_by_name(&base)
                .is_none_or(|s| s.kind != StructKind::Class)
            || !self.is_subclass(&class, &base)
        {
            return false;
        }
        self.polymorphic.insert(base);
        true
    }

    /// The union a type names, if any.
    pub(super) fn union_of_ty(&self, ty: &Ty) -> Option<&UnionInfo> {
        match self.types.resolve(ty) {
            Ty::Named(n) => self.union_by_name(&n),
            _ => None,
        }
    }

    pub(super) fn is_type_param(&self, name: &str) -> bool {
        self.fns
            .iter()
            .rev()
            .any(|f| f.type_params.iter().any(|t| t == name))
    }

    pub(super) fn record_use(&mut self, binding: NodeId, use_id: NodeId) {
        let depth = self.ctx().loop_depth;
        let fn_id = self.ctx().id;
        let path = self.branch.clone();
        self.uses
            .entry(binding)
            .or_default()
            .push((use_id, depth, fn_id, path));
    }

    /// `x`, `this.a.b`: the path of a simple expression (no calls, no indexes).
    pub(super) fn path_of(&self, e: &Expr) -> Option<Path> {
        match &e.kind {
            ExprKind::Ident(name) => {
                let id = self.lookup(name)?;
                Some(Path {
                    root: Root::Var(id),
                    fields: Vec::new(),
                })
            }
            ExprKind::This => Some(Path {
                root: Root::This,
                fields: Vec::new(),
            }),
            ExprKind::Member {
                obj,
                prop,
                optional: false,
            } => {
                let mut p = self.path_of(obj)?;
                p.fields.push(prop.clone());
                Some(p)
            }
            ExprKind::NonNull(inner) => self.path_of(inner),
            _ => None,
        }
    }

    pub(super) fn active_narrow(&self, path: &Path) -> Option<usize> {
        self.narrows
            .iter()
            .rposition(|n| n.root == path.root && n.path == path.fields)
    }

    /// The flags of a regular expression expression, when they are written in the program.
    pub(super) fn regex_flags(&self, e: &Expr) -> Option<String> {
        match &e.kind {
            ExprKind::Regex { flags, .. } => Some(flags.clone()),
            ExprKind::New { callee, args, .. } if matches!(&callee.kind, ExprKind::Ident(n) if n == "RegExp") => {
                match args.get(1) {
                    None => Some(String::new()),
                    Some(a) => match &a.expr.kind {
                        ExprKind::Str(f) => Some(f.clone()),
                        _ => None,
                    },
                }
            }
            ExprKind::Ident(name) => self
                .lookup(name)
                .and_then(|id| self.vars.get(&id))
                .and_then(|v| v.regex_flags.clone()),
            _ => None,
        }
    }

    /// Whether an expression is the result of a generator's `next()`, directly or through
    /// a variable.
    pub(super) fn is_iter_result(&self, e: &Expr) -> bool {
        if matches!(
            self.calls.get(&e.id),
            Some(Callee::Builtin("generator.next"))
        ) {
            return true;
        }
        match &e.kind {
            ExprKind::Ident(name) => self
                .lookup(name)
                .and_then(|id| self.vars.get(&id))
                .is_some_and(|v| v.iter_result),
            _ => false,
        }
    }

    pub(super) fn root_binding(&self, e: &Expr) -> Option<NodeId> {
        match &e.kind {
            ExprKind::Ident(name) => self.lookup(name),
            ExprKind::Member { obj, .. } | ExprKind::Index { obj, .. } => self.root_binding(obj),
            ExprKind::NonNull(inner) => self.root_binding(inner),
            _ => None,
        }
    }
}
