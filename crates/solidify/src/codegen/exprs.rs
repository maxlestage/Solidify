//! Expressions.

use super::stmts::Side;
use super::{
    E, GResult, Gen, PREC_ADD, PREC_AND, PREC_ASSIGN, PREC_ATOM, PREC_CMP, PREC_MUL, PREC_OR,
    PREC_POSTFIX, PREC_UNARY, Repr, Want, format_text, pascal, string_literal,
};
use crate::ast::*;
use crate::infer::{Callee, Role};
use crate::types::Ty;

impl<'a> Gen<'a> {
    pub(super) fn expr(&mut self, e: &'a Expr) -> GResult<E> {
        if self.p.auto_awaits.contains(&e.id) && !self.casting.contains(&e.id) {
            self.casting.insert(e.id);
            let r = self.expr(e);
            self.casting.remove(&e.id);
            return Ok(self.await_code(&r?));
        }
        if let Some((from, to)) = self.p.dyn_casts.get(&e.id).cloned()
            && !self.casting.contains(&e.id)
        {
            self.casting.insert(e.id);
            let r = self.dyn_converted(e, &from, &to);
            self.casting.remove(&e.id);
            return r;
        }
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
        if self.p.dyn_literals.contains(&e.id) {
            return self.dyn_literal(e);
        }
        match &e.kind {
            ExprKind::Num { value, raw, int } => Ok(self.number(*value, raw, *int, &ty)),
            ExprKind::BigInt(digits) => {
                let h = self.helper(super::Helper::BigInt);
                Ok(match digits.parse::<i64>() {
                    Ok(n) => E::owned(format!("{h}::from_i64({n})"), PREC_POSTFIX),
                    Err(_) => E::owned(
                        format!("{h}::parse({})", string_literal(digits)),
                        PREC_POSTFIX,
                    ),
                })
            }
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
                    if name == "self" || self.this_is_handle() {
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
            // `arr[i++]`, `return n--`: the update, then the old (or new) value.
            ExprKind::Update { inc, prefix, arg } => {
                let text = self.update_text(*inc, arg)?;
                let read = self.expr_as(arg, Want::Owned)?;
                Ok(E::atom(
                    if *prefix {
                        format!("{{ {text}; {read} }}")
                    } else {
                        format!("{{ let old = {read}; {text}; old }}")
                    },
                    Repr::Owned,
                ))
            }
            // `a = b = 0`, `f(x = 2)`: the assignment, then the value assigned.
            ExprKind::Assign {
                target: AssignTarget::Expr(t),
                ..
            } => {
                let lines = self.capture(|g| g.expr_stmt(e))?;
                let stmt = lines
                    .lines()
                    .map(str::trim)
                    .filter(|l| !l.is_empty())
                    .collect::<Vec<_>>()
                    .join(" ");
                let read = self.expr_as(t, Want::Owned)?;
                Ok(E::atom(format!("{{ {stmt} {read} }}"), Repr::Owned))
            }
            ExprKind::Assign { .. } => self.err(
                "a destructuring assignment cannot be used as a value here: write it on its own line",
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
            ExprKind::Index {
                obj,
                index,
                optional: true,
            } => self.optional_index(obj, index),
            ExprKind::Index { obj, index, .. }
                if matches!(self.p.calls.get(&e.id), Some(Callee::Builtin("enum.name"))) =>
            {
                // `Level[x]`: the member's name.
                let binding = self.p.refs[&obj.id];
                let en = pascal(&self.p.var(binding).name);
                let it = self.ty_of(index);
                if matches!(self.resolve(&it), Ty::Num(_)) {
                    let x = self.expr_as(index, Want::Owned)?;
                    let x = if self.is_float(&it) {
                        x
                    } else {
                        format!("{x} as f64")
                    };
                    Ok(E::new(
                        format!("{en}::name_of({x})"),
                        PREC_POSTFIX,
                        Repr::Str,
                    ))
                } else {
                    let x = self.expr(index)?;
                    Ok(E::new(
                        format!("{}.name()", Self::paren(&x, PREC_POSTFIX)),
                        PREC_POSTFIX,
                        Repr::Str,
                    ))
                }
            }
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
                let t = self.ty_of(inner);
                let x = self.expr(inner)?;
                // A value that was never a promise: what follows still runs later.
                if !self.is_promise(&t) {
                    let h = self.helper(super::Helper::Promise);
                    if matches!(inner.kind, ExprKind::Null | ExprKind::Undefined) {
                        return Ok(E::owned(format!("{h}::resolved(()).await"), PREC_POSTFIX));
                    }
                    return Ok(E::owned(
                        format!("{{ let v_ = {}; {h}::resolved(()).await; v_ }}", x.code),
                        PREC_ATOM,
                    ));
                }
                Ok(self.await_code(&x))
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
            ExprKind::Regex { pattern, flags } => {
                let h = self.helper(super::Helper::Regex);
                Ok(E::owned(
                    format!(
                        "{h}::new({}, {})",
                        string_literal(pattern),
                        string_literal(flags)
                    ),
                    PREC_POSTFIX,
                ))
            }
            ExprKind::Delete(target) => self.delete_expr(target),
            ExprKind::Tagged { .. }
            | ExprKind::Super
            | ExprKind::Class(_)
            | ExprKind::Yield { .. } => self.err("this expression is not supported", e.span),
        }
    }

    /// `typeof x`: known at compile time, except for a union, which asks its value.
    fn typeof_expr(&mut self, inner: &'a Expr) -> GResult<E> {
        let ty = self.ty_of(inner);
        if self.is_dynamic(&ty) {
            let x = self.dyn_ref(inner)?;
            return Ok(E::new(format!("js_typeof({x})"), PREC_POSTFIX, Repr::Str));
        }
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
                Ty::Named(n, _) if n == "Symbol" => "symbol",
                Ty::Named(n, _) if n == "bigint" => "bigint",
                _ => "object",
            }
        };
        if let Ty::Option(t) = self.resolve(&ty) {
            let x = self.recv(inner)?;
            let some = word(self, &t);
            // `typeof null` is "object", `typeof undefined` is "undefined".
            let none = if self.p.undefinedish(inner) {
                "undefined"
            } else {
                "object"
            };
            return Ok(E::owned(
                format!("if {x}.is_some() {{ \"{some}\" }} else {{ \"{none}\" }}"),
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
        let x = if want == Want::RefMut {
            self.expr_mut_root(e)?
        } else {
            self.expr(e)?
        };
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
        let name = match &e.kind {
            ExprKind::Ident(name) => name,
            // `ns.x` for `import * as ns`.
            ExprKind::Member { prop, .. } => prop,
            _ => unreachable!(),
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
                let mutable = std::mem::take(&mut self.want_mut_root);
                if let Some(&changed) = self.statics.get(&binding) {
                    let name = self.name(binding);
                    let code = if changed {
                        format!("(*{name}.{}())", if mutable { "write" } else { "read" })
                    } else {
                        format!("(*{name})")
                    };
                    return Ok(E::new(code, PREC_ATOM, Repr::Place));
                }
                if self.shared.contains(&binding) {
                    // Shared with a closure: read or written through the cell.
                    let borrow = if mutable { "borrow_mut" } else { "borrow" };
                    return Ok(E::new(
                        format!("(*{}.{borrow}())", self.name(binding)),
                        PREC_ATOM,
                        Repr::Place,
                    ));
                }
                let mut x = E::atom(self.name(binding), self.repr_of(binding));
                x.var = Some((binding, e.id));
                Ok(x)
            }
        }
    }

    /// An expression that is written through (an assignment target, a receiver of a
    /// changing method): a shared variable at its root is borrowed mutably.
    pub(super) fn expr_mut_root(&mut self, e: &'a Expr) -> GResult<E> {
        self.want_mut_root = true;
        let r = self.expr(e);
        self.want_mut_root = false;
        r
    }

    fn template(&mut self, quasis: &[String], exprs: &'a [Expr]) -> GResult<E> {
        if exprs.is_empty() {
            return Ok(E::atom(string_literal(&quasis[0]), Repr::Str));
        }
        let mut fmt = String::new();
        let mut args = Vec::new();
        let before = self.inline_names;
        self.inline_names = !exprs.iter().any(super::changes_something);
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
        let in_order = !self.inline_names;
        self.inline_names = before;
        Ok(E::owned(
            super::calls::in_order_call("format!", &fmt, &args, in_order),
            PREC_ATOM,
        ))
    }

    /// An error (its Rust value `e`) as JavaScript writes it: `Error: message`.
    pub(super) fn error_text(&mut self, e: &str) -> String {
        if self.p.error_classes.is_empty() {
            let h = self.helper(super::Helper::ErrorText);
            format!("{h}(&{e})")
        } else {
            self.helper(super::Helper::Error);
            format!("js_error_text_named(&{e})")
        }
    }

    /// `x` (a Rust expression of type `ty`) as JavaScript's `String(x)`.
    pub(super) fn string_of(&mut self, code: &str, ty: &Ty) -> String {
        if self.is_inspect_ty(ty) {
            self.inspect_call(code, ty, false)
        } else if self.is_float(ty) {
            // Called on the items of an iterator: references.
            let h = self.helper(super::Helper::NumText);
            format!("{h}(*{code})")
        } else {
            self.bound_needed(ty, "std::fmt::Display");
            format!("{code}.to_string()")
        }
    }

    /// A value shown by `console.log`: objects, arrays, maps and sets are inspected like
    /// Node does; the rest prints as in a template string.
    pub(super) fn log_arg(&mut self, x: &'a Expr) -> GResult<(String, Option<String>)> {
        let ty = self.ty_of(x);
        // A big integer is shown with its `n`.
        if self.is_big(&ty) {
            let x = self.expr(x)?;
            let code = self.coerce(x, &ty, Want::Display);
            return Ok(("{}n".into(), Some(code)));
        }
        // A value known at run time: a string is shown as it is.
        if self.is_dynamic(&ty) {
            let v = self.dyn_ref(x)?;
            return Ok(("{}".into(), Some(format!("js_show({v})"))));
        }
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
        self.elems_undefined = self.p.elements_undefinedish(x);
        let call = self.inspect_call(&code, &ty, true);
        self.elems_undefined = false;
        // A union, or a type parameter, may hold a string, shown as it is.
        let call = if self.union_of(&ty).is_some()
            || matches!(self.resolve(&ty), Ty::Named(ref n, ..) if self.is_type_param(n))
        {
            call.replacen("js_inspect(", "js_show(", 1)
        } else {
            call
        };
        Ok(("{}".into(), Some(call)))
    }

    /// `undefined` for the methods that give it, `null` otherwise.
    fn missing_word(&self, x: &Expr) -> &'static str {
        if matches!(
            self.p.calls.get(&x.id),
            Some(Callee::Builtin("global.parseInt"))
        ) {
            return "NaN";
        }
        if self.p.undefinedish(x) {
            "undefined"
        } else {
            "null"
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
            && !matches!(self.resolve(&ty), Ty::Named(ref n, ..) if self.p.find_method(n, "toString", MethodKind::Method).is_some())
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
            self.elems_undefined = self.p.elements_undefinedish(x);
            let call = self.inspect_call(&code, &ty, false);
            self.elems_undefined = false;
            return Ok(("{}".into(), Some(call)));
        }
        // An error as text: `Error: message`, like JavaScript's `String(e)`.
        if matches!(self.resolve(&ty), Ty::Named(ref n, ..) if n == "Error") {
            let code = self.coerce(e, &ty, Want::Display);
            return Ok(("{}".into(), Some(self.error_text(&code))));
        }
        // A struct with `toString()` prints through it.
        if let Ty::Named(n, ..) = self.resolve(&ty)
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
            let shown = if self.is_float(&inner) {
                let h = self.helper(super::Helper::NumText);
                format!("{h}(*v)")
            } else if inner_spec == "{}" {
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
        // A double as JavaScript prints it (`1e+21`, `Infinity`).
        if self.is_float(&ty) {
            let code = self.coerce(e, &ty, Want::Owned);
            let h = self.helper(super::Helper::NumText);
            return Ok(("{}".into(), Some(format!("{h}({code})"))));
        }
        let code = self.coerce(e, &ty, Want::Display);
        if is_plain_ident(&code) && !code.starts_with("r#") && self.inline_names {
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
                            // A borrowed array (a slice parameter): a copy.
                            Ty::Array(_) if matches!(s.repr, Repr::Ref | Repr::RefMut) => {
                                format!("{recv}.to_vec()")
                            }
                            ref other if self.struct_iteration(other).is_some() => {
                                let (_, generator) =
                                    self.struct_iteration(other).expect("iterable");
                                let source = self.iteration_source(&recv, generator);
                                format!("{source}.collect::<Vec<_>>()")
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
        // Each field: given in the literal, copied from a spread of another shape, or
        // taken from a spread of this very shape (`..base`).
        let mut given: Vec<(String, String)> = Vec::new();
        let mut copied: Vec<(String, String)> = Vec::new();
        let mut base: Option<String> = None;
        let mut prelude = String::new();
        let mut temps = 0;
        for p in props {
            match &p.value {
                PropValue::Spread(s) => {
                    let st = self.ty_of(s);
                    let x = self.expr(s)?;
                    match self.resolve(&st) {
                        Ty::Named(n, ..) if n == struct_name => {
                            base = Some(self.owned(x, &st));
                            copied.clear();
                        }
                        Ty::Named(n, ..) => {
                            let Some(src) = self.p.struct_info(&n).cloned() else {
                                return self.err("cannot spread this value", s.span);
                            };
                            // A variable is read directly; a temporary is bound first.
                            let recv = if super::is_place_kind(s) {
                                Self::paren(&x, PREC_POSTFIX)
                            } else {
                                temps += 1;
                                let v = self.owned(x, &st);
                                prelude.push_str(&format!("let spread_{temps} = {v}; "));
                                format!("spread_{temps}")
                            };
                            for f in &src.fields {
                                if info.field(&f.name).is_none() {
                                    continue;
                                }
                                let fty = if f.optional {
                                    Ty::Option(Box::new(f.ty.clone()))
                                } else {
                                    f.ty.clone()
                                };
                                let access = format!("{recv}.{}", snake(&f.name));
                                let code = if self.is_copy(&fty) {
                                    access
                                } else {
                                    format!("{access}.clone()")
                                };
                                copied.retain(|(k, _)| *k != f.name);
                                copied.push((f.name.clone(), code));
                            }
                        }
                        _ => return self.err("cannot spread this value", s.span),
                    }
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
                    let code = if val == rust_key {
                        rust_key
                    } else {
                        format!("{rust_key}: {val}")
                    };
                    copied.retain(|(k, _)| *k != key);
                    given.retain(|(k, _)| *k != key);
                    given.push((key, code));
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
                    let code = if val == rust_key {
                        rust_key
                    } else {
                        format!("{rust_key}: {val}")
                    };
                    copied.retain(|(k, _)| *k != key);
                    given.retain(|(k, _)| *k != key);
                    given.push((key, code));
                }
                _ => return self.err("object methods are not supported", p.span),
            }
        }
        let mut fields = Vec::new();
        for f in &info.fields {
            if let Some((_, code)) = given.iter().find(|(k, _)| *k == f.name) {
                fields.push(code.clone());
            } else if let Some((_, code)) = copied.iter().find(|(k, _)| *k == f.name) {
                fields.push(format!("{}: {code}", snake(&f.name)));
            } else if base.is_none() && f.optional {
                fields.push(format!("{}: None", snake(&f.name)));
            }
        }
        if let Some(b) = base {
            fields.push(format!("..{b}"));
        }
        // A field sharing its collection with a variable: the variable's cell.
        let fields: Vec<String> = fields
            .into_iter()
            .map(|f| {
                let (key, val) = match f.split_once(": ") {
                    Some((k, v)) => (k.to_string(), v.to_string()),
                    None => (f.clone(), f.clone()),
                };
                let Some(info_field) = info.fields.iter().find(|x| snake(&x.name) == key) else {
                    return f;
                };
                if !self.p.is_shared_field(&struct_name, &info_field.name) {
                    return f;
                }
                self.uses_rc = true;
                let source = props.iter().find_map(|p| {
                    if p.key.name().as_deref() != Some(info_field.name.as_str()) {
                        return None;
                    }
                    match &p.value {
                        PropValue::Expr(x) => self.p.refs.get(&x.id).copied(),
                        PropValue::Shorthand(id) => self.p.refs.get(id).copied(),
                        _ => None,
                    }
                });
                match source {
                    Some(b) if self.shared.contains(&b) => {
                        format!("{key}: {}.clone()", self.name(b))
                    }
                    _ => format!("{key}: Rc::new(RefCell::new({val}))"),
                }
            })
            .collect();
        let name = pascal(&struct_name);
        let literal = if fields.is_empty() {
            format!("{name} {{}}")
        } else {
            format!("{name} {{ {} }}", fields.join(", "))
        };
        let literal = if !prelude.is_empty() {
            format!("{{ {prelude}{literal} }}")
        } else {
            literal
        };
        if self.p.is_shared(&struct_name) && !self.p.is_dyn(&struct_name) {
            let h = self.helper(super::Helper::Shared);
            return Ok(E::owned(format!("{h}::new({literal})"), PREC_POSTFIX));
        }
        // A shape held dynamically (classes stand in for it): boxed.
        if self.p.is_dyn(&struct_name) {
            let args = match self.ty_of(e) {
                Ty::Named(_, args) => args,
                _ => Vec::new(),
            };
            let g = self.dyn_generics(&struct_name, &args, true, e.span)?;
            if self.p.is_shared(&struct_name) {
                let h = self.helper(super::Helper::Shared);
                return Ok(E::owned(
                    format!(
                        "{h}(std::rc::Rc::new(std::cell::RefCell::new({literal})) as std::rc::Rc<std::cell::RefCell<dyn {name}Like{g}>>)"
                    ),
                    PREC_POSTFIX,
                ));
            }
            return Ok(E::owned(
                format!("Box::new({literal}) as Box<dyn {name}Like{g}>"),
                super::PREC_CAST,
            ));
        }
        Ok(E::atom(literal, Repr::Owned))
    }

    fn unary(&mut self, op: UnaryOp, arg: &'a Expr) -> GResult<E> {
        match op {
            UnaryOp::Not => {
                let c = self.cond(arg)?;
                let c = if c.contains(' ') { format!("({c})") } else { c };
                Ok(E::owned(format!("!{c}"), PREC_UNARY))
            }
            UnaryOp::Neg if self.is_big(&self.ty_of(arg)) => {
                let x = self.expr(arg)?;
                Ok(E::owned(
                    format!("{}.negated()", Self::paren(&x, PREC_POSTFIX)),
                    PREC_POSTFIX,
                ))
            }
            UnaryOp::Neg | UnaryOp::Pos if self.is_dynamic(&self.ty_of(arg)) => {
                let x = self.float_operand(arg)?;
                Ok(if op == UnaryOp::Neg {
                    E::owned(format!("-{x}"), PREC_UNARY)
                } else {
                    E::owned(x, PREC_POSTFIX)
                })
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
                if let Some(v) = const_number(arg) {
                    return Ok(E::atom(int_literal((!js_to_int32(v)) as i64), Repr::Owned));
                }
                let x = self.int32_operand(arg)?;
                Ok(E::owned(format!("(!{x}) as i64"), PREC_CAST_PLUS))
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
        // `later - earlier` on dates: milliseconds.
        if op == BinOp::Sub && matches!(self.resolve(&lt), Ty::Named(ref n, ..) if n == "Date") {
            let l = self.expr(left)?;
            let r = self.expr(right)?;
            return Ok(E::owned(
                format!(
                    "{}.get_time() - {}.get_time()",
                    Self::paren(&l, PREC_POSTFIX),
                    Self::paren(&r, PREC_POSTFIX)
                ),
                PREC_ADD,
            ));
        }
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
        if self.is_dynamic(&lt) || self.is_dynamic(&rt) {
            return self.dyn_binary(op, left, right);
        }
        if self.is_big(&lt) || self.is_big(&rt) {
            return self.big_binary(op, left, right);
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
                // A double, as in JavaScript.
                if let (Some(a), Some(b)) = (const_number(left), const_number(right)) {
                    let v = a.powf(b);
                    if v.fract() == 0.0 && v.abs() < 9007199254740992.0 {
                        return Ok(E::atom(format!("{v:.1}"), Repr::Owned));
                    }
                }
                let lc = self.float_operand(left)?;
                let rc = self.float_operand(right)?;
                Ok(E::owned(
                    format!("{}.powf({rc})", num_recv(&lc, true)),
                    PREC_POSTFIX,
                ))
            }
            BinOp::Shl
            | BinOp::Shr
            | BinOp::UShr
            | BinOp::BitAnd
            | BinOp::BitOr
            | BinOp::BitXor => {
                // 32-bit integers, as in JavaScript (`1 << 31` is negative).
                if let (Some(a), Some(b)) = (const_number(left), const_number(right)) {
                    return Ok(E::atom(int_literal(js_bitwise(op, a, b)), Repr::Owned));
                }
                let a = self.int32_operand(left)?;
                let b = self.int32_operand(right)?;
                Ok(E::owned(self.bitwise_code(op, &a, &b), PREC_CAST_PLUS))
            }
            BinOp::Lt | BinOp::Gt | BinOp::LtEq | BinOp::GtEq => {
                if let Ty::Named(n, ..) = self.resolve(&lt)
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
                let sub = match (self.p.refs.get(&right.id), &right.kind) {
                    (Some(b), _) => self.p.var(*b).name.clone(),
                    (None, ExprKind::Ident(sub)) => sub.clone(),
                    _ => return self.err("`instanceof` needs a class name", right.span),
                };
                let sub = &sub;
                let Ty::Named(class, ..) = self.resolve(&lt) else {
                    self.expr(left)?;
                    return Ok(E::atom("false", Repr::Owned));
                };
                if class == "Error" && self.p.is_error_class(sub) {
                    self.helper(super::Helper::Error);
                    let x = self.expr(left)?;
                    let x = self.coerce(x, &lt, Want::Ref);
                    return Ok(E::owned(
                        format!("js_error_is({x}, \"{sub}\")"),
                        PREC_POSTFIX,
                    ));
                }
                if self.p.is_subclass(&class, sub) {
                    self.expr(left)?;
                    return Ok(E::atom("true", Repr::Owned));
                }
                if self.p.is_dyn(&class) && self.p.is_subclass(sub, &class) {
                    self.instance_tests
                        .insert((class.clone(), sub.clone(), false));
                    let x = self.recv(left)?;
                    let x = if self.p.is_shared(&class) {
                        format!("{x}.borrow()")
                    } else {
                        x
                    };
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

    /// A number as a double (`n as f64`), for `**` and `Math.pow`.
    pub(super) fn float_operand(&mut self, e: &'a Expr) -> GResult<String> {
        if let Some(v) = const_number(e) {
            return Ok(format!("{v:?}"));
        }
        let t = self.ty_of(e);
        if self.is_dynamic(&t) {
            let x = self.expr(e)?;
            return Ok(format!("{}.to_number()", Self::paren(&x, PREC_POSTFIX)));
        }
        let x = self.expr(e)?;
        let owned = self.coerce(x, &t, Want::Owned);
        if self.is_float(&t) {
            return Ok(owned);
        }
        let x = E::owned(owned, PREC_POSTFIX);
        Ok(self.as_f64(&x))
    }

    /// A number as JavaScript's bitwise operators see it: a 32-bit integer (`x as i32`;
    /// a double goes through `js_to_int32`).
    pub(super) fn int32_operand(&mut self, e: &'a Expr) -> GResult<String> {
        if let Some(v) = const_number(e) {
            return Ok(js_to_int32(v).to_string());
        }
        let t = self.ty_of(e);
        if self.is_dynamic(&t) {
            let x = self.float_operand(e)?;
            let h = self.helper(super::Helper::ToInt32);
            return Ok(format!("{h}({x})"));
        }
        let x = self.expr(e)?;
        let prec = x.prec;
        let owned = self.coerce(x, &t, Want::Owned);
        if self.is_float(&t) {
            let h = self.helper(super::Helper::ToInt32);
            return Ok(format!("{h}({owned})"));
        }
        let x = E::owned(owned, prec);
        Ok(format!("({} as i32)", Self::paren(&x, PREC_CAST_PLUS)))
    }

    /// `a op b` on 32-bit integers, as an `i64`.
    pub(super) fn bitwise_code(&self, op: BinOp, a: &str, b: &str) -> String {
        match op {
            BinOp::Shl => format!("{a}.wrapping_shl({b} as u32) as i64"),
            BinOp::Shr => format!("{a}.wrapping_shr({b} as u32) as i64"),
            BinOp::UShr => format!("({a} as u32).wrapping_shr({b} as u32) as i64"),
            BinOp::BitAnd => format!("({a} & {b}) as i64"),
            BinOp::BitOr => format!("({a} | {b}) as i64"),
            _ => format!("({a} ^ {b}) as i64"),
        }
    }

    pub(super) fn as_f64(&mut self, x: &E) -> String {
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
        // A nullable value compared with a value: the value is `Some` of it.
        let rt_r = self.resolve(rt);
        match (&lt, &rt_r) {
            (Ty::Option(_), Ty::Option(_) | Ty::Null) | (Ty::Null, _) | (_, Ty::Null) => {}
            (Ty::Option(_), _) => {
                let lc = self.coerce(l, &lt, Want::Ref);
                let rc = self.coerce(r, &rt_r, Want::Ref);
                return (format!("({lc}).as_ref()"), format!("Some({rc})"));
            }
            (_, Ty::Option(_)) => {
                let lc = self.coerce(l, &lt, Want::Ref);
                let rc = self.coerce(r, &rt_r, Want::Ref);
                return (format!("Some({lc})"), format!("({rc}).as_ref()"));
            }
            _ => {}
        }
        if self.is_copy(&lt) {
            let lc = self.coerce(l, &lt, Want::Owned);
            let rc = self.coerce(r, rt, Want::Owned);
            return (paren_cmp(&lc), paren_cmp(&rc));
        }
        self.bound_needed(&lt, "PartialEq");
        if let Ty::Named(n, ..) = &lt
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
        // `v ?? d`, `v || d` on a value known at run time.
        if self.is_dynamic(&lt) && op != LogicalOp::And {
            let l = self.expr_as(left, Want::Owned)?;
            let r = self.expr_as(right, Want::Owned)?;
            let keep = if op == LogicalOp::Nullish {
                "!v.is_nullish()"
            } else {
                "v.truthy()"
            };
            return Ok(E::owned(
                format!("{{ let v = {l}; if {keep} {{ v }} else {{ {r} }} }}"),
                PREC_ATOM,
            ));
        }
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
                    ref d if d.is_dynamic() => {
                        format!("{}.truthy()", Self::paren(&x, PREC_POSTFIX))
                    }
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
                Ty::Named(b, ..) => b,
                _ => String::new(),
            };
            self.instance_tests
                .insert((base.clone(), sub.clone(), mutated));
            let recv = Self::paren(&x, PREC_POSTFIX);
            if base == "Error" {
                // A caught error of the program's own class: read back.
                self.helper(super::Helper::Error);
                let sub_ty = Ty::named(sub.clone());
                let _ = self.mark_json(&sub_ty, Default::default());
                let rt = self
                    .rust_ty(&sub_ty, Default::default())
                    .unwrap_or_default();
                (
                    format!("js_error_as::<{rt}>(&{recv}, \"{sub}\")"),
                    Repr::Owned,
                )
            } else if self.p.is_shared(&base) {
                // The subclass's handle: the guard is dropped before the block.
                (
                    format!(
                        "{{ let o = {recv}.borrow().as_{}_shared(&{recv}); o }}",
                        snake(sub)
                    ),
                    Repr::Owned,
                )
            } else if mutated {
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
        let mutable = self.want_mut_root;
        // `ns.x` for `import * as ns`: the name itself.
        if self.p.refs.contains_key(&e.id) {
            return self.ident(e);
        }
        if !self.p.calls.contains_key(&e.id) && self.is_dynamic(&self.ty_of(obj)) {
            return self.dyn_member(obj, prop, optional);
        }
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
                    "Number.MAX_SAFE_INTEGER" if self.is_float(&ty) => "9007199254740991.0",
                    "Number.MIN_SAFE_INTEGER" if self.is_float(&ty) => "-9007199254740991.0",
                    "Number.MAX_SAFE_INTEGER" => "9007199254740991_i64",
                    "Number.MIN_SAFE_INTEGER" => "-9007199254740991_i64",
                    "Number.MAX_VALUE" => "f64::MAX",
                    "Number.MIN_VALUE" => "5e-324",
                    "Number.EPSILON" => "f64::EPSILON",
                    "Number.POSITIVE_INFINITY" => "f64::INFINITY",
                    "Number.NEGATIVE_INFINITY" => "f64::NEG_INFINITY",
                    "Number.NaN" => "f64::NAN",
                    "process.argv" => "std::env::args().collect::<Vec<String>>()",
                    "process.platform" => {
                        self.helper(super::Helper::Node);
                        "js_node_os_info(\"platform\")"
                    }
                    "process.env" => {
                        let map = self.map_name();
                        return Ok(E::owned(
                            format!(
                                "{{ let mut m = {map}::new(); for (k, v) in std::env::vars() {{ m.insert(k, v); }} m }}"
                            ),
                            PREC_ATOM,
                        ));
                    }
                    _ => return self.err(format!("`{name}` is not supported"), e.span),
                },
                Repr::Owned,
            ));
        }
        let ot = self.ty_of(obj);
        // Enum variants and static members (`E.A`, `C.x`, also through a namespace).
        if let Some(&binding) = self.p.refs.get(&obj.id)
            && matches!(self.p.var(binding).role, Role::Enum | Role::Class)
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
            Ty::Named(n, ..) if n == "RegExp" => Ok(match prop {
                "source" => E::owned(format!("{base}.source()"), PREC_POSTFIX),
                "flags" => E::owned(format!("{base}.flags()"), PREC_POSTFIX),
                "lastIndex" => E::owned(format!("{base}.last_index()"), PREC_POSTFIX),
                _ => {
                    let flag = match prop {
                        "global" => 'g',
                        "ignoreCase" => 'i',
                        "multiline" => 'm',
                        "dotAll" => 's',
                        "sticky" => 'y',
                        _ => 'u',
                    };
                    E::owned(format!("{base}.flags().contains('{flag}')"), PREC_POSTFIX)
                }
            }),
            Ty::Named(n, ..) if n == "Symbol" => {
                Ok(E::owned(format!("{base}.description()"), PREC_POSTFIX))
            }
            Ty::Named(n, ..) if n == "RegExpMatch" => Ok(match prop {
                "index" => E::new(format!("{base}.index"), PREC_POSTFIX, Repr::Place),
                "input" => E::new(format!("{base}.input"), PREC_POSTFIX, Repr::Place),
                _ => E::new(format!("{base}.len()"), PREC_POSTFIX, Repr::Usize),
            }),
            Ty::Named(n, ..) if n == "Error" && !self.p.error_classes.is_empty() => {
                self.helper(super::Helper::Error);
                let code = match prop {
                    "message" => format!("js_error_message(&{base})"),
                    "name" => format!("js_error_name(&{base})"),
                    _ => "String::new()".to_string(),
                };
                Ok(E::owned(code, PREC_POSTFIX))
            }
            Ty::Named(n, ..) if n == "Error" => Ok(match prop {
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
            Ty::Named(n, ..) if self.p.union_info(&n).is_some() => {
                self.union_accessors
                    .insert((n.clone(), prop.to_string(), false));
                Ok(E::new(
                    format!("{base}.{}()", snake(prop)),
                    PREC_POSTFIX,
                    Repr::Ref,
                ))
            }
            Ty::Named(..) if self.constrained(&ot).is_some() => {
                // `T extends HasId`: the field through the constraint's trait.
                Ok(E::new(
                    format!("{base}.{}()", snake(prop)),
                    PREC_POSTFIX,
                    Repr::Ref,
                ))
            }
            Ty::Named(n, ..) => {
                let info = self.p.struct_info(&n).expect("struct");
                if info.field(prop).is_some() {
                    // Through a `dyn` value, a field is read with its accessor.
                    if self.p.is_dyn(&n) && !matches!(obj.kind, ExprKind::This) {
                        if self.p.is_shared(&n) {
                            let field = snake(prop);
                            let code = if mutable {
                                return Ok(E::new(
                                    format!("(*{base}.borrow_mut().{field}_mut())"),
                                    PREC_ATOM,
                                    Repr::Place,
                                ));
                            } else if self.is_copy(&ty) {
                                format!("*{base}.borrow().{field}()")
                            } else {
                                format!("{base}.borrow().{field}().clone()")
                            };
                            return Ok(E::owned(code, PREC_UNARY));
                        }
                        return Ok(E::new(
                            format!("{base}.{}()", snake(prop)),
                            PREC_POSTFIX,
                            Repr::Ref,
                        ));
                    }
                    // A field sharing its collection with a variable: through the cell.
                    if self.p.is_shared_field(&n, prop) {
                        let holder = if self.through_handle(obj, &n) {
                            format!("{base}.borrow()")
                        } else {
                            base.clone()
                        };
                        let borrow = if mutable { "borrow_mut" } else { "borrow" };
                        return Ok(E::new(
                            format!("(*{holder}.{}.{borrow}())", snake(prop)),
                            PREC_ATOM,
                            Repr::Place,
                        ));
                    }
                    // A shared object: read through its cell (a copy), written in it.
                    if self.through_handle(obj, &n) {
                        let field = snake(prop);
                        if mutable {
                            return Ok(E::new(
                                format!("{base}.borrow_mut().{field}"),
                                PREC_POSTFIX,
                                Repr::Place,
                            ));
                        }
                        let code = if self.is_copy(&ty) {
                            format!("{base}.borrow().{field}")
                        } else {
                            format!("{base}.borrow().{field}.clone()")
                        };
                        return Ok(E::owned(code, PREC_POSTFIX));
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
                    let base =
                        if self.p.is_shared_hierarchy(&n) && !matches!(obj.kind, ExprKind::This) {
                            format!("{base}.borrow()")
                        } else {
                            base
                        };
                    return Ok(E::owned(format!("{base}.{}()", snake(prop)), PREC_POSTFIX));
                }
                self.err(format!("unknown property `{prop}`"), e.span)
            }
            Ty::Array(_) | Ty::Tuple(_) if prop == "length" => {
                Ok(E::new(format!("{base}.len()"), PREC_POSTFIX, Repr::Usize))
            }
            // `record.key`: `record["key"]`.
            Ty::Map(_, v, true) => {
                let key = string_literal(prop);
                if self.is_dynamic(&v) && !mutable {
                    return Ok(E::owned(
                        format!("{base}.get({key}).cloned().unwrap_or(JsValue::Undefined)"),
                        PREC_POSTFIX,
                    ));
                }
                Ok(E::new(format!("{base}[{key}]"), PREC_POSTFIX, Repr::Place))
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
    /// An `Option` value as `map` and `if let` can read it: moved when it can be, borrowed
    /// with `as_ref()` otherwise.
    pub(super) fn option_source(&self, o: &E) -> String {
        let is_param_ref = self.opt_ref_param(o);
        match o.repr {
            Repr::OptRef => o.code.clone(),
            Repr::Owned if is_param_ref || self.can_move(o) => o.code.clone(),
            Repr::Owned | Repr::Place | Repr::Ref | Repr::RefMut => {
                format!("{}.as_ref()", Self::paren(o, PREC_POSTFIX))
            }
            _ => o.code.clone(),
        }
    }

    fn optional_member(&mut self, obj: &'a Expr, prop: &str, ot: &Ty, ty: &Ty) -> GResult<E> {
        let o = self.expr(obj)?;
        let source = self.option_source(&o);
        let Ty::Option(inner) = self.resolve(ot) else {
            return self.err("`?.` on a value that is never null", obj.span);
        };
        let field_ty = match self.resolve(&inner) {
            // `e?.message`, `e?.name`.
            Ty::Named(n, ..) if n == "Error" => {
                let code = if !self.p.error_classes.is_empty() {
                    self.helper(super::Helper::Error);
                    match prop {
                        "message" => "js_error_message(v)",
                        "name" => "js_error_name(v)",
                        _ => "String::new()",
                    }
                } else {
                    match prop {
                        "message" => "v.clone()",
                        "name" => "\"Error\".to_string()",
                        _ => "String::new()",
                    }
                };
                return Ok(E::owned(format!("{source}.map(|v| {code})"), PREC_POSTFIX));
            }
            Ty::Named(n, ..) => {
                let Some(info) = self.p.struct_info(&n) else {
                    return self.err(format!("unsupported property `{prop}`"), obj.span);
                };
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
        let access = if self.holds_shared(&inner) {
            format!("{v}.borrow().{}", snake(prop))
        } else {
            format!("{v}.{}", snake(prop))
        };
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

    /// `a?.[i]`: `a.as_ref().map(|v| v[i].clone())`.
    fn optional_index(&mut self, obj: &'a Expr, index: &'a Expr) -> GResult<E> {
        let ot = self.ty_of(obj);
        let o = self.expr(obj)?;
        let is_param_ref = self.opt_ref_param(&o);
        let source = match o.repr {
            Repr::OptRef => o.code.clone(),
            Repr::Owned if is_param_ref || self.can_move(&o) => o.code.clone(),
            _ => format!("{}.as_ref()", Self::paren(&o, PREC_POSTFIX)),
        };
        let Ty::Option(inner) = self.resolve(&ot) else {
            return self.err("`?.[` on a value that is never null", obj.span);
        };
        Ok(match self.resolve(&inner) {
            Ty::Named(n, ..) if n == "RegExpMatch" => {
                let i = self.expr_as(index, Want::Owned)?;
                E::owned(format!("{source}.map(|v| v.get({i}))"), PREC_POSTFIX)
            }
            Ty::Array(item) => {
                let i = self.expr_as(index, Want::Index)?;
                let get = if self.is_copy(&item) {
                    format!("{source}.and_then(|v| v.get({i}).copied())")
                } else {
                    format!("{source}.and_then(|v| v.get({i}).cloned())")
                };
                E::owned(get, PREC_POSTFIX)
            }
            Ty::Map(k, v, _) => {
                let key = self.expr_as(
                    index,
                    if self.is_str(&k) {
                        Want::StrRef
                    } else {
                        Want::Ref
                    },
                )?;
                let get = if self.is_copy(&v) {
                    format!("{source}.and_then(|m| m.get({key}).copied())")
                } else {
                    format!("{source}.and_then(|m| m.get({key}).cloned())")
                };
                E::owned(get, PREC_POSTFIX)
            }
            _ => return self.err("`?.[` is only supported on arrays and maps", obj.span),
        })
    }

    fn index(&mut self, e: &'a Expr, obj: &'a Expr, index: &'a Expr) -> GResult<E> {
        let ot = self.ty_of(obj);
        if self.is_dynamic(&ot) {
            return self.dyn_index(obj, index);
        }
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
            Ty::Map(k, v, _) => {
                let key = self.expr_as(
                    index,
                    if self.is_str(&k) {
                        Want::StrRef
                    } else {
                        Want::Ref
                    },
                )?;
                // A value known at run time: `undefined` when missing.
                if self.is_dynamic(&v) && !self.want_mut_root {
                    return Ok(E::owned(
                        format!("{base}.get({key}).cloned().unwrap_or(JsValue::Undefined)"),
                        PREC_POSTFIX,
                    ));
                }
                Ok(E::new(format!("{base}[{key}]"), PREC_POSTFIX, Repr::Place))
            }
            Ty::Named(n, ..) if n == "RegExpMatch" => {
                let i = self.expr_as(index, Want::Owned)?;
                Ok(E::owned(format!("{base}.get({i})"), PREC_POSTFIX))
            }
            Ty::Named(..) => {
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
        // `({ x, y }) => …`: the parameter is taken apart at the start of the body.
        let mut prelude: Vec<String> = Vec::new();
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
                let lines = self.capture(|g| match &p.pat {
                    Pattern::Object { props, rest, .. } => {
                        g.destructure_object_rest(props, rest, &name, &pty)
                    }
                    Pattern::Array { elems, rest, .. } => {
                        g.destructure_array(elems, rest, &name, &pty)
                    }
                    Pattern::Ident { .. } => Ok(()),
                })?;
                prelude.extend(
                    lines
                        .lines()
                        .map(str::trim)
                        .filter(|l| !l.is_empty())
                        .map(str::to_string),
                );
            }
        }
        let ret_ty = if info.is_async {
            self.promised(&info.sig.ret)
        } else {
            info.sig.ret.clone()
        };
        self.fns.push(super::FnGen {
            id: f.id,
            returns_result: info.is_async && info.throws,
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
            handle_this: self.fns.last().is_some_and(|c| c.handle_this),
        });
        let body = match &f.body {
            FnBody::Expr(x) if matches!(self.resolve(&ret_ty), Ty::Void) => {
                // The value of the expression is dropped: a statement.
                let xt = self.ty_of(x);
                let plain_call = matches!(self.resolve(&xt), Ty::Void)
                    && matches!(
                        x.kind,
                        ExprKind::Call { .. } | ExprKind::New { .. } | ExprKind::Await(_)
                    );
                let r = if plain_call {
                    self.expr(x).map(|e| e.code)
                } else {
                    self.capture(|g| g.expr_stmt(x))
                        .map(|text| format!("{{ {} }}", text.trim()))
                };
                self.fns.pop();
                self.pop_scope();
                r?
            }
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
        let body = if prelude.is_empty() {
            body
        } else if let Some(rest) = body.strip_prefix("{\n") {
            let pad = "    ".repeat(self.indent + 1);
            let mut s = String::from("{\n");
            for l in &prelude {
                s.push_str(&pad);
                s.push_str(l);
                s.push('\n');
            }
            s.push_str(rest);
            s
        } else {
            let inner = body
                .strip_prefix("{ ")
                .and_then(|b| b.strip_suffix(" }"))
                .unwrap_or(&body);
            format!("{{ {} {inner} }}", prelude.join(" "))
        };
        // An async closure: each call starts a task, which owns copies of what it uses.
        let body = if info.is_async {
            let h = self.helper(super::Helper::Promise);
            let spawn = if info.throws { "spawn_result" } else { "spawn" };
            let block = if body.starts_with('{') {
                body
            } else {
                format!("{{ {body} }}")
            };
            let block = if info.throws {
                match block.strip_suffix('}') {
                    Some(b) if matches!(f.body, FnBody::Expr(_)) => {
                        let inner = b.trim_start_matches('{').trim();
                        format!("{{ Ok({inner}) }}")
                    }
                    _ => block,
                }
            } else {
                block
            };
            let copies: String = info
                .captures
                .iter()
                .filter(|c| self.p.vars.contains_key(c) && !self.statics.contains_key(c))
                .map(|c| {
                    let n = self.name(*c);
                    format!("let {n} = {n}.clone(); ")
                })
                .collect();
            format!("{{ {copies}{h}::{spawn}(async move {block}) }}")
        } else {
            body
        };
        let m = if info.escapes || info.is_async {
            "move "
        } else {
            ""
        };
        let code = format!("{m}|{}| {body}", params.join(", "));
        // A kept closure takes its own handle on each shared variable.
        let clones: Vec<String> = if info.escapes || info.is_async {
            info.captures
                .iter()
                .filter(|c| self.shared.contains(c))
                .map(|c| {
                    let n = self.name(*c);
                    format!("let {n} = {n}.clone(); ")
                })
                .collect()
        } else {
            Vec::new()
        };
        if clones.is_empty() {
            return Ok(E::owned(code, PREC_ASSIGN));
        }
        Ok(E::owned(
            format!("{{ {}{code} }}", clones.join("")),
            PREC_ATOM,
        ))
    }
}

pub(super) const PREC_CAST_PLUS: u8 = super::PREC_CAST + 1;

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
                // `(m = re.exec(s)) !== null`: the test binds the value assigned.
                ExprKind::Assign { value: rhs, .. } => rhs,
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

pub(super) fn paren_unless_simple(code: &str) -> String {
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

/// The value of a numeric literal, possibly negated.
pub(super) fn const_number(e: &Expr) -> Option<f64> {
    match &e.kind {
        ExprKind::Num { value, .. } => Some(*value),
        ExprKind::Unary {
            op: UnaryOp::Neg,
            arg,
        } => const_number(arg).map(|v| -v),
        _ => None,
    }
}

/// JavaScript's `ToInt32`.
pub(super) fn js_to_int32(v: f64) -> i32 {
    if !v.is_finite() {
        return 0;
    }
    (v.trunc().rem_euclid(4294967296.0) as u32) as i32
}

/// A bitwise operator on two constants, as JavaScript computes it.
fn js_bitwise(op: BinOp, a: f64, b: f64) -> i64 {
    let (a, b) = (js_to_int32(a), js_to_int32(b));
    match op {
        BinOp::Shl => a.wrapping_shl(b as u32) as i64,
        BinOp::Shr => a.wrapping_shr(b as u32) as i64,
        BinOp::UShr => (a as u32).wrapping_shr(b as u32) as i64,
        BinOp::BitAnd => (a & b) as i64,
        BinOp::BitOr => (a | b) as i64,
        _ => (a ^ b) as i64,
    }
}

/// An integer literal; `_i64` when it does not fit the `i32` Rust assumes.
fn int_literal(v: i64) -> String {
    if i32::try_from(v).is_ok() {
        v.to_string()
    } else {
        format!("{v}_i64")
    }
}
