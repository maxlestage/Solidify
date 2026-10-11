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
mod dynamic;
mod exprs;
mod files;
mod finish;
mod growth;
mod stmts;
mod unions;

use std::collections::{BTreeSet, HashMap, HashSet};

use crate::ast::*;
use crate::error::{Error, Span};
use crate::types::{FuncTy, Ty, Types};

pub use finish::{pascal, same_shape_ty, substitute};

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
    /// `import * as ns from "./file"`.
    Namespace,
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
    /// Made from this other variable (`const b = a`) and sharing its cell.
    pub shares_with: Option<NodeId>,
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
    /// `<T extends HasId>`: the interface or class each constrained parameter stands for.
    pub type_bounds: Vec<(String, String)>,
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
    /// The last parameter is `...rest: T[]`: the extra arguments make an array.
    pub rest: bool,
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
    /// A method whose every `return` gives `this` (`add(x) { …; return this; }`): it
    /// returns a reference to the object, so that calls chain on the same one.
    pub returns_this: bool,
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
    /// `class C implements I`: the interfaces (also those a class is found to satisfy).
    pub implements: Vec<String>,
    /// `class Stack<T>`: the type parameters, named in the fields' and methods' types.
    pub type_params: Vec<String>,
    /// `class Repo<T extends HasId>`: the constrained parameters and their constraint.
    pub type_bounds: Vec<(String, String)>,
    /// `extends Base<number>`: the base class's type arguments.
    pub parent_args: Vec<Ty>,
    /// `implements Container<number>`: the type arguments given to each generic
    /// interface implemented.
    pub implements_args: Vec<(String, Vec<Ty>)>,
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
    /// `f?.()`: a call when `f` is not null.
    OptionalCall,
    /// `obj?.m(…)`: a method call when `obj` is not null.
    OptionalMethod {
        class: String,
        name: String,
        fn_id: NodeId,
    },
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
    /// A value converted between a type known when translating and a value known at
    /// run time (`JsValue`): its type, and the type its place takes.
    pub dyn_casts: HashMap<NodeId, (Ty, Ty)>,
    /// Object and array literals built as values known at run time.
    pub dyn_literals: HashSet<NodeId>,
    /// Classes and object types whose instances are shared between the places holding
    /// them (`Shared<T>`): changed through one, changed for all; `===` is identity.
    pub shared_types: HashSet<String>,
    /// Fields sharing their collection with a variable (`Rc<RefCell<T>>`).
    pub shared_fields: HashSet<(String, String)>,
    /// `eval(code)` with code known at run time: the variables it sees (a read of each).
    pub eval_scopes: HashMap<NodeId, Vec<(String, Expr)>>,
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
    /// Declarations not written: variables only assigned inside a null test, whose
    /// value the test binds.
    pub drop_decl: HashSet<NodeId>,
    /// `T extends C` given the type `N`: (C, N), which implements C's trait.
    pub bound_impls: BTreeSet<(String, String)>,
    /// Variables, functions and fields whose missing value is `undefined`, not `null`.
    pub undef_vars: HashSet<NodeId>,
    pub undef_fns: HashSet<NodeId>,
    pub undef_fields: HashSet<(String, String)>,
    /// Arrays whose missing elements are `undefined`.
    pub undef_elem_vars: HashSet<NodeId>,
    /// Numeric enums whose values are mapped back to names (`Level[x]`).
    pub enum_names: BTreeSet<String>,
}

/// `T | undefined` (and not `T | null`): a missing value is `undefined`.
pub(super) fn says_undefined(ann: &TypeAnn) -> bool {
    let undef = |a: &TypeAnn| match a {
        TypeAnn::Named { name, .. } => name == "undefined" || name == "void",
        TypeAnn::Lit(Lit::Undefined, _) => true,
        _ => false,
    };
    let null = |a: &TypeAnn| match a {
        TypeAnn::Named { name, .. } => name == "null",
        TypeAnn::Lit(Lit::Null, _) => true,
        _ => false,
    };
    match ann {
        TypeAnn::Union(members, _) => members.iter().any(undef) && !members.iter().any(null),
        other => undef(other),
    }
}

impl Program {
    /// Whether this expression, when it gives no value, gives `undefined` (as opposed to
    /// `null`): what `console.log` prints for it.
    pub fn undefinedish(&self, e: &Expr) -> bool {
        match &e.kind {
            ExprKind::Undefined => true,
            ExprKind::Ident(name) => match self.refs.get(&e.id) {
                Some(b) => self.undef_vars.contains(b),
                None => name == "undefined",
            },
            ExprKind::Member { optional: true, .. } | ExprKind::Index { optional: true, .. } => {
                true
            }
            ExprKind::Member { obj, prop, .. } => match self.types.resolve(&self.ty(obj.id)) {
                Ty::Named(n, _) => self.undef_fields.contains(&(n, prop.clone())),
                Ty::Option(inner) => match self.types.resolve(&inner) {
                    Ty::Named(n, _) => self.undef_fields.contains(&(n, prop.clone())),
                    _ => false,
                },
                _ => false,
            },
            // A map read with `[k]`: `undefined` when the key is missing.
            ExprKind::Index { obj, .. } => {
                matches!(self.types.resolve(&self.ty(obj.id)), Ty::Map(..))
            }
            ExprKind::Call { .. } => match self.calls.get(&e.id) {
                Some(Callee::Fn(b)) => self
                    .fn_bindings
                    .get(b)
                    .is_some_and(|f| self.undef_fns.contains(f)),
                Some(Callee::Closure(b)) => self
                    .vars
                    .get(b)
                    .and_then(|v| v.closure_fn)
                    .is_some_and(|f| self.undef_fns.contains(&f)),
                Some(Callee::Method { fn_id, .. }) => self.undef_fns.contains(fn_id),
                Some(Callee::OptionalCall | Callee::OptionalMethod { .. }) => true,
                Some(Callee::Builtin(n)) => matches!(
                    *n,
                    "map.get"
                        | "array.find"
                        | "array.findLast"
                        | "array.pop"
                        | "array.shift"
                        | "array.at"
                        | "Symbol.keyFor"
                ),
                _ => false,
            },
            ExprKind::Cond { cons, alt, .. } => {
                let missing = |x: &Expr| matches!(x.kind, ExprKind::Null | ExprKind::Undefined);
                if missing(cons) {
                    self.undefinedish(cons)
                } else if missing(alt) {
                    self.undefinedish(alt)
                } else {
                    self.undefinedish(cons) || self.undefinedish(alt)
                }
            }
            ExprKind::Logical {
                op: LogicalOp::Nullish | LogicalOp::Or,
                right,
                ..
            } => self.undefinedish(right),
            ExprKind::As(inner, _) | ExprKind::NonNull(inner) | ExprKind::Await(inner) => {
                self.undefinedish(inner)
            }
            ExprKind::Assign { value, .. } => self.undefinedish(value),
            _ => false,
        }
    }

    /// Whether the missing elements of this array are `undefined`: `[f(), g()]` with
    /// such calls, `xs.map(x => m.get(x))`, a variable holding one.
    pub fn elements_undefinedish(&self, e: &Expr) -> bool {
        match &e.kind {
            ExprKind::Array(elems) => elems.iter().any(|el| match el {
                ArrayElem::Expr(x) => self.undefinedish(x),
                ArrayElem::Spread(x) => self.elements_undefinedish(x),
                ArrayElem::Hole => true,
            }),
            ExprKind::Ident(_) => self
                .refs
                .get(&e.id)
                .is_some_and(|b| self.undef_elem_vars.contains(b)),
            ExprKind::Call { callee, args, .. } => match self.calls.get(&e.id) {
                Some(Callee::Builtin("array.map" | "array.flatMap")) => {
                    args.first().is_some_and(|a| match &a.expr.kind {
                        ExprKind::Arrow(f) | ExprKind::Func(f) => self.undef_fns.contains(&f.id),
                        _ => false,
                    })
                }
                Some(Callee::Builtin(
                    "array.filter" | "array.slice" | "array.concat" | "array.reverse"
                    | "array.toReversed" | "array.sort" | "array.toSorted",
                )) => match &callee.kind {
                    ExprKind::Member { obj, .. } => self.elements_undefinedish(obj),
                    _ => false,
                },
                _ => false,
            },
            ExprKind::As(inner, _) | ExprKind::NonNull(inner) => self.elements_undefinedish(inner),
            _ => false,
        }
    }

    /// Variables and functions whose value comes from an `undefined` source.
    pub(super) fn settle_undefined(
        &mut self,
        inits: &[(NodeId, &Expr)],
        returns: &[(NodeId, &Expr)],
    ) {
        loop {
            let mut changed = false;
            for (b, e) in inits {
                if !self.undef_vars.contains(b) && self.undefinedish(e) {
                    self.undef_vars.insert(*b);
                    changed = true;
                }
                if !self.undef_elem_vars.contains(b) && self.elements_undefinedish(e) {
                    self.undef_elem_vars.insert(*b);
                    changed = true;
                }
            }
            for (f, e) in returns {
                if !self.undef_fns.contains(f) && self.undefinedish(e) {
                    self.undef_fns.insert(*f);
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
    }

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

    /// Whether `class` is `base`, descends from it, or implements it.
    pub fn is_subclass(&self, class: &str, base: &str) -> bool {
        self.ancestry(class).iter().any(|c| {
            c == base
                || self
                    .struct_info(c)
                    .is_some_and(|s| s.implements.iter().any(|i| i == base))
        })
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

    /// Whether a field shares its collection with a variable.
    pub fn is_shared_field(&self, s: &str, f: &str) -> bool {
        self.shared_fields.contains(&(s.to_string(), f.to_string()))
    }

    /// Whether the instances of a struct are shared (`Shared<T>`).
    pub fn is_shared(&self, name: &str) -> bool {
        self.shared_types.contains(name)
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
            Ty::Named(n, ..) => self.union_info(&n),
            _ => None,
        }
    }

    /// Whether values of the type copy freely in Rust.
    pub fn is_copy(&self, ty: &Ty) -> bool {
        self.types.is_copy(ty, &|name| {
            self.is_enum(name) || name == "Date" || name == "Symbol"
        })
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
    /// The files of the program (one when translating a single source).
    pub(super) files: &'m [crate::project::FileInfo],
    /// `import * as ns`: the file each namespace binding stands for.
    pub(super) namespaces: HashMap<NodeId, usize>,
    /// For each file, its exported names and the global name each one stands for.
    pub(super) exports: Vec<HashMap<String, String>>,
    /// Variables assigned inside a null test (`(m = re.exec(s)) !== null`), with the ids
    /// of the assignment targets: the test binds the value itself.
    pub(super) assign_narrowed: HashMap<NodeId, Vec<NodeId>>,
    /// The null tests made on an assignment: `while let` binds a fresh value each time.
    pub(super) assign_tests: HashSet<NodeId>,
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
    pub(super) dyn_casts: HashMap<NodeId, (Ty, Ty)>,
    pub(super) dyn_literals: HashSet<NodeId>,
    /// Struct types changed through a path (`a.count = 1`, `list[0].items.push(x)`).
    pub(super) changed_types: HashSet<String>,
    /// Methods called on an instance that is not `this`, with its class.
    pub(super) instance_calls: Vec<(NodeId, String)>,
    /// Types of values copied out of a place (stored, returned).
    pub(super) copied_types: Vec<Ty>,
    /// Struct types compared with `===`.
    pub(super) identity_types: HashSet<String>,
    /// `const b = a`: (b, a).
    pub(super) var_aliases: Vec<(NodeId, NodeId)>,
    /// `{ items: list }`: (struct, field, variable).
    pub(super) field_sources: Vec<(String, String, NodeId)>,
    /// Fields changed through a path (`holder.items.push(x)`).
    pub(super) changed_fields: HashSet<(String, String)>,
    pub(super) eval_scopes: HashMap<NodeId, Vec<(String, Expr)>>,
    /// The type expected of the call being inferred (`JSON.parse` reads it).
    pub(super) call_expected: Option<Ty>,
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
    /// Loop variables over a field of `this`, with the method.
    pub(super) loop_this_sources: Vec<(NodeId, NodeId)>,
    pub(super) synth_names: HashSet<String>,
    pub(super) main_id: NodeId,
    pub(super) has_main: bool,
    /// Functions declared with `function`: binding → fn id.
    pub(super) fn_bindings: HashMap<NodeId, NodeId>,
    pub(super) this_class_stack: Vec<String>,
    /// (constraint, type given, where): see [`Self::record_bound_uses`].
    pub(super) bound_uses: Vec<(String, Ty, Span)>,
    /// Numeric enums whose values are mapped back to names (`Level[x]`).
    pub(super) enum_names: BTreeSet<String>,
    /// Every assignment, for the numbers that grow without bound (see `growth`).
    pub(super) assigns: Vec<(&'m AssignTarget, AssignOp, &'m Expr)>,
    /// `f<number>(…)`: the type arguments of the call being checked.
    pub(super) type_args: Option<(Span, Vec<Ty>)>,
    /// `return` statements of each function: (giving `this`, giving something else).
    pub(super) this_returns: HashMap<NodeId, (usize, usize)>,
    /// Where a missing value is `undefined` rather than `null` (console.log shows which):
    /// variables, functions, (struct, field)s known so, and the initializers and returned
    /// values that decide the others.
    pub(super) undef_vars: HashSet<NodeId>,
    pub(super) undef_fns: HashSet<NodeId>,
    pub(super) undef_fields: HashSet<(String, String)>,
    pub(super) undef_params: HashSet<(NodeId, usize)>,
    pub(super) var_inits: Vec<(NodeId, &'m Expr)>,
    pub(super) fn_returns: Vec<(NodeId, &'m Expr)>,
    /// The type parameters of the generic classes, interfaces and aliases being read.
    pub(super) param_scope: Vec<Vec<String>>,
    /// `type Pair<A, B> = [A, B]`: the parameters and the type, instantiated at each use.
    pub(super) generic_aliases: HashMap<String, (Vec<String>, Ty)>,
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

/// The native functions the embedded Node modules (`src/node/*.ts`) are written with:
/// name, parameters (`s` text, `b` boolean), result (`s`, `b`, `i` integer, `a` texts,
/// `v` nothing).
pub(crate) const INTRINSICS: &[(&str, &str, char)] = &[
    ("__fs_read", "s", 's'),
    ("__fs_write", "ss", 'v'),
    ("__fs_append", "ss", 'v'),
    ("__fs_exists", "s", 'b'),
    ("__fs_is_dir", "s", 'b'),
    ("__fs_size", "s", 'i'),
    ("__fs_readdir", "s", 'a'),
    ("__fs_mkdir", "sb", 'v'),
    ("__fs_remove", "sb", 'v'),
    ("__fs_rename", "ss", 'v'),
    ("__fs_copy", "ss", 'v'),
    ("__os_info", "s", 's'),
    ("__os_cpus", "", 'i'),
];

pub(super) const GLOBALS: &[&str] = &[
    "console",
    "eval",
    "Function",
    "Proxy",
    "Reflect",
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
    "Symbol",
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

pub fn infer<'m>(
    module: &'m Module,
    src: &'m str,
    files: &'m [crate::project::FileInfo],
) -> IResult<Program> {
    let main_id = module.node_count;
    let mut inf = Infer {
        src,
        files,
        namespaces: HashMap::new(),
        exports: Vec::new(),
        assign_narrowed: HashMap::new(),
        assign_tests: HashSet::new(),
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
        dyn_casts: HashMap::new(),
        dyn_literals: HashSet::new(),
        changed_types: HashSet::new(),
        instance_calls: Vec::new(),
        copied_types: Vec::new(),
        identity_types: HashSet::new(),
        var_aliases: Vec::new(),
        field_sources: Vec::new(),
        changed_fields: HashSet::new(),
        eval_scopes: HashMap::new(),
        call_expected: None,
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
        loop_this_sources: Vec::new(),
        synth_names: HashSet::new(),
        main_id,
        has_main: false,
        fn_bindings: HashMap::new(),
        this_class_stack: Vec::new(),
        param_scope: Vec::new(),
        this_returns: HashMap::new(),
        undef_vars: HashSet::new(),
        undef_fns: HashSet::new(),
        undef_fields: HashSet::new(),
        undef_params: HashSet::new(),
        var_inits: Vec::new(),
        fn_returns: Vec::new(),
        type_args: None,
        enum_names: BTreeSet::new(),
        assigns: Vec::new(),
        bound_uses: Vec::new(),
        generic_aliases: HashMap::new(),
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

    /// Like [`Self::expect`] where a value goes somewhere (an argument, a returned or
    /// assigned value): an integer may go into a double, converted.
    pub(super) fn expect_flow(&mut self, found: &Ty, expected: &Ty, e: &Expr) -> IResult<()> {
        let span = e.span;
        if self.types.flow(found, expected) {
            return Ok(());
        }
        if self.dyn_cast(e, found, expected) {
            return Ok(());
        }
        if let (Ty::Num(_), Ty::Option(inner)) =
            (self.types.resolve(found), self.types.resolve(expected))
            && self.types.flow(found, &inner)
        {
            return Ok(());
        }
        self.expect(found, expected, span)
    }

    /// A value known at run time going where a type is known, or the other way: converted
    /// there (`false` when neither or both are dynamic, or a type is not known yet).
    pub(super) fn dyn_cast(&mut self, e: &Expr, found: &Ty, expected: &Ty) -> bool {
        let (f, x) = (self.types.resolve(found), self.types.resolve(expected));
        if f.is_dynamic() == x.is_dynamic() {
            return false;
        }
        if matches!(f, Ty::Var(_)) || matches!(x, Ty::Var(_)) {
            return false;
        }
        // Read as the expected type: through its JSON reading.
        if f.is_dynamic() {
            self.json_targets.push(x.clone());
        }
        self.dyn_casts
            .insert(e.id, (found.clone(), expected.clone()));
        true
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
                shares_with: None,
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
    /// `let` and `const` cannot declare a name twice in one scope (JavaScript refuses it).
    pub(super) fn check_redeclaration(&self, kind: VarKind, pat: &Pattern) -> IResult<()> {
        if kind == VarKind::Var {
            return Ok(());
        }
        let Some(scope) = self.scopes.last() else {
            return Ok(());
        };
        for (name, _) in pat.names() {
            if scope.vars.contains_key(name) {
                return self.err(
                    format!("`{name}` is already declared in this scope"),
                    pat.span(),
                );
            }
        }
        Ok(())
    }

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
                    Ty::Named(n, ..) => match self.struct_by_name(n) {
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
            || self.generic_aliases.contains_key(name)
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
        self.ancestry(class).iter().any(|c| {
            c == base
                || self
                    .struct_by_name(c)
                    .is_some_and(|s| s.implements.iter().any(|i| i == base))
        })
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
        let (Ty::Named(base, base_args), Ty::Named(class, ..)) = (target, found) else {
            return false;
        };
        if base == class {
            return false;
        }
        let Some(base_info) = self.struct_by_name(&base).cloned() else {
            return false;
        };
        if self
            .struct_by_name(&class)
            .is_none_or(|s| s.kind != StructKind::Class)
        {
            return false;
        }
        match base_info.kind {
            StructKind::Class => {
                if !self.is_subclass(&class, &base) {
                    return false;
                }
            }
            // An interface the class declares, or satisfies with its fields and methods.
            _ => {
                if !self.is_subclass(&class, &base) {
                    // A generic interface: its parameters as the expected type gives them.
                    let args: Vec<Ty> = if base_args.len() == base_info.type_params.len() {
                        base_args.clone()
                    } else {
                        base_info
                            .type_params
                            .iter()
                            .map(|_| self.types.fresh())
                            .collect()
                    };
                    let given: HashMap<String, Ty> = base_info
                        .type_params
                        .iter()
                        .cloned()
                        .zip(args.iter().cloned())
                        .collect();
                    if !self.satisfies(&class, &base_info, &given) {
                        return false;
                    }
                    if let Some(s) = self.struct_mut(&class) {
                        s.implements.push(base.clone());
                        if !args.is_empty() {
                            s.implements_args.push((base.clone(), args));
                        }
                    }
                } else if !base_args.is_empty() {
                    // Declared: the arguments it was declared with are those expected.
                    let declared = self
                        .ancestry(&class)
                        .iter()
                        .filter_map(|c| self.struct_by_name(c))
                        .find_map(|s| {
                            s.implements_args
                                .iter()
                                .find(|(i, _)| *i == base)
                                .map(|(_, a)| a.clone())
                        });
                    if let Some(declared) = declared {
                        for (x, y) in declared.iter().zip(&base_args) {
                            if self.types.unify(x, y).is_err() {
                                return false;
                            }
                        }
                    }
                }
            }
        }
        self.polymorphic.insert(base);
        true
    }

    /// Whether a class has every field and method of an interface, with the same types.
    fn satisfies(&mut self, class: &str, iface: &StructInfo, given: &HashMap<String, Ty>) -> bool {
        let sub = |this: &Self, t: &Ty| finish::substitute(t, given, &this.types);
        for f in &iface.fields {
            let Some(cf) = self.ancestry(class).iter().find_map(|c| {
                self.struct_by_name(c)
                    .and_then(|s| s.field(&f.name).cloned())
            }) else {
                return false;
            };
            let ft = sub(self, &f.ty);
            if cf.optional != f.optional || self.types.unify(&cf.ty, &ft).is_err() {
                return false;
            }
        }
        for m in &iface.methods {
            let Some((_, cm)) = self.find_method(class, &m.name, m.kind) else {
                return false;
            };
            let (a, b) = (
                self.funcs[&m.fn_id].sig.clone(),
                self.funcs[&cm.fn_id].sig.clone(),
            );
            if a.params.len() != b.params.len()
                || a.params.iter().zip(&b.params).any(|(x, y)| {
                    let x = sub(self, x);
                    self.types.unify(&x, y).is_err()
                })
                || {
                    let ret = sub(self, &a.ret);
                    self.types.unify(&ret, &b.ret).is_err()
                }
            {
                return false;
            }
            self.link_implementation(m.fn_id, cm.fn_id);
        }
        true
    }

    /// The method of a class implements one of an interface: the calls through the
    /// interface are dispatched to it.
    pub(super) fn link_implementation(&mut self, iface_fn: NodeId, class_fn: NodeId) {
        let params = self.funcs[&class_fn].params.clone();
        if let Some(f) = self.funcs.get_mut(&iface_fn)
            && f.params.is_empty()
        {
            f.params = params;
        }
        let list = self.overrides.entry(iface_fn).or_default();
        if !list.contains(&class_fn) {
            list.push(class_fn);
        }
    }

    /// The union a type names, if any.
    pub(super) fn union_of_ty(&self, ty: &Ty) -> Option<&UnionInfo> {
        match self.types.resolve(ty) {
            Ty::Named(n, ..) => self.union_by_name(&n),
            _ => None,
        }
    }

    pub(super) fn is_type_param(&self, name: &str) -> bool {
        self.fns
            .iter()
            .rev()
            .any(|f| f.type_params.iter().any(|t| t == name))
            || self
                .param_scope
                .iter()
                .any(|ps| ps.iter().any(|t| t == name))
    }

    /// The values an object with `[Symbol.iterator]()` gives to `for … of`: what its
    /// generator yields, or the `value` of what its iterator's `next()` returns.
    pub(super) fn struct_iteration(&self, ty: &Ty) -> Option<Ty> {
        let Ty::Named(n, _) = self.types.resolve(ty) else {
            return None;
        };
        let (_, m) = self.find_method(&n, "@@iterator", MethodKind::Method)?;
        let subst = self.instance_subst(ty);
        let ret = finish::substitute(&self.funcs[&m.fn_id].sig.ret, &subst, &self.types);
        match self.types.resolve(&ret) {
            Ty::Gen(t) => Some(*t),
            Ty::Named(it, _) => {
                let next = self.struct_by_name(&it)?.field("next")?.ty.clone();
                let Ty::Func(f) = self.types.resolve(&next) else {
                    return None;
                };
                let Ty::Named(r, _) = self.types.resolve(&f.ret) else {
                    return None;
                };
                Some(self.struct_by_name(&r)?.field("value")?.ty.clone())
            }
            _ => None,
        }
    }

    /// `T extends HasId`: the struct a type parameter in scope is constrained to.
    pub(super) fn constraint_of(&self, param: &str) -> Option<String> {
        for ctx in self.fns.iter().rev() {
            if let Some(f) = self.funcs.get(&ctx.id)
                && let Some((_, c)) = f.type_bounds.iter().find(|(p, _)| p == param)
            {
                return Some(c.clone());
            }
            if ctx.type_params.iter().any(|t| t == param) {
                return None;
            }
        }
        let class = self.this_class_stack.last()?;
        let s = self.struct_by_name(class)?;
        s.type_bounds
            .iter()
            .find(|(p, _)| p == param)
            .map(|(_, c)| c.clone())
    }

    /// Type arguments given to constrained parameters: checked at the end, and each type
    /// given gets the constraint's trait.
    pub(super) fn record_bound_uses(
        &mut self,
        bounds: &[(String, String)],
        subst: &HashMap<String, Ty>,
        span: Span,
    ) {
        for (p, c) in bounds {
            if let Some(t) = subst.get(p) {
                self.bound_uses.push((c.clone(), t.clone(), span));
            }
        }
    }

    /// The type arguments of a generic struct type, by parameter: `Stack<number>` gives
    /// `T → number`, and so does `NumberStack` for `class NumberStack extends
    /// Stack<number>`. Empty for other types, and inside a generic class itself.
    pub(super) fn instance_subst(&self, ty: &Ty) -> HashMap<String, Ty> {
        let mut out = HashMap::new();
        let Ty::Named(n, args) = self.types.resolve(ty) else {
            return out;
        };
        if let Some(s) = self.struct_by_name(&n) {
            for (p, a) in s.type_params.iter().zip(args) {
                out.insert(p.clone(), a);
            }
        }
        self.ancestor_subst(&n, out)
    }

    /// `own` (the parameters of `class`) with those of its generic ancestors, as each
    /// class gives them to its base.
    pub(super) fn ancestor_subst(
        &self,
        class: &str,
        own: HashMap<String, Ty>,
    ) -> HashMap<String, Ty> {
        let mut out = own;
        let mut current = class.to_string();
        let mut seen = HashSet::new();
        while seen.insert(current.clone()) {
            let Some(s) = self.struct_by_name(&current) else {
                break;
            };
            let Some(parent) = s.parent.clone() else {
                break;
            };
            if let Some(p) = self.struct_by_name(&parent) {
                let given: Vec<Ty> = s
                    .parent_args
                    .iter()
                    .map(|t| finish::substitute(t, &out, &self.types))
                    .collect();
                for (name, t) in p.type_params.iter().zip(given) {
                    out.entry(name.clone()).or_insert(t);
                }
            }
            current = parent;
        }
        out
    }

    /// The type of `this` in a class: `Stack<T>` in a generic one.
    pub(super) fn self_type(&self, class: &str) -> Ty {
        let params = self
            .struct_by_name(class)
            .map(|s| s.type_params.clone())
            .unwrap_or_default();
        Ty::Named(
            class.to_string(),
            params.into_iter().map(Ty::named).collect(),
        )
    }

    /// A fresh instance of a struct: `Stack<?>` with a new variable per parameter (the
    /// struct itself when it is not generic).
    pub(super) fn fresh_instance(&mut self, name: &str) -> (Ty, HashMap<String, Ty>) {
        let params = self
            .struct_by_name(name)
            .map(|s| s.type_params.clone())
            .unwrap_or_default();
        let mut subst = HashMap::new();
        let mut args = Vec::new();
        for p in params {
            let v = self.types.fresh();
            subst.insert(p, v.clone());
            args.push(v);
        }
        let bounds = self
            .struct_by_name(name)
            .map(|s| s.type_bounds.clone())
            .unwrap_or_default();
        self.record_bound_uses(&bounds, &subst, Span::default());
        (Ty::Named(name.to_string(), args), subst)
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
