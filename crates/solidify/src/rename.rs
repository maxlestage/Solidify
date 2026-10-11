//! Programs of several files share one top-level namespace when translated. A name
//! declared at the top of two files is renamed in the later one (`platform` in
//! `node:process` becomes `platform_node_process`), everywhere it means that declaration
//! in the file; the file still exports it under its own name.

use std::collections::{HashMap, HashSet};

use crate::ast::*;

/// Renames, in each file but the entry, the top-level names another file declares.
/// `bodies[0]` is the entry; `words` names each file (for the suffix). Returns, for each
/// file, the renames done.
pub fn separate_files(bodies: &mut [Vec<Stmt>], words: &[String]) -> Vec<HashMap<String, String>> {
    let mut taken: HashSet<String> = HashSet::new();
    let mut renames = vec![HashMap::new(); bodies.len()];
    for (i, body) in bodies.iter_mut().enumerate() {
        let names = top_names(body);
        let mut map = HashMap::new();
        for n in &names {
            if taken.contains(n) {
                let mut new = format!("{n}_{}", words[i]);
                while taken.contains(&new) || names.contains(&new) {
                    new.push('_');
                }
                map.insert(n.clone(), new);
            }
        }
        for n in &names {
            taken.insert(map.get(n).cloned().unwrap_or_else(|| n.clone()));
        }
        if !map.is_empty() {
            rename_top(body, &map);
        }
        renames[i] = map;
    }
    renames
}

/// The names a file declares at its top level (values and types).
fn top_names(body: &[Stmt]) -> Vec<String> {
    let mut out = Vec::new();
    for s in body {
        match &s.kind {
            StmtKind::Func(f) => out.extend(f.name.clone()),
            StmtKind::Class(c) => out.push(c.name.clone()),
            StmtKind::Enum(e) => out.push(e.name.clone()),
            StmtKind::Interface(i) => out.push(i.name.clone()),
            StmtKind::TypeAlias(a) => out.push(a.name.clone()),
            StmtKind::Var(v) => {
                for d in &v.decls {
                    out.extend(d.pat.names().into_iter().map(|(n, _)| n.to_string()));
                }
            }
            _ => {}
        }
    }
    out
}

/// `export { a as b }` entries.
type ExportList = Vec<(String, Option<String>)>;

fn rename_top(body: &mut Vec<Stmt>, map: &HashMap<String, String>) {
    let mut exports: Vec<(usize, ExportList)> = Vec::new();
    for (i, s) in body.iter_mut().enumerate() {
        // A renamed declaration is still exported under its name.
        let mut renamed_export = |old: &str, exported: &mut bool| {
            if *exported && let Some(new) = map.get(old) {
                *exported = false;
                exports.push((i, vec![(new.clone(), Some(old.to_string()))]));
            }
        };
        match &mut s.kind {
            StmtKind::Func(f) => {
                if let Some(n) = f.name.clone() {
                    renamed_export(&n, &mut f.exported);
                    if let Some(new) = map.get(&n) {
                        f.name = Some(new.clone());
                    }
                }
            }
            StmtKind::Class(c) => {
                let n = c.name.clone();
                renamed_export(&n, &mut c.exported);
                if let Some(new) = map.get(&n) {
                    c.name = new.clone();
                }
            }
            StmtKind::Enum(e) => {
                let n = e.name.clone();
                renamed_export(&n, &mut e.exported);
                if let Some(new) = map.get(&n) {
                    e.name = new.clone();
                }
            }
            StmtKind::Interface(it) => {
                let n = it.name.clone();
                renamed_export(&n, &mut it.exported);
                if let Some(new) = map.get(&n) {
                    it.name = new.clone();
                }
            }
            StmtKind::TypeAlias(a) => {
                let n = a.name.clone();
                renamed_export(&n, &mut a.exported);
                if let Some(new) = map.get(&n) {
                    a.name = new.clone();
                }
            }
            StmtKind::Var(v) => {
                let names: Vec<String> = v
                    .decls
                    .iter()
                    .flat_map(|d| d.pat.names().into_iter().map(|(n, _)| n.to_string()))
                    .collect();
                if v.exported && names.iter().any(|n| map.contains_key(n)) {
                    v.exported = false;
                    exports.push((
                        i,
                        names
                            .iter()
                            .map(|n| match map.get(n) {
                                Some(new) => (new.clone(), Some(n.clone())),
                                None => (n.clone(), None),
                            })
                            .collect(),
                    ));
                }
                for d in &mut v.decls {
                    rename_pattern_decl(&mut d.pat, map);
                }
            }
            StmtKind::ExportNames(names) => {
                for (name, alias) in names.iter_mut() {
                    if let Some(new) = map.get(name) {
                        if alias.is_none() {
                            *alias = Some(name.clone());
                        }
                        *name = new.clone();
                    }
                }
            }
            _ => {}
        }
    }
    let mut r = Renamer;
    r.stmts(body, map);
    for (i, names) in exports.into_iter().rev() {
        let span = body[i].span;
        body.insert(
            i + 1,
            Stmt {
                kind: StmtKind::ExportNames(names),
                span,
                comments: Vec::new(),
            },
        );
    }
}

/// The names a declaring pattern binds, renamed.
fn rename_pattern_decl(p: &mut Pattern, map: &HashMap<String, String>) {
    match p {
        Pattern::Ident { name, .. } => {
            if let Some(new) = map.get(name) {
                *name = new.clone();
            }
        }
        Pattern::Array { elems, rest, .. } => {
            for e in elems.iter_mut().flatten() {
                rename_pattern_decl(&mut e.pat, map);
            }
            if let Some(r) = rest {
                rename_pattern_decl(r, map);
            }
        }
        Pattern::Object { props, rest, .. } => {
            for prop in props {
                rename_pattern_decl(&mut prop.value, map);
            }
            if let Some(r) = rest {
                rename_pattern_decl(r, map);
            }
        }
    }
}

/// Every use of the renamed names, except where a closer declaration hides them.
struct Renamer;

impl Renamer {
    fn stmts(&mut self, ss: &mut [Stmt], map: &HashMap<String, String>) {
        for s in ss {
            self.stmt(s, map);
        }
    }

    fn block(&mut self, b: &mut Block, map: &HashMap<String, String>) {
        self.stmts(&mut b.stmts, map);
    }

    fn stmt(&mut self, s: &mut Stmt, map: &HashMap<String, String>) {
        match &mut s.kind {
            StmtKind::Var(v) => {
                for d in &mut v.decls {
                    self.pattern_parts(&mut d.pat, map);
                    if let Some(t) = &mut d.ty {
                        self.ty(t, map);
                    }
                    if let Some(e) = &mut d.init {
                        self.expr(e, map);
                    }
                }
            }
            StmtKind::Func(f) => self.function(f, map),
            StmtKind::Class(c) => self.class(c, map),
            StmtKind::Interface(i) => {
                for t in &mut i.extends {
                    self.ty(t, map);
                }
                for m in &mut i.members {
                    self.member(m, map);
                }
            }
            StmtKind::TypeAlias(a) => self.ty(&mut a.ty, map),
            StmtKind::Enum(e) => {
                for m in &mut e.members {
                    if let Some(x) = &mut m.init {
                        self.expr(x, map);
                    }
                }
            }
            StmtKind::Return(Some(e))
            | StmtKind::Throw(e)
            | StmtKind::Expr(e)
            | StmtKind::ExportDefault(e) => self.expr(e, map),
            StmtKind::If {
                test,
                then,
                otherwise,
            } => {
                self.expr(test, map);
                self.stmt(then, map);
                if let Some(o) = otherwise {
                    self.stmt(o, map);
                }
            }
            StmtKind::While { test, body } | StmtKind::DoWhile { body, test } => {
                self.expr(test, map);
                self.stmt(body, map);
            }
            StmtKind::For {
                init,
                test,
                update,
                body,
            } => {
                if let Some(i) = init {
                    self.stmt(i, map);
                }
                if let Some(t) = test {
                    self.expr(t, map);
                }
                if let Some(u) = update {
                    self.expr(u, map);
                }
                self.stmt(body, map);
            }
            StmtKind::ForOf {
                pat, iter, body, ..
            } => {
                self.pattern_parts(pat, map);
                self.expr(iter, map);
                self.stmt(body, map);
            }
            StmtKind::Switch { disc, cases } => {
                self.expr(disc, map);
                for c in cases {
                    if let Some(t) = &mut c.test {
                        self.expr(t, map);
                    }
                    self.stmts(&mut c.body, map);
                }
            }
            StmtKind::Try {
                block,
                catch,
                finally,
            } => {
                self.block(block, map);
                if let Some(c) = catch {
                    self.block(&mut c.body, map);
                }
                if let Some(f) = finally {
                    self.block(f, map);
                }
            }
            StmtKind::Block(b) => self.block(b, map),
            StmtKind::Labeled { body, .. } => self.stmt(body, map),
            _ => {}
        }
    }

    /// The expressions inside a pattern (defaults), not its names.
    fn pattern_parts(&mut self, p: &mut Pattern, map: &HashMap<String, String>) {
        match p {
            Pattern::Ident { .. } => {}
            Pattern::Array { elems, rest, .. } => {
                for e in elems.iter_mut().flatten() {
                    self.pattern_parts(&mut e.pat, map);
                    if let Some(d) = &mut e.default {
                        self.expr(d, map);
                    }
                }
                if let Some(r) = rest {
                    self.pattern_parts(r, map);
                }
            }
            Pattern::Object { props, rest, .. } => {
                for prop in props {
                    self.pattern_parts(&mut prop.value, map);
                    if let Some(d) = &mut prop.default {
                        self.expr(d, map);
                    }
                }
                if let Some(r) = rest {
                    self.pattern_parts(r, map);
                }
            }
        }
    }

    /// A pattern assigned to (`[a, b] = …`): its names are uses.
    fn pattern_uses(&mut self, p: &mut Pattern, map: &HashMap<String, String>) {
        rename_pattern_decl(p, map);
        self.pattern_parts(p, map);
    }

    fn function(&mut self, f: &mut Function, map: &HashMap<String, String>) {
        let mut bound = HashSet::new();
        for p in &f.params {
            for (n, _) in p.pat.names() {
                bound.insert(n.to_string());
            }
        }
        if let FnBody::Block(b) = &f.body {
            declared_in(&b.stmts, &mut bound);
        }
        let inner: HashMap<String, String> = map
            .iter()
            .filter(|(k, _)| !bound.contains(*k))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        for p in &mut f.params {
            if let Some(t) = &mut p.ty {
                self.ty(t, map);
            }
            if let Some(d) = &mut p.default {
                self.expr(d, &inner);
            }
            self.pattern_parts(&mut p.pat, &inner);
        }
        if let Some(r) = &mut f.ret {
            self.ty(r, map);
        }
        match &mut f.body {
            FnBody::Block(b) => self.block(b, &inner),
            FnBody::Expr(e) => self.expr(e, &inner),
        }
    }

    fn class(&mut self, c: &mut Class, map: &HashMap<String, String>) {
        if let Some(e) = &mut c.extends {
            self.expr(e, map);
        }
        for t in c.extends_args.iter_mut().chain(c.implements.iter_mut()) {
            self.ty(t, map);
        }
        for m in &mut c.members {
            match m {
                ClassMember::Field(f) => {
                    if let Some(t) = &mut f.ty {
                        self.ty(t, map);
                    }
                    if let Some(e) = &mut f.init {
                        self.expr(e, map);
                    }
                }
                ClassMember::Method(m) => self.function(&mut m.func, map),
                ClassMember::StaticBlock(b) => self.block(b, map),
            }
        }
    }

    fn member(&mut self, m: &mut TypeMember, map: &HashMap<String, String>) {
        match m {
            TypeMember::Prop { ty, .. } => self.ty(ty, map),
            TypeMember::Method { params, ret, .. } | TypeMember::Call { params, ret, .. } => {
                for p in params {
                    if let Some(t) = &mut p.ty {
                        self.ty(t, map);
                    }
                }
                if let Some(r) = ret {
                    self.ty(r, map);
                }
            }
            TypeMember::Index { key, value, .. } => {
                self.ty(key, map);
                self.ty(value, map);
            }
        }
    }

    fn ty(&mut self, t: &mut TypeAnn, map: &HashMap<String, String>) {
        match t {
            TypeAnn::Named { name, args, .. } => {
                if let Some(new) = map.get(name) {
                    *name = new.clone();
                }
                for a in args {
                    self.ty(a, map);
                }
            }
            TypeAnn::Array(x, _) | TypeAnn::KeyOf(x, _) | TypeAnn::Readonly(x, _) => {
                self.ty(x, map)
            }
            TypeAnn::Tuple(xs, _) | TypeAnn::Union(xs, _) | TypeAnn::Intersection(xs, _) => {
                for x in xs {
                    self.ty(x, map);
                }
            }
            TypeAnn::Object(ms, _) => {
                for m in ms {
                    self.member(m, map);
                }
            }
            TypeAnn::Func { params, ret, .. } => {
                for p in params {
                    if let Some(t) = &mut p.ty {
                        self.ty(t, map);
                    }
                }
                self.ty(ret, map);
            }
            TypeAnn::TypeOf(name, _) => {
                if let Some(new) = map.get(name) {
                    *name = new.clone();
                }
            }
            TypeAnn::Indexed(a, b, _) => {
                self.ty(a, map);
                self.ty(b, map);
            }
            TypeAnn::Lit(..) => {}
        }
    }

    fn expr(&mut self, e: &mut Expr, map: &HashMap<String, String>) {
        match &mut e.kind {
            ExprKind::Ident(n) => {
                if let Some(new) = map.get(n) {
                    *n = new.clone();
                }
            }
            ExprKind::Template { exprs, .. } => {
                for x in exprs {
                    self.expr(x, map);
                }
            }
            ExprKind::Tagged { tag, exprs, .. } => {
                self.expr(tag, map);
                for x in exprs {
                    self.expr(x, map);
                }
            }
            ExprKind::Array(items) => {
                for i in items {
                    match i {
                        ArrayElem::Expr(x) | ArrayElem::Spread(x) => self.expr(x, map),
                        ArrayElem::Hole => {}
                    }
                }
            }
            ExprKind::Object(props) => {
                for p in props {
                    if let PropKey::Computed(k) = &mut p.key {
                        self.expr(k, map);
                    }
                    match &mut p.value {
                        PropValue::Expr(x) | PropValue::Spread(x) => self.expr(x, map),
                        // `{ name }` of a renamed variable: `{ name: renamed }`.
                        PropValue::Shorthand(id) => {
                            let key = p.key.name().unwrap_or_default();
                            if let Some(new) = map.get(&key) {
                                p.value = PropValue::Expr(Expr {
                                    kind: ExprKind::Ident(new.clone()),
                                    span: p.span,
                                    id: *id,
                                });
                            }
                        }
                        PropValue::Method(f) | PropValue::Get(f) | PropValue::Set(f) => {
                            self.function(f, map)
                        }
                    }
                }
            }
            ExprKind::Func(f) | ExprKind::Arrow(f) => {
                // A named function expression sees its own name.
                let map = match &f.name {
                    Some(n) if map.contains_key(n) => {
                        let mut m = map.clone();
                        m.remove(n);
                        m
                    }
                    _ => map.clone(),
                };
                self.function(f, &map);
            }
            ExprKind::Class(c) => self.class(c, map),
            ExprKind::Unary { arg, .. }
            | ExprKind::Update { arg, .. }
            | ExprKind::NonNull(arg)
            | ExprKind::TypeOf(arg)
            | ExprKind::Void(arg)
            | ExprKind::Delete(arg)
            | ExprKind::Await(arg) => self.expr(arg, map),
            ExprKind::Binary { left, right, .. } | ExprKind::Logical { left, right, .. } => {
                self.expr(left, map);
                self.expr(right, map);
            }
            ExprKind::Assign { target, value, .. } => {
                match target {
                    AssignTarget::Expr(t) => self.expr(t, map),
                    AssignTarget::Pat(p) => self.pattern_uses(p, map),
                }
                self.expr(value, map);
            }
            ExprKind::Cond { test, cons, alt } => {
                self.expr(test, map);
                self.expr(cons, map);
                self.expr(alt, map);
            }
            ExprKind::Call {
                callee,
                args,
                type_args,
                ..
            }
            | ExprKind::New {
                callee,
                args,
                type_args,
            } => {
                self.expr(callee, map);
                for a in args {
                    self.expr(&mut a.expr, map);
                }
                for t in type_args {
                    self.ty(t, map);
                }
            }
            ExprKind::Member { obj, .. } => self.expr(obj, map),
            ExprKind::Index { obj, index, .. } => {
                self.expr(obj, map);
                self.expr(index, map);
            }
            ExprKind::As(x, t) => {
                self.expr(x, map);
                self.ty(t, map);
            }
            ExprKind::Yield { value: Some(v), .. } => self.expr(v, map),
            ExprKind::Seq(xs) => {
                for x in xs {
                    self.expr(x, map);
                }
            }
            _ => {}
        }
    }
}

/// The names declared in a function's statements (not inside nested functions).
fn declared_in(stmts: &[Stmt], out: &mut HashSet<String>) {
    for s in stmts {
        match &s.kind {
            StmtKind::Var(v) => {
                for d in &v.decls {
                    out.extend(d.pat.names().into_iter().map(|(n, _)| n.to_string()));
                }
            }
            StmtKind::Func(f) => out.extend(f.name.clone()),
            StmtKind::Class(c) => {
                out.insert(c.name.clone());
            }
            StmtKind::If {
                then, otherwise, ..
            } => {
                declared_in(std::slice::from_ref(then), out);
                if let Some(o) = otherwise {
                    declared_in(std::slice::from_ref(o), out);
                }
            }
            StmtKind::While { body, .. }
            | StmtKind::DoWhile { body, .. }
            | StmtKind::Labeled { body, .. } => declared_in(std::slice::from_ref(body), out),
            StmtKind::For { init, body, .. } => {
                if let Some(i) = init {
                    declared_in(std::slice::from_ref(i), out);
                }
                declared_in(std::slice::from_ref(body), out);
            }
            StmtKind::ForOf { pat, body, .. } => {
                out.extend(pat.names().into_iter().map(|(n, _)| n.to_string()));
                declared_in(std::slice::from_ref(body), out);
            }
            StmtKind::Switch { cases, .. } => {
                for c in cases {
                    declared_in(&c.body, out);
                }
            }
            StmtKind::Try {
                block,
                catch,
                finally,
            } => {
                declared_in(&block.stmts, out);
                if let Some(c) = catch {
                    if let Some(p) = &c.param {
                        out.extend(p.names().into_iter().map(|(n, _)| n.to_string()));
                    }
                    declared_in(&c.body.stmts, out);
                }
                if let Some(f) = finally {
                    declared_in(&f.stmts, out);
                }
            }
            StmtKind::Block(b) => declared_in(&b.stmts, out),
            _ => {}
        }
    }
}
