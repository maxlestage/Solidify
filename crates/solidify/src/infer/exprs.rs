//! Expressions: literals, operators, calls, members, object literals and closures.

use std::collections::{HashMap, HashSet};
use std::rc::Rc;

#[allow(unused_imports)]
use super::MethodInfo;
use super::finish::{leak, same_shape_ty, substitute};
use super::{
    Callee, EnumKind, FieldInfo, FnInfo, GLOBALS, IResult, Infer, Path, Role, Root, StructInfo,
    StructKind,
};
use crate::ast::*;
use crate::error::Span;
use crate::types::{Flavor, FuncTy, Ty};

impl<'m> Infer<'m> {
    pub(super) fn expr(&mut self, e: &'m Expr, expected: Option<&Ty>) -> IResult<Ty> {
        let ty = self.expr_inner(e, expected)?;
        Ok(self.set_ty(e.id, ty))
    }

    fn expr_inner(&mut self, e: &'m Expr, expected: Option<&Ty>) -> IResult<Ty> {
        let expected_res = expected.map(|t| self.types.resolve(t));
        let expected_res = match (&e.kind, expected_res) {
            (
                ExprKind::Array(_) | ExprKind::Object(_) | ExprKind::Str(_),
                Some(Ty::Option(inner)),
            ) => Some(self.types.resolve(&inner)),
            (_, other) => other,
        };
        match &e.kind {
            ExprKind::Num { int, .. } => Ok(if *int {
                self.types.int()
            } else {
                self.types.float()
            }),
            ExprKind::Str(s) => {
                if let Some(Ty::Named(n)) = &expected_res
                    && let Some(en) = self.enum_by_name(n)
                    && en.kind == EnumKind::Literals
                {
                    let Some(v) = en.variant_for_literal(s) else {
                        return self.err(format!("`{s}` is not a value of {n}"), e.span);
                    };
                    let variant = v.name.clone();
                    self.enum_literal.insert(e.id, (n.clone(), variant));
                    return Ok(Ty::Named(n.clone()));
                }
                Ok(Ty::Str)
            }
            ExprKind::Template { exprs, .. } => {
                for x in exprs {
                    self.expr(x, None)?;
                }
                Ok(Ty::Str)
            }
            ExprKind::Tagged { .. } => self.err("tagged templates are not supported", e.span),
            ExprKind::Bool(_) => Ok(Ty::Bool),
            ExprKind::Null | ExprKind::Undefined => Ok(Ty::Null),
            ExprKind::Regex { .. } => self.err("regular expressions are not supported", e.span),
            ExprKind::This => {
                let Some(class) = self.ctx().this.clone() else {
                    return self.err("`this` outside a class", e.span);
                };
                if self.ctx().is_static {
                    return self.err("`this` in a static method", e.span);
                }
                let id = self.ctx().this_fn;
                if let Some(f) = self.funcs.get_mut(&id) {
                    f.uses_this = true;
                }
                let path = Path {
                    root: Root::This,
                    fields: Vec::new(),
                };
                if let Some(i) = self.active_narrow(&path) {
                    let n = &self.narrows[i];
                    self.narrowed_uses.insert(e.id, (n.name.clone(), n.test_id));
                    return Ok(n.inner.clone());
                }
                Ok(Ty::Named(class))
            }
            ExprKind::Super => {
                self.err("`super` is used as `super(…)` or `super.method(…)`", e.span)
            }
            ExprKind::Ident(name) => self.ident(name, e),
            ExprKind::Array(elems) => {
                let elem_expected = match &expected_res {
                    Some(Ty::Array(t)) | Some(Ty::Set(t)) => Some((**t).clone()),
                    Some(Ty::Tuple(ts)) => {
                        if ts.len() != elems.len() {
                            return self
                                .err(format!("expected a tuple of {} elements", ts.len()), e.span);
                        }
                        let ts = ts.clone();
                        for (el, t) in elems.iter().zip(&ts) {
                            let ArrayElem::Expr(x) = el else {
                                return self
                                    .err("holes and spreads are not allowed in a tuple", e.span);
                            };
                            let xt = self.expr(x, Some(t))?;
                            self.mark_stored(x);
                            self.expect(&xt, t, x.span)?;
                        }
                        return Ok(Ty::Tuple(ts));
                    }
                    _ => None,
                };
                let item = elem_expected.unwrap_or_else(|| self.types.fresh());
                let hint = self.hints.get(&e.id).cloned().unwrap_or_default();
                for el in elems {
                    match el {
                        ArrayElem::Expr(x) => {
                            if !hint.is_empty() {
                                self.hints.insert(x.id, singular(&hint));
                            }
                            let xt = self.expr(x, Some(&item))?;
                            self.mark_stored(x);
                            self.expect(&xt, &item, x.span)?;
                        }
                        ArrayElem::Spread(x) => {
                            let arr = Ty::Array(Box::new(item.clone()));
                            let xt = self.expr(x, Some(&arr))?;
                            // Arrays, sets, maps (pairs) and strings (characters) spread.
                            match self.types.resolve(&xt) {
                                Ty::Set(t) | Ty::Gen(t) => self.expect(&t, &item, x.span)?,
                                Ty::Map(k, v, _) => {
                                    self.expect(&Ty::Tuple(vec![*k, *v]), &item, x.span)?
                                }
                                Ty::Str => self.expect(&Ty::Str, &item, x.span)?,
                                _ => self.expect(&xt, &arr, x.span)?,
                            }
                        }
                        ArrayElem::Hole => {
                            return self.err("holes in arrays are not supported", e.span);
                        }
                    }
                }
                Ok(Ty::Array(Box::new(item)))
            }
            ExprKind::Object(props) => self.object_literal(e, props, expected_res.as_ref()),
            ExprKind::Func(f) | ExprKind::Arrow(f) => self.closure(f, expected_res.as_ref()),
            ExprKind::Class(c) => self.err("class expressions are not supported", c.span),
            ExprKind::Unary { op, arg } => match op {
                UnaryOp::Not => {
                    self.truthy(arg)?;
                    Ok(Ty::Bool)
                }
                UnaryOp::Neg | UnaryOp::Pos => {
                    let t = self.expr(arg, None)?;
                    match self.types.resolve(&t) {
                        Ty::Str if *op == UnaryOp::Pos => Ok(self.types.float()),
                        _ => {
                            let n = self.types.num(Flavor::Unknown);
                            self.expect(&t, &n, arg.span)?;
                            Ok(n)
                        }
                    }
                }
                UnaryOp::BitNot => {
                    let t = self.expr(arg, None)?;
                    let n = self.types.int();
                    self.expect(&t, &n, arg.span)?;
                    Ok(n)
                }
            },
            ExprKind::Update { arg, .. } => {
                let t = self.expr(arg, None)?;
                let n = self.types.num(Flavor::Unknown);
                self.expect(&t, &n, arg.span)?;
                self.mark_mutated(arg);
                Ok(n)
            }
            ExprKind::Binary { op, left, right } => self.binary(*op, left, right, e.span),
            ExprKind::Logical { op, left, right } => match op {
                LogicalOp::Nullish => {
                    let lt = self.expr(left, None)?;
                    let inner = match self.types.resolve(&lt) {
                        Ty::Option(t) => *t,
                        Ty::Var(_) => {
                            let t = self.types.fresh();
                            self.unify(&lt, &Ty::Option(Box::new(t.clone())), left.span)?;
                            t
                        }
                        other => {
                            let d = self.describe(&other);
                            return self.err(
                                format!("`??` needs a nullable value on the left, found {d}"),
                                left.span,
                            );
                        }
                    };
                    let rt = self.expr(right, Some(&inner))?;
                    self.expect(&rt, &inner, right.span)?;
                    Ok(inner)
                }
                LogicalOp::Or => {
                    // `test(x) || rest`: the right side sees what `x` is when the test fails.
                    let (_, neg) = self.condition(left)?;
                    let lt = self.expr_ty.get(&left.id).cloned().unwrap_or(Ty::Bool);
                    match self.types.resolve(&lt) {
                        Ty::Bool => {
                            let before = self.narrows.len();
                            self.narrows.extend(neg);
                            let rt = self.truthy(right);
                            self.narrows.truncate(before);
                            let rt = rt?;
                            self.expect(&rt, &Ty::Bool, right.span)?;
                            Ok(Ty::Bool)
                        }
                        Ty::Option(inner) => {
                            let rt = self.expr(right, Some(&inner))?;
                            self.expect(&rt, &inner, right.span)?;
                            Ok(*inner)
                        }
                        _ => {
                            let rt = self.expr(right, Some(&lt))?;
                            self.expect(&rt, &lt, right.span)?;
                            Ok(lt)
                        }
                    }
                }
                LogicalOp::And => {
                    let (pos, _) = self.condition(left)?;
                    let before = self.narrows.len();
                    self.narrows.extend(pos);
                    let rt = self.truthy(right);
                    self.narrows.truncate(before);
                    let rt = rt?;
                    let lt = self.expr_ty.get(&left.id).cloned().unwrap_or(Ty::Bool);
                    match (self.types.resolve(&lt), self.types.resolve(&rt)) {
                        (Ty::Bool | Ty::Option(_), Ty::Bool) => Ok(Ty::Bool),
                        (Ty::Bool | Ty::Option(_), _) => Ok(Ty::Bool),
                        _ => {
                            self.expect(&rt, &Ty::Bool, right.span)?;
                            Ok(Ty::Bool)
                        }
                    }
                }
            },
            ExprKind::Assign { op, target, value } => self.assign(*op, target, value, e.span),
            ExprKind::Cond { test, cons, alt } => {
                let (pos, neg) = self.condition(test)?;
                let before = self.narrows.len();
                self.narrows.extend(pos);
                self.branch.push((test.id, 0));
                let ct = self.expr(cons, expected);
                self.branch.pop();
                self.narrows.truncate(before);
                let ct = ct?;
                // Both branches may fill a union: each is checked against it.
                let union_expected = expected_res
                    .as_ref()
                    .filter(|t| self.union_of_ty(t).is_some())
                    .cloned();
                let alt_expected = union_expected.clone().unwrap_or_else(|| ct.clone());
                self.narrows.extend(neg);
                self.branch.push((test.id, 1));
                let at = self.expr(alt, Some(&alt_expected));
                self.branch.pop();
                self.narrows.truncate(before);
                let at = at?;
                self.mark_stored(cons);
                self.mark_stored(alt);
                if let Some(u) = union_expected {
                    self.expect(&ct, &u, cons.span)?;
                    self.expect(&at, &u, alt.span)?;
                    return Ok(u);
                }
                if self.types.unify(&ct, &at).is_err() {
                    return self.err(
                        format!(
                            "the two branches have different types: {} and {}",
                            self.describe(&ct),
                            self.describe(&at)
                        ),
                        e.span,
                    );
                }
                Ok(match self.types.resolve(&ct) {
                    Ty::Null => self.types.resolve(&at),
                    _ => ct,
                })
            }
            ExprKind::Call {
                callee,
                args,
                optional,
                type_args,
            } => {
                if !type_args.is_empty() {
                    return self.err("explicit type arguments are not supported", e.span);
                }
                self.call(e, callee, args, *optional)
            }
            ExprKind::New { callee, args, .. } => self.new_expr(e, callee, args),
            ExprKind::Member {
                obj,
                prop,
                optional,
            } => self.member(e, obj, prop, *optional),
            ExprKind::Index {
                obj,
                index,
                optional,
            } => {
                if *optional {
                    return self.err("`?.[` is not supported", e.span);
                }
                self.index(obj, index, e.span)
            }
            ExprKind::As(inner, ty) => {
                if let TypeAnn::Named { name, .. } = ty
                    && name == "const"
                {
                    return self.expr(inner, expected);
                }
                let target = self.ann(ty, "")?;
                let it = self.expr(inner, Some(&target))?;
                if self.types.unify(&it, &target).is_err() {
                    let (a, b) = (self.describe(&it), self.describe(&target));
                    return self.err(format!("cannot convert {a} to {b} with `as`"), e.span);
                }
                Ok(target)
            }
            ExprKind::NonNull(inner) => {
                let t = self.expr(inner, None)?;
                Ok(match self.types.resolve(&t) {
                    Ty::Option(t) => *t,
                    other => other,
                })
            }
            ExprKind::TypeOf(inner) => {
                self.expr(inner, None)?;
                Ok(Ty::Str)
            }
            ExprKind::Void(inner) => {
                self.expr(inner, None)?;
                Ok(Ty::Void)
            }
            ExprKind::Delete(_) => self.err(
                "`delete` is not supported: use `Map.delete` or `Option`",
                e.span,
            ),
            ExprKind::Await(inner) => {
                let id = self.ctx().id;
                let is_async = self.funcs.get(&id).is_some_and(|f| f.is_async);
                if !is_async && id != self.main_id {
                    return self.err("`await` outside an async function", e.span);
                }
                if id == self.main_id {
                    self.funcs.get_mut(&id).expect("main").is_async = true;
                }
                self.expr(inner, expected)
            }
            ExprKind::Yield { value, delegate } => {
                let Some(t) = self.ctx().yield_ty.clone() else {
                    return self.err("`yield` outside a generator function", e.span);
                };
                match (value, delegate) {
                    (Some(v), false) => {
                        let vt = self.expr(v, Some(&t))?;
                        self.mark_stored(v);
                        self.expect(&vt, &t, v.span)?;
                    }
                    (Some(v), true) => {
                        let vt = self.expr(v, None)?;
                        match self.types.resolve(&vt) {
                            Ty::Gen(item) | Ty::Array(item) | Ty::Set(item) => {
                                self.expect(&item, &t, v.span)?
                            }
                            other => {
                                let d = self.describe(&other);
                                return self.err(format!("`yield*` cannot go through {d}"), v.span);
                            }
                        }
                    }
                    (None, _) => return self.err("`yield` needs a value", e.span),
                }
                Ok(Ty::Void)
            }
            ExprKind::Seq(items) => {
                let mut last = Ty::Void;
                for i in items {
                    last = self.expr(i, None)?;
                }
                Ok(last)
            }
        }
    }

    fn ident(&mut self, name: &str, e: &'m Expr) -> IResult<Ty> {
        if let Some(id) = self.lookup(name) {
            self.refs.insert(e.id, id);
            self.record_use(id, e.id);
            let current = self.ctx().id;
            let main_id = self.main_id;
            let outer_fn = self
                .fns
                .iter()
                .rev()
                .find(|c| c.id == main_id || !self.funcs.get(&c.id).is_some_and(|f| f.is_closure))
                .map(|c| c.id);
            let v = self.vars.get_mut(&id).expect("var");
            if v.func != Some(current) {
                match v.role {
                    Role::TopLevel => {
                        if outer_fn != Some(main_id) {
                            v.used_in_fn = true;
                        }
                        if current != main_id {
                            v.captured = true;
                        }
                    }
                    Role::Func | Role::Class | Role::Enum => {}
                    _ => {
                        v.captured = true;
                        // Every function between the use and the declaration captures it.
                        let owner = v.func;
                        for ctx in self.fns.iter_mut().rev() {
                            if Some(ctx.id) == owner {
                                break;
                            }
                            if !ctx.captures.contains(&id) {
                                ctx.captures.push(id);
                            }
                        }
                    }
                }
            }
            let path = Path {
                root: Root::Var(id),
                fields: Vec::new(),
            };
            if let Some(i) = self.active_narrow(&path) {
                let n = &self.narrows[i];
                self.narrowed_uses.insert(e.id, (n.name.clone(), n.test_id));
                return Ok(n.inner.clone());
            }
            let v = &self.vars[&id];
            return match v.role {
                Role::Class => self.err(
                    format!("`{name}` is a class: use `new {name}(…)` or a static member"),
                    e.span,
                ),
                Role::Enum => self.err(format!("`{name}` is an enum: use `{name}.Member`"), e.span),
                _ => Ok(v.ty.clone()),
            };
        }
        match name {
            "Infinity" | "NaN" => Ok(self.types.float()),
            _ if GLOBALS.contains(&name) => {
                self.err(format!("`{name}` cannot be used as a value here"), e.span)
            }
            _ => self.err(format!("unknown name `{name}`"), e.span),
        }
    }

    fn binary(&mut self, op: BinOp, left: &'m Expr, right: &'m Expr, span: Span) -> IResult<Ty> {
        match op {
            BinOp::Add => {
                let lt = self.expr(left, None)?;
                let lr = self.types.resolve(&lt);
                let rt = self.expr(
                    right,
                    if matches!(lr, Ty::Str) {
                        None
                    } else {
                        Some(&lt)
                    },
                )?;
                let rr = self.types.resolve(&rt);
                if matches!(lr, Ty::Str) || matches!(rr, Ty::Str) {
                    return Ok(Ty::Str);
                }
                let n = self.types.num(Flavor::Unknown);
                self.expect(&lt, &n, left.span)?;
                self.expect(&rt, &n, right.span)?;
                Ok(n)
            }
            BinOp::Sub | BinOp::Mul | BinOp::Mod | BinOp::Pow => {
                let n = self.types.num(Flavor::Unknown);
                let lt = self.expr(left, Some(&n))?;
                self.expect(&lt, &n, left.span)?;
                let rt = self.expr(right, Some(&n))?;
                self.expect(&rt, &n, right.span)?;
                Ok(n)
            }
            BinOp::Div => {
                let n = self.types.num(Flavor::Unknown);
                let lt = self.expr(left, Some(&n))?;
                self.expect(&lt, &n, left.span)?;
                let rt = self.expr(right, Some(&n))?;
                self.expect(&rt, &n, right.span)?;
                Ok(self.types.float())
            }
            BinOp::Lt | BinOp::Gt | BinOp::LtEq | BinOp::GtEq => {
                let lt = self.expr(left, None)?;
                let rt = self.expr(right, Some(&lt))?;
                if self.types.unify(&lt, &rt).is_err() {
                    let (a, b) = (self.describe(&lt), self.describe(&rt));
                    return self.err(format!("cannot compare {a} with {b}"), span);
                }
                match self.types.resolve(&lt) {
                    Ty::Num(_) | Ty::Str | Ty::Var(_) => Ok(Ty::Bool),
                    Ty::Named(n) if self.is_type_param(&n) => Ok(Ty::Bool),
                    other => {
                        let d = self.describe(&other);
                        self.err(format!("cannot order {d}"), span)
                    }
                }
            }
            BinOp::Eq | BinOp::NotEq | BinOp::StrictEq | BinOp::StrictNotEq => {
                let lt = self.expr(left, None)?;
                let rt = self.expr(right, Some(&lt))?;
                if self.types.unify(&lt, &rt).is_err()
                    && !self.union_accepts(&lt, &rt)
                    && !self.union_accepts(&rt, &lt)
                {
                    let (a, b) = (self.describe(&lt), self.describe(&rt));
                    return self.err(format!("cannot compare {a} with {b}"), span);
                }
                Ok(Ty::Bool)
            }
            BinOp::Shl
            | BinOp::Shr
            | BinOp::UShr
            | BinOp::BitAnd
            | BinOp::BitOr
            | BinOp::BitXor => {
                let n = self.types.int();
                let lt = self.expr(left, Some(&n))?;
                self.expect(&lt, &n, left.span)?;
                let rt = self.expr(right, Some(&n))?;
                self.expect(&rt, &n, right.span)?;
                Ok(n)
            }
            BinOp::In => {
                let rt = self.expr(right, None)?;
                match self.types.resolve(&rt) {
                    Ty::Map(k, _, _) => {
                        let lt = self.expr(left, Some(&k))?;
                        self.expect(&lt, &k, left.span)?;
                        Ok(Ty::Bool)
                    }
                    _ => self.err("`in` is only supported on a Map or a Record", span),
                }
            }
            BinOp::InstanceOf => {
                // Outside a condition: decided by the types.
                self.expr(left, None)?;
                let ExprKind::Ident(class) = &right.kind else {
                    return self.err("`instanceof` needs a class name", right.span);
                };
                if self
                    .struct_by_name(class)
                    .is_none_or(|s| s.kind != StructKind::Class)
                {
                    return self.err(format!("`{class}` is not a class"), right.span);
                }
                Ok(Ty::Bool)
            }
        }
    }

    fn assign(
        &mut self,
        op: AssignOp,
        target: &'m AssignTarget,
        value: &'m Expr,
        span: Span,
    ) -> IResult<Ty> {
        match target {
            AssignTarget::Pat(p) => {
                if op != AssignOp::Assign {
                    return self.err("compound assignment to a pattern", span);
                }
                let vt = self.expr(value, None)?;
                self.assign_pattern(p, &vt)?;
                Ok(Ty::Void)
            }
            AssignTarget::Expr(t) => {
                self.check_assignable(t)?;
                // A plain `x = …` writes `x` without reading it: not a use that keeps the
                // previous value alive.
                let tt = match (&t.kind, op) {
                    (ExprKind::Ident(name), AssignOp::Assign) if self.lookup(name).is_some() => {
                        let id = self.lookup(name).expect("binding");
                        self.refs.insert(t.id, id);
                        let ty = self.vars[&id].ty.clone();
                        self.set_ty(t.id, ty)
                    }
                    _ => self.expr(t, None)?,
                };
                match op {
                    AssignOp::Assign => {
                        let vt = self.expr(value, Some(&tt))?;
                        self.expect(&vt, &tt, value.span)?;
                        if !matches!(t.kind, ExprKind::Ident(_)) {
                            self.mark_stored(value);
                        } else if !super::stmts::is_place(value)
                            && let Some(b) = self.root_binding(t)
                            && let Some(v) = self.vars.get_mut(&b)
                        {
                            v.alias = false;
                        }
                        self.mark_mutated(t);
                        Ok(tt)
                    }
                    AssignOp::Nullish | AssignOp::Or | AssignOp::And => {
                        let inner = match self.types.resolve(&tt) {
                            Ty::Option(i) if op == AssignOp::Nullish => *i,
                            Ty::Bool if op != AssignOp::Nullish => Ty::Bool,
                            _ => {
                                return self
                                    .err("this logical assignment is not supported here", span);
                            }
                        };
                        let vt = self.expr(value, Some(&inner))?;
                        self.expect(&vt, &inner, value.span)?;
                        self.mark_mutated(t);
                        Ok(tt)
                    }
                    _ => {
                        let bin = op.binary().expect("compound");
                        let vt = self.expr(value, Some(&tt))?;
                        match (bin, self.types.resolve(&tt)) {
                            (BinOp::Add, Ty::Str) => {}
                            (BinOp::Div, _) => {
                                let f = self.types.float();
                                self.expect(&tt, &f, t.span)?;
                                self.expect(&vt, &f, value.span)?;
                            }
                            (b, _) if b.is_bitwise() => {
                                let i = self.types.int();
                                self.expect(&tt, &i, t.span)?;
                                self.expect(&vt, &i, value.span)?;
                            }
                            _ => {
                                let n = self.types.num(Flavor::Unknown);
                                self.expect(&tt, &n, t.span)?;
                                self.expect(&vt, &n, value.span)?;
                            }
                        }
                        self.mark_mutated(t);
                        Ok(tt)
                    }
                }
            }
        }
    }

    fn check_assignable(&self, t: &Expr) -> IResult<()> {
        match &t.kind {
            ExprKind::Ident(name) => {
                if let Some(id) = self.lookup(name) {
                    let v = &self.vars[&id];
                    if matches!(v.role, Role::Func | Role::Class | Role::Enum) {
                        return self.err(format!("cannot assign to `{name}`"), t.span);
                    }
                }
                Ok(())
            }
            ExprKind::Member { .. } | ExprKind::Index { .. } | ExprKind::NonNull(_) => Ok(()),
            _ => self.err("invalid assignment target", t.span),
        }
    }

    /// `[a, b] = …`: assignments to existing variables.
    fn assign_pattern(&mut self, p: &'m Pattern, ty: &Ty) -> IResult<()> {
        match p {
            Pattern::Ident { name, span, id } => {
                let Some(binding) = self.lookup(name) else {
                    return self.err(format!("unknown name `{name}`"), *span);
                };
                self.refs.insert(*id, binding);
                let vt = self.vars[&binding].ty.clone();
                self.expect(ty, &vt, *span)?;
                let v = self.vars.get_mut(&binding).expect("var");
                v.reassigned = true;
                Ok(())
            }
            Pattern::Array { elems, rest, span } => {
                if rest.is_some() {
                    return self.err("rest elements are not supported in assignments", *span);
                }
                let ty = self.types.resolve(ty);
                for (i, el) in elems.iter().enumerate() {
                    let Some(el) = el else { continue };
                    let item = match &ty {
                        Ty::Tuple(ts) => ts.get(i).cloned().unwrap_or(Ty::Any),
                        Ty::Array(t) => (**t).clone(),
                        other => {
                            let d = self.describe(other);
                            return self.err(format!("cannot destructure {d}"), *span);
                        }
                    };
                    if el.default.is_some() {
                        return self.err("defaults are not supported in assignments", *span);
                    }
                    self.assign_pattern(&el.pat, &item)?;
                }
                Ok(())
            }
            Pattern::Object { span, .. } => {
                self.err("object patterns are not supported in assignments", *span)
            }
        }
    }

    fn object_literal(
        &mut self,
        e: &'m Expr,
        props: &'m [Prop],
        expected: Option<&Ty>,
    ) -> IResult<Ty> {
        let expected = expected.filter(|t| !matches!(t, Ty::Var(_)));
        // A map literal.
        if let Some(Ty::Map(k, v, record)) = expected {
            let (k, v, record) = ((**k).clone(), (**v).clone(), *record);
            for p in props {
                let PropValue::Expr(value) = &p.value else {
                    return self.err(
                        "only `key: value` entries are allowed in a map literal",
                        p.span,
                    );
                };
                match &p.key {
                    PropKey::Computed(kx) => {
                        let kt = self.expr(kx, Some(&k))?;
                        self.expect(&kt, &k, kx.span)?;
                    }
                    PropKey::Num(_) => {
                        let n = self.types.num(Flavor::Unknown);
                        self.expect(&n, &k, p.span)?;
                    }
                    _ => self.expect(&Ty::Str, &k, p.span)?,
                }
                let vt = self.expr(value, Some(&v))?;
                self.mark_stored(value);
                self.expect(&vt, &v, value.span)?;
            }
            return Ok(Ty::Map(Box::new(k), Box::new(v), record));
        }
        let spread_struct = match props.iter().find_map(|p| match &p.value {
            PropValue::Spread(s) => Some(s),
            _ => None,
        }) {
            Some(s) if expected.is_none() => {
                let st = self.expr(s, None)?;
                match self.types.resolve(&st) {
                    Ty::Named(n) if self.struct_by_name(&n).is_some() => Some(n),
                    _ => None,
                }
            }
            _ => None,
        };
        let struct_name = match (expected, spread_struct) {
            (Some(Ty::Named(n)), _) if self.struct_by_name(n).is_some() => n.clone(),
            (Some(Ty::Named(n)), _) if self.union_by_name(n).is_some() => {
                let u = self.union_by_name(n).expect("union").clone();
                match self.union_literal_member(&u, props) {
                    Some(member) => member,
                    None => {
                        return self.err(format!("this object matches no member of {n}"), e.span);
                    }
                }
            }
            (_, Some(n)) => n,
            _ => return self.anonymous_object(e, props),
        };
        self.literal_struct.insert(e.id, struct_name.clone());
        let fields = self
            .struct_by_name(&struct_name)
            .expect("struct")
            .fields
            .clone();
        let mut seen = HashSet::new();
        let mut spread = false;
        for p in props {
            if let PropValue::Spread(s) = &p.value {
                let st = self.expr(s, Some(&Ty::Named(struct_name.clone())))?;
                self.expect(&st, &Ty::Named(struct_name.clone()), s.span)?;
                spread = true;
                continue;
            }
            let Some(key) = p.key.name() else {
                return self.err("computed keys are not allowed here", p.span);
            };
            let Some(f) = fields.iter().find(|f| f.name == key) else {
                return self.err(format!("{struct_name} has no property `{key}`"), p.span);
            };
            let fty = if f.optional {
                Ty::Option(Box::new(f.ty.clone()))
            } else {
                f.ty.clone()
            };
            match &p.value {
                PropValue::Expr(v) => {
                    self.hints.insert(v.id, format!("{struct_name} {key}"));
                    let vt = self.expr(v, Some(&fty))?;
                    self.mark_stored(v);
                    self.expect(&vt, &fty, v.span)?;
                }
                PropValue::Shorthand(id) => {
                    let vt = self.shorthand(&key, *id, p.span)?;
                    self.expect(&vt, &fty, p.span)?;
                }
                _ => return self.err("object methods are not supported: use a class", p.span),
            }
            seen.insert(key);
        }
        if !spread {
            for f in &fields {
                if !f.optional && !seen.contains(&f.name) {
                    return self.err(
                        format!("property `{}` of {struct_name} is missing", f.name),
                        e.span,
                    );
                }
            }
        }
        Ok(Ty::Named(struct_name))
    }

    /// `{ x }`: the variable named like the key.
    fn shorthand(&mut self, key: &str, id: NodeId, span: Span) -> IResult<Ty> {
        let Some(binding) = self.lookup(key) else {
            return self.err(format!("unknown name `{key}`"), span);
        };
        self.refs.insert(id, binding);
        self.record_use(binding, id);
        let v = self.vars.get_mut(&binding).expect("var");
        v.stored = true;
        if let Some(p) = v.param_of {
            self.param_stored.insert(p);
        }
        Ok(self.vars[&binding].ty.clone())
    }

    /// An object literal with no expected type: a struct named after the context.
    fn anonymous_object(&mut self, e: &'m Expr, props: &'m [Prop]) -> IResult<Ty> {
        let hint = self.hints.get(&e.id).cloned().unwrap_or_default();
        let mut fields = Vec::new();
        for p in props {
            let Some(key) = p.key.name() else {
                return self.err("computed keys need a `Record` or `Map` type", p.span);
            };
            let fty = match &p.value {
                PropValue::Expr(v) => {
                    self.hints.insert(v.id, format!("{hint} {key}"));
                    let t = self.expr(v, None)?;
                    self.mark_stored(v);
                    t
                }
                PropValue::Shorthand(id) => self.shorthand(&key, *id, p.span)?,
                PropValue::Spread(_) => {
                    return self.err("spread needs a known struct type", p.span);
                }
                _ => return self.err("object methods are not supported: use a class", p.span),
            };
            let fty = match self.types.resolve(&fty) {
                Ty::Null => {
                    let t = self.types.fresh();
                    let opt = Ty::Option(Box::new(t));
                    self.unify(&fty, &opt, p.span)?;
                    opt
                }
                _ => fty,
            };
            fields.push(FieldInfo {
                name: key,
                ty: fty,
                optional: false,
                access: None,
                readonly: false,
                has_init: false,
                literal: None,
                span: p.span,
            });
        }
        // The same shape seen before: reuse it.
        if let Some(existing) = self.same_shape(&fields) {
            let shape = self
                .struct_by_name(&existing)
                .expect("struct")
                .fields
                .clone();
            for (f, g) in fields.iter().zip(&shape) {
                self.unify(&f.ty, &g.ty, f.span)?;
            }
            self.literal_struct.insert(e.id, existing.clone());
            return Ok(Ty::Named(existing));
        }
        let name = self.synth_name(&hint);
        self.structs.push(StructInfo {
            name: name.clone(),
            kind: StructKind::Synth,
            fields,
            methods: Vec::new(),
            ctor: None,
            exported: false,
            span: e.span,
            statics: Vec::new(),
            parent: None,
            is_abstract: false,
        });
        self.literal_struct.insert(e.id, name.clone());
        Ok(Ty::Named(name))
    }

    fn same_shape(&self, fields: &[FieldInfo]) -> Option<String> {
        'outer: for s in &self.structs {
            if s.fields.len() != fields.len() || s.kind == StructKind::Class {
                continue;
            }
            for f in fields {
                let Some(g) = s.field(&f.name) else {
                    continue 'outer;
                };
                let (a, b) = (self.types.zonk(&f.ty), self.types.zonk(&g.ty));
                if g.optional != f.optional || !same_shape_ty(&a, &b) {
                    continue 'outer;
                }
            }
            return Some(s.name.clone());
        }
        None
    }

    fn closure(&mut self, f: &'m Function, expected: Option<&Ty>) -> IResult<Ty> {
        let mut info = self.signature(f, None, None, false)?;
        if let Some(Ty::Func(ex)) = expected {
            for (i, p) in ex.params.iter().enumerate() {
                if let Some(mine) = info.sig.params.get(i) {
                    let _ = self.types.unify(mine, p);
                }
            }
            let _ = self.types.unify(&info.sig.ret, &ex.ret);
        }
        info.is_closure = true;
        let ty = Ty::Func(Rc::new(info.sig.clone()));
        let this = self.fns.last().and_then(|c| c.this.clone());
        self.enter_function(f, info, this, false)?;
        Ok(ty)
    }

    fn member(&mut self, e: &'m Expr, obj: &'m Expr, prop: &str, optional: bool) -> IResult<Ty> {
        // Globals: `Math.PI`, `Number.MAX_SAFE_INTEGER`.
        if let ExprKind::Ident(name) = &obj.kind
            && self.lookup(name).is_none()
        {
            let t = match (name.as_str(), prop) {
                (
                    "Math",
                    "PI" | "E" | "LN2" | "LN10" | "LOG2E" | "LOG10E" | "SQRT2" | "SQRT1_2",
                ) => self.types.float(),
                ("Number", "MAX_SAFE_INTEGER" | "MIN_SAFE_INTEGER") => self.types.int(),
                (
                    "Number",
                    "MAX_VALUE" | "MIN_VALUE" | "EPSILON" | "POSITIVE_INFINITY"
                    | "NEGATIVE_INFINITY" | "NaN",
                ) => self.types.float(),
                ("process", "argv") => Ty::Array(Box::new(Ty::Str)),
                _ if GLOBALS.contains(&name.as_str()) => {
                    return self.err(format!("`{name}.{prop}` is not supported"), e.span);
                }
                _ => return self.err(format!("unknown name `{name}`"), obj.span),
            };
            self.calls
                .insert(e.id, Callee::Builtin(leak(format!("{name}.{prop}"))));
            return Ok(t);
        }
        // A narrowed path?
        if let Some(path) = self.path_of(e)
            && let Some(i) = self.active_narrow(&path)
        {
            let n = &self.narrows[i];
            let (name, inner, test_id) = (n.name.clone(), n.inner.clone(), n.test_id);
            self.expr(obj, None)?;
            self.narrowed_uses.insert(e.id, (name, test_id));
            return Ok(inner);
        }
        // Static members and enum variants.
        if let ExprKind::Ident(name) = &obj.kind
            && let Some(id) = self.lookup(name)
        {
            match self.vars[&id].role {
                Role::Enum => {
                    self.refs.insert(obj.id, id);
                    let en = self.enum_by_name(name).expect("enum");
                    if !en.variants.iter().any(|v| v.source == prop) {
                        return self.err(format!("{name} has no member `{prop}`"), e.span);
                    }
                    self.set_ty(obj.id, Ty::Named(name.clone()));
                    return Ok(Ty::Named(name.clone()));
                }
                Role::Class => {
                    self.refs.insert(obj.id, id);
                    self.set_ty(obj.id, Ty::Named(name.clone()));
                    if let Some((_, t)) = self.find_static(name, prop) {
                        return Ok(t);
                    }
                    if let Some((_, m)) = self.find_method(name, prop, MethodKind::Get)
                        && m.is_static
                    {
                        return Ok(self.funcs[&m.fn_id].sig.ret.clone());
                    }
                    return self.err(format!("{name} has no static member `{prop}`"), e.span);
                }
                _ => {}
            }
        }
        let ot = self.expr(obj, None)?;
        let ot = self.types.resolve(&ot);
        // `it.next().value` / `.done`: the result of a generator step is its value.
        if let Ty::Option(_) = &ot
            && !optional
            && self.is_iter_result(obj)
        {
            return Ok(match prop {
                "value" => ot.clone(),
                "done" => Ty::Bool,
                _ => return self.err(format!("a generator step has no property `{prop}`"), e.span),
            });
        }
        let (ot, lifted) = match (&ot, optional) {
            (Ty::Option(inner), true) => (self.types.resolve(inner), true),
            (Ty::Option(_), false) => {
                let d = self.describe(&ot);
                return self.err(
                    format!(
                        "`{}` may be null here ({d}): test it with `if (… !== null)`, or use `?.` or `!`",
                        self.text(obj)
                    ),
                    e.span,
                );
            }
            _ => (ot, false),
        };
        let t = self.member_of(&ot, prop, e.span, obj)?;
        Ok(if lifted {
            match self.types.resolve(&t) {
                Ty::Option(t) => Ty::Option(t),
                t => Ty::Option(Box::new(t)),
            }
        } else {
            t
        })
    }

    pub(super) fn member_of(&mut self, ot: &Ty, prop: &str, span: Span, obj: &Expr) -> IResult<Ty> {
        match ot {
            Ty::Named(n) => {
                if n == "Error" {
                    return match prop {
                        "message" | "name" | "stack" => Ok(Ty::Str),
                        _ => self.err(format!("Error has no property `{prop}`"), span),
                    };
                }
                if self.is_type_param(n) {
                    return self.err(
                        format!("`{prop}` is not known on the type parameter {n}"),
                        span,
                    );
                }
                let Some(s) = self.struct_by_name(n) else {
                    if self.enum_by_name(n).is_some() {
                        return self.err(format!("enum values have no property `{prop}`"), span);
                    }
                    if let Some(u) = self.union_by_name(n) {
                        return match self.union_field(u, prop) {
                            Some(t) => Ok(t),
                            None => self.err(
                                format!(
                                    "`{prop}` is not a property of every member of {n}: test which member `{}` is first",
                                    self.text(obj)
                                ),
                                span,
                            ),
                        };
                    }
                    return self.err(format!("{n} has no property `{prop}`"), span);
                };
                if let Some(f) = s.field(prop) {
                    return Ok(if f.optional {
                        Ty::Option(Box::new(f.ty.clone()))
                    } else {
                        f.ty.clone()
                    });
                }
                if let Some((_, m)) = self.find_method(n, prop, MethodKind::Get) {
                    return Ok(self.funcs[&m.fn_id].sig.ret.clone());
                }
                if self.find_method(n, prop, MethodKind::Method).is_some() {
                    return self.err(
                        format!(
                            "`{prop}` is a method: call it, `{}.{prop}()`",
                            self.text(obj)
                        ),
                        span,
                    );
                }
                self.err(format!("{n} has no property `{prop}`"), span)
            }
            Ty::Array(_) | Ty::Str | Ty::Tuple(_) if prop == "length" => Ok(self.types.int()),
            Ty::Map(..) | Ty::Set(_) if prop == "size" => Ok(self.types.int()),
            Ty::Var(_) => self.err(
                format!(
                    "the type of `{}` is not known yet: add an annotation",
                    self.text(obj)
                ),
                obj.span,
            ),
            Ty::Any => self.err(
                format!(
                    "`{}` is `any`: give it a type to use `.{prop}`",
                    self.text(obj)
                ),
                obj.span,
            ),
            other => {
                let d = self.describe(other);
                self.err(format!("{d} has no property `{prop}`"), span)
            }
        }
    }

    fn index(&mut self, obj: &'m Expr, index: &'m Expr, span: Span) -> IResult<Ty> {
        let ot = self.expr(obj, None)?;
        let ot = self.types.resolve(&ot);
        match &ot {
            Ty::Array(t) => {
                let it = self.expr(index, None)?;
                let n = self.types.num(Flavor::Unknown);
                self.expect(&it, &n, index.span)?;
                Ok((**t).clone())
            }
            Ty::Tuple(ts) => {
                let ExprKind::Num { value, .. } = &index.kind else {
                    return self.err("tuples are indexed with a constant", index.span);
                };
                let it = self.types.int();
                self.set_ty(index.id, it);
                ts.get(*value as usize).cloned().ok_or_else(|| {
                    crate::error::Error::new(format!("the tuple has no element {value}"), span)
                })
            }
            Ty::Str => {
                let it = self.expr(index, None)?;
                let n = self.types.num(Flavor::Unknown);
                self.expect(&it, &n, index.span)?;
                Ok(Ty::Str)
            }
            Ty::Map(k, v, _) => {
                let it = self.expr(index, Some(k))?;
                self.expect(&it, k, index.span)?;
                Ok((**v).clone())
            }
            Ty::Named(n) if self.struct_by_name(n).is_some() => {
                let ExprKind::Str(key) = &index.kind else {
                    return self.err(format!("{n} is indexed with a property name"), index.span);
                };
                self.set_ty(index.id, Ty::Str);
                let key = key.clone();
                self.member_of(&ot, &key, span, obj)
            }
            Ty::Option(_) => self.err(
                format!("`{}` may be null here: test it first", self.text(obj)),
                obj.span,
            ),
            Ty::Var(_) => {
                let t = self.types.fresh();
                self.unify(&ot, &Ty::Array(Box::new(t.clone())), obj.span)?;
                let it = self.expr(index, None)?;
                let n = self.types.num(Flavor::Unknown);
                self.expect(&it, &n, index.span)?;
                Ok(t)
            }
            other => {
                let d = self.describe(other);
                self.err(format!("{d} cannot be indexed"), span)
            }
        }
    }

    fn new_expr(&mut self, e: &'m Expr, callee: &'m Expr, args: &'m [Arg]) -> IResult<Ty> {
        let ExprKind::Ident(name) = &callee.kind else {
            return self.err("`new` needs a class name", callee.span);
        };
        if let Some(id) = self.lookup(name) {
            if self.vars[&id].role != Role::Class {
                return self.err(format!("`{name}` is not a class"), callee.span);
            }
            self.refs.insert(callee.id, id);
            self.set_ty(callee.id, Ty::Named(name.clone()));
            let s = self.struct_by_name(name).expect("class");
            if s.is_abstract {
                return self.err(
                    format!("{name} is abstract: `new` one of its subclasses"),
                    e.span,
                );
            }
            match self.nearest_ctor(name) {
                Some((_, ctor)) => {
                    let info = self.funcs[&ctor].clone();
                    self.args(args, &info, ctor, e.span)?;
                    let caller = self.ctx().id;
                    self.funcs
                        .get_mut(&caller)
                        .expect("fn")
                        .calls
                        .push((ctor, e.id));
                }
                None => {
                    if !args.is_empty() {
                        return self.err(
                            format!("{name} has no constructor: `new {name}()` takes no arguments"),
                            e.span,
                        );
                    }
                }
            }
            self.calls.insert(e.id, Callee::Ctor(name.clone()));
            return Ok(Ty::Named(name.clone()));
        }
        match name.as_str() {
            "Error" | "TypeError" | "RangeError" => {
                if let Some(a) = args.first() {
                    let t = self.expr(&a.expr, Some(&Ty::Str))?;
                    self.expect(&t, &Ty::Str, a.expr.span)?;
                }
                self.calls.insert(e.id, Callee::Builtin("new Error"));
                Ok(Ty::Named("Error".into()))
            }
            "Map" => {
                let (k, v) = (self.types.fresh(), self.types.fresh());
                if let Some(a) = args.first() {
                    let entries = Ty::Array(Box::new(Ty::Tuple(vec![k.clone(), v.clone()])));
                    let t = self.expr(&a.expr, Some(&entries))?;
                    self.expect(&t, &entries, a.expr.span)?;
                }
                self.calls.insert(e.id, Callee::Builtin("new Map"));
                Ok(Ty::Map(Box::new(k), Box::new(v), false))
            }
            "Set" => {
                let t = self.types.fresh();
                if let Some(a) = args.first() {
                    let arr = Ty::Array(Box::new(t.clone()));
                    let at = self.expr(&a.expr, Some(&arr))?;
                    self.expect(&at, &arr, a.expr.span)?;
                }
                self.calls.insert(e.id, Callee::Builtin("new Set"));
                Ok(Ty::Set(Box::new(t)))
            }
            "Array" => {
                let t = self.types.fresh();
                if let Some(a) = args.first() {
                    let n = self.types.int();
                    let at = self.expr(&a.expr, Some(&n))?;
                    self.expect(&at, &n, a.expr.span)?;
                }
                self.calls.insert(e.id, Callee::Builtin("new Array"));
                Ok(Ty::Array(Box::new(t)))
            }
            "Date" => self.err("`Date` is not supported", e.span),
            "Promise" => self.err(
                "`new Promise` is not supported: use `async` functions",
                e.span,
            ),
            "RegExp" => self.err("regular expressions are not supported", e.span),
            _ => self.err(format!("unknown class `{name}`"), callee.span),
        }
    }

    /// Checks the arguments of a call against a signature, with contextual types.
    pub(super) fn args(
        &mut self,
        args: &'m [Arg],
        info: &FnInfo,
        fn_id: NodeId,
        span: Span,
    ) -> IResult<()> {
        let required = info
            .optional
            .iter()
            .zip(&info.defaults)
            .filter(|(o, d)| !**o && !**d)
            .count();
        if args.len() < required || args.len() > info.sig.params.len() {
            let name = info.name.clone().unwrap_or_else(|| "the function".into());
            let n = info.sig.params.len();
            return self.err(
                format!(
                    "`{name}` takes {} argument{}, {} given",
                    if required == n {
                        required.to_string()
                    } else {
                        format!("{required} to {n}")
                    },
                    if n == 1 { "" } else { "s" },
                    args.len()
                ),
                span,
            );
        }
        // Type parameters are instantiated afresh for each call.
        let mut subst: HashMap<String, Ty> = HashMap::new();
        for tp in &info.type_params {
            subst.insert(tp.clone(), self.types.fresh());
        }
        for (i, a) in args.iter().enumerate() {
            if a.spread {
                return self.err("spread arguments are not supported here", a.expr.span);
            }
            let mut pt = info.sig.params[i].clone();
            if !subst.is_empty() {
                pt = substitute(&pt, &subst, &self.types);
            }
            let inner = match (self.types.resolve(&pt), info.optional[i]) {
                (Ty::Option(t), true) => Some(*t),
                _ => None,
            };
            let at = self.expr(&a.expr, Some(inner.as_ref().unwrap_or(&pt)))?;
            self.expect(&at, &pt, a.expr.span)?;
            if let ExprKind::Ident(name) = &a.expr.kind
                && let Some(b) = self.lookup(name)
            {
                self.arg_bindings.push((fn_id, i, b));
            }
        }
        Ok(())
    }

    fn call(
        &mut self,
        e: &'m Expr,
        callee: &'m Expr,
        args: &'m [Arg],
        optional: bool,
    ) -> IResult<Ty> {
        if optional {
            return self.err("optional calls `f?.()` are not supported", e.span);
        }
        if self.ctx().try_depth > 0 {
            self.calls_in_try.insert(e.id);
        }
        match &callee.kind {
            ExprKind::Super => self.super_call(e, args),
            ExprKind::Member { obj, prop, .. } if matches!(obj.kind, ExprKind::Super) => {
                self.super_method_call(e, prop, args)
            }
            ExprKind::Ident(name) => {
                if let Some(id) = self.lookup(name) {
                    self.refs.insert(callee.id, id);
                    self.record_use(id, callee.id);
                    let v = self.vars[&id].clone();
                    match v.role {
                        Role::Func => {
                            let fn_id = self.fn_bindings[&id];
                            let info = self.funcs[&fn_id].clone();
                            self.set_ty(callee.id, Ty::Func(Rc::new(info.sig.clone())));
                            self.args(args, &info, fn_id, e.span)?;
                            self.calls.insert(e.id, Callee::Fn(id));
                            let caller = self.ctx().id;
                            self.funcs
                                .get_mut(&caller)
                                .expect("fn")
                                .calls
                                .push((fn_id, e.id));
                            if !info.type_params.is_empty() {
                                return self.instantiate_ret(&info, args);
                            }
                            return Ok(info.sig.ret.clone());
                        }
                        Role::Class => {
                            return self.err(
                                format!("`{name}` is a class: use `new {name}(…)`"),
                                callee.span,
                            );
                        }
                        _ => {
                            let ft = self.types.resolve(&v.ty);
                            let Ty::Func(f) = ft else {
                                if let Ty::Var(_) = ft {
                                    return self.err(
                                        format!("the type of `{name}` is not known yet"),
                                        callee.span,
                                    );
                                }
                                let d = self.describe(&ft);
                                return self
                                    .err(format!("`{name}` is {d}, not a function"), callee.span);
                            };
                            self.set_ty(callee.id, Ty::Func(f.clone()));
                            self.dynamic_args(args, &f, e.span)?;
                            self.calls.insert(e.id, Callee::Closure(id));
                            return Ok(f.ret.clone());
                        }
                    }
                }
                self.global_call(e, name, args, callee.span)
            }
            ExprKind::Member {
                obj,
                prop,
                optional,
            } => {
                if *optional {
                    return self.err(
                        "optional method calls are not supported: test the value first",
                        e.span,
                    );
                }
                self.method_call(e, obj, prop, args)
            }
            _ => {
                let ct = self.expr(callee, None)?;
                let Ty::Func(f) = self.types.resolve(&ct) else {
                    let d = self.describe(&ct);
                    return self.err(format!("cannot call {d}"), callee.span);
                };
                self.dynamic_args(args, &f, e.span)?;
                self.calls.insert(e.id, Callee::Dynamic);
                Ok(f.ret.clone())
            }
        }
    }

    /// `super(args)`: the constructor of the base class runs first.
    fn super_call(&mut self, e: &'m Expr, args: &'m [Arg]) -> IResult<Ty> {
        let Some(class) = self.ctx().this.clone() else {
            return self.err("`super(…)` outside a class", e.span);
        };
        if !self.ctx().in_ctor {
            return self.err("`super(…)` belongs in the constructor", e.span);
        }
        let Some(parent) = self.struct_by_name(&class).and_then(|s| s.parent.clone()) else {
            return self.err(format!("{class} has no base class"), e.span);
        };
        match self.nearest_ctor(&parent) {
            Some((owner, ctor)) => {
                let info = self.funcs[&ctor].clone();
                self.args(args, &info, ctor, e.span)?;
                let caller = self.ctx().id;
                self.funcs
                    .get_mut(&caller)
                    .expect("fn")
                    .calls
                    .push((ctor, e.id));
                self.calls.insert(e.id, Callee::SuperCtor(owner));
            }
            None => {
                if !args.is_empty() {
                    return self.err(
                        format!("{parent} has no constructor: `super()` takes no arguments"),
                        e.span,
                    );
                }
                self.calls.insert(e.id, Callee::SuperCtor(parent));
            }
        }
        Ok(Ty::Void)
    }

    /// `super.m(args)`: the base class's version of a method.
    fn super_method_call(&mut self, e: &'m Expr, prop: &str, args: &'m [Arg]) -> IResult<Ty> {
        let Some(class) = self.ctx().this.clone() else {
            return self.err("`super` outside a class", e.span);
        };
        let Some(parent) = self.struct_by_name(&class).and_then(|s| s.parent.clone()) else {
            return self.err(format!("{class} has no base class"), e.span);
        };
        let Some((owner, m)) = self.find_method(&parent, prop, MethodKind::Method) else {
            return self.err(
                format!("no method `{prop}` in the base classes of {class}"),
                e.span,
            );
        };
        if m.is_abstract {
            return self.err(
                format!("`{prop}` is abstract in {owner}: nothing to call"),
                e.span,
            );
        }
        let info = self.funcs[&m.fn_id].clone();
        self.args(args, &info, m.fn_id, e.span)?;
        let caller = self.ctx().this_fn;
        if !self.ctx().in_ctor {
            self.this_calls.push((m.fn_id, caller));
        }
        let caller = self.ctx().id;
        self.funcs
            .get_mut(&caller)
            .expect("fn")
            .calls
            .push((m.fn_id, e.id));
        self.calls.insert(
            e.id,
            Callee::SuperMethod {
                class: owner,
                name: prop.to_string(),
                fn_id: m.fn_id,
            },
        );
        Ok(info.sig.ret.clone())
    }

    fn instantiate_ret(&mut self, info: &FnInfo, args: &'m [Arg]) -> IResult<Ty> {
        let mut subst: HashMap<String, Ty> = HashMap::new();
        for tp in &info.type_params {
            subst.insert(tp.clone(), self.types.fresh());
        }
        for (i, a) in args.iter().enumerate() {
            let pt = substitute(&info.sig.params[i], &subst, &self.types);
            let at = self.expr_ty[&a.expr.id].clone();
            self.expect(&at, &pt, a.expr.span)?;
        }
        Ok(substitute(&info.sig.ret, &subst, &self.types))
    }

    pub(super) fn dynamic_args(&mut self, args: &'m [Arg], f: &FuncTy, span: Span) -> IResult<()> {
        if args.len() > f.params.len() {
            return self.err(
                format!("too many arguments: {} expected", f.params.len()),
                span,
            );
        }
        for (i, a) in args.iter().enumerate() {
            if a.spread {
                return self.err("spread arguments are not supported", a.expr.span);
            }
            let pt = f.params[i].clone();
            let at = self.expr(&a.expr, Some(&pt))?;
            self.expect(&at, &pt, a.expr.span)?;
        }
        Ok(())
    }

    /// `parseInt(…)`, `String(…)`…
    fn global_call(&mut self, e: &'m Expr, name: &str, args: &'m [Arg], span: Span) -> IResult<Ty> {
        let ty = match name {
            "parseInt" => self.parse_int_call(args, name, span)?,
            "parseFloat" => {
                self.arg_typed(args, 0, Some(Ty::Str), name, span)?;
                self.types.float()
            }
            "Number" => {
                let t = self.arg_typed(args, 0, None, name, span)?;
                match self.types.resolve(&t) {
                    Ty::Str | Ty::Bool => self.types.float(),
                    Ty::Num(_) => t,
                    other => {
                        let d = self.describe(&other);
                        return self.err(format!("cannot convert {d} to a number"), span);
                    }
                }
            }
            "String" => {
                self.arg_typed(args, 0, None, name, span)?;
                Ty::Str
            }
            "Boolean" => {
                let Some(a) = args.first() else {
                    return self.err("`Boolean` needs an argument", span);
                };
                self.truthy(&a.expr)?;
                Ty::Bool
            }
            "isNaN" | "isFinite" => {
                self.nan_test_arg(args, name, span)?;
                Ty::Bool
            }
            "structuredClone" => self.arg_typed(args, 0, None, name, span)?,
            "require" => {
                return self.err(
                    "`require` is not supported: Solidify translates one file",
                    span,
                );
            }
            "setTimeout" | "setInterval" => return self.err("timers are not supported", span),
            "alert" | "prompt" | "confirm" => {
                return self.err("browser dialogs are not supported", span);
            }
            _ => return self.err(format!("unknown function `{name}`"), span),
        };
        self.calls
            .insert(e.id, Callee::Builtin(leak(format!("global.{name}"))));
        Ok(ty)
    }

    /// `parseInt(s)` is `Option<i64>` (`NaN` is `None`); `parseInt(n)` truncates a number.
    pub(super) fn parse_int_call(
        &mut self,
        args: &'m [Arg],
        name: &str,
        span: Span,
    ) -> IResult<Ty> {
        let t = self.arg_typed(args, 0, None, name, span)?;
        if args.len() > 1 {
            let i = self.types.int();
            self.arg_typed(args, 1, Some(i), name, span)?;
        }
        Ok(match self.types.resolve(&t) {
            Ty::Num(_) => self.types.int(),
            other => {
                self.expect(&other, &Ty::Str, args[0].expr.span)?;
                Ty::Option(Box::new(self.types.int()))
            }
        })
    }

    /// The argument of `isNaN` / `isFinite`: a number, or the `Option` of `parseInt`.
    pub(super) fn nan_test_arg(&mut self, args: &'m [Arg], name: &str, span: Span) -> IResult<Ty> {
        let t = self.arg_typed(args, 0, None, name, span)?;
        match self.types.resolve(&t) {
            Ty::Option(inner) => {
                let n = self.types.num(Flavor::Unknown);
                self.expect(&inner, &n, args[0].expr.span)?;
            }
            other => {
                let n = self.types.num(Flavor::Unknown);
                self.expect(&other, &n, args[0].expr.span)?;
            }
        }
        Ok(t)
    }

    /// Argument `i` of a builtin call, typed.
    pub(super) fn arg_typed(
        &mut self,
        args: &'m [Arg],
        i: usize,
        expected: Option<Ty>,
        name: &str,
        span: Span,
    ) -> IResult<Ty> {
        let Some(a) = args.get(i) else {
            return self.err(format!("`{name}` needs more arguments"), span);
        };
        if a.spread {
            return self.err("spread arguments are not supported here", a.expr.span);
        }
        let t = self.expr(&a.expr, expected.as_ref())?;
        if let Some(ex) = expected {
            self.expect(&t, &ex, a.expr.span)?;
        }
        Ok(t)
    }

    fn method_call(
        &mut self,
        e: &'m Expr,
        obj: &'m Expr,
        prop: &str,
        args: &'m [Arg],
    ) -> IResult<Ty> {
        // Namespaces: console, Math, Object, Array, Number, JSON…
        if let ExprKind::Ident(ns) = &obj.kind
            && self.lookup(ns).is_none()
        {
            if !GLOBALS.contains(&ns.as_str()) {
                return self.err(format!("unknown name `{ns}`"), obj.span);
            }
            return self.namespace_call(e, ns, prop, args);
        }
        // Static methods.
        if let ExprKind::Ident(name) = &obj.kind
            && let Some(id) = self.lookup(name)
            && self.vars[&id].role == Role::Class
        {
            self.refs.insert(obj.id, id);
            self.set_ty(obj.id, Ty::Named(name.clone()));
            let Some((owner, m)) = self
                .find_method(name, prop, MethodKind::Method)
                .filter(|(_, m)| m.is_static)
            else {
                return self.err(format!("{name} has no static method `{prop}`"), e.span);
            };
            let (fn_id, class) = (m.fn_id, owner);
            let info = self.funcs[&fn_id].clone();
            self.args(args, &info, fn_id, e.span)?;
            self.calls.insert(
                e.id,
                Callee::Method {
                    class,
                    name: prop.to_string(),
                    fn_id,
                    is_static: true,
                },
            );
            let caller = self.ctx().id;
            self.funcs
                .get_mut(&caller)
                .expect("fn")
                .calls
                .push((fn_id, e.id));
            return Ok(info.sig.ret.clone());
        }
        let ot = self.expr(obj, None)?;
        let ot = self.types.resolve(&ot);
        match &ot {
            Ty::Named(n) if n != "Error" && !self.is_type_param(n) => {
                let Some(s) = self.struct_by_name(n) else {
                    if self.union_by_name(n).is_some() {
                        return self.err(
                            format!(
                                "`{}` is a {n}: test which member it is before calling `{prop}`",
                                self.text(obj)
                            ),
                            obj.span,
                        );
                    }
                    return self.err(format!("{n} has no method `{prop}`"), e.span);
                };
                if let Some(f) = s.field(prop) {
                    // A field holding a function.
                    let ft = self.types.resolve(&f.ty);
                    let Ty::Func(fun) = ft else {
                        return self.err(format!("`{prop}` is a field, not a method"), e.span);
                    };
                    self.dynamic_args(args, &fun, e.span)?;
                    self.calls.insert(e.id, Callee::Dynamic);
                    return Ok(fun.ret.clone());
                }
                let Some((_, m)) = self.find_method(n, prop, MethodKind::Method) else {
                    return self.err(format!("{n} has no method `{prop}`"), e.span);
                };
                if m.is_static {
                    return self.err(format!("`{prop}` is static: call `{n}.{prop}()`"), e.span);
                }
                let (fn_id, class) = (m.fn_id, n.clone());
                let info = self.funcs[&fn_id].clone();
                self.args(args, &info, fn_id, e.span)?;
                // Whether the method mutates `self` is settled at the end: remember the
                // receiver.
                if super::stmts::uses_this(obj) {
                    let caller = self.ctx().this_fn;
                    if !self.ctx().in_ctor {
                        self.this_calls.push((fn_id, caller));
                    }
                } else if let Some(root) = self.root_binding(obj) {
                    self.method_receivers.push((fn_id, root));
                    if info.mutates_self {
                        self.mark_receiver_mutated(obj);
                    }
                }
                self.calls.insert(
                    e.id,
                    Callee::Method {
                        class,
                        name: prop.to_string(),
                        fn_id,
                        is_static: false,
                    },
                );
                let caller = self.ctx().id;
                self.funcs
                    .get_mut(&caller)
                    .expect("fn")
                    .calls
                    .push((fn_id, e.id));
                Ok(info.sig.ret.clone())
            }
            Ty::Option(_) => self.err(
                format!(
                    "`{}` may be null here: test it with `if (… !== null)` first",
                    self.text(obj)
                ),
                obj.span,
            ),
            Ty::Var(_) => {
                // A method that only strings or only arrays have decides the type.
                let guessed = if STRING_METHODS.contains(&prop) {
                    Some(Ty::Str)
                } else if ARRAY_METHODS.contains(&prop) {
                    Some(Ty::Array(Box::new(self.types.fresh())))
                } else {
                    None
                };
                let Some(guess) = guessed else {
                    return self.err(
                        format!(
                            "the type of `{}` is not known yet: add an annotation",
                            self.text(obj)
                        ),
                        obj.span,
                    );
                };
                self.unify(&ot, &guess, obj.span)?;
                let ot = self.types.resolve(&guess);
                self.builtin_method(e, obj, &ot, prop, args)
            }
            _ => self.builtin_method(e, obj, &ot, prop, args),
        }
    }
}

/// Methods only strings have: calling one on a value of unknown type makes it a string.
const STRING_METHODS: &[&str] = &[
    "toUpperCase",
    "toLowerCase",
    "trim",
    "trimStart",
    "trimEnd",
    "charAt",
    "charCodeAt",
    "codePointAt",
    "startsWith",
    "endsWith",
    "substring",
    "substr",
    "padStart",
    "padEnd",
    "repeat",
    "split",
    "replace",
    "replaceAll",
    "localeCompare",
    "match",
    "matchAll",
    "search",
];

/// Methods only arrays have.
const ARRAY_METHODS: &[&str] = &[
    "push",
    "pop",
    "shift",
    "unshift",
    "map",
    "filter",
    "forEach",
    "reduce",
    "reduceRight",
    "find",
    "findIndex",
    "findLast",
    "findLastIndex",
    "some",
    "every",
    "join",
    "splice",
    "reverse",
    "sort",
    "fill",
    "flat",
    "flatMap",
    "toSorted",
    "toReversed",
];

/// `users` → `user`, `entries` → `entry`.
fn singular(hint: &str) -> String {
    let hint = hint.trim();
    if let Some(stem) = hint.strip_suffix("ies") {
        format!("{stem}y")
    } else if let Some(stem) = hint.strip_suffix("ses") {
        stem.to_string()
    } else if hint.ends_with("ss") {
        hint.to_string()
    } else if let Some(stem) = hint.strip_suffix('s') {
        stem.to_string()
    } else {
        hint.to_string()
    }
}
