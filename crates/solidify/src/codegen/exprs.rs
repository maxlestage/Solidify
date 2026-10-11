//! Expressions.

use super::stmts::Side;
use super::{
    E, GResult, Gen, PREC_ADD, PREC_AND, PREC_ASSIGN, PREC_ATOM, PREC_BITAND, PREC_BITOR,
    PREC_BITXOR, PREC_CMP, PREC_MUL, PREC_OR, PREC_POSTFIX, PREC_SHIFT, PREC_UNARY, Repr, Want,
    format_text, pascal, string_literal,
};
use crate::ast::*;
use crate::infer::{Callee, Role};
use crate::types::Ty;

impl<'a> Gen<'a> {
    pub(super) fn expr(&mut self, e: &'a Expr) -> GResult<E> {
        if let Some((name, test_id)) = self.p.narrowed_uses.get(&e.id) {
            let test = self.p.narrow_tests.get(test_id);
            if test.is_some_and(|t| t.unwrap) {
                // Narrowed inside a `while`: read through `unwrap` each time.
                let t = test.expect("test");
                if t.union.is_some() {
                    return self.err(
                        "a union member tested by a `while` is not supported: test it with `if` inside the loop",
                        e.span,
                    );
                }
                let x = self.expr_plain(e)?;
                let base = Self::paren(&x, PREC_POSTFIX);
                let mutable = t.mutated || x.repr == Repr::RefMut || self.root_is_mut(e);
                if x.repr == Repr::OptRef {
                    let repr = if self.is_str(&t.inner) {
                        Repr::Str
                    } else {
                        Repr::Ref
                    };
                    return Ok(E::new(format!("{base}.unwrap()"), PREC_POSTFIX, repr));
                }
                return Ok(if self.is_copy(&t.inner) && !mutable {
                    E::owned(format!("{base}.unwrap()"), PREC_POSTFIX)
                } else if mutable {
                    E::new(
                        format!("{base}.as_mut().unwrap()"),
                        PREC_POSTFIX,
                        Repr::RefMut,
                    )
                } else {
                    E::new(format!("{base}.as_ref().unwrap()"), PREC_POSTFIX, Repr::Ref)
                });
            }
            let name = snake(name);
            let repr = self
                .narrowed
                .iter()
                .rev()
                .find(|(n, _)| *n == name)
                .map_or(Repr::Owned, |(_, r)| *r);
            return Ok(E::atom(name, repr));
        }
        self.expr_plain(e)
    }

    /// Whether the variable at the root of a path is held mutably.
    fn root_is_mut(&self, e: &Expr) -> bool {
        match &e.kind {
            ExprKind::Ident(_) => self.p.refs.get(&e.id).is_some_and(|b| {
                self.repr_of(*b) == Repr::RefMut
                    || (self.p.var(*b).mutated && self.repr_of(*b) == Repr::Owned)
            }),
            ExprKind::This => {
                self.fn_ctx().class.is_some()
                    && self
                        .p
                        .funcs
                        .get(&self.fn_ctx().id)
                        .is_some_and(|f| f.mutates_self)
            }
            ExprKind::Member { obj, .. } | ExprKind::Index { obj, .. } => self.root_is_mut(obj),
            ExprKind::NonNull(inner) => self.root_is_mut(inner),
            _ => false,
        }
    }

    /// An expression, ignoring narrowing of the expression itself.
    fn expr_plain(&mut self, e: &'a Expr) -> GResult<E> {
        let ty = self.ty_of(e);
        match &e.kind {
            ExprKind::Num { value, raw, int } => Ok(self.number(*value, raw, *int, &ty)),
            ExprKind::Str(s) => {
                if let Some((en, variant)) = self.p.enum_literal.get(&e.id) {
                    return Ok(E::atom(format!("{}::{}", pascal(en), variant), Repr::Owned));
                }
                Ok(E::atom(string_literal(s), Repr::Str))
            }
            ExprKind::Template { quasis, exprs } => self.template(quasis, exprs),
            ExprKind::Bool(b) => Ok(E::atom(if *b { "true" } else { "false" }, Repr::Owned)),
            ExprKind::Null | ExprKind::Undefined => Ok(E::atom("None", Repr::Owned)),
            ExprKind::This => {
                let name = self.fn_ctx().this_name;
                Ok(E::atom(
                    name,
                    if name == "self" {
                        Repr::Ref
                    } else {
                        Repr::Owned
                    },
                ))
            }
            ExprKind::Ident(_) => self.ident(e),
            ExprKind::Array(elems) => self.array(e, elems, &ty),
            ExprKind::Object(props) => self.object(e, props, &ty),
            ExprKind::Func(f) | ExprKind::Arrow(f) => self.closure(f, &[], false),
            ExprKind::Unary { op, arg } => self.unary(*op, arg),
            ExprKind::Update { .. } | ExprKind::Assign { .. } => self.err(
                "an assignment cannot be used as a value here: write it on its own line",
                e.span,
            ),
            ExprKind::Binary { op, left, right } => self.binary(*op, left, right, e),
            ExprKind::Logical { op, left, right } => self.logical(*op, left, right, e),
            ExprKind::Cond { test, cons, alt } => {
                let (ct, at) = (self.ty_of(cons), self.ty_of(alt));
                let before = self.narrowed.len();
                if let Some((n, path_expr, sides)) = self.union_test_of(test) {
                    let pt = self.ty_of(path_expr);
                    let x = self.expr(path_expr)?;
                    let name = snake(&n.name);
                    let (_, source) = self.narrow_binding(&n, x, &pt);
                    let repr = self.narrowed.last().map_or(Repr::Ref, |(_, r)| *r);
                    self.narrowed.truncate(before);
                    let en = pascal(&sides.union);
                    // An arm: the pattern, and the value of the branch with the member bound.
                    let arm = |g: &mut Self,
                               side: &Side,
                               branch: &'a Expr,
                               bt: &Ty|
                     -> GResult<(String, String)> {
                        let (pattern, prelude, r) = match side {
                            Side::Variant(v) => (format!("{en}::{v}({name})"), String::new(), repr),
                            Side::Union(t) => {
                                let conv = g.union_conversion("other", &sides.union, t, repr)?;
                                (
                                    "other".to_string(),
                                    format!("let {name} = {conv}; "),
                                    Repr::Owned,
                                )
                            }
                        };
                        g.narrowed.push((name.clone(), r));
                        let b = g.expr(branch)?;
                        let b = g.owned_as(b, bt, &ty);
                        g.narrowed.truncate(before);
                        Ok((pattern, format!("{prelude}{b}")))
                    };
                    return Ok(match (&sides.when_true, &sides.when_false) {
                        (Some(t), Some(f)) => {
                            let (p1, a) = arm(self, t, cons, &ct)?;
                            let (p2, b) = arm(self, f, alt, &at)?;
                            E::owned(
                                format!(
                                    "match {source} {{ {p1} => {{ {a} }}, {p2} => {{ {b} }} }}"
                                ),
                                PREC_ATOM,
                            )
                        }
                        (Some(t), None) => {
                            let (p1, a) = arm(self, t, cons, &ct)?;
                            let b = self.expr(alt)?;
                            let b = self.owned_as(b, &at, &ty);
                            E::owned(
                                format!("if let {p1} = {source} {{ {a} }} else {{ {b} }}"),
                                PREC_ATOM,
                            )
                        }
                        (None, Some(f)) => {
                            let (p2, b) = arm(self, f, alt, &at)?;
                            let a = self.expr(cons)?;
                            let a = self.owned_as(a, &ct, &ty);
                            E::owned(
                                format!("if let {p2} = {source} {{ {b} }} else {{ {a} }}"),
                                PREC_ATOM,
                            )
                        }
                        (None, None) => unreachable!(),
                    });
                }
                if let Some((n, path_expr)) = self.negative_narrow(test)
                    && !n.unwrap
                {
                    let pt = self.ty_of(path_expr);
                    let x = self.expr(path_expr)?;
                    let (pattern, source) = self.narrow_binding(&n, x, &pt);
                    let b = self.expr(alt)?;
                    let b = self.owned_as(b, &at, &ty);
                    self.narrowed.truncate(before);
                    let a = self.expr(cons)?;
                    let a = self.owned_as(a, &ct, &ty);
                    return Ok(E::owned(
                        format!("if let {pattern} = {source} {{ {b} }} else {{ {a} }}"),
                        PREC_ATOM,
                    ));
                }
                let c = self.cond(test)?;
                let a = self.expr(cons)?;
                let a = self.owned_as(a, &ct, &ty);
                self.narrowed.truncate(before);
                let b = self.expr(alt)?;
                let b = self.owned_as(b, &at, &ty);
                Ok(E::owned(
                    format!("if {c} {{ {a} }} else {{ {b} }}"),
                    PREC_ATOM,
                ))
            }
            ExprKind::Call { callee, args, .. } => self.call(e, callee, args),
            ExprKind::New { callee, args, .. } => self.new_expr(e, callee, args),
            ExprKind::Member {
                obj,
                prop,
                optional,
            } => self.member(e, obj, prop, *optional),
            ExprKind::Index { obj, index, .. } => self.index(e, obj, index),
            ExprKind::As(inner, _) | ExprKind::NonNull(inner) => {
                let inner_ty = self.ty_of(inner);
                let x = self.expr(inner)?;
                if matches!(self.resolve(&inner_ty), Ty::Option(_))
                    && !matches!(self.resolve(&ty), Ty::Option(_))
                {
                    let code = self.owned(x, &inner_ty);
                    if self.is_boxed(&ty) {
                        return Ok(E::owned(format!("*{code}.unwrap()"), PREC_UNARY));
                    }
                    return Ok(E::owned(format!("{code}.unwrap()"), PREC_POSTFIX));
                }
                Ok(x)
            }
            ExprKind::Await(inner) => {
                let x = self.expr(inner)?;
                if x.code.ends_with(".await") {
                    return Ok(x);
                }
                Ok(E::owned(
                    format!("{}.await", Self::paren(&x, PREC_POSTFIX)),
                    PREC_POSTFIX,
                ))
            }
            ExprKind::Void(_) => Ok(E::atom("()", Repr::Owned)),
            ExprKind::Seq(items) => {
                let mut parts = Vec::new();
                for (i, item) in items.iter().enumerate() {
                    let x = self.expr(item)?;
                    if i + 1 == items.len() {
                        parts.push(x.code);
                    } else {
                        parts.push(format!("{};", x.code));
                    }
                }
                Ok(E::owned(format!("{{ {} }}", parts.join(" ")), PREC_ATOM))
            }
            ExprKind::TypeOf(inner) => self.typeof_expr(inner),
            ExprKind::Tagged { .. }
            | ExprKind::Regex { .. }
            | ExprKind::Super
            | ExprKind::Class(_)
            | ExprKind::Delete(_)
            | ExprKind::Yield { .. } => self.err("this expression is not supported", e.span),
        }
    }

    /// `typeof x`: known at compile time, except for a union, which asks its value.
    fn typeof_expr(&mut self, inner: &'a Expr) -> GResult<E> {
        let ty = self.ty_of(inner);
        if let Some(u) = self.union_of(&ty) {
            let name = u.name.clone();
            self.union_typeof.insert(name);
            let x = self.recv(inner)?;
            return Ok(E::owned(format!("{x}.js_typeof()"), PREC_POSTFIX));
        }
        let word = |g: &Self, t: &Ty| -> &'static str {
            match g.resolve(t) {
                Ty::Num(_) => "number",
                Ty::Str => "string",
                Ty::Bool => "boolean",
                Ty::Func(_) => "function",
                Ty::Void | Ty::Null => "undefined",
                _ => "object",
            }
        };
        if let Ty::Option(t) = self.resolve(&ty) {
            let x = self.recv(inner)?;
            let some = word(self, &t);
            return Ok(E::owned(
                format!("if {x}.is_some() {{ \"{some}\" }} else {{ \"object\" }}"),
                PREC_ATOM,
            ));
        }
        let w = word(self, &ty);
        self.expr(inner)?;
        Ok(E::atom(format!("\"{w}\""), Repr::Str))
    }

    /// An expression as the context wants it.
    pub(super) fn expr_as(&mut self, e: &'a Expr, want: Want) -> GResult<String> {
        let ty = self.ty_of(e);
        let x = self.expr(e)?;
        Ok(match want {
            Want::Owned => self.owned(x, &ty),
            other => self.coerce(x, &ty, other),
        })
    }

    /// An owned value of the expression, converted to `target` (`Some(…)` for an option).
    pub(super) fn expr_to(&mut self, e: &'a Expr, target: &Ty) -> GResult<String> {
        let ty = self.ty_of(e);
        let x = self.expr(e)?;
        Ok(self.owned_as(x, &ty, target))
    }

    fn number(&self, value: f64, raw: &str, int: bool, ty: &Ty) -> E {
        if self.is_float(ty) {
            let text = if int {
                if value.fract() == 0.0 && value.abs() < 1e16 {
                    format!("{value:.1}")
                } else {
                    format!("{value:?}")
                }
            } else {
                let r = raw.replace('_', "");
                if r.starts_with('.') {
                    format!("0{r}")
                } else if r.ends_with('.') {
                    format!("{r}0")
                } else if r.contains(['e', 'E']) && !r.contains('.') {
                    format!("{value:?}")
                } else {
                    r
                }
            };
            return E::atom(text, Repr::Owned);
        }
        let r = raw.replace('_', "");
        let text = if int
            && (r.starts_with("0x")
                || r.starts_with("0X")
                || r.starts_with("0b")
                || r.starts_with("0o"))
        {
            r.to_lowercase()
        } else if value.abs() > i32::MAX as f64 {
            format!("{}_i64", value as i64)
        } else {
            format!("{}", value as i64)
        };
        E::atom(text, Repr::Owned)
    }

    pub(super) fn ident(&mut self, e: &'a Expr) -> GResult<E> {
        let ExprKind::Ident(name) = &e.kind else {
            unreachable!()
        };
        let Some(&binding) = self.p.refs.get(&e.id) else {
            return match name.as_str() {
                "Infinity" => Ok(E::atom("f64::INFINITY", Repr::Owned)),
                "NaN" => Ok(E::atom("f64::NAN", Repr::Owned)),
                _ => self.err(format!("unknown name `{name}`"), e.span),
            };
        };
        let var = self.p.var(binding);
        match var.role {
            Role::Func => {
                let fn_id = self.p.fn_bindings[&binding];
                let f = &self.p.funcs[&fn_id];
                if f.is_closure {
                    return Ok(E::atom(self.name(binding), Repr::Owned));
                }
                Ok(E::atom(snake(name), Repr::Owned))
            }
            Role::TopLevel if var.const_literal => {
                let repr = if self.is_str(&var.ty) {
                    Repr::Str
                } else {
                    Repr::Owned
                };
                Ok(E::atom(self.name(binding), repr))
            }
            _ => {
                let mut x = E::atom(self.name(binding), self.repr_of(binding));
                x.var = Some((binding, e.id));
                Ok(x)
            }
        }
    }

    fn template(&mut self, quasis: &[String], exprs: &'a [Expr]) -> GResult<E> {
        if exprs.is_empty() {
            return Ok(E::owned(
                format!("{}.to_string()", string_literal(&quasis[0])),
                PREC_POSTFIX,
            ));
        }
        let mut fmt = String::new();
        let mut args = Vec::new();
        for (i, q) in quasis.iter().enumerate() {
            fmt.push_str(&format_text(q));
            if let Some(x) = exprs.get(i) {
                let (spec, arg) = self.format_arg(x)?;
                fmt.push_str(&spec);
                if let Some(a) = arg {
                    args.push(a);
                }
            }
        }
        Ok(E::owned(format_call("format!", &fmt, &args), PREC_ATOM))
    }

    /// `x` (a Rust expression of type `ty`) as JavaScript's `String(x)`.
    pub(super) fn string_of(&mut self, code: &str, ty: &Ty) -> String {
        if self.is_inspect_ty(ty) {
            self.inspect_call(code, ty, false)
        } else {
            format!("{code}.to_string()")
        }
    }

    /// A value shown by `console.log`: objects, arrays, maps and sets are inspected like
    /// Node does; the rest prints as in a template string.
    pub(super) fn log_arg(&mut self, x: &'a Expr) -> GResult<(String, Option<String>)> {
        let ty = self.ty_of(x);
        if !self.is_inspect_ty(&ty) {
            return self.format_arg(x);
        }
        if let Some((shown, _)) = empty_literal(x) {
            return Ok((format_text(shown), None));
        }
        let e = self.expr(x)?;
        if let Ty::Option(inner) = self.resolve(&ty) {
            let missing = self.missing_word(x);
            let code = self.coerce(e, &ty, Want::Display);
            let shown = self.inspect_call("v", &inner, true);
            return Ok((
                "{}".into(),
                Some(format!(
                    "{code}.as_ref().map_or(\"{missing}\".to_string(), |v| {shown})"
                )),
            ));
        }
        let code = self.coerce(e, &ty, Want::Display);
        let call = self.inspect_call(&code, &ty, true);
        // A union may hold a string, shown as it is.
        let call = if self.union_of(&ty).is_some() {
            call.replacen("js_inspect(", "js_show(", 1)
        } else {
            call
        };
        Ok(("{}".into(), Some(call)))
    }

    /// `undefined` for the methods that give it, `null` otherwise.
    fn missing_word(&self, x: &Expr) -> &'static str {
        match self.p.calls.get(&x.id) {
            Some(Callee::Builtin(n))
                if matches!(
                    *n,
                    "map.get"
                        | "array.find"
                        | "array.findLast"
                        | "array.pop"
                        | "array.shift"
                        | "array.at"
                ) =>
            {
                "undefined"
            }
            Some(Callee::Builtin("global.parseInt")) => "NaN",
            _ => "null",
        }
    }

    /// A value in a format string: `{name}` inline when it is a plain variable, else a
    /// `{}` and the argument.
    pub(super) fn format_arg(&mut self, x: &'a Expr) -> GResult<(String, Option<String>)> {
        let ty = self.ty_of(x);
        let spec = self.fmt_spec(&ty);
        let e = self.expr(x)?;
        // Objects, arrays, maps and sets become strings the way JavaScript does it.
        if self.is_inspect_ty(&ty)
            && !matches!(self.resolve(&ty), Ty::Named(ref n) if self.p.find_method(n, "toString", MethodKind::Method).is_some())
        {
            if let Some((_, text)) = empty_literal(x) {
                return Ok((format_text(text), None));
            }
            if let Ty::Option(inner) = self.resolve(&ty) {
                let missing = self.missing_word(x);
                let code = self.coerce(e, &ty, Want::Display);
                let shown = self.inspect_call("v", &inner, false);
                return Ok((
                    "{}".into(),
                    Some(format!(
                        "{code}.as_ref().map_or(\"{missing}\".to_string(), |v| {shown})"
                    )),
                ));
            }
            let code = self.coerce(e, &ty, Want::Display);
            return Ok(("{}".into(), Some(self.inspect_call(&code, &ty, false))));
        }
        // A struct with `toString()` prints through it.
        if let Ty::Named(n) = self.resolve(&ty)
            && n != "Error"
            && !self.p.is_enum(&n)
            && self
                .p
                .find_method(&n, "toString", MethodKind::Method)
                .is_some()
        {
            let code = self.coerce(e, &ty, Want::Display);
            return Ok(("{}".into(), Some(format!("{code}.to_string()"))));
        }
        if let Ty::Option(inner) = self.resolve(&ty) {
            let missing = self.missing_word(x);
            let code = self.coerce(e, &ty, Want::Display);
            let inner_spec = self.fmt_spec(&inner);
            let shown = if inner_spec == "{}" {
                "v.to_string()".to_string()
            } else {
                "format!(\"{:?}\", v)".to_string()
            };
            return Ok((
                "{}".into(),
                Some(format!(
                    "{code}.as_ref().map_or(\"{missing}\".to_string(), |v| {shown})"
                )),
            ));
        }
        if spec == "{:?}" {
            self.bound_needed(&ty, "std::fmt::Debug");
        } else {
            self.bound_needed(&ty, "std::fmt::Display");
        }
        let code = self.coerce(e, &ty, Want::Display);
        if is_plain_ident(&code) && !code.starts_with("r#") {
            let inline = if spec == "{}" {
                format!("{{{code}}}")
            } else {
                format!("{{{code}:?}}")
            };
            return Ok((inline, None));
        }
        Ok((spec.to_string(), Some(code)))
    }

    fn array(&mut self, e: &'a Expr, elems: &'a [ArrayElem], ty: &Ty) -> GResult<E> {
        let item = match self.resolve(ty) {
            Ty::Array(t) => *t,
            Ty::Tuple(ts) => {
                let mut parts = Vec::new();
                for (el, t) in elems.iter().zip(&ts) {
                    let ArrayElem::Expr(x) = el else {
                        unreachable!()
                    };
                    parts.push(self.expr_to(x, t)?);
                }
                if parts.len() == 1 {
                    return Ok(E::atom(format!("({},)", parts[0]), Repr::Owned));
                }
                return Ok(E::atom(format!("({})", parts.join(", ")), Repr::Owned));
            }
            _ => return self.err("unexpected array type", e.span),
        };
        if elems.is_empty() {
            return Ok(E::atom("Vec::new()", Repr::Owned));
        }
        if elems.iter().any(|el| matches!(el, ArrayElem::Spread(_))) {
            let mut parts = Vec::new();
            let mut run: Vec<String> = Vec::new();
            for el in elems {
                match el {
                    ArrayElem::Expr(x) => run.push(self.expr_to(x, &item)?),
                    ArrayElem::Spread(x) => {
                        if !run.is_empty() {
                            parts.push(format!("vec![{}]", run.join(", ")));
                            run.clear();
                        }
                        let xt = self.ty_of(x);
                        let s = self.expr(x)?;
                        let recv = Self::paren(&s, PREC_POSTFIX);
                        parts.push(match self.resolve(&xt) {
                            Ty::Set(_) => format!("{recv}.iter().cloned().collect::<Vec<_>>()"),
                            Ty::Gen(_) => format!("{recv}.collect::<Vec<_>>()"),
                            Ty::Map(..) => format!(
                                "{recv}.iter().map(|(k, v)| (k.clone(), v.clone())).collect::<Vec<_>>()"
                            ),
                            Ty::Str => {
                                format!("{recv}.chars().map(String::from).collect::<Vec<_>>()")
                            }
                            _ => self.owned(s, &xt),
                        });
                    }
                    ArrayElem::Hole => {}
                }
            }
            if !run.is_empty() {
                parts.push(format!("vec![{}]", run.join(", ")));
            }
            if parts.len() == 1 {
                return Ok(E::owned(parts.pop().unwrap_or_default(), PREC_POSTFIX));
            }
            return Ok(E::owned(
                format!("[{}].concat()", parts.join(", ")),
                PREC_POSTFIX,
            ));
        }
        let mut parts = Vec::new();
        for el in elems {
            let ArrayElem::Expr(x) = el else {
                unreachable!()
            };
            parts.push(self.expr_to(x, &item)?);
        }
        Ok(E::atom(format!("vec![{}]", parts.join(", ")), Repr::Owned))
    }

    fn object(&mut self, e: &'a Expr, props: &'a [Prop], ty: &Ty) -> GResult<E> {
        if let Ty::Map(k, v, _) = self.resolve(ty) {
            let map_name = self.map_name();
            let mut entries = Vec::new();
            for p in props {
                let PropValue::Expr(value) = &p.value else {
                    return self.err("unsupported map entry", p.span);
                };
                let key = match &p.key {
                    PropKey::Computed(kx) => self.expr_to(kx, &k)?,
                    PropKey::Num(n) => {
                        if self.is_float(&k) {
                            format!("{n:?}")
                        } else {
                            format!("{}", *n as i64)
                        }
                    }
                    other => {
                        let s = other.name().unwrap_or_default();
                        if self.is_str(&k) {
                            format!("{}.to_string()", string_literal(&s))
                        } else {
                            s
                        }
                    }
                };
                let val = self.expr_to(value, &v)?;
                entries.push(format!("({key}, {val})"));
            }
            if entries.is_empty() {
                return Ok(E::atom(format!("{map_name}::new()"), Repr::Owned));
            }
            return Ok(E::atom(
                format!("{map_name}::from([{}])", entries.join(", ")),
                Repr::Owned,
            ));
        }
        let Some(struct_name) = self.p.literal_struct.get(&e.id).cloned() else {
            return self.err("this object has no known type", e.span);
        };
        let info = self.p.struct_info(&struct_name).expect("struct").clone();
        let mut fields = Vec::new();
        let mut base = None;
        let mut seen = Vec::new();
        for p in props {
            match &p.value {
                PropValue::Spread(s) => {
                    let st = self.ty_of(s);
                    let x = self.expr(s)?;
                    base = Some(self.owned(x, &st));
                }
                PropValue::Expr(v) => {
                    let key = p.key.name().unwrap_or_default();
                    let f = info.field(&key).expect("field");
                    let target = if f.optional {
                        Ty::Option(Box::new(f.ty.clone()))
                    } else {
                        f.ty.clone()
                    };
                    let val = self.expr_to(v, &target)?;
                    let rust_key = snake(&key);
                    seen.push(key);
                    if val == rust_key {
                        fields.push(rust_key);
                    } else {
                        fields.push(format!("{rust_key}: {val}"));
                    }
                }
                PropValue::Shorthand(id) => {
                    let key = p.key.name().unwrap_or_default();
                    let f = info.field(&key).expect("field");
                    let target = if f.optional {
                        Ty::Option(Box::new(f.ty.clone()))
                    } else {
                        f.ty.clone()
                    };
                    let binding = self.p.refs[id];
                    let var_ty = self.p.var(binding).ty.clone();
                    let mut x = E::atom(self.name(binding), self.repr_of(binding));
                    x.var = Some((binding, *id));
                    let val = self.owned_as(x, &var_ty, &target);
                    let rust_key = snake(&key);
                    seen.push(key);
                    if val == rust_key {
                        fields.push(rust_key);
                    } else {
                        fields.push(format!("{rust_key}: {val}"));
                    }
                }
                _ => return self.err("object methods are not supported", p.span),
            }
        }
        if base.is_none() {
            for f in &info.fields {
                if f.optional && !seen.contains(&f.name) {
                    fields.push(format!("{}: None", snake(&f.name)));
                }
            }
        }
        if let Some(b) = base {
            fields.push(format!("..{b}"));
        }
        let name = pascal(&struct_name);
        if fields.is_empty() {
            return Ok(E::atom(format!("{name} {{}}"), Repr::Owned));
        }
        Ok(E::atom(
            format!("{name} {{ {} }}", fields.join(", ")),
            Repr::Owned,
        ))
    }

    fn unary(&mut self, op: UnaryOp, arg: &'a Expr) -> GResult<E> {
        match op {
            UnaryOp::Not => {
                let c = self.cond(arg)?;
                let c = if c.contains(' ') { format!("({c})") } else { c };
                Ok(E::owned(format!("!{c}"), PREC_UNARY))
            }
            UnaryOp::Neg => {
                let ty = self.ty_of(arg);
                let x = self.expr(arg)?;
                let code = self.coerce(x, &ty, Want::Owned);
                let inner = E::owned(code, PREC_ATOM);
                let needs_paren = inner.code.starts_with('-') || !is_simple(&inner.code);
                Ok(E::owned(
                    if needs_paren {
                        format!("-({})", inner.code)
                    } else {
                        format!("-{}", inner.code)
                    },
                    PREC_UNARY,
                ))
            }
            UnaryOp::Pos => {
                let ty = self.ty_of(arg);
                if self.is_str(&ty) {
                    let s = self.expr_as(arg, Want::StrRef)?;
                    let h = self.helper(super::Helper::ParseFloat);
                    return Ok(E::owned(format!("{h}({s})"), PREC_POSTFIX));
                }
                let x = self.expr(arg)?;
                let code = self.coerce(x, &ty, Want::Owned);
                Ok(E::owned(code, PREC_ATOM))
            }
            UnaryOp::BitNot => {
                let x = self.expr_as(arg, Want::Owned)?;
                Ok(E::owned(
                    format!("!{}", paren_unless_simple(&x)),
                    PREC_UNARY,
                ))
            }
        }
    }

    /// Both operands of an arithmetic or comparison operator, as owned numbers; integers
    /// that are lengths stay `usize` when the other side is one too (or a literal).
    fn numeric_operands(&mut self, left: &'a Expr, right: &'a Expr) -> GResult<(E, E, Repr)> {
        let (lt, rt) = (self.ty_of(left), self.ty_of(right));
        let l = self.expr(left)?;
        let r = self.expr(right)?;
        let both_usize = (l.repr == Repr::Usize || r.repr == Repr::Usize)
            && (l.repr == Repr::Usize || is_int_literal_e(&l))
            && (r.repr == Repr::Usize || is_int_literal_e(&r))
            && !self.is_float(&lt);
        if both_usize {
            return Ok((l, r, Repr::Usize));
        }
        let lc = self.coerce(l.clone(), &lt, Want::Owned);
        let rc = self.coerce(r.clone(), &rt, Want::Owned);
        Ok((
            E::new(
                lc,
                l.prec
                    .max(if l.repr == Repr::Usize { 0 } else { PREC_ATOM })
                    .min(l.prec),
                Repr::Owned,
            ),
            E::new(rc, r.prec, Repr::Owned),
            Repr::Owned,
        ))
    }

    fn binary(&mut self, op: BinOp, left: &'a Expr, right: &'a Expr, e: &'a Expr) -> GResult<E> {
        let ty = self.ty_of(e);
        let (lt, rt) = (self.ty_of(left), self.ty_of(right));
        // String concatenation.
        if op == BinOp::Add && self.is_str(&ty) {
            let mut parts = Vec::new();
            flatten_concat(e, &mut parts);
            let mut fmt = String::new();
            let mut args = Vec::new();
            for part in parts {
                if let ExprKind::Str(s) = &part.kind
                    && !self.p.enum_literal.contains_key(&part.id)
                {
                    fmt.push_str(&format_text(s));
                    continue;
                }
                if let ExprKind::Template { quasis, exprs } = &part.kind {
                    for (i, q) in quasis.iter().enumerate() {
                        fmt.push_str(&format_text(q));
                        if let Some(x) = exprs.get(i) {
                            let (spec, arg) = self.format_arg(x)?;
                            fmt.push_str(&spec);
                            args.extend(arg);
                        }
                    }
                    continue;
                }
                let (spec, arg) = self.format_arg(part)?;
                fmt.push_str(&spec);
                args.extend(arg);
            }
            return Ok(E::owned(format_call("format!", &fmt, &args), PREC_ATOM));
        }
        match op {
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Mod => {
                let (l, r, repr) = self.numeric_operands(left, right)?;
                let prec = if matches!(op, BinOp::Add | BinOp::Sub) {
                    PREC_ADD
                } else {
                    PREC_MUL
                };
                let lc = Self::paren(&l, prec);
                let rc = Self::paren(&r, prec + 1);
                Ok(E::new(format!("{lc} {} {rc}", op.symbol()), prec, repr))
            }
            BinOp::Div => {
                let (l, r, _) = self.numeric_operands(left, right)?;
                let (lc, rc) = if self.is_float(&lt) {
                    (Self::paren(&l, PREC_MUL), Self::paren(&r, PREC_MUL + 1))
                } else {
                    (self.as_f64(&l), self.as_f64(&r))
                };
                Ok(E::owned(format!("{lc} / {rc}"), PREC_MUL))
            }
            BinOp::Pow => {
                let (l, r, _) = self.numeric_operands(left, right)?;
                let lc = num_recv(&Self::paren(&l, PREC_POSTFIX), self.is_float(&lt));
                if self.is_float(&lt) {
                    let rc = self.as_f64(&r);
                    return Ok(E::owned(format!("{lc}.powf({rc})"), PREC_POSTFIX));
                }
                Ok(E::owned(
                    format!("{lc}.pow({} as u32)", Self::paren(&r, PREC_CAST_PLUS)),
                    PREC_POSTFIX,
                ))
            }
            BinOp::Shl
            | BinOp::Shr
            | BinOp::UShr
            | BinOp::BitAnd
            | BinOp::BitOr
            | BinOp::BitXor => {
                let (l, r, _) = self.numeric_operands(left, right)?;
                let (sym, prec) = match op {
                    BinOp::Shl => ("<<", PREC_SHIFT),
                    BinOp::Shr | BinOp::UShr => (">>", PREC_SHIFT),
                    BinOp::BitAnd => ("&", PREC_BITAND),
                    BinOp::BitOr => ("|", PREC_BITOR),
                    _ => ("^", PREC_BITXOR),
                };
                let lc = Self::paren(&l, prec);
                let rc = Self::paren(&r, prec + 1);
                if op == BinOp::UShr {
                    return Ok(E::owned(
                        format!("(({lc} as u32) >> {rc}) as i64"),
                        PREC_CAST_PLUS,
                    ));
                }
                Ok(E::owned(format!("{lc} {sym} {rc}"), prec))
            }
            BinOp::Lt | BinOp::Gt | BinOp::LtEq | BinOp::GtEq => {
                if let Ty::Named(n) = self.resolve(&lt)
                    && self.is_type_param(&n)
                {
                    self.bound_needed(&lt, "PartialOrd");
                    let l = self.expr_as(left, Want::Ref)?;
                    let r = self.expr_as(right, Want::Ref)?;
                    return Ok(E::owned(format!("{l} {} {r}", op.symbol()), PREC_CMP));
                }
                if self.is_str(&lt) {
                    let l = self.expr_as(left, Want::StrRef)?;
                    let r = self.expr_as(right, Want::StrRef)?;
                    return Ok(E::owned(
                        format!("{} {} {}", deref_str(&l), op.symbol(), deref_str(&r)),
                        PREC_CMP,
                    ));
                }
                let (l, r, _) = self.numeric_operands(left, right)?;
                let lc = Self::paren(&l, PREC_CMP + 1);
                let rc = Self::paren(&r, PREC_CMP + 1);
                Ok(E::owned(format!("{lc} {} {rc}", op.symbol()), PREC_CMP))
            }
            BinOp::Eq | BinOp::NotEq | BinOp::StrictEq | BinOp::StrictNotEq => {
                let negate = matches!(op, BinOp::NotEq | BinOp::StrictNotEq);
                let sym = if negate { "!=" } else { "==" };
                // Comparisons with null.
                let (lnull, rnull) = (
                    matches!(left.kind, ExprKind::Null | ExprKind::Undefined),
                    matches!(right.kind, ExprKind::Null | ExprKind::Undefined),
                );
                if lnull || rnull {
                    let value = if rnull { left } else { right };
                    let vt = self.ty_of(value);
                    if matches!(self.resolve(&vt), Ty::Option(_)) {
                        let v = self.expr(value)?;
                        let code = Self::paren(&v, PREC_POSTFIX);
                        return Ok(E::owned(
                            format!("{code}.{}()", if negate { "is_some" } else { "is_none" }),
                            PREC_POSTFIX,
                        ));
                    }
                    if matches!(self.resolve(&vt), Ty::Null) {
                        return Ok(E::atom(if negate { "false" } else { "true" }, Repr::Owned));
                    }
                    // A value that can never be null.
                    return Ok(E::atom(if negate { "true" } else { "false" }, Repr::Owned));
                }
                let (l, r) = (self.expr(left)?, self.expr(right)?);
                let (lc, rc) = self.equality_operands(l, r, &lt, &rt);
                Ok(E::owned(format!("{lc} {sym} {rc}"), PREC_CMP))
            }
            BinOp::In => {
                let r = self.expr(right)?;
                let rc = Self::paren(&r, PREC_POSTFIX);
                let Ty::Map(k, _, _) = self.resolve(&rt) else {
                    return self.err("`in` needs a map", e.span);
                };
                let key = self.expr_as(
                    left,
                    if self.is_str(&k) {
                        Want::StrRef
                    } else {
                        Want::Ref
                    },
                )?;
                Ok(E::owned(format!("{rc}.contains_key({key})"), PREC_POSTFIX))
            }
            BinOp::InstanceOf => {
                let ExprKind::Ident(sub) = &right.kind else {
                    return self.err("`instanceof` needs a class name", right.span);
                };
                let Ty::Named(class) = self.resolve(&lt) else {
                    self.expr(left)?;
                    return Ok(E::atom("false", Repr::Owned));
                };
                if self.p.is_subclass(&class, sub) {
                    self.expr(left)?;
                    return Ok(E::atom("true", Repr::Owned));
                }
                if self.p.is_dyn(&class) && self.p.is_subclass(sub, &class) {
                    self.instance_tests
                        .insert((class.clone(), sub.clone(), false));
                    let x = self.recv(left)?;
                    return Ok(E::owned(
                        format!("{x}.as_{}().is_some()", snake(sub)),
                        PREC_POSTFIX,
                    ));
                }
                self.expr(left)?;
                Ok(E::atom("false", Repr::Owned))
            }
        }
    }

    fn as_f64(&mut self, x: &E) -> String {
        if is_int_literal_e(x) {
            return format!("{}.0", x.code);
        }
        if let Some(base) = x.code.strip_suffix(" as i64") {
            return format!("{base} as f64");
        }
        format!("{} as f64", Self::paren(x, PREC_CAST_PLUS))
    }

    /// Two values compared with `==`: references on both sides for non-copy values.
    fn equality_operands(&mut self, l: E, r: E, lt: &Ty, rt: &Ty) -> (String, String) {
        let lt = self.resolve(lt);
        // A union compared with one of its members: the member goes into its variant.
        if self.union_variant(rt, &lt).is_some() {
            let lc = self.coerce(l, &lt, Want::Ref);
            let rc = self.owned_as(r, rt, &lt);
            return (lc, format!("&{rc}"));
        }
        if self.union_variant(&lt, rt).is_some() {
            let lc = self.owned_as(l, &lt, rt);
            let rc = self.coerce(r, rt, Want::Ref);
            return (format!("&{lc}"), rc);
        }
        if self.is_copy(&lt) {
            let lc = self.coerce(l, &lt, Want::Owned);
            let rc = self.coerce(r, rt, Want::Owned);
            return (paren_cmp(&lc), paren_cmp(&rc));
        }
        self.bound_needed(&lt, "PartialEq");
        if let Ty::Named(n) = &lt
            && self.is_type_param(n)
        {
            let lc = self.coerce(l, &lt, Want::Ref);
            let rc = self.coerce(r, rt, Want::Ref);
            return (lc, rc);
        }
        if self.is_str(&lt) {
            // char == one-character literal
            if l.repr == Repr::Char || r.repr == Repr::Char {
                let (c, s) = if l.repr == Repr::Char {
                    (&l, &r)
                } else {
                    (&r, &l)
                };
                if let Some(ch) = single_char_literal(&s.code) {
                    let (a, b) = (c.code.clone(), format!("'{ch}'"));
                    return if l.repr == Repr::Char { (a, b) } else { (b, a) };
                }
            }
            let lc = self.coerce(l, &lt, Want::StrRef);
            let rc = self.coerce(r, rt, Want::StrRef);
            return (deref_str(&lc), deref_str(&rc));
        }
        let lc = self.coerce(l, &lt, Want::Ref);
        let rc = self.coerce(r, rt, Want::Ref);
        (lc, rc)
    }

    fn logical(
        &mut self,
        op: LogicalOp,
        left: &'a Expr,
        right: &'a Expr,
        e: &'a Expr,
    ) -> GResult<E> {
        let ty = self.ty_of(e);
        let lt = self.ty_of(left);
        match op {
            LogicalOp::Nullish => {
                let l = self.expr(left)?;
                let lc = if l.repr == Repr::Ref {
                    format!("{}.clone()", Self::paren(&l, PREC_POSTFIX))
                } else {
                    self.owned(l, &lt)
                };
                let r = self.expr(right)?;
                let rt = self.ty_of(right);
                let rc = self.owned_as(r, &rt, &ty);
                Ok(E::owned(format!("{lc}.unwrap_or({rc})"), PREC_POSTFIX))
            }
            LogicalOp::Or => {
                if matches!(self.resolve(&ty), Ty::Bool) {
                    let c = self.cond(e)?;
                    return Ok(E::owned(c, PREC_OR));
                }
                match self.resolve(&lt) {
                    Ty::Option(_) => {
                        let l = self.expr(left)?;
                        let lc = self.owned(l, &lt);
                        let rc = self.expr_to(right, &ty)?;
                        Ok(E::owned(format!("{lc}.unwrap_or({rc})"), PREC_POSTFIX))
                    }
                    Ty::Str => {
                        let l = self.expr(left)?;
                        let test = format!("{}.is_empty()", Self::paren(&l, PREC_POSTFIX));
                        let lc = self.owned(l, &lt);
                        let rc = self.expr_to(right, &ty)?;
                        Ok(E::owned(
                            format!("if {test} {{ {rc} }} else {{ {lc} }}"),
                            PREC_ATOM,
                        ))
                    }
                    Ty::Num(_) => {
                        let l = self.expr(left)?;
                        let lc = self.coerce(l, &lt, Want::Owned);
                        let zero = if self.is_float(&lt) { "0.0" } else { "0" };
                        let rc = self.expr_to(right, &ty)?;
                        Ok(E::owned(
                            format!("if {lc} == {zero} {{ {rc} }} else {{ {lc} }}"),
                            PREC_ATOM,
                        ))
                    }
                    _ => {
                        let l = self.cond(left)?;
                        let r = self.cond(right)?;
                        Ok(E::owned(format!("{l} || {r}"), PREC_OR))
                    }
                }
            }
            LogicalOp::And => {
                let l = self.cond(left)?;
                let r = self.cond(right)?;
                if l.starts_with("let ") || l.contains(" let ") {
                    // A narrowing test cannot be a value: use `if`.
                    return Ok(E::owned(
                        format!("if {l} {{ {r} }} else {{ false }}"),
                        PREC_ATOM,
                    ));
                }
                Ok(E::owned(
                    format!("{} && {}", paren_logical(&l, "||"), paren_logical(&r, "||")),
                    PREC_AND,
                ))
            }
        }
    }

    /// An expression used as a boolean: `if`, `while`, `!`, `&&`. Tests of nullable values
    /// become `let Some(x) = …` chains (edition 2024).
    pub(super) fn cond(&mut self, e: &'a Expr) -> GResult<String> {
        if let Some(n) = self.p.narrow_tests.get(&e.id).cloned()
            && !n.let_else
            && !n.unwrap
        {
            let path_expr = narrow_path_expr(e);
            let pt = self.ty_of(path_expr);
            let x = self.expr(path_expr)?;
            if n.positive {
                let (pattern, source) = self.narrow_binding(&n, x, &pt);
                return Ok(format!("let {pattern} = {source}"));
            }
            let code = Self::paren(&x, PREC_POSTFIX);
            return Ok(format!("{code}.is_none()"));
        }
        match &e.kind {
            ExprKind::Unary {
                op: UnaryOp::Not,
                arg,
            } => {
                // `!isNaN(n)`: the content exists; bind it.
                if let Some(n) = self.p.narrow_tests.get(&arg.id).cloned()
                    && !n.positive
                    && !n.let_else
                    && !n.unwrap
                {
                    let path_expr = narrow_path_expr(arg);
                    let pt = self.ty_of(path_expr);
                    let x = self.expr(path_expr)?;
                    let (pattern, source) = self.narrow_binding(&n, x, &pt);
                    return Ok(format!("let {pattern} = {source}"));
                }
                let inner = self.cond(arg)?;
                if let Some(rest) = inner.strip_suffix(".is_none()") {
                    return Ok(format!("{rest}.is_some()"));
                }
                if let Some(rest) = inner.strip_suffix(".is_some()") {
                    return Ok(format!("{rest}.is_none()"));
                }
                if let Some(rest) = inner.strip_suffix(".is_empty()") {
                    return Ok(format!("!{rest}.is_empty()"));
                }
                if let Some(rest) = inner.strip_prefix('!') {
                    return Ok(rest.to_string());
                }
                if inner.starts_with("let ") {
                    return self.err("a negated nullable test cannot be combined here", e.span);
                }
                Ok(format!("!{}", paren_unless_simple(&inner)))
            }
            ExprKind::Logical {
                op: LogicalOp::And,
                left,
                right,
            } => {
                let l = self.cond(left)?;
                let before = self.narrowed.len();
                self.push_narrows(left);
                let r = self.cond(right);
                self.narrowed.truncate(before);
                let r = r?;
                if l == "true" {
                    return Ok(r);
                }
                if r == "true" {
                    return Ok(l);
                }
                Ok(format!(
                    "{} && {}",
                    paren_logical(&l, "||"),
                    paren_logical(&r, "||")
                ))
            }
            ExprKind::Logical {
                op: LogicalOp::Or,
                left,
                right,
            } => {
                // `test(x) || rest` on a union: the right side sees the other side.
                if let Some((n, path_expr, sides)) = self.union_test_of(left)
                    && let Some(f) = &sides.when_false
                {
                    let pt = self.ty_of(path_expr);
                    let x = self.expr(path_expr)?;
                    let before = self.narrowed.len();
                    let (_, source) = self.narrow_binding(&n, x, &pt);
                    let repr = self.narrowed.last().map_or(Repr::Ref, |(_, r)| *r);
                    self.narrowed.truncate(before);
                    let name = snake(&n.name);
                    let en = pascal(&sides.union);
                    let (pf, prelude, rf) = match f {
                        Side::Variant(v) => (format!("{en}::{v}({name})"), String::new(), repr),
                        Side::Union(t) => {
                            let conv = self.union_conversion("other", &sides.union, t, repr)?;
                            (
                                "other".to_string(),
                                format!("let {name} = {conv}; "),
                                Repr::Owned,
                            )
                        }
                    };
                    self.narrowed.push((name.clone(), rf));
                    let r = self.cond(right);
                    self.narrowed.truncate(before);
                    let r = r?;
                    if r.starts_with("let ") || r.contains(" let ") {
                        return self.err(
                            "a nullable test cannot be combined with `||`: nest the `if`s",
                            e.span,
                        );
                    }
                    return Ok(match &sides.when_true {
                        Some(Side::Variant(v)) => {
                            format!(
                                "match {source} {{ {en}::{v}(_) => true, {pf} => {{ {prelude}{r} }} }}"
                            )
                        }
                        _ => format!("match {source} {{ {pf} => {{ {prelude}{r} }}, _ => true }}"),
                    });
                }
                if let Some((n, path_expr)) = self.negative_narrow(left)
                    && !n.unwrap
                {
                    // `x === null || test(x)`: the right side sees the content.
                    let pt = self.ty_of(path_expr);
                    let x = self.expr(path_expr)?;
                    let copy = self.is_copy(&n.inner);
                    let source = if copy || self.opt_ref_param(&x) {
                        self.coerce(x, &pt, Want::Owned)
                    } else {
                        format!("{}.as_ref()", Self::paren(&x, PREC_POSTFIX))
                    };
                    let name = snake(&n.name);
                    let before = self.narrowed.len();
                    self.narrowed
                        .push((name.clone(), if copy { Repr::Owned } else { Repr::Ref }));
                    let r = self.cond(right);
                    self.narrowed.truncate(before);
                    let r = r?;
                    if r.starts_with("let ") || r.contains(" let ") {
                        return self.err(
                            "a nullable test cannot be combined with `||`: nest the `if`s",
                            e.span,
                        );
                    }
                    return Ok(format!("{source}.is_none_or(|{name}| {r})"));
                }
                let l = self.cond(left)?;
                let r = self.cond(right)?;
                if l.starts_with("let ") || r.starts_with("let ") {
                    return self.err(
                        "a nullable test cannot be combined with `||`: nest the `if`s",
                        e.span,
                    );
                }
                Ok(format!(
                    "{} || {}",
                    paren_logical(&l, "&&"),
                    paren_logical(&r, "&&")
                ))
            }
            _ => {
                let ty = self.ty_of(e);
                let x = self.expr(e)?;
                Ok(match self.resolve(&ty) {
                    Ty::Bool => self.coerce(x, &ty, Want::Owned),
                    Ty::Option(_) => format!("{}.is_some()", Self::paren(&x, PREC_POSTFIX)),
                    Ty::Str => format!("!{}.is_empty()", Self::paren(&x, PREC_POSTFIX)),
                    Ty::Num(_) => {
                        let code = self.coerce(x, &ty, Want::Owned);
                        let zero = if self.is_float(&ty) { "0.0" } else { "0" };
                        format!("{} != {zero}", paren_cmp(&code))
                    }
                    Ty::Array(_) => "true".into(),
                    _ => return self.err("this value cannot be used as a condition", e.span),
                })
            }
        }
    }

    /// The pattern and source of `let Some(name) = source`, with the representation the
    /// bound name gets.
    pub(super) fn narrow_binding(
        &mut self,
        n: &crate::infer::NarrowTest,
        x: E,
        path_ty: &Ty,
    ) -> (String, String) {
        let (mutated, inner) = (n.mutated, &n.inner);
        let name = snake(&n.name);
        let copy = self.is_copy(inner);
        let boxed = self.is_boxed(inner) && n.union.is_none();
        let (source, repr) = if let Some(sub) = &n.instance_of {
            // `x.as_sub()` gives the subclass when `x` is one.
            let base = match self.resolve(path_ty) {
                Ty::Named(b) => b,
                _ => String::new(),
            };
            self.instance_tests.insert((base, sub.clone(), mutated));
            let recv = Self::paren(&x, PREC_POSTFIX);
            if mutated {
                (format!("{recv}.as_{}_mut()", snake(sub)), Repr::RefMut)
            } else {
                (format!("{recv}.as_{}()", snake(sub)), Repr::Ref)
            }
        } else if n.union.is_some() {
            // The enum is matched where it is: its members are borrowed, or moved when
            // the value is not needed afterwards.
            if mutated {
                match x.repr {
                    Repr::RefMut => (x.code.clone(), Repr::RefMut),
                    _ => (
                        format!("&mut {}", Self::paren(&x, PREC_UNARY)),
                        Repr::RefMut,
                    ),
                }
            } else {
                match x.repr {
                    Repr::Ref | Repr::RefMut => (x.code.clone(), Repr::Ref),
                    Repr::Owned if self.can_move(&x) => (x.code.clone(), Repr::Owned),
                    _ => (format!("&{}", Self::paren(&x, PREC_UNARY)), Repr::Ref),
                }
            }
        } else if boxed {
            // `Option<Box<T>>`: the binding sees the `T`.
            let base = Self::paren(&x, PREC_POSTFIX);
            if mutated {
                (format!("{base}.as_deref_mut()"), Repr::RefMut)
            } else if x.repr == Repr::Owned && self.can_move(&x) {
                (format!("{base}.map(|b| *b)"), Repr::Owned)
            } else if self.opt_ref_param(&x) {
                (base, Repr::Ref)
            } else {
                (format!("{base}.as_deref()"), Repr::Ref)
            }
        } else if copy && x.repr != Repr::RefMut && !mutated {
            // `Option<i64>`: by value.
            match x.repr {
                Repr::Ref => (format!("*{}", Self::paren(&x, PREC_UNARY)), Repr::Owned),
                _ => (x.code.clone(), Repr::Owned),
            }
        } else if mutated {
            match x.repr {
                Repr::RefMut => (x.code.clone(), Repr::RefMut),
                _ => (
                    format!("&mut {}", Self::paren(&x, PREC_UNARY)),
                    Repr::RefMut,
                ),
            }
        } else {
            match x.repr {
                Repr::Ref | Repr::RefMut => (x.code.clone(), Repr::Ref),
                Repr::Owned if self.can_move(&x) => (x.code.clone(), Repr::Owned),
                _ => {
                    // `Option<&T>` parameters are already references.
                    if matches!(self.resolve(path_ty), Ty::Option(_)) && self.opt_ref_param(&x) {
                        (
                            x.code.clone(),
                            if self.is_str(inner) {
                                Repr::Str
                            } else {
                                Repr::Ref
                            },
                        )
                    } else {
                        (format!("&{}", Self::paren(&x, PREC_UNARY)), Repr::Ref)
                    }
                }
            }
        };
        self.narrowed.push((name.clone(), repr));
        let pattern = match &n.union {
            Some((en, variant)) => format!("{}::{variant}({name})", pascal(en)),
            None => format!("Some({name})"),
        };
        (pattern, source)
    }

    fn opt_ref_param(&self, x: &E) -> bool {
        if x.repr == Repr::OptRef {
            return true;
        }
        x.var.is_some_and(|(b, _)| {
            let v = self.p.var(b);
            v.role == Role::Param
                && self.repr_of(b) == Repr::Owned
                && !self.is_copy(&v.ty)
                && !v.stored
                && !v.mutated
                && !v.reassigned
        })
    }

    /// Activates the names bound by the narrowing tests of a condition (left side of `&&`).
    pub(super) fn push_narrows(&mut self, e: &'a Expr) {
        if let Some(n) = self.p.narrow_tests.get(&e.id)
            && n.positive
            && !self
                .narrowed
                .iter()
                .any(|(name, _)| *name == snake(&n.name))
        {
            let repr = if self.is_copy(&n.inner) && !n.mutated {
                Repr::Owned
            } else if n.mutated {
                Repr::RefMut
            } else {
                Repr::Ref
            };
            self.narrowed.push((snake(&n.name), repr));
        }
        if let ExprKind::Logical {
            op: LogicalOp::And,
            left,
            right,
        } = &e.kind
        {
            self.push_narrows(left);
            self.push_narrows(right);
        }
    }

    fn member(&mut self, e: &'a Expr, obj: &'a Expr, prop: &str, optional: bool) -> GResult<E> {
        let ty = self.ty_of(e);
        if let Some(Callee::Builtin(name)) = self.p.calls.get(&e.id) {
            return Ok(E::atom(
                match *name {
                    "Math.PI" => "std::f64::consts::PI",
                    "Math.E" => "std::f64::consts::E",
                    "Math.LN2" => "std::f64::consts::LN_2",
                    "Math.LN10" => "std::f64::consts::LN_10",
                    "Math.LOG2E" => "std::f64::consts::LOG2_E",
                    "Math.LOG10E" => "std::f64::consts::LOG10_E",
                    "Math.SQRT2" => "std::f64::consts::SQRT_2",
                    "Math.SQRT1_2" => "std::f64::consts::FRAC_1_SQRT_2",
                    "Number.MAX_SAFE_INTEGER" => "9007199254740991_i64",
                    "Number.MIN_SAFE_INTEGER" => "-9007199254740991_i64",
                    "Number.MAX_VALUE" => "f64::MAX",
                    "Number.MIN_VALUE" => "5e-324",
                    "Number.EPSILON" => "f64::EPSILON",
                    "Number.POSITIVE_INFINITY" => "f64::INFINITY",
                    "Number.NEGATIVE_INFINITY" => "f64::NEG_INFINITY",
                    "Number.NaN" => "f64::NAN",
                    "process.argv" => "std::env::args().collect::<Vec<String>>()",
                    _ => return self.err(format!("`{name}` is not supported"), e.span),
                },
                Repr::Owned,
            ));
        }
        let ot = self.ty_of(obj);
        // Enum variants and static members.
        if let ExprKind::Ident(_) = &obj.kind
            && let Some(&binding) = self.p.refs.get(&obj.id)
        {
            match self.p.var(binding).role {
                Role::Enum => {
                    let en = self.p.enum_info(&self.p.var(binding).name).expect("enum");
                    let v = en
                        .variants
                        .iter()
                        .find(|v| v.source == prop)
                        .expect("variant");
                    return Ok(E::atom(
                        format!("{}::{}", pascal(&en.name), v.name),
                        Repr::Owned,
                    ));
                }
                Role::Class => {
                    let class = pascal(&self.p.var(binding).name);
                    let info = self
                        .p
                        .struct_info(&self.p.var(binding).name)
                        .expect("class");
                    if let Some(m) = info.method(prop, MethodKind::Get) {
                        let _ = m;
                        return Ok(E::owned(
                            format!("{class}::{}()", snake(prop)),
                            PREC_POSTFIX,
                        ));
                    }
                    let repr = if self.is_str(&ty) {
                        Repr::Str
                    } else {
                        Repr::Owned
                    };
                    return Ok(E::atom(format!("{class}::{}", screaming_const(prop)), repr));
                }
                _ => {}
            }
        }
        if optional {
            return self.optional_member(obj, prop, &ot, &ty);
        }
        let o = self.expr(obj)?;
        let base = Self::paren(&o, PREC_POSTFIX);
        let iter_result = matches!(
            self.p.calls.get(&obj.id),
            Some(Callee::Builtin("generator.next"))
        ) || self
            .p
            .refs
            .get(&obj.id)
            .is_some_and(|b| self.p.var(*b).iter_result);
        if iter_result && matches!(self.resolve(&ot), Ty::Option(_)) {
            return Ok(match prop {
                "done" => E::owned(format!("{base}.is_none()"), PREC_POSTFIX),
                _ => o,
            });
        }
        match self.resolve(&ot) {
            Ty::Named(n) if n == "Error" => Ok(match prop {
                "message" => {
                    let mut x = E::atom(
                        base,
                        if o.repr == Repr::Owned {
                            Repr::Owned
                        } else {
                            o.repr
                        },
                    );
                    x.var = o.var;
                    x
                }
                "name" => E::atom("\"Error\"", Repr::Str),
                _ => E::atom("String::new()", Repr::Owned),
            }),
            Ty::Named(n) if self.p.union_info(&n).is_some() => {
                self.union_accessors
                    .insert((n.clone(), prop.to_string(), false));
                Ok(E::new(
                    format!("{base}.{}()", snake(prop)),
                    PREC_POSTFIX,
                    Repr::Ref,
                ))
            }
            Ty::Named(n) => {
                let info = self.p.struct_info(&n).expect("struct");
                if info.field(prop).is_some() {
                    // Through a `dyn` value, a field is read with its accessor.
                    if self.p.is_dyn(&n) && !matches!(obj.kind, ExprKind::This) {
                        return Ok(E::new(
                            format!("{base}.{}()", snake(prop)),
                            PREC_POSTFIX,
                            Repr::Ref,
                        ));
                    }
                    let repr = if o.repr == Repr::Owned && o.var.is_none() {
                        Repr::Owned
                    } else {
                        Repr::Place
                    };
                    let mut x = E::new(format!("{base}.{}", snake(prop)), PREC_POSTFIX, repr);
                    x.var = None;
                    return Ok(x);
                }
                if self.p.find_method(&n, prop, MethodKind::Get).is_some() {
                    return Ok(E::owned(format!("{base}.{}()", snake(prop)), PREC_POSTFIX));
                }
                self.err(format!("unknown property `{prop}`"), e.span)
            }
            Ty::Array(_) | Ty::Tuple(_) if prop == "length" => {
                Ok(E::new(format!("{base}.len()"), PREC_POSTFIX, Repr::Usize))
            }
            Ty::Str if prop == "length" => Ok(E::new(
                format!("{base}.chars().count()"),
                PREC_POSTFIX,
                Repr::Usize,
            )),
            Ty::Map(..) | Ty::Set(_) if prop == "size" => {
                Ok(E::new(format!("{base}.len()"), PREC_POSTFIX, Repr::Usize))
            }
            _ => self.err(format!("unsupported property `{prop}`"), e.span),
        }
    }

    /// `a?.b`: `a.as_ref().map(|a| a.b.clone())`.
    fn optional_member(&mut self, obj: &'a Expr, prop: &str, ot: &Ty, ty: &Ty) -> GResult<E> {
        let o = self.expr(obj)?;
        let is_param_ref = self.opt_ref_param(&o);
        let source = match o.repr {
            Repr::OptRef => o.code.clone(),
            Repr::Owned if is_param_ref || self.can_move(&o) => o.code.clone(),
            Repr::Owned | Repr::Place | Repr::Ref | Repr::RefMut => {
                format!("{}.as_ref()", Self::paren(&o, PREC_POSTFIX))
            }
            _ => o.code.clone(),
        };
        let Ty::Option(inner) = self.resolve(ot) else {
            return self.err("`?.` on a value that is never null", obj.span);
        };
        let field_ty = match self.resolve(&inner) {
            Ty::Named(n) => {
                let info = self.p.struct_info(&n).expect("struct");
                match info.field(prop) {
                    Some(f) => {
                        if f.optional {
                            Ty::Option(Box::new(f.ty.clone()))
                        } else {
                            f.ty.clone()
                        }
                    }
                    None => return self.err(format!("unknown property `{prop}`"), obj.span),
                }
            }
            Ty::Str if prop == "length" => {
                return Ok(E::owned(
                    format!("{source}.map(|v| v.chars().count() as i64)"),
                    PREC_POSTFIX,
                ));
            }
            Ty::Array(_) if prop == "length" => {
                return Ok(E::owned(
                    format!("{source}.map(|v| v.len() as i64)"),
                    PREC_POSTFIX,
                ));
            }
            Ty::Map(..) | Ty::Set(_) if prop == "size" => {
                return Ok(E::owned(
                    format!("{source}.map(|v| v.len() as i64)"),
                    PREC_POSTFIX,
                ));
            }
            _ => return self.err("`?.` is only supported on objects", obj.span),
        };
        let v = "v";
        let access = format!("{v}.{}", snake(prop));
        let value = if self.is_copy(&field_ty) {
            access
        } else {
            format!("{access}.clone()")
        };
        let _ = ty;
        Ok(if matches!(self.resolve(&field_ty), Ty::Option(_)) {
            E::owned(format!("{source}.and_then(|{v}| {value})"), PREC_POSTFIX)
        } else {
            E::owned(format!("{source}.map(|{v}| {value})"), PREC_POSTFIX)
        })
    }

    fn index(&mut self, e: &'a Expr, obj: &'a Expr, index: &'a Expr) -> GResult<E> {
        let ot = self.ty_of(obj);
        let o = self.expr(obj)?;
        let base = Self::paren(&o, PREC_POSTFIX);
        match self.resolve(&ot) {
            Ty::Array(_) => {
                let i = self.expr_as(index, Want::Index)?;
                Ok(E::new(format!("{base}[{i}]"), PREC_POSTFIX, Repr::Place))
            }
            Ty::Tuple(_) => {
                let ExprKind::Num { value, .. } = &index.kind else {
                    return self.err("tuples are indexed with a constant", index.span);
                };
                Ok(E::new(
                    format!("{base}.{}", *value as usize),
                    PREC_POSTFIX,
                    Repr::Place,
                ))
            }
            Ty::Str => {
                let s = self.coerce(o, &ot, Want::StrRef);
                let i = self.expr_as(index, Want::Owned)?;
                let h = self.helper(super::Helper::StrCharAt);
                Ok(E::owned(format!("{h}({s}, {i})"), PREC_POSTFIX))
            }
            Ty::Map(k, _, _) => {
                let key = self.expr_as(
                    index,
                    if self.is_str(&k) {
                        Want::StrRef
                    } else {
                        Want::Ref
                    },
                )?;
                Ok(E::new(format!("{base}[{key}]"), PREC_POSTFIX, Repr::Place))
            }
            Ty::Named(_) => {
                let ExprKind::Str(key) = &index.kind else {
                    return self.err("unsupported index", index.span);
                };
                Ok(E::new(
                    format!("{base}.{}", snake(key)),
                    PREC_POSTFIX,
                    Repr::Place,
                ))
            }
            _ => self.err("unsupported index", e.span),
        }
    }

    /// A closure. `ref_params` says which parameters are references (callbacks of
    /// iterators); `annotate` writes the parameter types.
    pub(super) fn closure(
        &mut self,
        f: &'a Function,
        ref_params: &[Repr],
        annotate: bool,
    ) -> GResult<E> {
        let info = self.p.funcs[&f.id].clone();
        let mut params = Vec::new();
        self.push_scope();
        for (i, p) in f.params.iter().enumerate() {
            let binding = info.params[i];
            let pty = self.resolve(&info.sig.params[i]);
            let repr = ref_params.get(i).copied().unwrap_or(Repr::Owned);
            let var = self.p.var(binding);
            let needs_mut =
                var.reassigned || (var.mutated && repr == Repr::Owned && !self.is_copy(&pty));
            let (pattern, repr) = match repr {
                Repr::Ref if self.is_copy(&pty) => ("&".to_string(), Repr::Owned),
                Repr::Place if self.is_copy(&pty) => ("&&".to_string(), Repr::Owned),
                Repr::Place => (String::new(), Repr::Ref),
                r => (String::new(), r),
            };
            let name = self.bind(binding, repr);
            let mut text = format!("{pattern}{}{name}", if needs_mut { "mut " } else { "" });
            if annotate && pattern.is_empty() {
                let t = self.fn_param_ty(&pty, p.span)?;
                let t = match t.strip_prefix("impl ") {
                    Some(rest) => format!("&dyn {rest}"),
                    None => t,
                };
                text.push_str(&format!(": {t}"));
            }
            params.push(text);
            if !matches!(p.pat, Pattern::Ident { .. }) {
                return self.err("destructured closure parameters are not supported", p.span);
            }
        }
        let ret_ty = info.sig.ret.clone();
        self.fns.push(super::FnGen {
            id: f.id,
            returns_result: false,
            ret_ty: ret_ty.clone(),
            yield_ty: None,
            targets: Vec::new(),
            try_ctx: Vec::new(),
            in_ctor: self.fns.last().is_some_and(|c| c.in_ctor),
            this_name: self.fns.last().map_or("self", |c| c.this_name),
            type_params: Vec::new(),
            bounds: Default::default(),
            class: self.fns.last().and_then(|c| c.class.clone()),
            labels: 0,
        });
        let body = match &f.body {
            FnBody::Expr(x) => {
                let r = self.expr_to(x, &ret_ty);
                self.fns.pop();
                self.pop_scope();
                let code = r?;
                if code.starts_with("if ") || code.starts_with("match ") {
                    format!("{{ {code} }}")
                } else {
                    code
                }
            }
            FnBody::Block(b) => {
                let text = self.capture(|g| {
                    g.indent += 1;
                    g.block_body(&b.stmts, true)?;
                    g.indent -= 1;
                    Ok(())
                });
                self.fns.pop();
                self.pop_scope();
                let text = text?;
                let mut s = String::from("{\n");
                s.push_str(&text);
                for _ in 0..self.indent {
                    s.push_str("    ");
                }
                s.push('}');
                s
            }
        };
        let m = if info.escapes { "move " } else { "" };
        Ok(E::owned(
            format!("{m}|{}| {body}", params.join(", ")),
            PREC_ASSIGN,
        ))
    }
}

const PREC_CAST_PLUS: u8 = super::PREC_CAST + 1;

/// The expression tested by a narrowing test: `x` in `x !== null`, or `x` itself.
pub(super) fn narrow_path_expr(e: &Expr) -> &Expr {
    match &e.kind {
        ExprKind::Binary {
            op: BinOp::InstanceOf,
            left,
            ..
        } => left,
        ExprKind::Binary {
            op: BinOp::In,
            right,
            ..
        } => right,
        ExprKind::Binary { left, right, .. } => {
            // `typeof x === "string"`, `x.kind === "circle"`, `x !== null`.
            let (value, other) = if matches!(
                right.kind,
                ExprKind::Null | ExprKind::Undefined | ExprKind::Str(_)
            ) {
                (&**left, &**right)
            } else {
                (&**right, &**left)
            };
            match &value.kind {
                ExprKind::TypeOf(x) => x,
                ExprKind::Member { obj, .. } if matches!(other.kind, ExprKind::Str(_)) => obj,
                _ => value,
            }
        }
        // `isNaN(n)`, `Array.isArray(x)`
        ExprKind::Call { args, .. } if args.len() == 1 => &args[0].expr,
        _ => e,
    }
}

/// `[]`, `new Map()`, `new Set()`: what `console.log` and `String(…)` show for them (their
/// element type may be unknown).
fn empty_literal(x: &Expr) -> Option<(&'static str, &'static str)> {
    match &x.kind {
        ExprKind::Array(elems) if elems.is_empty() => Some(("[]", "")),
        ExprKind::New { callee, args, .. } if args.is_empty() => match &callee.kind {
            ExprKind::Ident(n) if n == "Map" => Some(("Map(0) {}", "[object Map]")),
            ExprKind::Ident(n) if n == "Set" => Some(("Set(0) {}", "[object Set]")),
            _ => None,
        },
        _ => None,
    }
}

fn flatten_concat<'e>(e: &'e Expr, out: &mut Vec<&'e Expr>) {
    if let ExprKind::Binary {
        op: BinOp::Add,
        left,
        right,
    } = &e.kind
    {
        flatten_concat(left, out);
        out.push(right);
    } else {
        out.push(e);
    }
}

pub(super) fn format_call(name: &str, fmt: &str, args: &[String]) -> String {
    if args.is_empty() {
        format!("{name}(\"{fmt}\")")
    } else {
        format!("{name}(\"{fmt}\", {})", args.join(", "))
    }
}

fn is_plain_ident(code: &str) -> bool {
    !code.is_empty()
        && !matches!(code, "true" | "false" | "self" | "Self")
        && code
            .chars()
            .next()
            .is_some_and(|c| c.is_alphabetic() || c == '_')
        && code.chars().all(|c| c.is_alphanumeric() || c == '_')
        && code != "self"
}

fn is_simple(code: &str) -> bool {
    code.chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '.' || c == ':' || c == '(' || c == ')')
        && !code.contains("..")
}

fn paren_unless_simple(code: &str) -> String {
    if is_simple(code) {
        code.to_string()
    } else {
        format!("({code})")
    }
}

fn paren_cmp(code: &str) -> String {
    if code.contains(" as ") || code.contains("..") {
        format!("({code})")
    } else {
        code.to_string()
    }
}

/// Parenthesizes a condition that contains the given operator at its top level.
fn paren_logical(code: &str, op: &str) -> String {
    let mut depth = 0i32;
    let bytes = code.as_bytes();
    let mut i = 0;
    while i + op.len() <= bytes.len() {
        match bytes[i] {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            _ => {}
        }
        if depth == 0 && &code[i..i + op.len()] == op {
            return format!("({code})");
        }
        i += 1;
    }
    if code.starts_with("if ") {
        format!("({code})")
    } else {
        code.to_string()
    }
}

/// A numeric literal receiving a method call needs a type suffix.
pub(super) fn num_recv(code: &str, float: bool) -> String {
    let (open, inner, close) = match code.strip_prefix("(-").and_then(|c| c.strip_suffix(')')) {
        Some(inner) => ("(-", inner, ")"),
        None => ("", code, ""),
    };
    let is_float_literal =
        inner.chars().all(|c| c.is_ascii_digit() || c == '.') && inner.contains('.');
    if super::is_int_literal(inner) {
        format!("{open}{inner}_i64{close}")
    } else if float && is_float_literal {
        format!("{open}{inner}_f64{close}")
    } else {
        code.to_string()
    }
}

fn is_int_literal_e(e: &E) -> bool {
    super::is_int_literal(&e.code)
}

fn single_char_literal(code: &str) -> Option<char> {
    let inner = code.strip_prefix('"')?.strip_suffix('"')?;
    let mut chars = inner.chars();
    let c = chars.next()?;
    if c == '\\' || chars.next().is_some() {
        return None;
    }
    Some(c)
}

/// `&String` compared with `&str`: fine; `String` with `&str`: fine; both owned strings:
/// fine. Nothing to change, kept for symmetry.
fn deref_str(code: &str) -> String {
    code.to_string()
}

fn screaming_const(name: &str) -> String {
    super::screaming(name)
}

pub(super) use super::snake;
