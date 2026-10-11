//! Statements, function bodies, conditions and narrowing, mutation marks.

use super::{
    ActiveNarrow, FnCtx, FnInfo, IResult, Infer, NarrowTest, Path, Role, Root, StructKind,
};
use crate::ast::*;
use crate::types::Ty;

impl<'m> Infer<'m> {
    pub(super) fn block(&mut self, b: &'m Block) -> IResult<()> {
        self.push_scope();
        let narrows_before = self.narrows.len();
        self.collect_values(&b.stmts, false)?;
        self.stmts(&b.stmts)?;
        self.narrows.truncate(narrows_before);
        self.pop_scope();
        Ok(())
    }

    pub(super) fn stmts(&mut self, stmts: &'m [Stmt]) -> IResult<()> {
        for s in stmts {
            self.stmt(s)?;
        }
        Ok(())
    }

    pub(super) fn stmt(&mut self, s: &'m Stmt) -> IResult<()> {
        match &s.kind {
            StmtKind::Var(v) => self.var_decl(v),
            StmtKind::Func(f) => {
                if f.id == usize::MAX {
                    return Ok(());
                }
                let info = self.funcs.get(&f.id).cloned().expect("signature");
                self.enter_function(f, info, None, false)
            }
            StmtKind::Class(c) => self.class_bodies(c),
            StmtKind::Interface(_)
            | StmtKind::TypeAlias(_)
            | StmtKind::Enum(_)
            | StmtKind::Empty
            | StmtKind::ExportNames(_)
            | StmtKind::ExportFrom { .. }
            | StmtKind::Import(_) => Ok(()),
            StmtKind::Return(value) => {
                let ret = self.ctx().ret.clone();
                let id = self.ctx().id;
                let counts = self.this_returns.entry(id).or_default();
                if matches!(
                    value,
                    Some(Expr {
                        kind: ExprKind::This,
                        ..
                    })
                ) {
                    counts.0 += 1;
                } else {
                    counts.1 += 1;
                }
                match value {
                    Some(e) => self.fn_returns.push((id, e)),
                    None => {
                        self.undef_fns.insert(id);
                    }
                }
                match value {
                    Some(e) => {
                        let t = self.expr(e, Some(&ret))?;
                        self.mark_stored(e);
                        self.expect_flow(&t, &ret, e)?;
                        self.ctx_mut().has_return_value = true;
                    }
                    None => self.expect(&Ty::Void, &ret, s.span)?,
                }
                Ok(())
            }
            StmtKind::If {
                test,
                then,
                otherwise,
            } => {
                let (pos, neg) = self.condition(test)?;
                let before = self.narrows.len();
                self.narrows.extend(pos);
                self.branch.push((test.id, 0));
                let r = self.stmt(then);
                self.branch.pop();
                r?;
                self.narrows.truncate(before);
                let exits = always_exits(then);
                match otherwise {
                    Some(o) => {
                        let neg2: Vec<ActiveNarrow> =
                            neg.iter().map(ActiveNarrow::duplicate).collect();
                        self.narrows.extend(neg2);
                        self.branch.push((test.id, 1));
                        let r = self.stmt(o);
                        self.branch.pop();
                        r?;
                        self.narrows.truncate(before);
                    }
                    None if exits && !neg.is_empty() => {
                        // `if (x === null) return;`: the rest of the block sees `x`.
                        for n in &neg {
                            if let Some(t) = self.narrow_tests.get_mut(&n.test_id) {
                                t.let_else = true;
                            }
                        }
                        self.narrows.extend(neg);
                    }
                    None => {}
                }
                Ok(())
            }
            StmtKind::While { test, body } => {
                let (pos, _) = self.condition(test)?;
                for n in &pos {
                    // The value may change in the body: read through the variable, except
                    // when the test assigns it, which `while let` does afresh each time.
                    if !self.assign_tests.contains(&n.test_id)
                        && let Some(t) = self.narrow_tests.get_mut(&n.test_id)
                    {
                        t.unwrap = true;
                    }
                }
                let before = self.narrows.len();
                self.narrows.extend(pos);
                self.ctx_mut().loop_depth += 1;
                self.stmt(body)?;
                self.ctx_mut().loop_depth -= 1;
                self.narrows.truncate(before);
                Ok(())
            }
            StmtKind::DoWhile { body, test } => {
                self.ctx_mut().loop_depth += 1;
                self.stmt(body)?;
                self.ctx_mut().loop_depth -= 1;
                self.condition(test)?;
                Ok(())
            }
            StmtKind::For {
                init,
                test,
                update,
                body,
            } => {
                self.push_scope();
                if let Some(i) = init {
                    self.stmt(i)?;
                }
                if let Some(t) = test {
                    self.condition(t)?;
                }
                self.ctx_mut().loop_depth += 1;
                if let Some(u) = update {
                    self.expr(u, None)?;
                }
                self.stmt(body)?;
                self.ctx_mut().loop_depth -= 1;
                self.pop_scope();
                Ok(())
            }
            StmtKind::ForOf {
                pat,
                iter,
                body,
                is_in,
                ..
            } => {
                let it = self.expr(iter, None)?;
                let it = self.types.resolve(&it);
                let item = if *is_in {
                    match &it {
                        Ty::Map(k, _, _) => (**k).clone(),
                        Ty::Array(_) => self.types.int(),
                        d if d.is_dynamic() => Ty::Str,
                        other => {
                            let d = self.describe(other);
                            return self.err(
                                format!("`for … in` over {d} is not supported: use `for … of`"),
                                iter.span,
                            );
                        }
                    }
                } else {
                    match &it {
                        Ty::Array(t) | Ty::Set(t) | Ty::Gen(t) => (**t).clone(),
                        Ty::Str => Ty::Str,
                        d if d.is_dynamic() => Ty::dynamic(),
                        Ty::Map(k, v, _) => Ty::Tuple(vec![(**k).clone(), (**v).clone()]),
                        Ty::Var(_) => {
                            let t = self.types.fresh();
                            self.unify(&it, &Ty::Array(Box::new(t.clone())), iter.span)?;
                            t
                        }
                        other => match self.struct_iteration(other) {
                            Some(t) => t,
                            None => {
                                let d = self.describe(other);
                                return self.err(
                                    format!(
                                        "cannot iterate over {d}: give it a `[Symbol.iterator]()` method"
                                    ),
                                    iter.span,
                                );
                            }
                        },
                    }
                };
                self.push_scope();
                self.ctx_mut().loop_depth += 1;
                self.bind_pattern(pat, &item, Role::Loop)?;
                // `for (const l of this.listeners)`: changing `l` changes `this`.
                if uses_this(iter) && !self.ctx().in_ctor {
                    let method = self.ctx().this_fn;
                    for (_, id) in pat.names() {
                        self.loop_this_sources.push((id, method));
                    }
                }
                if let Some(root) = self.root_binding(iter) {
                    for (_, id) in pat.names() {
                        self.loop_sources.push((id, root));
                    }
                }
                self.stmt(body)?;
                self.ctx_mut().loop_depth -= 1;
                self.pop_scope();
                Ok(())
            }
            StmtKind::Switch { disc, cases } => {
                // `switch (shape.kind)`: each case sees its member of the union.
                let tagged = if self.member_is_tag(disc) {
                    let ExprKind::Member { obj, .. } = &disc.kind else {
                        unreachable!()
                    };
                    let ot = self.expr(obj, None)?;
                    self.union_of_ty(&ot).cloned().map(|u| (&**obj, u))
                } else {
                    None
                };
                let dt = self.expr(disc, None)?;
                for (i, c) in cases.iter().enumerate() {
                    let mut narrow = None;
                    if let Some(t) = &c.test {
                        let tt = self.expr(t, Some(&dt))?;
                        self.expect(&tt, &dt, t.span)?;
                        if let Some((obj, u)) = &tagged
                            && let ExprKind::Str(s) = &t.kind
                        {
                            let Some(m) = u.member_with_tag(s) else {
                                return self.err(
                                    format!(
                                        "`{s}` is not a `{}` of {}",
                                        u.discriminant.clone().unwrap_or_default(),
                                        u.name
                                    ),
                                    t.span,
                                );
                            };
                            let test = super::unions::UnionTest {
                                value: obj,
                                union: u.name.clone(),
                                variant: m.name.clone(),
                                inner: m.ty.clone(),
                                positive: true,
                            };
                            narrow = Some(self.narrow_union(&test, t.id)?.0);
                        }
                    }
                    self.push_scope();
                    let before = self.narrows.len();
                    self.narrows.extend(narrow);
                    self.branch.push((disc.id, i.min(254) as u8));
                    let r = self
                        .collect_values(&c.body, false)
                        .and_then(|()| self.stmts(&c.body));
                    self.branch.pop();
                    self.narrows.truncate(before);
                    self.pop_scope();
                    r?;
                }
                Ok(())
            }
            StmtKind::Break(_) | StmtKind::Continue(_) => Ok(()),
            StmtKind::Throw(e) => {
                let t = self.expr(e, None)?;
                match self.types.resolve(&t) {
                    Ty::Str | Ty::Named(..) => {}
                    other => {
                        let d = self.describe(&other);
                        return self
                            .err(format!("throw a string or `new Error(…)`, not {d}"), e.span);
                    }
                }
                if self.ctx().try_depth == 0 {
                    let id = self.ctx().id;
                    self.funcs.get_mut(&id).expect("fn").throw_direct = true;
                }
                Ok(())
            }
            StmtKind::Try {
                block,
                catch,
                finally,
            } => {
                self.ctx_mut().try_depth += 1;
                let before = self.narrows.len();
                self.block(block)?;
                self.narrows.truncate(before);
                self.ctx_mut().try_depth -= 1;
                if block_returns(&block.stmts) {
                    self.try_returns.insert(block.span.start);
                }
                if let Some(c) = catch {
                    self.push_scope();
                    if let Some(p) = &c.param {
                        self.bind_pattern(p, &Ty::named("Error"), Role::Catch)?;
                    }
                    self.block(&c.body)?;
                    self.pop_scope();
                }
                if let Some(f) = finally {
                    self.block(f)?;
                }
                Ok(())
            }
            StmtKind::Block(b) => self.block(b),
            StmtKind::Expr(e) => {
                self.expr(e, None)?;
                Ok(())
            }
            StmtKind::ExportDefault(e) => {
                self.err("`export default` of a value is not supported", e.span)
            }
            StmtKind::Labeled { body, .. } => self.stmt(body),
        }
    }

    fn var_decl(&mut self, v: &'m VarDecl) -> IResult<()> {
        let top = self.fns.len() == 1 && self.scopes.len() == 1;
        for d in &v.decls {
            if !top {
                self.check_redeclaration(v.kind, &d.pat)?;
            }
            let hint = match &d.pat {
                Pattern::Ident { name, .. } => name.clone(),
                _ => String::new(),
            };
            let declared = if top {
                // Already declared by `collect_values`: its type.
                match &d.pat {
                    Pattern::Ident { id, .. } => Some(self.vars[id].ty.clone()),
                    _ => None,
                }
            } else {
                match &d.ty {
                    Some(t) => Some(self.ann(t, &hint)?),
                    None => None,
                }
            };
            if let Some(init) = &d.init {
                self.hints.insert(init.id, hint.clone());
            }
            let from_var = d
                .init
                .as_ref()
                .filter(|i| matches!(i.kind, ExprKind::Ident(_)));
            let ty = match (&d.init, &declared) {
                (Some(init), Some(t)) => {
                    let it = self.expr(init, Some(t))?;
                    self.expect_flow(&it, t, init)?;
                    t.clone()
                }
                (Some(init), None) => self.expr(init, None)?,
                (None, Some(t)) => t.clone(),
                (None, None) => self.types.fresh(),
            };
            // `const b = a`: two variables, one object.
            if let Some(init) = from_var {
                self.copied_types.push(ty.clone());
                if let (Pattern::Ident { id, .. }, Some(source)) =
                    (&d.pat, self.refs.get(&init.id).copied())
                {
                    self.var_aliases.push((*id, source));
                }
            }
            let ty = match self.types.resolve(&ty) {
                // `let x = null`: nullable, content decided by a later assignment.
                Ty::Null => {
                    let inner = self.types.fresh();
                    let opt = Ty::Option(Box::new(inner));
                    self.unify(&ty, &opt, d.span)?;
                    opt
                }
                _ => ty,
            };
            if top {
                if !matches!(d.pat, Pattern::Ident { .. }) {
                    self.bind_pattern(&d.pat, &ty, Role::TopLevel)?;
                }
            } else {
                self.bind_pattern(&d.pat, &ty, Role::Local)?;
            }
            // `let x: T | undefined`, `let x;`: missing means `undefined`; otherwise the
            // initializer decides.
            if let Pattern::Ident { id, .. } = &d.pat {
                match (&d.ty, &d.init) {
                    (Some(t), _) if super::says_undefined(t) => {
                        self.undef_vars.insert(*id);
                    }
                    (_, None) => {
                        self.undef_vars.insert(*id);
                    }
                    (_, Some(init)) => self.var_inits.push((*id, init)),
                }
            }
            if let (Pattern::Ident { id, .. }, Some(init)) = (&d.pat, &d.init) {
                let alias = is_place(init);
                let closure_fn = match &init.kind {
                    ExprKind::Arrow(f) | ExprKind::Func(f) => Some(f.id),
                    _ => None,
                };
                let iter_result = self.is_iter_result(init);
                let regex_flags = self.regex_flags(init);
                if let Some(v) = self.vars.get_mut(id) {
                    v.alias = alias;
                    v.closure_fn = closure_fn;
                    v.iter_result = iter_result;
                    v.regex_flags = regex_flags;
                }
                if alias && let Some(&(_, test)) = self.narrowed_uses.get(&init.id) {
                    self.alias_of_narrow.push((*id, test));
                }
            }
        }
        Ok(())
    }

    /// Infers the body of a function whose signature is known.
    pub(super) fn enter_function(
        &mut self,
        f: &'m Function,
        info: FnInfo,
        this: Option<String>,
        in_ctor: bool,
    ) -> IResult<()> {
        let parent = self.fns.last().map(|c| c.id);
        let parent_this = self.fns.last().and_then(|c| c.this.clone());
        let parent_this_fn = self.fns.last().map(|c| c.this_fn);
        let parent_static = self.fns.last().is_some_and(|c| c.is_static);
        let (this, this_fn, is_static) = if f.is_arrow {
            (parent_this, parent_this_fn.unwrap_or(f.id), parent_static)
        } else {
            (this, f.id, info.is_static)
        };
        let in_ctor = if f.is_arrow {
            self.fns.last().is_some_and(|c| c.in_ctor)
        } else {
            in_ctor
        };
        // A generator body yields values and returns nothing.
        let (ret, yield_ty) = match (f.is_generator, self.types.resolve(&info.sig.ret)) {
            (true, Ty::Gen(t)) => (Ty::Void, Some(*t)),
            _ => (info.sig.ret.clone(), None),
        };
        self.fns.push(FnCtx {
            id: f.id,
            ret,
            this,
            in_ctor,
            is_static,
            try_depth: 0,
            loop_depth: 0,
            type_params: info.type_params.clone(),
            captures: Vec::new(),
            this_fn,
            has_return_value: false,
            yield_ty,
        });
        self.push_scope();
        let mut info = info;
        info.parent = parent.filter(|p| *p != self.main_id);
        self.funcs.insert(f.id, info.clone());
        let mut param_ids = Vec::new();
        for (i, p) in f.params.iter().enumerate() {
            let ty = info.sig.params[i].clone();
            match &p.pat {
                Pattern::Ident { name, span, id } => {
                    self.declare(name, *id, ty.clone(), Role::Param, *span);
                    param_ids.push(*id);
                }
                other => {
                    let id = self.synth_id();
                    // Named after its type when it has a name: `{ x, y }: Point` is `point`.
                    let mut name = match &p.ty {
                        Some(TypeAnn::Named { name, .. }) => {
                            let mut c = name.chars();
                            c.next()
                                .map(|f| f.to_lowercase().collect::<String>() + c.as_str())
                                .unwrap_or_default()
                        }
                        _ => String::new(),
                    };
                    let taken: Vec<String> =
                        other.names().iter().map(|(n, _)| n.to_string()).collect();
                    if name.is_empty() || taken.contains(&name) || self.lookup(&name).is_some() {
                        name = format!("arg{i}");
                    }
                    self.declare(&name, id, ty.clone(), Role::Param, other.span());
                    param_ids.push(id);
                    self.bind_pattern(other, &ty, Role::Local)?;
                }
            }
            if let Some(v) = self.vars.get_mut(&param_ids[i]) {
                v.param_of = Some((f.id, i));
                // A parameter property is kept in the instance; a generator keeps its
                // parameters for as long as it runs.
                if (in_ctor && (p.access.is_some() || p.readonly)) || f.is_generator {
                    v.stored = true;
                    self.param_stored.insert((f.id, i));
                }
            }
            if let Some(d) = &p.default {
                let inner = match self.types.resolve(&ty) {
                    Ty::Option(t) if p.optional => *t,
                    t => t,
                };
                let dt = self.expr(d, Some(&inner))?;
                self.expect(&dt, &inner, d.span)?;
            }
        }
        self.funcs.get_mut(&f.id).expect("fn").params = param_ids;
        if info.is_abstract {
            // A signature only.
            self.pop_scope();
            self.fns.pop();
            return Ok(());
        }
        match &f.body {
            FnBody::Block(b) => {
                self.collect_values(&b.stmts, false)?;
                let before = self.narrows.len();
                self.stmts(&b.stmts)?;
                self.narrows.truncate(before);
                if !self.ctx().has_return_value && !block_returns(&b.stmts) {
                    let ret = self.ctx().ret.clone();
                    if self.types.unify(&ret, &Ty::Void).is_err() {
                        let d = self.describe(&ret);
                        return self.err(
                            format!("the function must return {d} on every path"),
                            f.span,
                        );
                    }
                }
            }
            FnBody::Expr(e) => {
                self.fn_returns.push((f.id, e));
                let ret = self.ctx().ret.clone();
                if matches!(self.types.resolve(&ret), Ty::Void) {
                    // A callback whose result nobody wants: the value is dropped.
                    self.expr(e, None)?;
                } else {
                    let t = self.expr(e, Some(&ret))?;
                    self.mark_stored(e);
                    self.expect_flow(&t, &ret, e)?;
                }
            }
        }
        self.pop_scope();
        let ctx = self.fns.pop().expect("ctx");
        let info = self.funcs.get_mut(&f.id).expect("fn");
        info.captures = ctx.captures;
        info.is_closure = !info.captures.is_empty() || f.is_arrow || f.name.is_none();
        // A declared function whose parameters nothing in its body pins down is generic:
        // each call instantiates it (`function largest(items)` works on numbers and strings).
        if !info.is_closure && info.method_of.is_none() {
            let params = info.sig.params.clone();
            let mut type_params = info.type_params.clone();
            let mut vars = Vec::new();
            for p in &params {
                super::finish::collect_vars(&self.types, p, &mut vars);
            }
            for v in vars {
                let name = super::finish::fresh_type_param(&type_params);
                self.types.bind(v, Ty::named(name.clone()));
                type_params.push(name);
            }
            self.funcs.get_mut(&f.id).expect("fn").type_params = type_params;
        }
        Ok(())
    }

    fn class_bodies(&mut self, c: &'m Class) -> IResult<()> {
        self.param_scope
            .push(c.type_params.iter().map(|t| t.name.clone()).collect());
        let r = self.class_bodies_inner(c);
        self.param_scope.pop();
        r
    }

    fn class_bodies_inner(&mut self, c: &'m Class) -> IResult<()> {
        self.this_class_stack.push(c.name.clone());
        for m in &c.members {
            match m {
                ClassMember::Field(f) => {
                    let Some(init) = &f.init else { continue };
                    let fty = if f.is_static {
                        self.struct_by_name(&c.name)
                            .and_then(|s| s.statics.iter().find(|(n, _, _)| *n == f.name))
                            .map(|(_, t, _)| t.clone())
                    } else {
                        self.struct_by_name(&c.name)
                            .and_then(|s| s.field(&f.name))
                            .map(|fi| {
                                if fi.optional {
                                    Ty::Option(Box::new(fi.ty.clone()))
                                } else {
                                    fi.ty.clone()
                                }
                            })
                    };
                    let fty = fty.expect("field");
                    // Initializers run in the constructor.
                    self.fns.push(FnCtx {
                        id: c.id,
                        ret: Ty::Void,
                        this: Some(c.name.clone()),
                        in_ctor: true,
                        is_static: f.is_static,
                        try_depth: 0,
                        loop_depth: 0,
                        type_params: Vec::new(),
                        captures: Vec::new(),
                        this_fn: c.id,
                        has_return_value: false,
                        yield_ty: None,
                    });
                    self.hints.insert(init.id, format!("{} {}", c.name, f.name));
                    let t = self.expr(init, Some(&fty));
                    self.fns.pop();
                    let t = t?;
                    self.expect(&t, &fty, init.span)?;
                    if f.is_static {
                        let s = self.struct_mut(&c.name).expect("class");
                        if let Some(st) = s.statics.iter_mut().find(|(n, _, _)| *n == f.name) {
                            st.2 = Some(init.id);
                        }
                    }
                }
                ClassMember::Method(m) => {
                    let info = self.funcs.get(&m.func.id).cloned().expect("method");
                    let this = if m.is_static {
                        None
                    } else {
                        Some(c.name.clone())
                    };
                    self.enter_function(&m.func, info, this, m.kind == MethodKind::Constructor)?;
                }
                ClassMember::StaticBlock(_) => {}
            }
        }
        self.this_class_stack.pop();
        Ok(())
    }

    // ----- conditions and narrowing -----

    /// Infers a condition and returns what it narrows when true, and when false.
    pub(super) fn condition(
        &mut self,
        e: &'m Expr,
    ) -> IResult<(Vec<ActiveNarrow>, Vec<ActiveNarrow>)> {
        match &e.kind {
            ExprKind::Unary {
                op: UnaryOp::Not,
                arg,
            } => {
                let (pos, neg) = self.condition(arg)?;
                self.set_ty(e.id, Ty::Bool);
                Ok((neg, pos))
            }
            ExprKind::Logical {
                op: LogicalOp::And,
                left,
                right,
            } => {
                let (lpos, _) = self.condition(left)?;
                let before = self.narrows.len();
                self.narrows
                    .extend(lpos.iter().map(ActiveNarrow::duplicate));
                let r = self.condition(right);
                self.narrows.truncate(before);
                let (rpos, _) = r?;
                let mut pos = lpos;
                pos.extend(rpos);
                self.set_ty(e.id, Ty::Bool);
                Ok((pos, Vec::new()))
            }
            ExprKind::Logical {
                op: LogicalOp::Or,
                left,
                right,
            } => {
                let (_, lneg) = self.condition(left)?;
                let before = self.narrows.len();
                self.narrows
                    .extend(lneg.iter().map(ActiveNarrow::duplicate));
                let r = self.condition(right);
                self.narrows.truncate(before);
                let (_, rneg) = r?;
                let mut neg = lneg;
                neg.extend(rneg);
                self.set_ty(e.id, Ty::Bool);
                Ok((Vec::new(), neg))
            }
            // `x instanceof C` on a value of a base class: `x` is a `C` when true.
            ExprKind::Binary {
                op: BinOp::InstanceOf,
                left,
                right,
            } if self.path_of(left).is_some() && self.instance_test(left, right)?.is_some() => {
                let Some((class, sub)) = self.instance_test(left, right)? else {
                    unreachable!()
                };
                let _ = class;
                let path = self.path_of(left).expect("path");
                let name = match (&path.root, path.fields.last()) {
                    (Root::Var(id), None) => self.vars[id].name.clone(),
                    (_, Some(last)) => last.clone(),
                    (Root::This, None) => "this".into(),
                };
                self.set_ty(e.id, Ty::Bool);
                self.narrow_tests.insert(
                    e.id,
                    NarrowTest {
                        root: path.root.clone(),
                        path: path.fields.clone(),
                        name: name.clone(),
                        positive: true,
                        union: None,
                        complement: None,
                        instance_of: Some(sub.clone()),
                        mutated: false,
                        inner: Ty::named(sub.clone()),
                        let_else: false,
                        unwrap: false,
                    },
                );
                let n = ActiveNarrow {
                    root: path.root,
                    path: path.fields,
                    name,
                    inner: Ty::named(sub),
                    test_id: e.id,
                };
                Ok((vec![n], Vec::new()))
            }
            // Tests of a union member: `typeof x === "string"`, `x.kind === "circle"`,
            // `x instanceof C`, `"field" in x`, `Array.isArray(x)`.
            _ if self.is_union_test_shape(e) => {
                if let Some(t) = self.union_test(e)? {
                    let (n, other) = self.narrow_union(&t, e.id)?;
                    let other: Vec<ActiveNarrow> = other.into_iter().collect();
                    return Ok(if t.positive {
                        (vec![n], other)
                    } else {
                        (other, vec![n])
                    });
                }
                self.truthy(e)?;
                Ok((Vec::new(), Vec::new()))
            }
            ExprKind::Binary { op, left, right } if op.is_equality() => {
                let null_right = matches!(right.kind, ExprKind::Null | ExprKind::Undefined);
                let null_left = matches!(left.kind, ExprKind::Null | ExprKind::Undefined);
                if null_right || null_left {
                    let value: &'m Expr = if null_right { left } else { right };
                    let positive = matches!(op, BinOp::NotEq | BinOp::StrictNotEq);
                    let other = if null_right { right } else { left };
                    self.set_ty(other.id, Ty::Null);
                    self.set_ty(e.id, Ty::Bool);
                    // `(m = re.exec(s)) !== null`: the assignment happens, then `m` is tested.
                    let value: &'m Expr = match &value.kind {
                        ExprKind::Assign {
                            op: AssignOp::Assign,
                            target: AssignTarget::Expr(t),
                            ..
                        } if matches!(t.kind, ExprKind::Ident(_)) => {
                            self.expr(value, None)?;
                            if let Some(b) = self.root_binding(t) {
                                self.assign_narrowed.entry(b).or_default().push(t.id);
                            }
                            self.assign_tests.insert(e.id);
                            t
                        }
                        _ => value,
                    };
                    if let Some(n) = self.narrow_value(value, e.id, positive)? {
                        return Ok(if positive {
                            (vec![n], Vec::new())
                        } else {
                            (Vec::new(), vec![n])
                        });
                    }
                    // Not nullable (or already narrowed): the test is always decided.
                    if !self.expr_ty.contains_key(&value.id) {
                        self.expr(value, None)?;
                    }
                    return Ok((Vec::new(), Vec::new()));
                }
                self.expr(e, None)?;
                Ok((Vec::new(), Vec::new()))
            }
            ExprKind::Ident(_) | ExprKind::Member { .. } | ExprKind::This => {
                if let Some(n) = self.narrow_value(e, e.id, true)? {
                    return Ok((vec![n], Vec::new()));
                }
                self.truthy(e)?;
                Ok((Vec::new(), Vec::new()))
            }
            // `isNaN(n)` on the `Option` of `parseInt`: `n` exists when the test is false.
            ExprKind::Call { callee, args, .. }
                if args.len() == 1 && nan_test_name(callee).is_some() =>
            {
                let positive = nan_test_name(callee) == Some(false);
                let value = &args[0].expr;
                if self.path_of(value).is_some()
                    && matches!(
                        self.expr(value, None).map(|t| self.types.resolve(&t)),
                        Ok(Ty::Option(_))
                    )
                    && let Some(n) = self.narrow_value(value, e.id, positive)?
                {
                    self.set_ty(e.id, Ty::Bool);
                    return Ok(if positive {
                        (vec![n], Vec::new())
                    } else {
                        (Vec::new(), vec![n])
                    });
                }
                self.truthy(e)?;
                Ok((Vec::new(), Vec::new()))
            }
            _ => {
                self.truthy(e)?;
                Ok((Vec::new(), Vec::new()))
            }
        }
    }

    /// `x instanceof C` where `x` is of a base class of `C`: (base, C). Marks the base
    /// as dynamically dispatched.
    fn instance_test(
        &mut self,
        value: &'m Expr,
        class: &Expr,
    ) -> IResult<Option<(String, String)>> {
        // The class may be imported under another name, or through a namespace.
        let sub = match self.type_ref(class) {
            Some((canonical, id)) if self.vars[&id].role == Role::Class => {
                if let ExprKind::Member { obj: ns, .. } = &class.kind {
                    self.set_ty(ns.id, Ty::Void);
                }
                canonical
            }
            _ => match &class.kind {
                ExprKind::Ident(s) => s.clone(),
                _ => return Ok(None),
            },
        };
        let sub = &sub;
        if self
            .struct_by_name(sub)
            .is_none_or(|s| s.kind != StructKind::Class)
        {
            return Ok(None);
        }
        let vt = self.expr(value, None)?;
        let Ty::Named(base, ..) = self.types.resolve(&vt) else {
            return Ok(None);
        };
        if self.union_by_name(&base).is_some() || base == *sub || !self.is_subclass(sub, &base) {
            return Ok(None);
        }
        self.polymorphic.insert(base.clone());
        // Instances of the subclasses of `sub` are seen as `sub` too.
        if self
            .structs
            .iter()
            .any(|s| s.name != *sub && self.is_subclass(&s.name, sub))
        {
            self.polymorphic.insert(sub.clone());
        }
        Ok(Some((base, sub.clone())))
    }

    /// Whether an expression has the shape of a union member test (the union itself is
    /// checked by `union_test`).
    fn is_union_test_shape(&self, e: &Expr) -> bool {
        match &e.kind {
            ExprKind::Binary { op, left, right } if op.is_equality() => {
                matches!(left.kind, ExprKind::TypeOf(_))
                    || matches!(right.kind, ExprKind::TypeOf(_))
                    || (matches!(left.kind, ExprKind::Member { .. })
                        && matches!(right.kind, ExprKind::Str(_))
                        && self.member_is_tag(left))
                    || (matches!(right.kind, ExprKind::Member { .. })
                        && matches!(left.kind, ExprKind::Str(_))
                        && self.member_is_tag(right))
            }
            ExprKind::Binary {
                op: BinOp::InstanceOf | BinOp::In,
                ..
            } => true,
            ExprKind::Call { callee, .. } => {
                matches!(&callee.kind, ExprKind::Member { obj, prop, .. }
                if prop == "isArray" && matches!(&obj.kind, ExprKind::Ident(n) if n == "Array"))
            }
            _ => false,
        }
    }

    /// `x.kind` where `x` is a union with the discriminant `kind`.
    fn member_is_tag(&self, e: &Expr) -> bool {
        let ExprKind::Member { obj, prop, .. } = &e.kind else {
            return false;
        };
        let Some(path) = self.path_of(obj) else {
            return false;
        };
        if let Some(i) = self.active_narrow(&path) {
            let inner = self.narrows[i].inner.clone();
            return self
                .union_of_ty(&inner)
                .is_some_and(|u| u.discriminant.as_deref() == Some(prop.as_str()));
        }
        let ty = match &path.root {
            Root::Var(id) => self.vars[id].ty.clone(),
            Root::This => match self.ctx().this.clone() {
                Some(c) => Ty::named(c),
                None => return false,
            },
        };
        let mut ty = self.types.resolve(&ty);
        for f in &path.fields {
            let Ty::Named(n, ..) = &ty else { return false };
            let Some(s) = self.struct_by_name(n) else {
                return false;
            };
            let Some(field) = s.field(f) else {
                return false;
            };
            ty = self.types.resolve(&field.ty);
        }
        self.union_of_ty(&ty)
            .is_some_and(|u| u.discriminant.as_deref() == Some(prop.as_str()))
    }

    /// A value used as a condition: boolean, number, string, list or nullable.
    pub(super) fn truthy(&mut self, e: &'m Expr) -> IResult<Ty> {
        let t = self.expr(e, None)?;
        match self.types.resolve(&t) {
            Ty::Bool | Ty::Num(_) | Ty::Str | Ty::Option(_) | Ty::Array(_) | Ty::Any => Ok(t),
            d if d.is_dynamic() => Ok(t),
            Ty::Var(_) => {
                self.unify(&t, &Ty::Bool, e.span)?;
                Ok(t)
            }
            other => {
                let d = self.describe(&other);
                self.err(format!("{d} cannot be used as a condition"), e.span)
            }
        }
    }

    /// If `e` is a path to a nullable value, the narrowing of a test of it.
    fn narrow_value(
        &mut self,
        e: &'m Expr,
        test_id: NodeId,
        positive: bool,
    ) -> IResult<Option<ActiveNarrow>> {
        let Some(path) = self.path_of(e) else {
            return Ok(None);
        };
        let t = self.expr(e, None)?;
        let Ty::Option(inner) = self.types.resolve(&t) else {
            return Ok(None);
        };
        let name = self.narrow_name(&path);
        self.narrow_tests.insert(
            test_id,
            NarrowTest {
                root: path.root.clone(),
                path: path.fields.clone(),
                name: name.clone(),
                positive,
                union: None,
                complement: None,
                instance_of: None,
                mutated: false,
                inner: (*inner).clone(),
                let_else: false,
                unwrap: false,
            },
        );
        Ok(Some(ActiveNarrow {
            root: path.root,
            path: path.fields,
            name,
            inner: *inner,
            test_id,
        }))
    }

    fn narrow_name(&self, path: &Path) -> String {
        match (&path.root, path.fields.last()) {
            (Root::Var(id), None) => self.vars[id].name.clone(),
            (_, Some(last)) => {
                if self.lookup(last).is_none() {
                    last.clone()
                } else {
                    let mut name = match &path.root {
                        Root::Var(id) => self.vars[id].name.clone(),
                        Root::This => "this".into(),
                    };
                    for f in &path.fields {
                        name.push('_');
                        name.push_str(f);
                    }
                    name
                }
            }
            (Root::This, None) => "this".into(),
        }
    }

    // ----- mutation and escape marking -----

    /// The struct types a change through `target` goes through: `list[0].items.push(x)`
    /// changes an instance held in `list`.
    pub(super) fn note_changed_types(&mut self, target: &Expr) {
        let mut e = target;
        loop {
            match &e.kind {
                ExprKind::Member { obj, .. } | ExprKind::Index { obj, .. } => {
                    if let Some(t) = self.expr_ty.get(&obj.id)
                        && let Ty::Named(n, _) = self.types.resolve(t)
                        && self.struct_by_name(&n).is_some()
                    {
                        if let ExprKind::Member { prop, .. } = &e.kind {
                            self.changed_fields.insert((n.clone(), prop.clone()));
                        }
                        if !matches!(obj.kind, ExprKind::This) {
                            self.changed_types.insert(n);
                        }
                    }
                    e = obj;
                }
                ExprKind::NonNull(inner) => e = inner,
                _ => break,
            }
        }
    }

    pub(super) fn mark_mutated(&mut self, target: &Expr) {
        self.note_changed_types(target);
        if let Some(path) = self.path_of(target)
            && !matches!(target.kind, ExprKind::Ident(_))
        {
            // The narrowed value is changed: bind it mutably.
            for n in self.narrows.iter().rev() {
                if n.root == path.root && path.fields.starts_with(&n.path) {
                    if let Some(t) = self.narrow_tests.get_mut(&n.test_id) {
                        t.mutated = true;
                    }
                    break;
                }
            }
        }
        if uses_this(target) {
            self.mark_this_mutated();
            return;
        }
        let in_place = !matches!(target.kind, ExprKind::Ident(_));
        if let Some(id) = self.root_binding(target) {
            let current = self.ctx().id;
            if let Some(v) = self.vars.get_mut(&id) {
                if in_place {
                    v.mutated = true;
                } else {
                    v.reassigned = true;
                }
                if v.func != Some(current) && v.role != Role::TopLevel {
                    v.mutated_in_closure = true;
                }
                if let Some(p) = v.param_of
                    && in_place
                {
                    self.param_mutated.insert(p);
                }
            }
        }
    }

    /// The receiver of a method that changes it (`list.push(x)`, `map.set(k, v)`).
    pub(super) fn mark_receiver_mutated(&mut self, obj: &Expr) {
        self.note_changed_types(obj);
        if let Some(path) = self.path_of(obj) {
            for n in self.narrows.iter().rev() {
                if n.root == path.root && path.fields.starts_with(&n.path) {
                    if let Some(t) = self.narrow_tests.get_mut(&n.test_id) {
                        t.mutated = true;
                    }
                    break;
                }
            }
        }
        if uses_this(obj) {
            self.mark_this_mutated();
            return;
        }
        if let Some(id) = self.root_binding(obj) {
            let current = self.ctx().id;
            if let Some(v) = self.vars.get_mut(&id) {
                v.mutated = true;
                if v.func != Some(current) && v.role != Role::TopLevel {
                    v.mutated_in_closure = true;
                }
                if let Some(p) = v.param_of {
                    self.param_mutated.insert(p);
                }
            }
        }
    }

    pub(super) fn mark_this_mutated(&mut self) {
        if self.ctx().in_ctor {
            return;
        }
        let id = self.ctx().this_fn;
        if let Some(f) = self.funcs.get_mut(&id) {
            f.mutates_self = true;
        }
    }

    /// A value kept by someone else: a variable passed here must be owned.
    pub(super) fn mark_stored(&mut self, e: &Expr) {
        let e = match &e.kind {
            ExprKind::NonNull(inner) => inner,
            _ => e,
        };
        // A copy of what a place holds: the place and the copy are one object in
        // JavaScript.
        if is_place(e)
            && let Some(t) = self.expr_ty.get(&e.id)
        {
            self.copied_types.push(t.clone());
        }
        if let ExprKind::Ident(name) = &e.kind
            && let Some(id) = self.lookup(name)
            && let Some(v) = self.vars.get_mut(&id)
        {
            v.stored = true;
            if let Some(p) = v.param_of {
                self.param_stored.insert(p);
            }
        }
        if let ExprKind::Arrow(f) | ExprKind::Func(f) = &e.kind
            && let Some(info) = self.funcs.get_mut(&f.id)
        {
            info.escapes = true;
        }
    }
}

/// `isNaN` / `Number.isNaN` (true) or `isFinite` / `Number.isFinite` (false).
pub(super) fn nan_test_name(callee: &Expr) -> Option<bool> {
    match &callee.kind {
        ExprKind::Ident(n) if n == "isNaN" => Some(true),
        ExprKind::Ident(n) if n == "isFinite" => Some(false),
        ExprKind::Member { obj, prop, .. } if matches!(&obj.kind, ExprKind::Ident(n) if n == "Number") => {
            match prop.as_str() {
                "isNaN" => Some(true),
                "isFinite" => Some(false),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Whether a statement always leaves the block (return, throw, break, continue).
pub(super) fn always_exits(s: &Stmt) -> bool {
    match &s.kind {
        StmtKind::Return(_) | StmtKind::Throw(_) | StmtKind::Break(_) | StmtKind::Continue(_) => {
            true
        }
        StmtKind::Block(b) => b.stmts.last().is_some_and(always_exits),
        StmtKind::If {
            then,
            otherwise: Some(o),
            ..
        } => always_exits(then) && always_exits(o),
        _ => false,
    }
}

/// Whether a block ends by returning or throwing on every path.
pub(super) fn block_returns(stmts: &[Stmt]) -> bool {
    stmts.iter().any(|s| match &s.kind {
        StmtKind::Return(_) | StmtKind::Throw(_) => true,
        StmtKind::Block(b) => block_returns(&b.stmts),
        StmtKind::If {
            then,
            otherwise: Some(o),
            ..
        } => block_returns(std::slice::from_ref(then)) && block_returns(std::slice::from_ref(o)),
        StmtKind::Try {
            block,
            catch,
            finally,
        } => {
            (block_returns(&block.stmts)
                && catch.as_ref().is_none_or(|c| block_returns(&c.body.stmts)))
                || finally.as_ref().is_some_and(|f| block_returns(&f.stmts))
        }
        StmtKind::Switch { cases, .. } => {
            cases.iter().any(|c| c.test.is_none()) && cases.iter().all(|c| block_returns(&c.body))
        }
        StmtKind::While { test, .. } => matches!(test.kind, ExprKind::Bool(true)),
        StmtKind::Labeled { body, .. } => block_returns(std::slice::from_ref(body)),
        _ => false,
    })
}

/// A field or an element of something: a value that can be borrowed.
pub(super) fn is_place(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Member {
            optional: false, ..
        }
        | ExprKind::Index {
            optional: false, ..
        } => true,
        ExprKind::NonNull(inner) => is_place(inner),
        _ => false,
    }
}

pub(super) fn uses_this(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::This => true,
        ExprKind::Member { obj, .. } | ExprKind::Index { obj, .. } => uses_this(obj),
        ExprKind::NonNull(inner) => uses_this(inner),
        _ => false,
    }
}
