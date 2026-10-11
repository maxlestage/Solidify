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
pub fn desugar(body: &mut Vec<Stmt>, next_id: &mut NodeId) -> Option<String> {
    plain_syntax(body, next_id);
    static_state(body, next_id);
    let default_export = common_js(body, next_id);
    eval_declarations(body, next_id);
    for_each_function(body, &mut |f| {
        if let FnBody::Block(b) = &mut f.body {
            eval_declarations(&mut b.stmts, next_id);
        }
    });
    literal_code(body, next_id);
    error_classes(body, next_id);
    object_methods(body, next_id);
    flatten_patterns(body, next_id);
    proxy_targets(body);
    let mut ids = Ids(next_id);
    constructor_functions(body, &mut ids);
    omitted_parameters(body);
    for_each_function(body, &mut |f| rest_arguments(f, &mut ids));
    for_each_class(body, &mut |c| implicit_fields(c));
    default_export
}

// ----- syntax with a plainer form -----

/// Tagged templates become calls, `const A = class { … }` a class declaration, and
/// static blocks the statements after their class.
fn plain_syntax(body: &mut Vec<Stmt>, next_id: &mut NodeId) {
    let mut ids = Ids(next_id);
    // `` tag`a${x}b` `` is `tag(["a", "b"], x)`.
    for_each_expr_mut(body, &mut |e| {
        if let ExprKind::Tagged { .. } = e.kind {
            let ExprKind::Tagged { tag, quasis, exprs } =
                std::mem::replace(&mut e.kind, ExprKind::Undefined)
            else {
                unreachable!()
            };
            let span = e.span;
            let strings: Vec<ArrayElem> = quasis
                .into_iter()
                .map(|q| ArrayElem::Expr(ids.expr(ExprKind::Str(q), span)))
                .collect();
            let mut args = vec![Arg {
                expr: ids.expr(ExprKind::Array(strings), span),
                spread: false,
            }];
            args.extend(exprs.into_iter().map(|expr| Arg {
                expr,
                spread: false,
            }));
            e.kind = ExprKind::Call {
                callee: tag,
                args,
                optional: false,
                type_args: Vec::new(),
            };
        }
        true
    });
    each_list(body, &mut class_statements);
    for_each_function(body, &mut |func| {
        if let FnBody::Block(b) = &mut func.body {
            each_list(&mut b.stmts, &mut class_statements);
        }
    });
}

/// Calls `f` on a list of statements and on every list nested in its statements (not in
/// functions).
fn each_list(stmts: &mut Vec<Stmt>, f: &mut dyn FnMut(&mut Vec<Stmt>)) {
    f(stmts);
    for s in stmts.iter_mut() {
        each_list_in(s, f);
    }
}

fn each_list_in(s: &mut Stmt, f: &mut dyn FnMut(&mut Vec<Stmt>)) {
    match &mut s.kind {
        StmtKind::Block(b) => each_list(&mut b.stmts, f),
        StmtKind::If {
            then, otherwise, ..
        } => {
            each_list_in(then, f);
            if let Some(o) = otherwise {
                each_list_in(o, f);
            }
        }
        StmtKind::While { body, .. }
        | StmtKind::DoWhile { body, .. }
        | StmtKind::For { body, .. }
        | StmtKind::ForOf { body, .. }
        | StmtKind::Labeled { body, .. } => each_list_in(body, f),
        StmtKind::Switch { cases, .. } => {
            for c in cases {
                each_list(&mut c.body, f);
            }
        }
        StmtKind::Try {
            block,
            catch,
            finally,
        } => {
            each_list(&mut block.stmts, f);
            if let Some(c) = catch {
                each_list(&mut c.body.stmts, f);
            }
            if let Some(b) = finally {
                each_list(&mut b.stmts, f);
            }
        }
        _ => {}
    }
}

/// `const A = class { … }` is `class A { … }`; a class's `static { … }` blocks run right
/// after it, with `this` the class.
fn class_statements(stmts: &mut Vec<Stmt>) {
    let mut i = 0;
    while i < stmts.len() {
        if let StmtKind::Var(v) = &mut stmts[i].kind
            && v.decls.len() == 1
            && let Pattern::Ident { name, .. } = &v.decls[0].pat
            && let Some(Expr {
                kind: ExprKind::Class(_),
                ..
            }) = &v.decls[0].init
        {
            let name = name.clone();
            let exported = v.exported;
            let Some(Expr {
                kind: ExprKind::Class(mut c),
                ..
            }) = v.decls[0].init.take()
            else {
                unreachable!()
            };
            c.name = name;
            c.exported = exported;
            stmts[i].kind = StmtKind::Class(*c);
        }
        if let StmtKind::Class(c) = &mut stmts[i].kind
            && c.members
                .iter()
                .any(|m| matches!(m, ClassMember::StaticBlock(_)))
        {
            let class = c.name.clone();
            let mut blocks = Vec::new();
            c.members.retain_mut(|m| match m {
                ClassMember::StaticBlock(b) => {
                    blocks.push(std::mem::replace(
                        b,
                        Block {
                            stmts: Vec::new(),
                            span: b.span,
                        },
                    ));
                    false
                }
                _ => true,
            });
            for (k, mut b) in blocks.into_iter().enumerate() {
                for_each_expr_mut(&mut b.stmts, &mut |e| {
                    if matches!(e.kind, ExprKind::This) {
                        e.kind = ExprKind::Ident(class.clone());
                    }
                    // `this` in a function of its own is another one.
                    !matches!(e.kind, ExprKind::Func(_) | ExprKind::Class(_))
                });
                let span = b.span;
                stmts.insert(
                    i + 1 + k,
                    Stmt {
                        kind: StmtKind::Block(b),
                        span,
                        comments: Vec::new(),
                    },
                );
            }
        }
        i += 1;
    }
}

/// Static fields of top-level classes that change, or hold more than a literal: variables
/// of the module, `C.f` and `this.f` in static methods read and write them.
fn static_state(body: &mut Vec<Stmt>, next_id: &mut NodeId) {
    use std::collections::{HashMap, HashSet};
    let mut parents: HashMap<String, Option<String>> = HashMap::new();
    let mut fields: HashMap<String, Vec<String>> = HashMap::new();
    for s in body.iter() {
        if let StmtKind::Class(c) = &s.kind {
            let parent = match c.extends.as_ref().map(|e| &e.kind) {
                Some(ExprKind::Ident(p)) => Some(p.clone()),
                _ => None,
            };
            parents.insert(c.name.clone(), parent);
            let statics: Vec<String> = c
                .members
                .iter()
                .filter_map(|m| match m {
                    ClassMember::Field(f) if f.is_static => Some(f.name.clone()),
                    _ => None,
                })
                .collect();
            if !statics.is_empty() {
                fields.insert(c.name.clone(), statics);
            }
        }
    }
    if fields.is_empty() {
        return;
    }
    // The class declaring `f` for `X.f`: X or one of its bases.
    let owner = |x: &str, f: &str| -> Option<String> {
        let mut c = Some(x.to_string());
        let mut steps = 0;
        while let Some(name) = c {
            if fields
                .get(&name)
                .is_some_and(|fs| fs.iter().any(|g| g == f))
            {
                return Some(name);
            }
            c = parents.get(&name).cloned().flatten();
            steps += 1;
            if steps > 64 {
                break;
            }
        }
        None
    };
    // `this.f` in a static method is `C.f`.
    for s in body.iter_mut() {
        let StmtKind::Class(c) = &mut s.kind else {
            continue;
        };
        let class = c.name.clone();
        for m in &mut c.members {
            let ClassMember::Method(m) = m else { continue };
            if !m.is_static {
                continue;
            }
            fn_body_mut(&mut m.func, &mut |e| {
                if let ExprKind::Member { obj, prop, .. } = &mut e.kind
                    && matches!(obj.kind, ExprKind::This)
                    && owner(&class, prop).is_some()
                {
                    obj.kind = ExprKind::Ident(class.clone());
                }
                !matches!(e.kind, ExprKind::Func(_) | ExprKind::Class(_))
            });
        }
    }
    // Which ones change.
    let mut changed: HashSet<(String, String)> = HashSet::new();
    for_each_expr(body, &mut |e| {
        let target = match &e.kind {
            ExprKind::Assign {
                target: AssignTarget::Expr(t),
                ..
            } => Some(&**t),
            ExprKind::Update { arg, .. } => Some(&**arg),
            _ => None,
        };
        if let Some(Expr {
            kind: ExprKind::Member { obj, prop, .. },
            ..
        }) = target
            && let ExprKind::Ident(x) = &obj.kind
            && let Some(o) = owner(x, prop)
        {
            changed.insert((o, prop.clone()));
        }
        true
    });
    let literal = |e: &Expr| match &e.kind {
        ExprKind::Num { .. } | ExprKind::Str(_) | ExprKind::Bool(_) => true,
        ExprKind::Unary {
            op: UnaryOp::Neg,
            arg,
        } => matches!(arg.kind, ExprKind::Num { .. }),
        _ => false,
    };
    let taken: HashSet<String> = body.iter().flat_map(declared_names).collect();
    let mut vars: HashMap<(String, String), String> = HashMap::new();
    let mut ids = Ids(next_id);
    let mut i = 0;
    while i < body.len() {
        let StmtKind::Class(c) = &mut body[i].kind else {
            i += 1;
            continue;
        };
        let class = c.name.clone();
        let exported = c.exported;
        let mut moved = Vec::new();
        c.members.retain_mut(|m| {
            let ClassMember::Field(f) = m else {
                return true;
            };
            let is_changed = changed.contains(&(class.clone(), f.name.clone()));
            let keep = !f.is_static
                || match &f.init {
                    Some(init) => literal(init) && !is_changed,
                    None => !is_changed,
                };
            if !keep {
                moved.push(f.clone());
            }
            keep
        });
        let mut at = i + 1;
        for f in moved {
            let mut first = class.chars();
            let base = first
                .next()
                .map(|h| h.to_lowercase().collect::<String>() + first.as_str())
                .unwrap_or_default();
            let mut fc = f.name.chars();
            let field = fc
                .next()
                .map(|h| h.to_uppercase().collect::<String>() + fc.as_str())
                .unwrap_or_default();
            let mut name = format!("{base}{field}");
            while taken.contains(&name) {
                name.push_str("Static");
            }
            vars.insert((class.clone(), f.name.clone()), name.clone());
            let span = f.span;
            let init = f.init;
            let id = ids.next();
            body.insert(
                at,
                Stmt {
                    kind: StmtKind::Var(VarDecl {
                        kind: VarKind::Let,
                        decls: vec![Declarator {
                            pat: Pattern::Ident { name, span, id },
                            ty: f.ty,
                            init,
                            span,
                        }],
                        exported,
                    }),
                    span,
                    comments: Vec::new(),
                },
            );
            at += 1;
        }
        i = at;
    }
    if vars.is_empty() {
        return;
    }
    for_each_expr_mut(body, &mut |e| {
        if let ExprKind::Member { obj, prop, .. } = &e.kind
            && let ExprKind::Ident(x) = &obj.kind
            && let Some(o) = owner(x, prop)
            && let Some(v) = vars.get(&(o, prop.clone()))
        {
            e.kind = ExprKind::Ident(v.clone());
            return false;
        }
        true
    });
}

/// The names a top-level statement declares.
fn declared_names(s: &Stmt) -> Vec<String> {
    match &s.kind {
        StmtKind::Var(v) => v
            .decls
            .iter()
            .flat_map(|d| d.pat.names().into_iter().map(|(n, _)| n.to_string()))
            .collect(),
        StmtKind::Func(f) => f.name.clone().into_iter().collect(),
        StmtKind::Class(c) => vec![c.name.clone()],
        _ => Vec::new(),
    }
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
pub fn for_each_expr(stmts: &[Stmt], f: &mut dyn FnMut(&Expr) -> bool) {
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

/// `eval("1 + x")` and `new Function("a", "b", "return a + b")` with code written in the
/// program: the code itself, translated with the rest (`eval` of an expression is that
/// expression; of statements, a function called at once that gives the value of the last
/// one; `new Function` is an arrow function).
fn literal_code(body: &mut [Stmt], next_id: &mut NodeId) {
    for_each_expr_mut(body, &mut |e| {
        if let Some(code) = literal_code_of(e)
            && let Ok(mut stmts) = crate::parser::parse_snippet(&code, e.span, next_id)
        {
            literal_code(&mut stmts, next_id);
            // An immediately called arrow function: `(() => { … })()`.
            if let Some(Stmt {
                kind: StmtKind::Expr(x),
                ..
            }) = stmts.first_mut()
                && let ExprKind::Call { callee, .. } = &mut x.kind
                && let ExprKind::Arrow(f) = &mut callee.kind
                && let FnBody::Block(b) = &mut f.body
                && let Some(last) = b.stmts.last_mut()
                && let StmtKind::Expr(v) = &mut last.kind
            {
                let v = std::mem::replace(v, placeholder());
                last.kind = StmtKind::Return(Some(v));
            }
            if let [
                Stmt {
                    kind: StmtKind::Expr(x),
                    ..
                },
            ] = stmts.as_mut_slice()
            {
                *e = std::mem::replace(x, placeholder());
                return true;
            }
        }
        true
    });
}

/// `eval("var x = 1; function f() { … }")` as a statement: its `var`s and functions are
/// declared where the `eval` is, like in JavaScript.
fn eval_declarations(stmts: &mut Vec<Stmt>, next_id: &mut NodeId) {
    let mut i = 0;
    while i < stmts.len() {
        let code = match &stmts[i].kind {
            StmtKind::Expr(Expr {
                kind: ExprKind::Call { callee, args, .. },
                ..
            }) if matches!(&callee.kind, ExprKind::Ident(n) if n == "eval") && args.len() == 1 => {
                match &args[0].expr.kind {
                    ExprKind::Str(s) => Some(s.clone()),
                    _ => None,
                }
            }
            _ => None,
        };
        let parsed =
            code.and_then(|c| crate::parser::parse_snippet(&c, stmts[i].span, next_id).ok());
        let declares = |s: &Stmt| {
            matches!(
                s.kind,
                StmtKind::Var(VarDecl {
                    kind: VarKind::Var,
                    ..
                }) | StmtKind::Func(_)
            )
        };
        let block_scoped = |s: &Stmt| {
            matches!(
                s.kind,
                StmtKind::Var(VarDecl {
                    kind: VarKind::Let | VarKind::Const,
                    ..
                }) | StmtKind::Class(_)
            )
        };
        match parsed {
            Some(new) if new.iter().any(declares) && !new.iter().any(block_scoped) => {
                let n = new.len();
                stmts.splice(i..=i, new);
                i += n;
            }
            _ => i += 1,
        }
    }
}

/// The code of `eval("…")` (as a function called at once when it is not a single
/// expression) or of `new Function(…)` (as an arrow function), all of it written in the
/// program.
fn literal_code_of(e: &Expr) -> Option<String> {
    let strings = |args: &[Arg]| -> Option<Vec<String>> {
        args.iter()
            .map(|a| match &a.expr.kind {
                ExprKind::Str(s) if !a.spread => Some(s.clone()),
                ExprKind::Template { quasis, exprs } if exprs.is_empty() && !a.spread => {
                    Some(quasis.concat())
                }
                _ => None,
            })
            .collect()
    };
    match &e.kind {
        ExprKind::Call { callee, args, .. }
            if matches!(&callee.kind, ExprKind::Ident(n) if n == "eval") && args.len() == 1 =>
        {
            let code = strings(args)?.pop()?;
            // A single expression stays one; statements run in a function.
            let trimmed = code.trim().trim_end_matches(';');
            let mut probe_id = 0;
            let single = matches!(
                crate::parser::parse_snippet(trimmed, e.span, &mut probe_id).as_deref(),
                Ok([Stmt {
                    kind: StmtKind::Expr(_),
                    ..
                }])
            ) && !trimmed.starts_with('{');
            Some(if single {
                format!("({trimmed})")
            } else {
                format!("(() => {{\n{code}\n}})()")
            })
        }
        ExprKind::Call { callee, args, .. } | ExprKind::New { callee, args, .. } if matches!(&callee.kind, ExprKind::Ident(n) if n == "Function") =>
        {
            let mut parts = strings(args)?;
            let body = parts.pop().unwrap_or_default();
            Some(format!("(({}) => {{\n{body}\n}})", parts.join(", ")))
        }
        _ => None,
    }
}

fn placeholder() -> Expr {
    Expr {
        kind: ExprKind::Undefined,
        span: Span::default(),
        id: 0,
    }
}

/// A variable given to `new Proxy(…)` is shared with the proxy: it holds a value known at
/// run time (changed through the proxy, changed for it too).
fn proxy_targets(body: &mut [Stmt]) {
    let mut names = Vec::new();
    for_each_expr(body, &mut |e| {
        if let ExprKind::New { callee, args, .. } = &e.kind
            && matches!(&callee.kind, ExprKind::Ident(n) if n == "Proxy")
            && let Some(ExprKind::Ident(target)) = args.first().map(|a| &a.expr.kind)
        {
            names.push(target.clone());
        }
        true
    });
    if names.is_empty() {
        return;
    }
    fn mark(stmts: &mut [Stmt], names: &[String]) {
        for s in stmts {
            match &mut s.kind {
                StmtKind::Var(v) => {
                    for d in &mut v.decls {
                        if let Pattern::Ident { name, span, .. } = &d.pat
                            && names.contains(name)
                            && d.ty.is_none()
                        {
                            d.ty = Some(TypeAnn::Named {
                                name: crate::types::DYNAMIC.to_string(),
                                args: Vec::new(),
                                span: *span,
                            });
                        }
                    }
                }
                StmtKind::Block(b) => mark(&mut b.stmts, names),
                StmtKind::Func(f) => {
                    if let FnBody::Block(b) = &mut f.body {
                        mark(&mut b.stmts, names);
                    }
                }
                _ => {}
            }
        }
    }
    mark(body, &names);
}

/// CommonJS: `const x = require("m")`, `const { a } = require("m")` become imports;
/// `module.exports = …` and `exports.x = …` become exports. The name exported by
/// `module.exports = name` is given back (the default export).
fn common_js(body: &mut Vec<Stmt>, next_id: &mut NodeId) -> Option<String> {
    let mut default = None;
    let mut out = Vec::with_capacity(body.len());
    for mut s in std::mem::take(body) {
        let span = s.span;
        let import =
            |default: Option<String>, namespace: Option<String>, names, from: String| Stmt {
                kind: StmtKind::Import(Import {
                    default,
                    namespace,
                    names,
                    from,
                    type_only: false,
                }),
                span,
                comments: Vec::new(),
            };
        match &mut s.kind {
            // `const x = require("m")`, `const { a, b: c } = require("m")`.
            StmtKind::Var(v)
                if v.decls.len() == 1 && require_of(v.decls[0].init.as_ref()).is_some() =>
            {
                let from = require_of(v.decls[0].init.as_ref()).expect("require");
                match &v.decls[0].pat {
                    Pattern::Ident { name, .. } => {
                        out.push(import(Some(name.clone()), None, Vec::new(), from))
                    }
                    Pattern::Object { props, .. } => {
                        let mut names = Vec::new();
                        for p in props {
                            let local = match &p.value {
                                Pattern::Ident { name, .. } => name.clone(),
                                _ => p.key.clone(),
                            };
                            let alias = (local != p.key).then_some(local);
                            names.push((p.key.clone(), alias));
                        }
                        out.push(import(None, None, names, from));
                    }
                    _ => out.push(s),
                }
                continue;
            }
            // `require("m")`: for what it does.
            StmtKind::Expr(e) if require_of(Some(e)).is_some() => {
                let from = require_of(Some(e)).expect("require");
                out.push(import(None, None, Vec::new(), from));
                continue;
            }
            StmtKind::Expr(Expr {
                kind:
                    ExprKind::Assign {
                        op: AssignOp::Assign,
                        target: AssignTarget::Expr(t),
                        value,
                    },
                ..
            }) => {
                // `module.exports = …`
                if is_path(t, &["module", "exports"]) {
                    match &value.kind {
                        ExprKind::Ident(name) => default = Some(name.clone()),
                        ExprKind::Object(props) => {
                            let mut names = Vec::new();
                            for p in props {
                                let Some(key) = p.key.name() else { continue };
                                match &p.value {
                                    PropValue::Shorthand(_) => names.push((key, None)),
                                    PropValue::Expr(Expr {
                                        kind: ExprKind::Ident(local),
                                        ..
                                    }) => {
                                        names.push((local.clone(), (*local != key).then_some(key)));
                                    }
                                    PropValue::Expr(x) => {
                                        out.push(export_const(&key, x.clone(), span, next_id));
                                    }
                                    _ => {}
                                }
                            }
                            out.push(Stmt {
                                kind: StmtKind::ExportNames(names),
                                span,
                                comments: Vec::new(),
                            });
                        }
                        // `module.exports = function f() {}`, `= class {}`…: a name for it.
                        _ => {
                            let name = "moduleExports".to_string();
                            out.push(export_const(&name, (**value).clone(), span, next_id));
                            default = Some(name);
                        }
                    }
                    continue;
                }
                // `exports.x = …`, `module.exports.x = …`
                if let ExprKind::Member { obj, prop, .. } = &t.kind
                    && (is_path(obj, &["exports"]) || is_path(obj, &["module", "exports"]))
                {
                    out.push(export_const(prop, (**value).clone(), span, next_id));
                    continue;
                }
            }
            _ => {}
        }
        out.push(s);
    }
    // `require("m")` inside an expression: an import of its own, read by name.
    let mut hoisted: Vec<(String, String)> = Vec::new();
    for_each_expr_mut(&mut out, &mut |e| {
        if let Some(from) = require_of(Some(e)) {
            let local = format!(
                "required_{}",
                from.chars()
                    .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
                    .collect::<String>()
            );
            if !hoisted.iter().any(|(_, l)| *l == local) {
                hoisted.push((from, local.clone()));
            }
            e.kind = ExprKind::Ident(local);
            return false;
        }
        true
    });
    let mut all: Vec<Stmt> = hoisted
        .into_iter()
        .map(|(from, local)| Stmt {
            kind: StmtKind::Import(Import {
                default: Some(local),
                namespace: None,
                names: Vec::new(),
                from,
                type_only: false,
            }),
            span: Span::default(),
            comments: Vec::new(),
        })
        .collect();
    all.extend(out);
    *body = all;
    default
}

/// `require("m")`: the module name.
fn require_of(e: Option<&Expr>) -> Option<String> {
    let ExprKind::Call { callee, args, .. } = &e?.kind else {
        return None;
    };
    if !matches!(&callee.kind, ExprKind::Ident(n) if n == "require") || args.len() != 1 {
        return None;
    }
    match &args[0].expr.kind {
        ExprKind::Str(s) => Some(s.clone()),
        _ => None,
    }
}

/// Whether `e` is the dotted path `parts` (`module.exports`).
fn is_path(e: &Expr, parts: &[&str]) -> bool {
    match (&e.kind, parts) {
        (ExprKind::Ident(n), [only]) => n == only,
        (ExprKind::Member { obj, prop, .. }, [rest @ .., last]) if !rest.is_empty() => {
            prop == last && is_path(obj, rest)
        }
        _ => false,
    }
}

/// `export const name = value`.
fn export_const(name: &str, value: Expr, span: Span, next_id: &mut NodeId) -> Stmt {
    let id = *next_id;
    *next_id += 1;
    Stmt {
        kind: StmtKind::Var(VarDecl {
            kind: VarKind::Const,
            decls: vec![Declarator {
                pat: Pattern::Ident {
                    name: name.to_string(),
                    span,
                    id,
                },
                ty: None,
                init: Some(value),
                span,
            }],
            exported: true,
        }),
        span,
        comments: Vec::new(),
    }
}

/// The name a class extending `Error` (or `TypeError`…) extends instead: such a class
/// holds its `message` and `name` itself, and is thrown like an error.
pub const ERROR_BASE: &str = "__JsErrorBase";

/// `class NotFound extends Error { constructor(k) { super(`no ${k}`); … } }`: fields
/// `message` and `name` of its own, `super(m)` setting the message.
fn error_classes(body: &mut [Stmt], next_id: &mut NodeId) {
    const BASES: [&str; 6] = [
        "Error",
        "TypeError",
        "RangeError",
        "SyntaxError",
        "ReferenceError",
        "EvalError",
    ];
    for_each_class(body, &mut |c| {
        let Some(Expr {
            kind: ExprKind::Ident(base),
            span,
            ..
        }) = &c.extends
        else {
            return;
        };
        if !BASES.contains(&base.as_str()) {
            return;
        }
        let (base, span) = (base.clone(), *span);
        let has = |c: &Class, n: &str| {
            c.members
                .iter()
                .any(|m| matches!(m, ClassMember::Field(f) if f.name == n))
        };
        let mut extra = String::new();
        if !has(c, "message") {
            extra.push_str("message: string = \"\";\n");
        }
        if !has(c, "name") {
            extra.push_str(&format!("name: string = \"{base}\";\n"));
        }
        let has_ctor = c
            .members
            .iter()
            .any(|m| matches!(m, ClassMember::Method(m) if m.kind == MethodKind::Constructor));
        if !has_ctor {
            extra.push_str("constructor(message: string = \"\") { this.message = message; }\n");
        }
        let code = format!("class Snippet {{\n{extra}}}");
        let Ok(stmts) = crate::parser::parse_snippet(&code, span, next_id) else {
            return;
        };
        let Some(StmtKind::Class(snippet)) = stmts.into_iter().next().map(|s| s.kind) else {
            return;
        };
        // The fields first: the constructor sets them.
        let mut members = snippet.members;
        members.append(&mut c.members);
        c.members = members;
        // `super(m)`: the message.
        for m in &mut c.members {
            let ClassMember::Method(m) = m else { continue };
            if m.kind != MethodKind::Constructor {
                continue;
            }
            let FnBody::Block(b) = &mut m.func.body else {
                continue;
            };
            for st in &mut b.stmts {
                let StmtKind::Expr(e) = &st.kind else {
                    continue;
                };
                let ExprKind::Call { callee, args, .. } = &e.kind else {
                    continue;
                };
                if !matches!(callee.kind, ExprKind::Super) {
                    continue;
                }
                let replacement = match args.first() {
                    Some(a) => {
                        let id = *next_id;
                        *next_id += 3;
                        let this = Expr {
                            kind: ExprKind::This,
                            span: e.span,
                            id,
                        };
                        let target = Expr {
                            kind: ExprKind::Member {
                                obj: Box::new(this),
                                prop: "message".into(),
                                optional: false,
                            },
                            span: e.span,
                            id: id + 1,
                        };
                        StmtKind::Expr(Expr {
                            kind: ExprKind::Assign {
                                op: AssignOp::Assign,
                                target: AssignTarget::Expr(Box::new(target)),
                                value: Box::new(a.expr.clone()),
                            },
                            span: e.span,
                            id: id + 2,
                        })
                    }
                    None => StmtKind::Empty,
                };
                st.kind = replacement;
            }
        }
        if let Some(Expr {
            kind: ExprKind::Ident(b),
            ..
        }) = &mut c.extends
        {
            *b = ERROR_BASE.to_string();
        }
    });
}

/// The classes made for objects with methods are named from this prefix; they are shown
/// like plain objects.
pub const OBJECT_CLASS: &str = "__Object";

/// `{ count: 0, inc() { this.count++ }, get double() { … } }`: an object with methods is
/// an instance of a class made for it (its values given to the constructor).
fn object_methods(body: &mut Vec<Stmt>, next_id: &mut NodeId) {
    // The handler of a proxy keeps its traps.
    let mut handlers = std::collections::HashSet::new();
    for_each_expr(body, &mut |e| {
        if let ExprKind::New { callee, args, .. } = &e.kind
            && matches!(&callee.kind, ExprKind::Ident(n) if n == "Proxy")
            && let Some(h) = args.get(1)
        {
            handlers.insert(h.expr.id);
        }
        true
    });
    let mut classes: Vec<Class> = Vec::new();
    for_each_expr_mut(body, &mut |e| {
        if handlers.contains(&e.id) {
            return true;
        }
        let ExprKind::Object(props) = &e.kind else {
            return true;
        };
        let has_methods = props.iter().any(|p| {
            matches!(
                p.value,
                PropValue::Method(_) | PropValue::Get(_) | PropValue::Set(_)
            )
        });
        let plain_keys = props.iter().all(|p| {
            !matches!(p.key, PropKey::Computed(_)) && !matches!(p.value, PropValue::Spread(_))
        });
        if !has_methods || !plain_keys {
            return true;
        }
        let span = e.span;
        let name = format!("{OBJECT_CLASS}{}", classes.len() + 1);
        let mut fresh = || {
            let id = *next_id;
            *next_id += 1;
            id
        };
        let mut params = Vec::new();
        let mut args = Vec::new();
        let mut members = Vec::new();
        let mut assigns: Vec<Stmt> = Vec::new();
        let ExprKind::Object(props) = std::mem::replace(&mut e.kind, ExprKind::Null) else {
            unreachable!()
        };
        for p in props {
            let key = p.key.name().unwrap_or_default();
            match p.value {
                PropValue::Expr(_) | PropValue::Shorthand(_) => {
                    let value = match p.value {
                        PropValue::Expr(x) => x,
                        PropValue::Shorthand(id) => Expr {
                            kind: ExprKind::Ident(key.clone()),
                            span: p.span,
                            id,
                        },
                        _ => unreachable!(),
                    };
                    // A field in its place, set by the constructor.
                    params.push(Param {
                        pat: Pattern::Ident {
                            name: key.clone(),
                            span: p.span,
                            id: fresh(),
                        },
                        ty: None,
                        optional: false,
                        default: None,
                        rest: false,
                        access: None,
                        readonly: false,
                        span: p.span,
                    });
                    let this = Expr {
                        kind: ExprKind::This,
                        span: p.span,
                        id: fresh(),
                    };
                    let target = Expr {
                        kind: ExprKind::Member {
                            obj: Box::new(this),
                            prop: key.clone(),
                            optional: false,
                        },
                        span: p.span,
                        id: fresh(),
                    };
                    let read = Expr {
                        kind: ExprKind::Ident(key.clone()),
                        span: p.span,
                        id: fresh(),
                    };
                    assigns.push(Stmt {
                        kind: StmtKind::Expr(Expr {
                            kind: ExprKind::Assign {
                                op: AssignOp::Assign,
                                target: AssignTarget::Expr(Box::new(target)),
                                value: Box::new(read),
                            },
                            span: p.span,
                            id: fresh(),
                        }),
                        span: p.span,
                        comments: Vec::new(),
                    });
                    members.push(ClassMember::Field(Field {
                        name: key,
                        ty: None,
                        init: None,
                        is_static: false,
                        access: None,
                        readonly: false,
                        optional: false,
                        span: p.span,
                    }));
                    args.push(Arg {
                        expr: value,
                        spread: false,
                    });
                }
                PropValue::Method(f) => members.push(ClassMember::Method(Method {
                    name: key,
                    kind: MethodKind::Method,
                    func: f,
                    is_static: false,
                    access: None,
                    is_abstract: false,
                    span: p.span,
                })),
                PropValue::Get(f) => members.push(ClassMember::Method(Method {
                    name: key,
                    kind: MethodKind::Get,
                    func: f,
                    is_static: false,
                    access: None,
                    is_abstract: false,
                    span: p.span,
                })),
                PropValue::Set(f) => members.push(ClassMember::Method(Method {
                    name: key,
                    kind: MethodKind::Set,
                    func: f,
                    is_static: false,
                    access: None,
                    is_abstract: false,
                    span: p.span,
                })),
                PropValue::Spread(_) => {}
            }
        }
        let ctor_id = fresh();
        members.insert(
            0,
            ClassMember::Method(Method {
                name: "constructor".into(),
                kind: MethodKind::Constructor,
                func: Function {
                    name: None,
                    type_params: Vec::new(),
                    params,
                    ret: None,
                    body: FnBody::Block(Block {
                        stmts: assigns,
                        span,
                    }),
                    is_async: false,
                    is_arrow: false,
                    is_generator: false,
                    exported: false,
                    span,
                    id: ctor_id,
                },
                is_static: false,
                access: None,
                is_abstract: false,
                span,
            }),
        );
        let class_id = fresh();
        let callee_id = fresh();
        classes.push(Class {
            name: name.clone(),
            type_params: Vec::new(),
            extends: None,
            extends_args: Vec::new(),
            implements: Vec::new(),
            members,
            exported: false,
            is_abstract: false,
            span,
            id: class_id,
        });
        e.kind = ExprKind::New {
            callee: Box::new(Expr {
                kind: ExprKind::Ident(name),
                span,
                id: callee_id,
            }),
            args,
            type_args: Vec::new(),
        };
        true
    });
    for (i, c) in classes.into_iter().enumerate() {
        let span = c.span;
        body.insert(
            i,
            Stmt {
                kind: StmtKind::Class(c),
                span,
                comments: Vec::new(),
            },
        );
    }
}

/// Destructuring the rest of Solidify reads one level at a time: nested patterns
/// (`const { user: { id } } = x`), object patterns of `for … of` loops and of
/// assignments become simple ones, through temporaries.
fn flatten_patterns(body: &mut Vec<Stmt>, next_id: &mut NodeId) {
    let mut f = Flatten { next_id, count: 0 };
    f.list(body);
    for_each_function(body, &mut |func| {
        f.params(func);
        if let FnBody::Block(b) = &mut func.body {
            f.list(&mut b.stmts);
        }
    });
}

struct Flatten<'a> {
    next_id: &'a mut NodeId,
    count: usize,
}

impl Flatten<'_> {
    fn id(&mut self) -> NodeId {
        let id = *self.next_id;
        *self.next_id += 1;
        id
    }

    fn temp(&mut self) -> String {
        self.count += 1;
        format!("destructured{}", self.count)
    }

    fn ident(&mut self, name: &str, span: Span) -> Expr {
        Expr {
            kind: ExprKind::Ident(name.to_string()),
            span,
            id: self.id(),
        }
    }

    fn nested(p: &Pattern) -> bool {
        match p {
            Pattern::Ident { .. } => false,
            Pattern::Array { elems, rest, .. } => {
                elems
                    .iter()
                    .flatten()
                    .any(|e| !matches!(e.pat, Pattern::Ident { .. }))
                    || rest
                        .as_ref()
                        .is_some_and(|r| !matches!(**r, Pattern::Ident { .. }))
            }
            Pattern::Object { props, rest, .. } => {
                props
                    .iter()
                    .any(|p| !matches!(p.value, Pattern::Ident { .. }))
                    || rest
                        .as_ref()
                        .is_some_and(|r| !matches!(**r, Pattern::Ident { .. }))
            }
        }
    }

    /// The pattern with its nested parts replaced by temporaries, and those parts.
    fn lift(&mut self, p: &mut Pattern) -> Vec<(String, Pattern)> {
        let mut out = Vec::new();
        let mut take = |me: &mut Self, sub: &mut Pattern| {
            if !matches!(sub, Pattern::Ident { .. }) {
                let name = me.temp();
                let span = sub.span();
                let id = me.id();
                let inner = std::mem::replace(
                    sub,
                    Pattern::Ident {
                        name: name.clone(),
                        span,
                        id,
                    },
                );
                out.push((name, inner));
            }
        };
        match p {
            Pattern::Ident { .. } => {}
            Pattern::Array { elems, rest, .. } => {
                for e in elems.iter_mut().flatten() {
                    take(self, &mut e.pat);
                }
                if let Some(r) = rest {
                    take(self, r);
                }
            }
            Pattern::Object { props, rest, .. } => {
                for prop in props.iter_mut() {
                    take(self, &mut prop.value);
                }
                if let Some(r) = rest {
                    take(self, r);
                }
            }
        }
        out
    }

    fn decls(&mut self, v: &mut VarDecl) {
        let mut out = Vec::new();
        let mut pending: std::collections::VecDeque<Declarator> = v.decls.drain(..).collect();
        while let Some(mut d) = pending.pop_front() {
            let parts = self.lift(&mut d.pat);
            let span = d.span;
            out.push(d);
            for (name, pat) in parts.into_iter().rev() {
                let init = self.ident(&name, span);
                pending.push_front(Declarator {
                    pat,
                    ty: None,
                    init: Some(init),
                    span,
                });
            }
        }
        v.decls = out;
    }

    /// `function f({ a: { b } })`: the parameter taken apart in the body.
    fn params(&mut self, func: &mut Function) {
        let mut decls = Vec::new();
        for p in &mut func.params {
            if !Self::nested(&p.pat) {
                continue;
            }
            let name = self.temp();
            let span = p.pat.span();
            let id = self.id();
            let pat = std::mem::replace(
                &mut p.pat,
                Pattern::Ident {
                    name: name.clone(),
                    span,
                    id,
                },
            );
            let init = self.ident(&name, span);
            decls.push(Stmt {
                kind: StmtKind::Var(VarDecl {
                    kind: VarKind::Const,
                    decls: vec![Declarator {
                        pat,
                        ty: None,
                        init: Some(init),
                        span,
                    }],
                    exported: false,
                }),
                span,
                comments: Vec::new(),
            });
        }
        if decls.is_empty() {
            return;
        }
        let FnBody::Block(b) = &mut func.body else {
            // `({ a: { b } }) => b`: a block returning the value.
            let FnBody::Expr(e) = std::mem::replace(
                &mut func.body,
                FnBody::Block(Block {
                    stmts: Vec::new(),
                    span: func.span,
                }),
            ) else {
                unreachable!()
            };
            let span = e.span;
            decls.push(Stmt {
                kind: StmtKind::Return(Some(*e)),
                span,
                comments: Vec::new(),
            });
            func.body = FnBody::Block(Block { stmts: decls, span });
            return;
        };
        decls.append(&mut b.stmts);
        b.stmts = decls;
    }

    /// `pat = value` as a statement: the variables set one by one.
    fn assignment(&mut self, pat: Pattern, value: Expr, span: Span, out: &mut Vec<Stmt>) {
        let name = self.temp();
        let id = self.id();
        out.push(Stmt {
            kind: StmtKind::Var(VarDecl {
                kind: VarKind::Const,
                decls: vec![Declarator {
                    pat: Pattern::Ident {
                        name: name.clone(),
                        span,
                        id,
                    },
                    ty: None,
                    init: Some(value),
                    span,
                }],
                exported: false,
            }),
            span,
            comments: Vec::new(),
        });
        let mut parts: Vec<(Pattern, Expr, Option<Expr>)> = Vec::new();
        match pat {
            Pattern::Object { props, .. } => {
                for p in props {
                    let obj = self.ident(&name, span);
                    let read = Expr {
                        kind: ExprKind::Member {
                            obj: Box::new(obj),
                            prop: p.key.clone(),
                            optional: false,
                        },
                        span,
                        id: self.id(),
                    };
                    parts.push((p.value, read, p.default));
                }
            }
            Pattern::Array { elems, .. } => {
                for (i, e) in elems.into_iter().enumerate() {
                    let Some(e) = e else { continue };
                    let obj = self.ident(&name, span);
                    let index = Expr {
                        kind: ExprKind::Num {
                            value: i as f64,
                            raw: i.to_string(),
                            int: true,
                        },
                        span,
                        id: self.id(),
                    };
                    let read = Expr {
                        kind: ExprKind::Index {
                            obj: Box::new(obj),
                            index: Box::new(index),
                            optional: false,
                        },
                        span,
                        id: self.id(),
                    };
                    parts.push((e.pat, read, e.default));
                }
            }
            Pattern::Ident { .. } => {}
        }
        for (target, read, default) in parts {
            let read = match default {
                Some(d) => Expr {
                    kind: ExprKind::Logical {
                        op: LogicalOp::Nullish,
                        left: Box::new(read),
                        right: Box::new(d),
                    },
                    span,
                    id: self.id(),
                },
                None => read,
            };
            match target {
                Pattern::Ident { name, .. } => {
                    let t = self.ident(&name, span);
                    out.push(Stmt {
                        kind: StmtKind::Expr(Expr {
                            kind: ExprKind::Assign {
                                op: AssignOp::Assign,
                                target: AssignTarget::Expr(Box::new(t)),
                                value: Box::new(read),
                            },
                            span,
                            id: self.id(),
                        }),
                        span,
                        comments: Vec::new(),
                    });
                }
                nested => self.assignment(nested, read, span, out),
            }
        }
    }

    /// A statement list, and the lists nested in its statements (functions apart).
    fn list(&mut self, stmts: &mut Vec<Stmt>) {
        let mut i = 0;
        while i < stmts.len() {
            // `({ a, b } = obj);`
            let rewrite = match &stmts[i].kind {
                StmtKind::Expr(Expr {
                    kind:
                        ExprKind::Assign {
                            op: AssignOp::Assign,
                            target: AssignTarget::Pat(p),
                            ..
                        },
                    ..
                }) => matches!(p, Pattern::Object { .. }) || Self::nested(p),
                _ => false,
            };
            if rewrite {
                let s = stmts.remove(i);
                let StmtKind::Expr(Expr {
                    kind:
                        ExprKind::Assign {
                            target: AssignTarget::Pat(p),
                            value,
                            ..
                        },
                    ..
                }) = s.kind
                else {
                    unreachable!()
                };
                let mut out = Vec::new();
                self.assignment(p, *value, s.span, &mut out);
                stmts.insert(
                    i,
                    Stmt {
                        kind: StmtKind::Block(Block {
                            stmts: out,
                            span: s.span,
                        }),
                        span: s.span,
                        comments: s.comments,
                    },
                );
            }
            // `for (const { a, b } of items)`: the item taken apart in the body.
            if let StmtKind::ForOf {
                kind, pat, body, ..
            } = &mut stmts[i].kind
                && (matches!(pat, Pattern::Object { .. }) || Self::nested(pat))
            {
                let name = self.temp();
                let span = pat.span();
                let id = self.id();
                let inner = std::mem::replace(
                    pat,
                    Pattern::Ident {
                        name: name.clone(),
                        span,
                        id,
                    },
                );
                let init = self.ident(&name, span);
                let decl = Stmt {
                    kind: StmtKind::Var(VarDecl {
                        kind: *kind,
                        decls: vec![Declarator {
                            pat: inner,
                            ty: None,
                            init: Some(init),
                            span,
                        }],
                        exported: false,
                    }),
                    span,
                    comments: Vec::new(),
                };
                let old = std::mem::replace(
                    &mut **body,
                    Stmt {
                        kind: StmtKind::Empty,
                        span,
                        comments: Vec::new(),
                    },
                );
                let bspan = old.span;
                let stmts_in = match old.kind {
                    StmtKind::Block(b) => {
                        let mut v = vec![decl];
                        v.extend(b.stmts);
                        v
                    }
                    other => vec![
                        decl,
                        Stmt {
                            kind: other,
                            span: bspan,
                            comments: Vec::new(),
                        },
                    ],
                };
                **body = Stmt {
                    kind: StmtKind::Block(Block {
                        stmts: stmts_in,
                        span: bspan,
                    }),
                    span: bspan,
                    comments: Vec::new(),
                };
            }
            match &mut stmts[i].kind {
                StmtKind::Var(v) => {
                    if v.decls.iter().any(|d| Self::nested(&d.pat)) {
                        self.decls(v);
                    }
                }
                StmtKind::Block(b) => self.list(&mut b.stmts),
                StmtKind::If {
                    then, otherwise, ..
                } => {
                    self.one(then);
                    if let Some(o) = otherwise {
                        self.one(o);
                    }
                }
                StmtKind::While { body, .. }
                | StmtKind::DoWhile { body, .. }
                | StmtKind::Labeled { body, .. }
                | StmtKind::ForOf { body, .. } => self.one(body),
                StmtKind::For { init, body, .. } => {
                    if let Some(i) = init {
                        self.one(i);
                    }
                    self.one(body);
                }
                StmtKind::Switch { cases, .. } => {
                    for c in cases {
                        self.list(&mut c.body);
                    }
                }
                StmtKind::Try {
                    block,
                    catch,
                    finally,
                } => {
                    self.list(&mut block.stmts);
                    if let Some(c) = catch {
                        self.list(&mut c.body.stmts);
                    }
                    if let Some(f) = finally {
                        self.list(&mut f.stmts);
                    }
                }
                _ => {}
            }
            i += 1;
        }
    }

    fn one(&mut self, s: &mut Stmt) {
        let mut v = vec![std::mem::replace(
            s,
            Stmt {
                kind: StmtKind::Empty,
                span: Span::default(),
                comments: Vec::new(),
            },
        )];
        self.list(&mut v);
        *s = if v.len() == 1 {
            v.pop().expect("one")
        } else {
            let span = v[0].span;
            Stmt {
                kind: StmtKind::Block(Block { stmts: v, span }),
                span,
                comments: Vec::new(),
            }
        };
    }
}
