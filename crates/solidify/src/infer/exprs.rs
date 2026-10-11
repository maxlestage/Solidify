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
                self.types.int_literal()
            } else {
                self.types.float()
            }),
            ExprKind::Str(s) => {
                if let Some(Ty::Named(n, ..)) = &expected_res
                    && let Some(en) = self.enum_by_name(n)
                    && en.kind == EnumKind::Literals
                {
                    let Some(v) = en.variant_for_literal(s) else {
                        return self.err(format!("`{s}` is not a value of {n}"), e.span);
                    };
                    let variant = v.name.clone();
                    self.enum_literal.insert(e.id, (n.clone(), variant));
                    return Ok(Ty::named(n.clone()));
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
            ExprKind::Regex { pattern, flags } => {
                if let Err(msg) = crate::codegen::regex_rt::JsRegex::parse(pattern, flags) {
                    return self.err(format!("invalid regular expression: {msg}"), e.span);
                }
                Ok(Ty::named("RegExp"))
            }
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
                Ok(self.self_type(&class))
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
                // Without an expected type, elements of different types make a union.
                let free = elem_expected.is_none();
                let item = elem_expected.unwrap_or_else(|| self.types.fresh());
                let hint = self.hints.get(&e.id).cloned().unwrap_or_default();
                let mut others: Vec<(Ty, &'m Expr)> = Vec::new();
                for el in elems {
                    match el {
                        ArrayElem::Expr(x) => {
                            if !hint.is_empty() {
                                self.hints.insert(x.id, singular(&hint));
                            }
                            let xt = self.expr(x, Some(&item))?;
                            self.mark_stored(x);
                            match self.expect(&xt, &item, x.span) {
                                Ok(()) => {}
                                Err(_)
                                    if free && !matches!(self.types.resolve(&item), Ty::Var(_)) =>
                                {
                                    others.push((xt, x));
                                }
                                Err(e) => return Err(e),
                            }
                        }
                        ArrayElem::Spread(x) => {
                            let arr = Ty::Array(Box::new(item.clone()));
                            let xt = self.expr(x, Some(&arr))?;
                            // Arrays, sets, maps (pairs) and strings (characters) spread.
                            if !others.is_empty() {
                                return self.err(
                                    "a spread cannot be mixed with elements of other types",
                                    x.span,
                                );
                            }
                            match self.types.resolve(&xt) {
                                Ty::Set(t) | Ty::Gen(t) => self.expect(&t, &item, x.span)?,
                                Ty::Map(k, v, _) => {
                                    self.expect(&Ty::Tuple(vec![*k, *v]), &item, x.span)?
                                }
                                Ty::Str => self.expect(&Ty::Str, &item, x.span)?,
                                ref other if self.struct_iteration(other).is_some() => {
                                    let t = self.struct_iteration(other).expect("iterable");
                                    self.expect(&t, &item, x.span)?
                                }
                                _ => self.expect(&xt, &arr, x.span)?,
                            }
                        }
                        ArrayElem::Hole => {
                            return self.err("holes in arrays are not supported", e.span);
                        }
                    }
                }
                if others.is_empty() {
                    return Ok(Ty::Array(Box::new(item)));
                }
                // `[1, "two"]`: an array of the union of the element types.
                let mut members = vec![item.clone()];
                members.extend(others.iter().map(|(t, _)| t.clone()));
                let union = self.union_of(members, e.span)?;
                for el in elems {
                    if let ArrayElem::Expr(x) = el {
                        let xt = self.expr_ty.get(&x.id).cloned().unwrap_or(Ty::Void);
                        self.expect(&xt, &union, x.span)?;
                    }
                }
                Ok(Ty::Array(Box::new(union)))
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
                    let n = self.types.num(Flavor::Unknown);
                    self.expect(&t, &n, arg.span)?;
                    Ok(self.types.int())
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
                // `c ? x : undefined`: a value or nothing.
                match (self.types.resolve(&ct), self.types.resolve(&at)) {
                    (Ty::Null, Ty::Null) => return Ok(Ty::Null),
                    (Ty::Null, Ty::Option(_)) | (Ty::Option(_), Ty::Null) => {}
                    (Ty::Null, Ty::Var(_)) | (Ty::Var(_), Ty::Null) => {}
                    (Ty::Null, other) => return Ok(Ty::Option(Box::new(other))),
                    (other, Ty::Null) => return Ok(Ty::Option(Box::new(other))),
                    _ => {}
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
                let mut given = Vec::new();
                for a in type_args {
                    given.push(self.ann(a, "")?);
                }
                let before = self.type_args.take();
                self.type_args = (!given.is_empty()).then_some((e.span, given));
                let r = self.call(e, callee, args, *optional);
                self.type_args = before;
                r
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
                    // `a?.[i]`: nothing when `a` is null.
                    let ot = self.expr(obj, None)?;
                    let Ty::Option(inner) = self.types.resolve(&ot) else {
                        return self.err("`?.[` on a value that is never null", obj.span);
                    };
                    let t = self.index_of(&inner, index, e.span)?;
                    return Ok(match self.types.resolve(&t) {
                        Ty::Option(t) => Ty::Option(t),
                        t => Ty::Option(Box::new(t)),
                    });
                }
                // `Level[Level.High]`, `Level[2]`: the name of a numeric enum's member.
                if let Some((name, id)) = self.type_ref(obj)
                    && self.vars[&id].role == Role::Enum
                {
                    let numeric = self
                        .enum_by_name(&name)
                        .is_some_and(|en| en.kind == EnumKind::Numeric);
                    if !numeric {
                        return self.err(
                            format!("only numeric enums map values back to names: `{name}[…]`"),
                            e.span,
                        );
                    }
                    self.refs.insert(obj.id, id);
                    self.set_ty(obj.id, Ty::Void);
                    let it = self.expr(index, None)?;
                    match self.types.resolve(&it) {
                        Ty::Named(n, _) if n == name => {}
                        Ty::Num(_) => {}
                        other => {
                            let d = self.describe(&other);
                            return self.err(
                                format!("`{name}[…]` takes a number or a {name}, not {d}"),
                                index.span,
                            );
                        }
                    }
                    self.calls.insert(e.id, Callee::Builtin("enum.name"));
                    self.enum_names.insert(name);
                    return Ok(Ty::Str);
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
                    Role::Func | Role::Class | Role::Enum => {}
                    role => {
                        if role == Role::TopLevel {
                            if outer_fn != Some(main_id) {
                                v.used_in_fn = true;
                            }
                            if current != main_id {
                                v.captured = true;
                            }
                        } else {
                            v.captured = true;
                        }
                        // Every function between the use and the declaration captures it.
                        let owner = v.func.unwrap_or(main_id);
                        for ctx in self.fns.iter_mut().rev() {
                            if ctx.id == owner {
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
                self.beyond_safe(left, right, &n);
                Ok(n)
            }
            // `a ** b` is a double, as in JavaScript: `2 ** 64` does not overflow.
            BinOp::Pow => {
                let a = self.types.num(Flavor::Unknown);
                let lt = self.expr(left, Some(&a))?;
                self.expect(&lt, &a, left.span)?;
                let b = self.types.num(Flavor::Unknown);
                let rt = self.expr(right, Some(&b))?;
                self.expect(&rt, &b, right.span)?;
                Ok(self.types.float())
            }
            BinOp::Sub | BinOp::Mul | BinOp::Mod => {
                let n = self.types.num(Flavor::Unknown);
                let lt = if matches!(op, BinOp::Sub) {
                    self.expr(left, None)?
                } else {
                    self.expr(left, Some(&n))?
                };
                // `later - earlier` on dates: the milliseconds between them.
                if matches!(op, BinOp::Sub)
                    && matches!(self.types.resolve(&lt), Ty::Named(ref d, ..) if d == "Date")
                {
                    let date = Ty::named("Date");
                    let rt = self.expr(right, Some(&date))?;
                    self.expect(&rt, &date, right.span)?;
                    return Ok(self.types.int());
                }
                self.expect(&lt, &n, left.span)?;
                let rt = self.expr(right, Some(&n))?;
                self.expect(&rt, &n, right.span)?;
                if op != BinOp::Mod {
                    self.beyond_safe(left, right, &n);
                }
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
                    Ty::Named(n, ..) if self.is_type_param(&n) || n == "Date" => Ok(Ty::Bool),
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
                // Any numbers, taken as 32-bit integers; the result is one.
                for side in [left, right] {
                    let n = self.types.num(Flavor::Unknown);
                    let t = self.expr(side, Some(&n))?;
                    self.expect(&t, &n, side.span)?;
                }
                Ok(self.types.int())
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
                // The class may be imported under another name, or through a namespace.
                let class = match self.type_ref(right) {
                    Some((canonical, id)) if self.vars[&id].role == Role::Class => {
                        if let ExprKind::Member { obj: ns, .. } = &right.kind {
                            self.set_ty(ns.id, Ty::Void);
                        }
                        self.refs.insert(right.id, id);
                        canonical
                    }
                    _ => match &right.kind {
                        ExprKind::Ident(c) => c.clone(),
                        _ => return self.err("`instanceof` needs a class name", right.span),
                    },
                };
                let class = &class;
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

    /// Arithmetic on `Number.MAX_SAFE_INTEGER` (or a literal as large): past 2^53 an
    /// integer is not exact in JavaScript, so it is a double.
    fn beyond_safe(&mut self, left: &Expr, right: &Expr, n: &Ty) {
        let big = |e: &Expr| match &e.kind {
            ExprKind::Num { value, .. } => value.abs() >= 9007199254740991.0,
            ExprKind::Member { obj, prop, .. } => {
                matches!(&obj.kind, ExprKind::Ident(o) if o == "Number")
                    && matches!(prop.as_str(), "MAX_SAFE_INTEGER" | "MIN_SAFE_INTEGER")
            }
            _ => false,
        };
        // Constants whose result is past 2^53: `123456789 * 1000000000000`.
        let folded = match (const_value(left), const_value(right)) {
            (Some(a), Some(b)) => [a * b, a + b, a - b]
                .iter()
                .any(|v| v.abs() >= 9007199254740992.0),
            _ => false,
        };
        if (big(left) || big(right) || folded)
            && let Ty::Num(v) = self.types.resolve(n)
        {
            self.types.set_flavor(v, Flavor::Float);
        }
    }

    fn assign(
        &mut self,
        op: AssignOp,
        target: &'m AssignTarget,
        value: &'m Expr,
        span: Span,
    ) -> IResult<Ty> {
        self.assigns.push((target, op, value));
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
                        self.expect_flow(&vt, &tt, value.span)?;
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
                                let n = self.types.num(Flavor::Unknown);
                                self.expect(&vt, &n, value.span)?;
                            }
                            (BinOp::Pow, _) => {
                                let f = self.types.float();
                                self.expect(&tt, &f, t.span)?;
                                let n = self.types.num(Flavor::Unknown);
                                self.expect(&vt, &n, value.span)?;
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
        // Spreads of one same shape: the literal is of that shape too (when its own keys
        // fit); spreads of different shapes make a new one.
        let mut spread_struct: Option<String> = None;
        if expected.is_none() {
            for p in props {
                let PropValue::Spread(s) = &p.value else {
                    continue;
                };
                let st = self.expr(s, None)?;
                let name = match self.types.resolve(&st) {
                    Ty::Named(n, ..) if self.struct_by_name(&n).is_some() => n,
                    _ => {
                        spread_struct = None;
                        break;
                    }
                };
                match &spread_struct {
                    Some(prev) if *prev != name => {
                        spread_struct = None;
                        break;
                    }
                    _ => spread_struct = Some(name),
                }
            }
        }
        let struct_name = match (expected, spread_struct) {
            (Some(Ty::Named(n, ..)), _) if self.struct_by_name(n).is_some() => n.clone(),
            (Some(Ty::Named(n, ..)), _) if self.union_by_name(n).is_some() => {
                let u = self.union_by_name(n).expect("union").clone();
                match self.union_literal_member(&u, props) {
                    Some(member) => member,
                    None => {
                        return self.err(format!("this object matches no member of {n}"), e.span);
                    }
                }
            }
            (_, Some(n)) if self.props_fit(&n, props) => n,
            _ => return self.anonymous_object(e, props),
        };
        self.literal_struct.insert(e.id, struct_name.clone());
        // A generic interface: `Box<number>` from the annotation, or inferred.
        let (inst, subst) = match expected {
            Some(t @ Ty::Named(n, args)) if *n == struct_name && !args.is_empty() => {
                (t.clone(), self.instance_subst(t))
            }
            _ => self.fresh_instance(&struct_name),
        };
        let fields: Vec<FieldInfo> = self
            .struct_by_name(&struct_name)
            .expect("struct")
            .fields
            .iter()
            .map(|f| FieldInfo {
                ty: substitute(&f.ty, &subst, &self.types),
                ..f.clone()
            })
            .collect();
        let mut seen = HashSet::new();
        // A spread of this very shape gives every field; one of another shape gives the
        // fields the shapes share.
        let mut same_spread = false;
        let mut covered: HashSet<String> = HashSet::new();
        for p in props {
            if let PropValue::Spread(s) = &p.value {
                let st = self.expr(s, None)?;
                match self.types.resolve(&st) {
                    Ty::Named(n, ..) if n == struct_name => same_spread = true,
                    Ty::Named(n, ..) if self.struct_by_name(&n).is_some() => {
                        let src = self.struct_by_name(&n).expect("struct").fields.clone();
                        for sf in &src {
                            if let Some(tf) = fields.iter().find(|f| f.name == sf.name) {
                                let a = field_ty(sf);
                                let b = field_ty(tf);
                                self.expect(&a, &b, s.span)?;
                                covered.insert(sf.name.clone());
                            }
                        }
                    }
                    other => {
                        let d = self.describe(&other);
                        return self.err(format!("cannot spread {d} into an object"), s.span);
                    }
                }
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
        if !same_spread {
            for f in &fields {
                if !f.optional && !seen.contains(&f.name) && !covered.contains(&f.name) {
                    return self.err(
                        format!("property `{}` of {struct_name} is missing", f.name),
                        e.span,
                    );
                }
            }
        }
        Ok(inst)
    }

    /// Whether every key of a literal is a field of `struct_name` (spreads aside).
    fn props_fit(&self, struct_name: &str, props: &[Prop]) -> bool {
        let Some(s) = self.struct_by_name(struct_name) else {
            return false;
        };
        props.iter().all(|p| {
            matches!(p.value, PropValue::Spread(_))
                || p.key.name().is_some_and(|k| s.field(&k).is_some())
        })
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
            // `{ ...base, z }`: the fields of `base`, then the literal's own.
            if let PropValue::Spread(s) = &p.value {
                let st = self.expr(s, None)?;
                let src = match self.types.resolve(&st) {
                    Ty::Named(n, ..) => self.struct_by_name(&n).map(|s| s.fields.clone()),
                    _ => None,
                };
                let Some(src) = src else {
                    let d = self.describe(&st);
                    return self.err(format!("cannot spread {d} into an object"), s.span);
                };
                for sf in src {
                    let field = FieldInfo {
                        span: p.span,
                        access: None,
                        readonly: false,
                        has_init: false,
                        ..sf
                    };
                    match fields.iter().position(|f: &FieldInfo| f.name == field.name) {
                        Some(i) => fields[i] = field,
                        None => fields.push(field),
                    }
                }
                continue;
            }
            let fty = match &p.value {
                PropValue::Expr(v) => {
                    self.hints.insert(v.id, format!("{hint} {key}"));
                    let t = self.expr(v, None)?;
                    self.mark_stored(v);
                    t
                }
                PropValue::Shorthand(id) => self.shorthand(&key, *id, p.span)?,
                PropValue::Spread(_) => unreachable!(),
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
            let field = FieldInfo {
                name: key,
                ty: fty,
                optional: false,
                access: None,
                readonly: false,
                has_init: false,
                literal: None,
                span: p.span,
            };
            // A key given again (after a spread) keeps its place, with the new type.
            match fields.iter().position(|f: &FieldInfo| f.name == field.name) {
                Some(i) => fields[i] = field,
                None => fields.push(field),
            }
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
            return Ok(Ty::named(existing));
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
            implements: Vec::new(),
            type_params: Vec::new(),
            type_bounds: Vec::new(),
            parent_args: Vec::new(),
            implements_args: Vec::new(),
        });
        self.literal_struct.insert(e.id, name.clone());
        Ok(Ty::named(name))
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
        // `ns.x` for `import * as ns`: the exported name itself.
        if let Some(file) = self.namespace_of(obj) {
            let actual = self.namespace_member(file, prop, e.span)?;
            self.set_ty(obj.id, Ty::Void);
            return self.ident(&actual, e);
        }
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
        // Static members and enum variants: `C.x`, `E.A`, or through a namespace import.
        if let Some((canonical, id)) = self.type_ref(obj) {
            if let ExprKind::Member { obj: ns, .. } = &obj.kind {
                self.set_ty(ns.id, Ty::Void);
            }
            let name = &canonical;
            match self.vars[&id].role {
                Role::Enum => {
                    self.refs.insert(obj.id, id);
                    let en = self.enum_by_name(name).expect("enum");
                    if !en.variants.iter().any(|v| v.source == prop) {
                        return self.err(format!("{name} has no member `{prop}`"), e.span);
                    }
                    self.set_ty(obj.id, Ty::named(name.clone()));
                    return Ok(Ty::named(name.clone()));
                }
                Role::Class => {
                    self.refs.insert(obj.id, id);
                    self.set_ty(obj.id, Ty::named(name.clone()));
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
            Ty::Named(n, ..) => {
                if n == "Error" {
                    return match prop {
                        "message" | "name" | "stack" => Ok(Ty::Str),
                        _ => self.err(format!("Error has no property `{prop}`"), span),
                    };
                }
                if n == "RegExp" {
                    return match prop {
                        "source" | "flags" => Ok(Ty::Str),
                        "global" | "ignoreCase" | "multiline" | "dotAll" | "sticky" | "unicode" => {
                            Ok(Ty::Bool)
                        }
                        "lastIndex" => Ok(self.types.int()),
                        _ => self.err(format!("RegExp has no property `{prop}`"), span),
                    };
                }
                if n == "Symbol" {
                    return match prop {
                        // `undefined` for `Symbol()`.
                        "description" => Ok(Ty::Option(Box::new(Ty::Str))),
                        _ => self.err(format!("a symbol has no property `{prop}`"), span),
                    };
                }
                if n == "RegExpMatch" {
                    return match prop {
                        "index" | "length" => Ok(self.types.int()),
                        "input" => Ok(Ty::Str),
                        _ => self.err(
                            format!("a match has no property `{prop}`: its groups are `m[i]`"),
                            span,
                        ),
                    };
                }
                if self.is_type_param(n) {
                    // `T extends HasId`: the fields of the constraint.
                    if let Some(c) = self.constraint_of(n) {
                        return self.member_of(&Ty::named(c), prop, span, obj);
                    }
                    return self.err(
                        format!(
                            "`{prop}` is not known on the type parameter {n}: constrain it, `<{n} extends {{ {prop}: … }}>`"
                        ),
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
                let subst = self.instance_subst(ot);
                if let Some(f) = s.field(prop) {
                    let t = if f.optional {
                        Ty::Option(Box::new(f.ty.clone()))
                    } else {
                        f.ty.clone()
                    };
                    return Ok(substitute(&t, &subst, &self.types));
                }
                if let Some((_, m)) = self.find_method(n, prop, MethodKind::Get) {
                    let t = self.funcs[&m.fn_id].sig.ret.clone();
                    return Ok(substitute(&t, &subst, &self.types));
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
        self.index_of(&ot, index, span)
    }

    /// The type of `obj[index]` for a value of type `ot`.
    fn index_of(&mut self, ot: &Ty, index: &'m Expr, span: Span) -> IResult<Ty> {
        let ot = self.types.resolve(ot);
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
            Ty::Named(n, ..) if n == "RegExpMatch" => {
                let it = self.expr(index, None)?;
                let n = self.types.num(Flavor::Unknown);
                self.expect(&it, &n, index.span)?;
                Ok(Ty::Str)
            }
            Ty::Named(n, ..) if self.struct_by_name(n).is_some() => {
                let ExprKind::Str(key) = &index.kind else {
                    return self.err(format!("{n} is indexed with a property name"), index.span);
                };
                self.set_ty(index.id, Ty::Str);
                let key = key.clone();
                self.member_of(&ot, &key, span, index)
            }
            Ty::Option(_) => self.err("this value may be null here: test it first", span),
            Ty::Var(_) => {
                let t = self.types.fresh();
                self.unify(&ot, &Ty::Array(Box::new(t.clone())), span)?;
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
        // `new C(…)`, `new ns.C(…)`; the class may be imported under another name.
        if let Some((canonical, id)) = self.type_ref(callee) {
            if self.vars[&id].role != Role::Class {
                return self.err(format!("`{canonical}` is not a class"), callee.span);
            }
            if let ExprKind::Member { obj: ns, .. } = &callee.kind {
                self.set_ty(ns.id, Ty::Void);
            }
            let name = &canonical;
            self.refs.insert(callee.id, id);
            self.set_ty(callee.id, Ty::named(name.clone()));
            let s = self.struct_by_name(name).expect("class");
            if s.is_abstract {
                return self.err(
                    format!("{name} is abstract: `new` one of its subclasses"),
                    e.span,
                );
            }
            // `new Stack<number>()`, or `new Pair("x", 3)` with the arguments inferred.
            let (inst, subst) = self.fresh_instance(name);
            if let ExprKind::New { type_args, .. } = &e.kind
                && !type_args.is_empty()
            {
                let params = self
                    .struct_by_name(name)
                    .expect("class")
                    .type_params
                    .clone();
                if params.len() != type_args.len() {
                    return self.err(
                        format!(
                            "{name} takes {} type argument{}",
                            params.len(),
                            if params.len() == 1 { "" } else { "s" }
                        ),
                        e.span,
                    );
                }
                for (p, a) in params.iter().zip(type_args) {
                    let t = self.ann(a, name)?;
                    self.unify(&subst[p], &t, a.span())?;
                }
            }
            match self.nearest_ctor(name) {
                Some((_, ctor)) => {
                    let info = self.funcs[&ctor].clone();
                    self.args_in(args, &info, ctor, e.span, &subst)?;
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
            return Ok(inst);
        }
        let ExprKind::Ident(name) = &callee.kind else {
            return self.err("`new` needs a class name", callee.span);
        };
        if self.lookup(name).is_some() {
            return self.err(format!("`{name}` is not a class"), callee.span);
        }
        // `new Map<string, number>()`: the type arguments, when given.
        let mut given = Vec::new();
        if let ExprKind::New { type_args, .. } = &e.kind {
            for a in type_args {
                given.push(self.ann(a, name)?);
            }
        }
        let targ =
            |this: &mut Self, i: usize| given.get(i).cloned().unwrap_or_else(|| this.types.fresh());
        match name.as_str() {
            "Error" | "TypeError" | "RangeError" => {
                if let Some(a) = args.first() {
                    let t = self.expr(&a.expr, Some(&Ty::Str))?;
                    self.expect(&t, &Ty::Str, a.expr.span)?;
                }
                self.calls.insert(e.id, Callee::Builtin("new Error"));
                Ok(Ty::named("Error"))
            }
            "Map" => {
                let (k, v) = (targ(self, 0), targ(self, 1));
                if let Some(a) = args.first() {
                    let entries = Ty::Array(Box::new(Ty::Tuple(vec![k.clone(), v.clone()])));
                    let t = self.expr(&a.expr, Some(&entries))?;
                    self.expect(&t, &entries, a.expr.span)?;
                }
                self.calls.insert(e.id, Callee::Builtin("new Map"));
                Ok(Ty::Map(Box::new(k), Box::new(v), false))
            }
            "Set" => {
                let t = targ(self, 0);
                if let Some(a) = args.first() {
                    let arr = Ty::Array(Box::new(t.clone()));
                    let at = self.expr(&a.expr, Some(&arr))?;
                    self.expect(&at, &arr, a.expr.span)?;
                }
                self.calls.insert(e.id, Callee::Builtin("new Set"));
                Ok(Ty::Set(Box::new(t)))
            }
            "Array" => {
                let t = targ(self, 0);
                if let Some(a) = args.first() {
                    let n = self.types.int();
                    let at = self.expr(&a.expr, Some(&n))?;
                    self.expect(&at, &n, a.expr.span)?;
                }
                self.calls.insert(e.id, Callee::Builtin("new Array"));
                Ok(Ty::Array(Box::new(t)))
            }
            "Date" => {
                // `new Date()`, `new Date(ms)`, `new Date(text)`, `new Date(y, m, d, …)`.
                if args.len() == 1 {
                    let t = self.expr(&args[0].expr, None)?;
                    match self.types.resolve(&t) {
                        Ty::Str => {}
                        Ty::Num(_) => {
                            let i = self.types.int();
                            self.expect(&t, &i, args[0].expr.span)?;
                        }
                        Ty::Named(ref d, ..) if d == "Date" => {}
                        other => {
                            let d = self.describe(&other);
                            return self.err(
                                format!("`new Date` takes milliseconds or text, not {d}"),
                                args[0].expr.span,
                            );
                        }
                    }
                } else {
                    if args.len() > 7 {
                        return self.err("`new Date` takes at most 7 arguments", e.span);
                    }
                    for a in args {
                        let i = self.types.int();
                        let t = self.expr(&a.expr, Some(&i))?;
                        self.expect(&t, &i, a.expr.span)?;
                    }
                }
                self.calls.insert(e.id, Callee::Builtin("new Date"));
                Ok(Ty::named("Date"))
            }
            "Promise" => {
                // `new Promise((resolve, reject) => …)`, awaited where it is made.
                let ExprKind::New { type_args, .. } = &e.kind else {
                    unreachable!()
                };
                let value = match type_args.first() {
                    Some(t) => self.ann(t, "")?,
                    None => self.types.fresh(),
                };
                let resolve = Ty::Func(Rc::new(FuncTy {
                    params: vec![value.clone()],
                    ret: Ty::Void,
                }));
                let reject = Ty::Func(Rc::new(FuncTy {
                    params: vec![Ty::named("Error")],
                    ret: Ty::Void,
                }));
                let executor = Ty::Func(Rc::new(FuncTy {
                    params: vec![resolve, reject],
                    ret: Ty::Void,
                }));
                let Some(a) = args.first() else {
                    return self.err("`new Promise` needs an executor function", e.span);
                };
                let t = self.expr(&a.expr, Some(&executor))?;
                self.expect(&t, &executor, a.expr.span)?;
                let id = self.ctx().id;
                let is_async = self.funcs.get(&id).is_some_and(|f| f.is_async);
                if !is_async && id != self.main_id {
                    return self.err(
                        "`new Promise` must be made in an async function (it is awaited there)",
                        e.span,
                    );
                }
                if id == self.main_id {
                    self.funcs.get_mut(&id).expect("main").is_async = true;
                }
                self.calls.insert(e.id, Callee::Builtin("new Promise"));
                Ok(value)
            }
            "RegExp" => {
                self.arg_typed(args, 0, Some(Ty::Str), "RegExp", e.span)?;
                if args.len() > 1 {
                    self.arg_typed(args, 1, Some(Ty::Str), "RegExp", e.span)?;
                }
                if let (Some(ExprKind::Str(p)), f) = (
                    args.first().map(|a| &a.expr.kind),
                    args.get(1).map(|a| &a.expr.kind),
                ) {
                    let flags = match f {
                        None => String::new(),
                        Some(ExprKind::Str(f)) => f.clone(),
                        Some(_) => String::new(),
                    };
                    if let Err(msg) = crate::codegen::regex_rt::JsRegex::parse(p, &flags) {
                        return self.err(format!("invalid regular expression: {msg}"), e.span);
                    }
                }
                self.calls.insert(e.id, Callee::Builtin("new RegExp"));
                Ok(Ty::named("RegExp"))
            }
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
        self.args_in(args, info, fn_id, span, &HashMap::new())
            .map(|_| ())
    }

    /// Checks the arguments of a call; `outer` gives the type arguments of the receiver
    /// (a method of `Stack<number>` sees `T` as `number`). Returns every type parameter's
    /// type for this call.
    pub(super) fn args_in(
        &mut self,
        args: &'m [Arg],
        info: &FnInfo,
        fn_id: NodeId,
        span: Span,
        outer: &HashMap<String, Ty>,
    ) -> IResult<HashMap<String, Ty>> {
        // `...rest`: the last parameter takes every extra argument.
        let rest_at = if info.rest {
            Some(info.sig.params.len() - 1)
        } else {
            None
        };
        let fixed = rest_at.unwrap_or(info.sig.params.len());
        let required = info
            .optional
            .iter()
            .zip(&info.defaults)
            .take(fixed)
            .filter(|(o, d)| !**o && !**d)
            .count();
        if args.len() < required || (rest_at.is_none() && args.len() > info.sig.params.len()) {
            let name = info.name.clone().unwrap_or_else(|| "the function".into());
            let n = info.sig.params.len();
            return self.err(
                format!(
                    "`{name}` takes {} argument{}, {} given",
                    if rest_at.is_some() {
                        format!("at least {required}")
                    } else if required == n {
                        required.to_string()
                    } else {
                        format!("{required} to {n}")
                    },
                    if n == 1 && rest_at.is_none() { "" } else { "s" },
                    args.len()
                ),
                span,
            );
        }
        // Type parameters are instantiated afresh for each call, or given (`f<number>(…)`).
        let mut subst: HashMap<String, Ty> = outer.clone();
        // Only for the call they were written on (not a call among its arguments).
        let given = match self.type_args.take() {
            Some((at, tys)) if at == span => tys,
            other => {
                self.type_args = other;
                Vec::new()
            }
        };
        if !given.is_empty() && given.len() != info.type_params.len() {
            let name = info.name.clone().unwrap_or_else(|| "the function".into());
            return self.err(
                format!(
                    "`{name}` takes {} type argument{}",
                    info.type_params.len(),
                    if info.type_params.len() == 1 { "" } else { "s" }
                ),
                span,
            );
        }
        for (i, tp) in info.type_params.iter().enumerate() {
            let t = given.get(i).cloned().unwrap_or_else(|| self.types.fresh());
            subst.insert(tp.clone(), t);
        }
        if !info.type_bounds.is_empty() {
            let bounds = info.type_bounds.clone();
            self.record_bound_uses(&bounds, &subst, span);
        }
        for (i, a) in args.iter().enumerate() {
            if let Some(r) = rest_at
                && i >= r
            {
                let mut pt = info.sig.params[r].clone();
                if !subst.is_empty() {
                    pt = substitute(&pt, &subst, &self.types);
                }
                let Ty::Array(item) = self.types.resolve(&pt) else {
                    return self.err("the rest parameter is not an array", a.expr.span);
                };
                if a.spread {
                    if i != r || args.len() != r + 1 {
                        return self.err(
                            "a spread is the only argument a rest parameter can take: `f(a, ...xs)` mixes them",
                            a.expr.span,
                        );
                    }
                    let at = self.expr(&a.expr, Some(&pt))?;
                    self.expect(&at, &pt, a.expr.span)?;
                } else {
                    let at = self.expr(&a.expr, Some(&item))?;
                    self.expect_flow(&at, &item, a.expr.span)?;
                    self.mark_stored(&a.expr);
                }
                continue;
            }
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
            self.expect_flow(&at, &pt, a.expr.span)?;
            if let ExprKind::Ident(name) = &a.expr.kind
                && let Some(b) = self.lookup(name)
            {
                self.arg_bindings.push((fn_id, i, b));
            }
        }
        Ok(subst)
    }

    fn call(
        &mut self,
        e: &'m Expr,
        callee: &'m Expr,
        args: &'m [Arg],
        optional: bool,
    ) -> IResult<Ty> {
        if optional {
            // `f?.()`: nothing when `f` is null.
            let ct = self.expr(callee, None)?;
            let Ty::Option(inner) = self.types.resolve(&ct) else {
                return self.err("`?.()` on a value that is never null", callee.span);
            };
            let Ty::Func(f) = self.types.resolve(&inner) else {
                let d = self.describe(&inner);
                return self.err(format!("cannot call {d}"), callee.span);
            };
            self.dynamic_args(args, &f, e.span)?;
            self.calls.insert(e.id, Callee::OptionalCall);
            return Ok(optional_result(self.types.resolve(&f.ret)));
        }
        if self.ctx().try_depth > 0 {
            self.calls_in_try.insert(e.id);
        }
        match &callee.kind {
            ExprKind::Super => self.super_call(e, args),
            ExprKind::Member { obj, prop, .. } if matches!(obj.kind, ExprKind::Super) => {
                self.super_method_call(e, prop, args)
            }
            ExprKind::Member { obj, prop, .. } if self.namespace_of(obj).is_some() => {
                let file = self.namespace_of(obj).expect("namespace");
                let actual = self.namespace_member(file, prop, callee.span)?;
                self.set_ty(obj.id, Ty::Void);
                self.named_call(e, callee, &actual, args)
            }
            ExprKind::Ident(name) => self.named_call(e, callee, name, args),
            ExprKind::Member {
                obj,
                prop,
                optional,
            } => {
                if *optional {
                    return self.optional_method_call(e, obj, prop, args);
                }
                self.method_call(e, callee.id, obj, prop, args)
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

    /// `obj?.m(…)`: the method of the object when there is one, nothing otherwise.
    fn optional_method_call(
        &mut self,
        e: &'m Expr,
        obj: &'m Expr,
        prop: &str,
        args: &'m [Arg],
    ) -> IResult<Ty> {
        let ot = self.expr(obj, None)?;
        let Ty::Option(inner) = self.types.resolve(&ot) else {
            return self.err("`?.` on a value that is never null", obj.span);
        };
        let Ty::Named(n, ..) = self.types.resolve(&inner) else {
            return self.err(
                "`?.` before a method call works on objects: test the value first",
                e.span,
            );
        };
        let Some((_, m)) = self.find_method(&n, prop, MethodKind::Method) else {
            return self.err(format!("{n} has no method `{prop}`"), e.span);
        };
        if m.is_static {
            return self.err(format!("`{prop}` is static: call `{n}.{prop}()`"), e.span);
        }
        let fn_id = m.fn_id;
        let info = self.funcs[&fn_id].clone();
        self.args(args, &info, fn_id, e.span)?;
        if let Some(root) = self.root_binding(obj) {
            self.method_receivers.push((fn_id, root));
            if info.mutates_self {
                self.mark_receiver_mutated(obj);
            }
        }
        self.calls.insert(
            e.id,
            Callee::OptionalMethod {
                class: n,
                name: prop.to_string(),
                fn_id,
            },
        );
        let caller = self.ctx().id;
        self.funcs
            .get_mut(&caller)
            .expect("fn")
            .calls
            .push((fn_id, e.id));
        Ok(optional_result(self.types.resolve(&info.sig.ret)))
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
                let given = self.ancestor_subst(&class, HashMap::new());
                self.args_in(args, &info, ctor, e.span, &given)?;
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
        let given = self.ancestor_subst(&class, HashMap::new());
        let used = self.args_in(args, &info, m.fn_id, e.span, &given)?;
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
        Ok(substitute(&info.sig.ret, &used, &self.types))
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
            self.expect_flow(&at, &pt, a.expr.span)?;
        }
        Ok(())
    }

    /// `parseInt(…)`, `String(…)`…
    fn global_call(&mut self, e: &'m Expr, name: &str, args: &'m [Arg], span: Span) -> IResult<Ty> {
        // `Array(n)` is `new Array(n)`.
        if name == "Array" {
            self.no_more_than(args, 1, name, span)?;
            let t = self.types.fresh();
            if let Some(a) = args.first() {
                let n = self.types.int();
                let at = self.expr(&a.expr, Some(&n))?;
                self.expect(&at, &n, a.expr.span)?;
            }
            self.calls.insert(e.id, Callee::Builtin("new Array"));
            return Ok(Ty::Array(Box::new(t)));
        }
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
            // `Symbol("id")`: a value equal only to itself.
            "Symbol" => {
                if !args.is_empty() {
                    self.arg_typed(args, 0, Some(Ty::Str), name, span)?;
                }
                self.no_more_than(args, 1, name, span)?;
                Ty::named("Symbol")
            }
            "structuredClone" => self.arg_typed(args, 0, None, name, span)?,
            "require" => {
                return self.err(
                    "`require` is not supported: Solidify translates one file",
                    span,
                );
            }
            "setTimeout" | "setInterval" => {
                // The callback runs later, on its own: it keeps what it uses.
                let callback = Ty::Func(Rc::new(FuncTy {
                    params: Vec::new(),
                    ret: Ty::Void,
                }));
                let Some(cb) = args.first() else {
                    return self.err(format!("`{name}` needs a callback"), span);
                };
                let ct = self.expr(&cb.expr, Some(&callback))?;
                let Ty::Func(f) = self.types.resolve(&ct) else {
                    let d = self.describe(&ct);
                    return self.err(
                        format!("`{name}` needs a function, found {d}"),
                        cb.expr.span,
                    );
                };
                if f.params
                    .iter()
                    .any(|p| !matches!(self.types.resolve(p), Ty::Void | Ty::Var(_)))
                {
                    return self.err(
                        format!("the callback of `{name}` takes no argument"),
                        cb.expr.span,
                    );
                }
                self.mark_stored(&cb.expr);
                if args.len() > 1 {
                    let n = self.types.num(Flavor::Unknown);
                    self.arg_typed(args, 1, Some(n), name, span)?;
                }
                self.no_more_than(args, 2, name, span)?;
                self.uses_timers = true;
                self.types.int()
            }
            "clearTimeout" | "clearInterval" => {
                let n = self.types.num(Flavor::Unknown);
                self.arg_typed(args, 0, Some(n), name, span)?;
                self.uses_timers = true;
                Ty::Void
            }
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

    /// A call to a name: a function, a closure variable, a class used as a function…
    fn named_call(
        &mut self,
        e: &'m Expr,
        callee: &'m Expr,
        name: &str,
        args: &'m [Arg],
    ) -> IResult<Ty> {
        if let Some(id) = self.lookup(name) {
            self.refs.insert(callee.id, id);
            self.record_use(id, callee.id);
            let v = self.vars[&id].clone();
            match v.role {
                Role::Func => {
                    let fn_id = self.fn_bindings[&id];
                    let info = self.funcs[&fn_id].clone();
                    self.set_ty(callee.id, Ty::Func(Rc::new(info.sig.clone())));
                    let used = self.args_in(args, &info, fn_id, e.span, &HashMap::new())?;
                    self.calls.insert(e.id, Callee::Fn(id));
                    let caller = self.ctx().id;
                    self.funcs
                        .get_mut(&caller)
                        .expect("fn")
                        .calls
                        .push((fn_id, e.id));
                    return Ok(substitute(&info.sig.ret, &used, &self.types));
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
                        return self.err(format!("`{name}` is {d}, not a function"), callee.span);
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

    fn method_call(
        &mut self,
        e: &'m Expr,
        callee_id: NodeId,
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
        // Static methods: `C.m()`, or `ns.C.m()` through a namespace import.
        if let Some((canonical, id)) = self.type_ref(obj)
            && self.vars[&id].role == Role::Class
        {
            if let ExprKind::Member { obj: ns, .. } = &obj.kind {
                self.set_ty(ns.id, Ty::Void);
            }
            let name = &canonical;
            self.refs.insert(obj.id, id);
            self.set_ty(obj.id, Ty::named(name.clone()));
            let Some((owner, m)) = self
                .find_method(name, prop, MethodKind::Method)
                .filter(|(_, m)| m.is_static)
            else {
                return self.err(format!("{name} has no static method `{prop}`"), e.span);
            };
            let (fn_id, class) = (m.fn_id, owner);
            let info = self.funcs[&fn_id].clone();
            let used = self.args_in(args, &info, fn_id, e.span, &HashMap::new())?;
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
            return Ok(substitute(&info.sig.ret, &used, &self.types));
        }
        let ot = self.expr(obj, None)?;
        let ot = self.types.resolve(&ot);
        // `T extends Shape`: the methods of the constraint.
        let ot = match &ot {
            Ty::Named(n, args) if args.is_empty() && self.is_type_param(n) => {
                match self.constraint_of(n) {
                    Some(c) => Ty::named(c),
                    None => {
                        return self.err(
                            format!(
                                "`{prop}` is not known on the type parameter {n}: constrain it, `<{n} extends {{ {prop}(): … }}>`"
                            ),
                            e.span,
                        );
                    }
                }
            }
            _ => ot,
        };
        match &ot {
            Ty::Named(n, ..)
                if n != "Error"
                    && n != "RegExp"
                    && n != "RegExpMatch"
                    && n != "Date"
                    && n != "Symbol"
                    && !self.is_type_param(n) =>
            {
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
                let subst = self.instance_subst(&ot);
                if let Some(f) = s.field(prop) {
                    // A field holding a function.
                    let ft = substitute(&f.ty, &subst, &self.types);
                    let ft = self.types.resolve(&ft);
                    let Ty::Func(fun) = ft else {
                        return self.err(format!("`{prop}` is a field, not a method"), e.span);
                    };
                    self.dynamic_args(args, &fun, e.span)?;
                    self.calls.insert(e.id, Callee::Dynamic);
                    self.set_ty(callee_id, Ty::Func(fun.clone()));
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
                let used = self.args_in(args, &info, fn_id, e.span, &subst)?;
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
                Ok(substitute(&info.sig.ret, &used, &self.types))
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

/// The type of a field as a value: optional fields are `Option`.
fn field_ty(f: &FieldInfo) -> Ty {
    if f.optional {
        Ty::Option(Box::new(f.ty.clone()))
    } else {
        f.ty.clone()
    }
}

/// The type of `f?.()`: nothing when the call gives nothing, otherwise maybe its value.
fn optional_result(ret: Ty) -> Ty {
    match ret {
        Ty::Void => Ty::Void,
        Ty::Option(t) => Ty::Option(t),
        other => Ty::Option(Box::new(other)),
    }
}

/// The value of an arithmetic expression on literals.
fn const_value(e: &Expr) -> Option<f64> {
    match &e.kind {
        ExprKind::Num { value, .. } => Some(*value),
        ExprKind::Unary {
            op: UnaryOp::Neg,
            arg,
        } => const_value(arg).map(|v| -v),
        ExprKind::Binary { op, left, right } => {
            let (a, b) = (const_value(left)?, const_value(right)?);
            match op {
                BinOp::Add => Some(a + b),
                BinOp::Sub => Some(a - b),
                BinOp::Mul => Some(a * b),
                _ => None,
            }
        }
        _ => None,
    }
}
