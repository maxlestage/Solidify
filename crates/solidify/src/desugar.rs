//! Rewrites of older or more dynamic JavaScript into the forms the rest of Solidify reads,
//! run on each file once it is parsed:
//!
//! - constructor functions and their `prototype` (`function Person(name) { this.name =
//!   name }`, `Person.prototype.greet = function () { … }`, `Object.create`,
//!   `Parent.call(this, …)`) become classes;
//! - fields a class only assigns (`this.x = x` in the constructor of a JavaScript class)
//!   are declared, optional when the constructor does not always set them;
//! - `arguments` becomes a rest parameter (`function sum() { … arguments … }` is
//!   `function sum(...arguments)`).

use crate::ast::*;
use crate::error::Span;

/// Rewrites the statements of a file; new nodes take their ids from `next_id`.
pub fn desugar(body: &mut Vec<Stmt>, next_id: &mut NodeId) {
    let mut ids = Ids(next_id);
    constructor_functions(body, &mut ids);
    omitted_parameters(body);
    for_each_function(body, &mut |f| rest_arguments(f, &mut ids));
    for_each_class(body, &mut |c| implicit_fields(c));
}

struct Ids<'a>(&'a mut NodeId);

impl Ids<'_> {
    fn next(&mut self) -> NodeId {
        let id = *self.0;
        *self.0 += 1;
        id
    }

    fn expr(&mut self, kind: ExprKind, span: Span) -> Expr {
        Expr {
            kind,
            span,
            id: self.next(),
        }
    }
}

// ----- constructor functions -----

/// `F.prototype.name`, `F.prototype`, or `F.name` on the left of an assignment statement.
enum ProtoTarget {
    /// `F.prototype.m = …`
    Member(String, String),
    /// `F.prototype = Object.create(P.prototype)`
    Base(String, Option<String>),
    /// `F.prototype.constructor = F`
    Constructor(String),
    /// `F.m = …`: a static member.
    Static(String, String),
}

fn proto_target(s: &Stmt) -> Option<(ProtoTarget, &Expr)> {
    let StmtKind::Expr(Expr {
        kind:
            ExprKind::Assign {
                op: AssignOp::Assign,
                target: AssignTarget::Expr(t),
                value,
            },
        ..
    }) = &s.kind
    else {
        return None;
    };
    let ExprKind::Member { obj, prop, .. } = &t.kind else {
        return None;
    };
    match &obj.kind {
        // `F.prototype.x = …`
        ExprKind::Member {
            obj: f,
            prop: proto,
            ..
        } if proto == "prototype" => {
            let ExprKind::Ident(f) = &f.kind else {
                return None;
            };
            if prop == "constructor" {
                return Some((ProtoTarget::Constructor(f.clone()), value));
            }
            Some((ProtoTarget::Member(f.clone(), prop.clone()), value))
        }
        ExprKind::Ident(f) if prop == "prototype" => {
            // `Object.create(P.prototype)`
            let base = match &value.kind {
                ExprKind::Call { callee, args, .. } => match (&callee.kind, args.first()) {
                    (
                        ExprKind::Member {
                            obj: o,
                            prop: create,
                            ..
                        },
                        Some(a),
                    ) if create == "create"
                        && matches!(&o.kind, ExprKind::Ident(n) if n == "Object") =>
                    {
                        match &a.expr.kind {
                            ExprKind::Member {
                                obj: p, prop: pp, ..
                            } if pp == "prototype" => match &p.kind {
                                ExprKind::Ident(p) => Some(p.clone()),
                                _ => None,
                            },
                            _ => None,
                        }
                    }
                    _ => None,
                },
                _ => None,
            };
            Some((ProtoTarget::Base(f.clone(), base), value))
        }
        ExprKind::Ident(f) => Some((ProtoTarget::Static(f.clone(), prop.clone()), value)),
        _ => None,
    }
}

/// Whether a function body assigns `this.x` at its top level: a constructor.
fn assigns_this(f: &Function) -> bool {
    let FnBody::Block(b) = &f.body else {
        return false;
    };
    b.stmts.iter().any(|s| {
        matches!(&s.kind, StmtKind::Expr(Expr { kind: ExprKind::Assign { target: AssignTarget::Expr(t), .. }, .. })
            if matches!(&t.kind, ExprKind::Member { obj, .. } if matches!(obj.kind, ExprKind::This)))
    })
}

fn constructor_functions(body: &mut Vec<Stmt>, ids: &mut Ids) {
    // The functions written as constructors, with what the program attaches to them.
    let mut names: Vec<String> = Vec::new();
    for s in body.iter() {
        if let StmtKind::Func(f) = &s.kind
            && let Some(name) = &f.name
            && assigns_this(f)
        {
            names.push(name.clone());
        }
    }
    if names.is_empty() {
        return;
    }
    let mut used = std::collections::HashSet::new();
    for s in body.iter() {
        if let Some((t, _)) = proto_target(s) {
            match t {
                ProtoTarget::Member(f, _)
                | ProtoTarget::Base(f, _)
                | ProtoTarget::Constructor(f) => {
                    used.insert(f);
                }
                ProtoTarget::Static(_, _) => {}
            }
        }
    }
    // A constructor is also a function called with `new`.
    let mut newed = std::collections::HashSet::new();
    for_each_expr(body, &mut |e| {
        if let ExprKind::New { callee, .. } = &e.kind
            && let ExprKind::Ident(n) = &callee.kind
        {
            newed.insert(n.clone());
        }
        true
    });
    names.retain(|n| used.contains(n) || newed.contains(n));
    if names.is_empty() {
        return;
    }
    let mut members: std::collections::HashMap<String, Vec<ClassMember>> = Default::default();
    let mut bases: std::collections::HashMap<String, String> = Default::default();
    let mut kept = Vec::new();
    for s in std::mem::take(body) {
        let Some((t, value)) = proto_target(&s) else {
            kept.push(s);
            continue;
        };
        let owner = match &t {
            ProtoTarget::Member(f, _)
            | ProtoTarget::Base(f, _)
            | ProtoTarget::Constructor(f)
            | ProtoTarget::Static(f, _) => f.clone(),
        };
        if !names.contains(&owner) {
            kept.push(s);
            continue;
        }
        let value = value.clone();
        let is_static = matches!(t, ProtoTarget::Static(..));
        match t {
            ProtoTarget::Member(_, m) | ProtoTarget::Static(_, m) => {
                let member = match value.kind {
                    ExprKind::Func(func) | ExprKind::Arrow(func) => ClassMember::Method(Method {
                        name: m,
                        kind: MethodKind::Method,
                        func: *func,
                        is_static,
                        access: None,
                        is_abstract: false,
                        span: s.span,
                    }),
                    // A value shared by every instance: each has its own copy.
                    _ => ClassMember::Field(Field {
                        name: m,
                        ty: None,
                        init: Some(value),
                        is_static,
                        access: None,
                        readonly: false,
                        optional: false,
                        span: s.span,
                    }),
                };
                members.entry(owner).or_default().push(member);
            }
            ProtoTarget::Base(_, Some(base)) => {
                bases.insert(owner, base);
            }
            ProtoTarget::Base(_, None) | ProtoTarget::Constructor(_) => {}
        }
    }
    // The functions become classes, where they were declared.
    for s in &mut kept {
        let StmtKind::Func(f) = &s.kind else { continue };
        let Some(name) = f.name.clone() else { continue };
        if !names.contains(&name) {
            continue;
        }
        let StmtKind::Func(mut f) = std::mem::replace(&mut s.kind, StmtKind::Empty) else {
            unreachable!()
        };
        let base = bases.get(&name).cloned();
        // `Parent.call(this, a, b)` is `super(a, b)`.
        if let (Some(base), FnBody::Block(b)) = (&base, &mut f.body) {
            for st in &mut b.stmts {
                let StmtKind::Expr(e) = &mut st.kind else {
                    continue;
                };
                let ExprKind::Call { callee, args, .. } = &mut e.kind else {
                    continue;
                };
                let is_parent_call = matches!(&callee.kind, ExprKind::Member { obj, prop, .. }
                    if prop == "call" && matches!(&obj.kind, ExprKind::Ident(n) if n == base));
                if is_parent_call
                    && args
                        .first()
                        .is_some_and(|a| matches!(a.expr.kind, ExprKind::This))
                {
                    args.remove(0);
                    let span = callee.span;
                    **callee = ids.expr(ExprKind::Super, span);
                }
            }
        }
        let exported = f.exported;
        let span = f.span;
        let type_params = std::mem::take(&mut f.type_params);
        f.name = Some("constructor".into());
        let mut class_members = vec![ClassMember::Method(Method {
            name: "constructor".into(),
            kind: MethodKind::Constructor,
            func: f,
            is_static: false,
            access: None,
            is_abstract: false,
            span,
        })];
        class_members.extend(members.remove(&name).unwrap_or_default());
        let extends = base.map(|b| ids.expr(ExprKind::Ident(b), span));
        s.kind = StmtKind::Class(Class {
            name,
            type_params,
            extends,
            extends_args: Vec::new(),
            implements: Vec::new(),
            members: class_members,
            exported,
            is_abstract: false,
            span,
            id: ids.next(),
        });
    }
    // A subclass is declared after its base.
    let pos = |kept: &Vec<Stmt>, n: &str| {
        kept.iter()
            .position(|s| matches!(&s.kind, StmtKind::Class(c) if c.name == n))
    };
    for (sub, base) in &bases {
        if let (Some(i), Some(j)) = (pos(&kept, sub), pos(&kept, base))
            && i < j
        {
            let s = kept.remove(i);
            let j = pos(&kept, base).expect("base");
            kept.insert(j + 1, s);
        }
    }
    *body = kept;
}

// ----- omitted arguments -----

/// JavaScript lets a call leave out arguments, which are then `undefined`: a parameter
/// without a type that some call of the function leaves out is optional (`b?`).
fn omitted_parameters(body: &mut [Stmt]) {
    let mut fewest: std::collections::HashMap<String, usize> = Default::default();
    for_each_expr(body, &mut |e| {
        if let ExprKind::Call { callee, args, .. } = &e.kind
            && let ExprKind::Ident(n) = &callee.kind
            && !args.iter().any(|a| a.spread)
        {
            let k = fewest.entry(n.clone()).or_insert(usize::MAX);
            *k = (*k).min(args.len());
        }
        true
    });
    for s in body.iter_mut() {
        let StmtKind::Func(f) = &mut s.kind else {
            continue;
        };
        let Some(name) = &f.name else { continue };
        let Some(&given) = fewest.get(name) else {
            continue;
        };
        for p in f.params.iter_mut().skip(given) {
            if p.ty.is_none() && p.default.is_none() && !p.rest {
                p.optional = true;
            }
        }
    }
}

// ----- implicit fields -----

/// Declares the fields a class assigns (`this.x = …`) without declaring them.
fn implicit_fields(c: &mut Class) {
    let mut declared: Vec<String> = Vec::new();
    for m in &c.members {
        match m {
            ClassMember::Field(f) => declared.push(f.name.clone()),
            ClassMember::Method(m) => {
                declared.push(m.name.clone());
                if m.kind == MethodKind::Constructor {
                    for p in &m.func.params {
                        if (p.access.is_some() || p.readonly)
                            && let Pattern::Ident { name, .. } = &p.pat
                        {
                            declared.push(name.clone());
                        }
                    }
                }
            }
            ClassMember::StaticBlock(_) => {}
        }
    }
    // Set on every path of the constructor: its statements at the top level.
    let mut always: Vec<String> = Vec::new();
    let mut found: Vec<(String, Span)> = Vec::new();
    for m in &c.members {
        let ClassMember::Method(m) = m else { continue };
        if m.is_static {
            continue;
        }
        let FnBody::Block(b) = &m.func.body else {
            continue;
        };
        if m.kind == MethodKind::Constructor {
            for s in &b.stmts {
                if let Some(name) = this_assign(s) {
                    always.push(name);
                }
            }
        }
        for_each_expr_in(&b.stmts, &mut |e| {
            if let ExprKind::Assign {
                target: AssignTarget::Expr(t),
                ..
            } = &e.kind
                && let ExprKind::Member { obj, prop, .. } = &t.kind
                && matches!(obj.kind, ExprKind::This)
                && !found.iter().any(|(n, _)| n == prop)
            {
                found.push((prop.clone(), t.span));
            }
            // A nested function has its own `this`.
            !matches!(e.kind, ExprKind::Func(_) | ExprKind::Class(_))
        });
    }
    let mut fields = Vec::new();
    for (name, span) in found {
        if declared.contains(&name) {
            continue;
        }
        fields.push(ClassMember::Field(Field {
            optional: !always.contains(&name),
            name,
            ty: None,
            init: None,
            is_static: false,
            access: None,
            readonly: false,
            span,
        }));
    }
    c.members.splice(0..0, fields);
}

fn this_assign(s: &Stmt) -> Option<String> {
    let StmtKind::Expr(Expr {
        kind: ExprKind::Assign {
            target: AssignTarget::Expr(t),
            ..
        },
        ..
    }) = &s.kind
    else {
        return None;
    };
    match &t.kind {
        ExprKind::Member { obj, prop, .. } if matches!(obj.kind, ExprKind::This) => {
            Some(prop.clone())
        }
        _ => None,
    }
}

// ----- arguments -----

/// `arguments` in a function (not an arrow function, which sees its parent's): a rest
/// parameter of that name, or, beside declared parameters, `const arguments = [a, b,
/// ...more]`.
fn rest_arguments(f: &mut Function, ids: &mut Ids) {
    if f.is_arrow {
        return;
    }
    let FnBody::Block(b) = &f.body else { return };
    let mut uses = false;
    for_each_expr_in(&b.stmts, &mut |e| {
        if matches!(&e.kind, ExprKind::Ident(n) if n == "arguments") {
            uses = true;
        }
        // An arrow function sees this one's `arguments`; a function has its own.
        !matches!(e.kind, ExprKind::Func(_) | ExprKind::Class(_))
    });
    if !uses {
        return;
    }
    let span = f.span;
    let rest_param = |name: &str, ids: &mut Ids| Param {
        pat: Pattern::Ident {
            name: name.into(),
            span,
            id: ids.next(),
        },
        ty: None,
        optional: false,
        default: None,
        rest: true,
        access: None,
        readonly: false,
        span,
    };
    if f.params.is_empty() {
        f.params.push(rest_param("arguments", ids));
        return;
    }
    // Declared parameters (an optional one only when given), then the extra arguments.
    let mut items = Vec::new();
    for p in &f.params {
        let Pattern::Ident { name, .. } = &p.pat else {
            return;
        };
        let x = ids.expr(ExprKind::Ident(name.clone()), span);
        items.push(if p.rest {
            ArrayElem::Spread(x)
        } else if p.optional {
            // `...(b !== undefined ? [b] : [])`
            let test_x = ids.expr(ExprKind::Ident(name.clone()), span);
            let undef = ids.expr(ExprKind::Undefined, span);
            let test = ids.expr(
                ExprKind::Binary {
                    op: BinOp::StrictNotEq,
                    left: Box::new(test_x),
                    right: Box::new(undef),
                },
                span,
            );
            let one = ids.expr(ExprKind::Array(vec![ArrayElem::Expr(x)]), span);
            let none = ids.expr(ExprKind::Array(Vec::new()), span);
            ArrayElem::Spread(ids.expr(
                ExprKind::Cond {
                    test: Box::new(test),
                    cons: Box::new(one),
                    alt: Box::new(none),
                },
                span,
            ))
        } else {
            ArrayElem::Expr(x)
        });
    }
    if !f.params.last().is_some_and(|p| p.rest) {
        f.params.push(rest_param("__more", ids));
        let x = ids.expr(ExprKind::Ident("__more".into()), span);
        items.push(ArrayElem::Spread(x));
    }
    let init = ids.expr(ExprKind::Array(items), span);
    let decl = Stmt {
        kind: StmtKind::Var(VarDecl {
            kind: VarKind::Const,
            decls: vec![Declarator {
                pat: Pattern::Ident {
                    name: "arguments".into(),
                    span,
                    id: ids.next(),
                },
                ty: None,
                init: Some(init),
                span,
            }],
            exported: false,
        }),
        span,
        comments: Vec::new(),
    };
    if let FnBody::Block(b) = &mut f.body {
        b.stmts.insert(0, decl);
    }
}

// ----- walking -----

/// Calls `f` on every function of the statements, nested ones included (outer first).
fn for_each_function(stmts: &mut [Stmt], f: &mut dyn FnMut(&mut Function)) {
    for s in stmts {
        walk_stmt_fns(s, f);
    }
}

fn walk_stmt_fns(s: &mut Stmt, f: &mut dyn FnMut(&mut Function)) {
    match &mut s.kind {
        StmtKind::Func(func) => walk_fn(func, f),
        StmtKind::Class(c) => walk_class_fns(c, f),
        _ => {}
    }
    for_each_child_mut(s, &mut |e| {
        match &mut e.kind {
            ExprKind::Func(func) | ExprKind::Arrow(func) => walk_fn(func, f),
            ExprKind::Class(c) => walk_class_fns(c, f),
            _ => {}
        }
        // Functions inside are reached by `walk_fn`.
        !matches!(
            e.kind,
            ExprKind::Func(_) | ExprKind::Arrow(_) | ExprKind::Class(_)
        )
    });
}

fn walk_class_fns(c: &mut Class, f: &mut dyn FnMut(&mut Function)) {
    for m in &mut c.members {
        if let ClassMember::Method(m) = m {
            walk_fn(&mut m.func, f);
        }
    }
}

fn walk_fn(func: &mut Function, f: &mut dyn FnMut(&mut Function)) {
    f(func);
    match &mut func.body {
        FnBody::Block(b) => {
            for s in &mut b.stmts {
                walk_stmt_fns(s, f);
            }
        }
        FnBody::Expr(e) => {
            let mut one = Stmt {
                kind: StmtKind::Expr((**e).clone()),
                span: e.span,
                comments: Vec::new(),
            };
            walk_stmt_fns(&mut one, f);
            if let StmtKind::Expr(x) = one.kind {
                **e = x;
            }
        }
    }
}

/// Calls `f` on every class of the statements, nested ones included.
fn for_each_class(stmts: &mut [Stmt], f: &mut dyn FnMut(&mut Class)) {
    for s in stmts.iter_mut() {
        if let StmtKind::Class(c) = &mut s.kind {
            f(c);
        }
    }
    for_each_expr_mut(stmts, &mut |e| {
        if let ExprKind::Class(c) = &mut e.kind {
            f(c);
        }
        true
    });
}

/// Calls `f` on every expression (read-only), entering what `f` allows.
fn for_each_expr(stmts: &[Stmt], f: &mut dyn FnMut(&Expr) -> bool) {
    let mut copy = stmts.to_vec();
    for_each_expr_mut(&mut copy, &mut |e| f(e));
}

fn for_each_expr_in(stmts: &[Stmt], f: &mut dyn FnMut(&Expr) -> bool) {
    for_each_expr(stmts, f);
}

/// Calls `f` on every expression of the statements; `f` returns whether to enter it.
pub fn for_each_expr_mut(stmts: &mut [Stmt], f: &mut dyn FnMut(&mut Expr) -> bool) {
    for s in stmts {
        for_each_child_mut(s, f);
        // Nested statements of functions and classes declared as statements.
        match &mut s.kind {
            StmtKind::Func(func) => fn_body_mut(func, f),
            StmtKind::Class(c) => class_mut(c, f),
            _ => {}
        }
    }
}

fn fn_body_mut(func: &mut Function, f: &mut dyn FnMut(&mut Expr) -> bool) {
    for p in &mut func.params {
        if let Some(d) = &mut p.default {
            expr_mut(d, f);
        }
    }
    match &mut func.body {
        FnBody::Block(b) => for_each_expr_mut(&mut b.stmts, f),
        FnBody::Expr(e) => expr_mut(e, f),
    }
}

fn class_mut(c: &mut Class, f: &mut dyn FnMut(&mut Expr) -> bool) {
    if let Some(e) = &mut c.extends {
        expr_mut(e, f);
    }
    for m in &mut c.members {
        match m {
            ClassMember::Field(fl) => {
                if let Some(i) = &mut fl.init {
                    expr_mut(i, f);
                }
            }
            ClassMember::Method(m) => fn_body_mut(&mut m.func, f),
            ClassMember::StaticBlock(b) => for_each_expr_mut(&mut b.stmts, f),
        }
    }
}

/// The expressions directly in a statement (and in its nested statements).
fn for_each_child_mut(s: &mut Stmt, f: &mut dyn FnMut(&mut Expr) -> bool) {
    match &mut s.kind {
        StmtKind::Var(v) => {
            for d in &mut v.decls {
                if let Some(i) = &mut d.init {
                    expr_mut(i, f);
                }
            }
        }
        StmtKind::Return(Some(e)) | StmtKind::Throw(e) | StmtKind::Expr(e) => expr_mut(e, f),
        StmtKind::ExportDefault(e) => expr_mut(e, f),
        StmtKind::If {
            test,
            then,
            otherwise,
        } => {
            expr_mut(test, f);
            for_each_expr_mut(std::slice::from_mut(then), f);
            if let Some(o) = otherwise {
                for_each_expr_mut(std::slice::from_mut(o), f);
            }
        }
        StmtKind::While { test, body } | StmtKind::DoWhile { body, test } => {
            expr_mut(test, f);
            for_each_expr_mut(std::slice::from_mut(body), f);
        }
        StmtKind::For {
            init,
            test,
            update,
            body,
        } => {
            if let Some(i) = init {
                for_each_expr_mut(std::slice::from_mut(i), f);
            }
            if let Some(t) = test {
                expr_mut(t, f);
            }
            if let Some(u) = update {
                expr_mut(u, f);
            }
            for_each_expr_mut(std::slice::from_mut(body), f);
        }
        StmtKind::ForOf { iter, body, .. } => {
            expr_mut(iter, f);
            for_each_expr_mut(std::slice::from_mut(body), f);
        }
        StmtKind::Switch { disc, cases } => {
            expr_mut(disc, f);
            for c in cases {
                if let Some(t) = &mut c.test {
                    expr_mut(t, f);
                }
                for_each_expr_mut(&mut c.body, f);
            }
        }
        StmtKind::Try {
            block,
            catch,
            finally,
        } => {
            for_each_expr_mut(&mut block.stmts, f);
            if let Some(c) = catch {
                for_each_expr_mut(&mut c.body.stmts, f);
            }
            if let Some(b) = finally {
                for_each_expr_mut(&mut b.stmts, f);
            }
        }
        StmtKind::Block(b) => for_each_expr_mut(&mut b.stmts, f),
        StmtKind::Labeled { body, .. } => for_each_expr_mut(std::slice::from_mut(body), f),
        _ => {}
    }
}

/// `f` on `e`, then (when it says so) on the expressions inside.
pub fn expr_mut(e: &mut Expr, f: &mut dyn FnMut(&mut Expr) -> bool) {
    if !f(e) {
        return;
    }
    match &mut e.kind {
        ExprKind::Template { exprs, .. } => exprs.iter_mut().for_each(|x| expr_mut(x, f)),
        ExprKind::Tagged { tag, exprs, .. } => {
            expr_mut(tag, f);
            exprs.iter_mut().for_each(|x| expr_mut(x, f));
        }
        ExprKind::Array(items) => {
            for it in items {
                match it {
                    ArrayElem::Expr(x) | ArrayElem::Spread(x) => expr_mut(x, f),
                    ArrayElem::Hole => {}
                }
            }
        }
        ExprKind::Object(props) => {
            for p in props {
                if let PropKey::Computed(k) = &mut p.key {
                    expr_mut(k, f);
                }
                match &mut p.value {
                    PropValue::Expr(x) | PropValue::Spread(x) => expr_mut(x, f),
                    PropValue::Method(func) | PropValue::Get(func) | PropValue::Set(func) => {
                        fn_body_mut(func, f)
                    }
                    PropValue::Shorthand(_) => {}
                }
            }
        }
        ExprKind::Func(func) | ExprKind::Arrow(func) => fn_body_mut(func, f),
        ExprKind::Class(c) => class_mut(c, f),
        ExprKind::Unary { arg, .. } | ExprKind::Update { arg, .. } => expr_mut(arg, f),
        ExprKind::Binary { left, right, .. } | ExprKind::Logical { left, right, .. } => {
            expr_mut(left, f);
            expr_mut(right, f);
        }
        ExprKind::Assign { target, value, .. } => {
            if let AssignTarget::Expr(t) = target {
                expr_mut(t, f);
            }
            expr_mut(value, f);
        }
        ExprKind::Cond { test, cons, alt } => {
            expr_mut(test, f);
            expr_mut(cons, f);
            expr_mut(alt, f);
        }
        ExprKind::Call { callee, args, .. } | ExprKind::New { callee, args, .. } => {
            expr_mut(callee, f);
            args.iter_mut().for_each(|a| expr_mut(&mut a.expr, f));
        }
        ExprKind::Member { obj, .. } => expr_mut(obj, f),
        ExprKind::Index { obj, index, .. } => {
            expr_mut(obj, f);
            expr_mut(index, f);
        }
        ExprKind::As(x, _)
        | ExprKind::NonNull(x)
        | ExprKind::TypeOf(x)
        | ExprKind::Void(x)
        | ExprKind::Delete(x)
        | ExprKind::Await(x) => expr_mut(x, f),
        ExprKind::Yield { value: Some(v), .. } => expr_mut(v, f),
        ExprKind::Seq(xs) => xs.iter_mut().for_each(|x| expr_mut(x, f)),
        _ => {}
    }
}
