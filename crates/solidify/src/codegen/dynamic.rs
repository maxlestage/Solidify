//! Values known only at run time: `JsValue`, with JavaScript's operators, property
//! lookups and methods from the `Dynamic` runtime helper.

use super::exprs::{PREC_CAST_PLUS, paren_unless_simple};
use super::{
    E, GResult, Gen, Helper, PREC_ADD, PREC_ATOM, PREC_MUL, PREC_POSTFIX, PREC_UNARY, Want,
};
use crate::ast::*;
use crate::types::Ty;

impl<'a> Gen<'a> {
    /// Whether `this` is the handle of a shared object (in its methods).
    pub(super) fn this_is_handle(&self) -> bool {
        self.fns.last().is_some_and(|f| {
            f.handle_this
                || (!f.in_ctor
                    && f.this_name == "self"
                    && f.class.as_ref().is_some_and(|c| self.p.is_shared(c)))
        })
    }

    /// Whether `obj` (of struct `n`) is reached through a shared handle.
    pub(super) fn through_handle(&self, obj: &Expr, n: &str) -> bool {
        self.p.is_shared(n) && (!matches!(obj.kind, ExprKind::This) || self.this_is_handle())
    }

    /// A shared object, or maybe one: held by a handle, cloned to be kept.
    pub(super) fn holds_shared(&self, ty: &Ty) -> bool {
        match self.resolve(ty) {
            Ty::Named(n, _) => self.p.is_shared(&n),
            Ty::Option(t) => self.holds_shared(&t),
            _ => false,
        }
    }

    pub(super) fn is_dynamic(&self, ty: &Ty) -> bool {
        self.resolve(ty).is_dynamic()
    }

    /// A value converted at the boundary between a type known when translating and a
    /// value known at run time.
    pub(super) fn dyn_converted(&mut self, e: &'a Expr, from: &Ty, to: &Ty) -> GResult<E> {
        self.helper(Helper::Dynamic);
        if self.is_dynamic(to) {
            let code = self.dyn_owned(e)?;
            return Ok(E::owned(code, PREC_POSTFIX));
        }
        let x = self.expr(e)?;
        let x = self.coerce(x, from, Want::Display);
        Ok(self.read_dyn(&x, to, e.span))
    }

    /// A `JsValue` (`code`, a value or a reference) read as the type `to`.
    pub(super) fn read_dyn(&mut self, code: &str, to: &Ty, span: crate::error::Span) -> E {
        let recv = if is_simple_path(code) {
            code.to_string()
        } else {
            format!("({code})")
        };
        match self.resolve(to) {
            Ty::Str => E::owned(format!("js_to_string(&{recv})"), PREC_POSTFIX),
            Ty::Bool => E::owned(format!("{recv}.truthy()"), PREC_POSTFIX),
            Ty::Num(_) if self.is_float(to) => {
                E::owned(format!("{recv}.to_number()"), PREC_POSTFIX)
            }
            Ty::Num(_) => E::owned(format!("{recv}.to_number() as i64"), PREC_CAST_PLUS - 1),
            _ => {
                if self.mark_json(to, span).is_err() {
                    return E::owned(format!("{recv}.clone()"), PREC_POSTFIX);
                }
                let rt = self.rust_ty(to, span).unwrap_or_default();
                E::owned(format!("js_into::<{rt}>(&{recv})"), PREC_POSTFIX)
            }
        }
    }

    /// An owned `JsValue` of any expression.
    pub(super) fn dyn_owned(&mut self, e: &'a Expr) -> GResult<String> {
        self.helper(Helper::Dynamic);
        let ty = self.ty_of(e);
        if self.is_dynamic(&ty) {
            return self.expr_as(e, Want::Owned);
        }
        let mut inner = e;
        while let ExprKind::As(x, _) = &inner.kind {
            inner = x;
        }
        match &inner.kind {
            ExprKind::Object(props) if props.is_empty() => {
                return Ok("JsValue::Object(None, Vec::new())".into());
            }
            ExprKind::Array(items) if items.is_empty() => {
                return Ok("JsValue::Array(Vec::new())".into());
            }
            _ => {}
        }
        Ok(match &e.kind {
            ExprKind::Undefined => "JsValue::Undefined".into(),
            ExprKind::Null => "JsValue::Null".into(),
            _ if matches!(self.resolve(&ty), Ty::Num(_)) && !self.is_float(&ty) => {
                format!("JsValue::Int({})", self.expr_as(e, Want::Owned)?)
            }
            _ if self.is_float(&ty) => {
                let x = self.float_operand(e)?;
                format!("JsValue::number({x})")
            }
            _ => self.json_value_of(e)?,
        })
    }

    /// An object or array literal built as a `JsValue`.
    pub(super) fn dyn_literal(&mut self, e: &'a Expr) -> GResult<E> {
        self.helper(Helper::Dynamic);
        let mut lines = Vec::new();
        let code = match &e.kind {
            ExprKind::Array(elems) => {
                for el in elems {
                    match el {
                        ArrayElem::Expr(x) => {
                            let v = self.dyn_owned(x)?;
                            lines.push(format!("a.push({v});"));
                        }
                        ArrayElem::Spread(x) => {
                            let v = self.dyn_ref(x)?;
                            lines.push(format!("a.extend(js_items({v}));"));
                        }
                        ArrayElem::Hole => lines.push("a.push(JsValue::Undefined);".into()),
                    }
                }
                if lines.is_empty() {
                    return Ok(E::owned("JsValue::Array(Vec::new())", PREC_POSTFIX));
                }
                format!(
                    "{{ let mut a: Vec<JsValue> = Vec::new(); {} JsValue::Array(a) }}",
                    lines.join(" ")
                )
            }
            ExprKind::Object(props) => {
                for p in props {
                    let key = match &p.key {
                        PropKey::Computed(k) => {
                            let k = self.dyn_ref(k)?;
                            format!("&js_key_text({k})")
                        }
                        other => super::string_literal(&other.name().unwrap_or_default()),
                    };
                    match &p.value {
                        PropValue::Expr(x) => {
                            let v = self.dyn_owned(x)?;
                            lines.push(format!("js_set(&mut o, {key}, {v});"));
                        }
                        PropValue::Shorthand(id) => {
                            let binding = self.p.refs[id];
                            let ty = self.p.var(binding).ty.clone();
                            let name = self.name(binding);
                            let v = if self.is_dynamic(&ty) {
                                format!("{name}.clone()")
                            } else {
                                self.js_value_of(&name, &ty)
                            };
                            lines.push(format!("js_set(&mut o, {key}, {v});"));
                        }
                        PropValue::Spread(x) => {
                            let v = self.dyn_ref(x)?;
                            lines.push(format!(
                                "for (k, v) in js_entries({v}) {{ js_set(&mut o, &k, v); }}"
                            ));
                        }
                        _ => return self.err("a method in an object built at run time", p.span),
                    }
                }
                format!(
                    "{{ let mut o = JsValue::Object(None, Vec::new()); {} o }}",
                    lines.join(" ")
                )
            }
            _ => unreachable!("a literal"),
        };
        Ok(E::owned(code, PREC_ATOM))
    }

    /// A reference to a `JsValue` of any expression.
    pub(super) fn dyn_ref(&mut self, e: &'a Expr) -> GResult<String> {
        let ty = self.ty_of(e);
        if self.is_dynamic(&ty) {
            let x = self.expr(e)?;
            return Ok(format!("&{}", Self::paren(&x, PREC_UNARY)));
        }
        Ok(format!("&{}", self.dyn_owned(e)?))
    }

    /// A callback, its parameters annotated when it is written in place.
    pub(super) fn callback_code(&mut self, e: &'a Expr) -> GResult<String> {
        match &e.kind {
            ExprKind::Arrow(f) | ExprKind::Func(f) => Ok(self.closure(f, &[], true)?.code),
            _ => self.expr_as(e, Want::Owned),
        }
    }

    /// The variable at the root of a path into values known at run time
    /// (`&mut data`), and the keys down from it (`items`, `0`).
    pub(super) fn dyn_path(&mut self, e: &'a Expr) -> GResult<(String, Vec<String>)> {
        match &e.kind {
            ExprKind::Member { obj, prop, .. } if self.is_dynamic(&self.ty_of(obj)) => {
                let (root, mut keys) = self.dyn_path(obj)?;
                keys.push(format!("{}.to_string()", super::string_literal(prop)));
                Ok((root, keys))
            }
            ExprKind::Index { obj, index, .. } if self.is_dynamic(&self.ty_of(obj)) => {
                let (root, mut keys) = self.dyn_path(obj)?;
                let k = if self.is_str(&self.ty_of(index)) {
                    self.expr_as(index, Want::Owned)?
                } else {
                    let k = self.dyn_ref(index)?;
                    format!("js_key_text({k})")
                };
                keys.push(k);
                Ok((root, keys))
            }
            _ => {
                let x = self.expr_as(e, Want::RefMut)?;
                Ok((
                    if x.starts_with("&mut ") {
                        x
                    } else {
                        format!("&mut {x}")
                    },
                    Vec::new(),
                ))
            }
        }
    }

    /// `body` run on the place `e` names (`o` in it), its keys and the values in `lets`
    /// computed first.
    pub(super) fn dyn_with_mut(
        &mut self,
        e: &'a Expr,
        lets: &[(&str, String)],
        body: &str,
    ) -> GResult<String> {
        self.helper(Helper::Dynamic);
        let (root, keys) = self.dyn_path(e)?;
        let mut pre: Vec<String> = Vec::new();
        if !keys.is_empty() {
            pre.push(format!("let path = [{}];", keys.join(", ")));
        }
        for (name, code) in lets {
            pre.push(format!("let {name} = {code};"));
        }
        let call = if keys.is_empty() {
            format!("{{ let o = {root}; {body} }}")
        } else {
            format!("js_with_mut({root}, &path, move |o| {body})")
        };
        Ok(if pre.is_empty() {
            call
        } else {
            format!("{{ {} {call} }}", pre.join(" "))
        })
    }

    /// `place op= value` on a value known at run time.
    pub(super) fn dyn_assign(
        &mut self,
        op: AssignOp,
        t: &'a Expr,
        value: &'a Expr,
    ) -> GResult<String> {
        self.helper(Helper::Dynamic);
        let v = self.dyn_owned(value)?;
        let (obj, key) = match &t.kind {
            ExprKind::Member { obj, prop, .. } if self.is_dynamic(&self.ty_of(obj)) => {
                (obj, super::string_literal(prop))
            }
            ExprKind::Index { obj, index, .. } if self.is_dynamic(&self.ty_of(obj)) => {
                let k = if self.is_str(&self.ty_of(index)) {
                    self.expr_as(index, Want::Owned)?
                } else {
                    let k = self.dyn_ref(index)?;
                    format!("js_key_text({k})")
                };
                (obj, k)
            }
            // A variable holding a value known at run time.
            _ => {
                let sym = assign_symbol(op);
                let place =
                    self.dyn_with_mut(t, &[("v", v)], &format!("js_op_assign(o, \"{sym}\", v)"))?;
                return Ok(format!("{place};"));
            }
        };
        let body = match op {
            AssignOp::Assign => "js_set(o, &key, v)".to_string(),
            _ => format!("js_op_assign_prop(o, &key, \"{}\", v)", assign_symbol(op)),
        };
        let code = self.dyn_with_mut(obj, &[("key", key), ("v", v)], &body)?;
        Ok(format!("{code};"))
    }

    /// `x++` on a value known at run time: its new value.
    pub(super) fn dyn_update(&mut self, inc: bool, arg: &'a Expr) -> GResult<String> {
        let sym = if inc { "+" } else { "-" };
        match &arg.kind {
            ExprKind::Member { obj, prop, .. } if self.is_dynamic(&self.ty_of(obj)) => {
                let key = format!("{}.to_string()", super::string_literal(prop));
                self.dyn_with_mut(
                    obj,
                    &[("key", key)],
                    &format!("js_op_assign_prop(o, &key, \"{sym}\", JsValue::Int(1))"),
                )
            }
            ExprKind::Index { obj, index, .. } if self.is_dynamic(&self.ty_of(obj)) => {
                let k = self.dyn_ref(index)?;
                let key = format!("js_key_text({k})");
                self.dyn_with_mut(
                    obj,
                    &[("key", key)],
                    &format!("js_op_assign_prop(o, &key, \"{sym}\", JsValue::Int(1))"),
                )
            }
            _ => self.dyn_with_mut(
                arg,
                &[],
                &format!("js_op_assign(o, \"{sym}\", JsValue::Int(1))"),
            ),
        }
    }

    /// `delete v.key`.
    pub(super) fn dyn_delete(&mut self, target: &'a Expr) -> GResult<String> {
        let (obj, key) = match &target.kind {
            ExprKind::Member { obj, prop, .. } => {
                (obj, format!("{}.to_string()", super::string_literal(prop)))
            }
            ExprKind::Index { obj, index, .. } => {
                let k = self.dyn_ref(index)?;
                (obj, format!("js_key_text({k})"))
            }
            _ => return self.err("`delete` needs a property", target.span),
        };
        self.dyn_with_mut(obj, &[("key", key)], "js_delete(o, &key)")
    }

    /// `v.prop`, `v?.prop`.
    pub(super) fn dyn_member(&mut self, obj: &'a Expr, prop: &str, optional: bool) -> GResult<E> {
        self.helper(Helper::Dynamic);
        let o = self.dyn_ref(obj)?;
        let key = super::string_literal(prop);
        Ok(if optional {
            E::owned(
                format!(
                    "{{ let o = {o}; if o.is_nullish() {{ JsValue::Undefined }} else {{ js_get(o, {key}) }} }}"
                ),
                PREC_ATOM,
            )
        } else {
            E::owned(format!("js_get({o}, {key})"), PREC_POSTFIX)
        })
    }

    /// `v[key]`.
    pub(super) fn dyn_index(&mut self, obj: &'a Expr, index: &'a Expr) -> GResult<E> {
        self.helper(Helper::Dynamic);
        let o = self.dyn_ref(obj)?;
        let kt = self.ty_of(index);
        let code = if self.is_str(&kt) {
            let k = self.expr_as(index, Want::StrRef)?;
            format!("js_get({o}, {k})")
        } else {
            let k = self.dyn_ref(index)?;
            format!("js_index({o}, {k})")
        };
        Ok(E::owned(code, PREC_POSTFIX))
    }

    /// `a op b` with a value known at run time on one side.
    pub(super) fn dyn_binary(&mut self, op: BinOp, left: &'a Expr, right: &'a Expr) -> GResult<E> {
        self.helper(Helper::Dynamic);
        let arith = |g: &mut Self, sym: &str, prec: u8| -> GResult<E> {
            let a = g.float_operand(left)?;
            let b = g.float_operand(right)?;
            let (a, b) = (paren_unless_simple(&a), paren_unless_simple(&b));
            Ok(E::owned(format!("{a} {sym} {b}"), prec))
        };
        Ok(match op {
            BinOp::Add => {
                let (a, b) = (self.dyn_ref(left)?, self.dyn_ref(right)?);
                E::owned(format!("js_op_add({a}, {b})"), PREC_POSTFIX)
            }
            BinOp::Sub => arith(self, "-", PREC_ADD)?,
            BinOp::Mul => arith(self, "*", PREC_MUL)?,
            BinOp::Div => arith(self, "/", PREC_MUL)?,
            BinOp::Mod => arith(self, "%", PREC_MUL)?,
            BinOp::Pow => {
                let a = self.float_operand(left)?;
                let b = self.float_operand(right)?;
                let a = paren_unless_simple(&a);
                E::owned(format!("{a}.powf({b})"), PREC_POSTFIX)
            }
            BinOp::Lt | BinOp::Gt | BinOp::LtEq | BinOp::GtEq => {
                let sym = match op {
                    BinOp::Lt => "<",
                    BinOp::Gt => ">",
                    BinOp::LtEq => "<=",
                    _ => ">=",
                };
                let (a, b) = (self.dyn_ref(left)?, self.dyn_ref(right)?);
                E::owned(format!("js_op_cmp(\"{sym}\", {a}, {b})"), PREC_POSTFIX)
            }
            BinOp::StrictEq | BinOp::StrictNotEq | BinOp::Eq | BinOp::NotEq => {
                let f = if matches!(op, BinOp::StrictEq | BinOp::StrictNotEq) {
                    "js_strict_eq"
                } else {
                    "js_loose_eq"
                };
                let (a, b) = (self.dyn_ref(left)?, self.dyn_ref(right)?);
                let call = format!("{f}({a}, {b})");
                if matches!(op, BinOp::StrictNotEq | BinOp::NotEq) {
                    E::owned(format!("!{call}"), PREC_UNARY)
                } else {
                    E::owned(call, PREC_POSTFIX)
                }
            }
            BinOp::In => {
                let o = self.dyn_ref(right)?;
                let k = self.dyn_ref(left)?;
                E::owned(format!("js_has({o}, &js_key_text({k}))"), PREC_POSTFIX)
            }
            _ => {
                let a = self.int32_operand(left)?;
                let b = self.int32_operand(right)?;
                E::owned(self.bitwise_code(op, &a, &b), PREC_CAST_PLUS - 1)
            }
        })
    }

    /// `v.name(args)` on a value known at run time.
    pub(super) fn dyn_method(
        &mut self,
        obj: &'a Expr,
        method: &str,
        args: &'a [Arg],
    ) -> GResult<E> {
        self.helper(Helper::Dynamic);
        let callbacks = matches!(
            method,
            "map"
                | "filter"
                | "find"
                | "findLast"
                | "some"
                | "every"
                | "findIndex"
                | "findLastIndex"
                | "forEach"
                | "reduce"
        ) || (method == "sort" && !args.is_empty());
        if !callbacks {
            let mut list = Vec::new();
            for a in args {
                list.push(self.dyn_owned(&a.expr)?);
            }
            let name = super::string_literal(method);
            let list = list.join(", ");
            if is_mutating(method) {
                let code = self.dyn_with_mut(
                    obj,
                    &[("args", format!("vec![{list}]"))],
                    &format!("js_call_method_mut(o, {name}, args)"),
                )?;
                return Ok(E::owned(code, PREC_ATOM));
            }
            let o = self.dyn_ref(obj)?;
            return Ok(E::owned(
                format!("js_call_method({o}, {name}, vec![{list}])"),
                PREC_POSTFIX,
            ));
        }
        let o = self.dyn_ref(obj)?;
        let f = self.callback_code(&args[0].expr)?;
        let arity = callback_arity(&args[0].expr, &self.ty_of(&args[0].expr));
        // The callback called with the first `arity` of these arguments.
        let call = |given: &[&str]| -> String {
            let used: Vec<&str> = given.iter().take(arity).copied().collect();
            format!("f({})", used.join(", "))
        };
        let items = format!("js_items({o}).into_iter().enumerate()");
        let code = match method {
            "map" => format!(
                "JsValue::Array({items}.map(|(i, x)| {}).collect())",
                call(&["x", "i as i64"])
            ),
            "filter" => format!(
                "JsValue::Array({items}.filter(|(i, x)| {}).map(|(_, x)| x).collect())",
                call(&["x.clone()", "*i as i64"])
            ),
            "find" => format!(
                "{items}.find(|(i, x)| {}).map_or(JsValue::Undefined, |(_, x)| x)",
                call(&["x.clone()", "*i as i64"])
            ),
            "findLast" => format!(
                "{items}.rev().find(|(i, x)| {}).map_or(JsValue::Undefined, |(_, x)| x)",
                call(&["x.clone()", "*i as i64"])
            ),
            "findIndex" => format!(
                "{items}.find(|(i, x)| {}).map_or(-1, |(i, _)| i as i64)",
                call(&["x.clone()", "*i as i64"])
            ),
            "findLastIndex" => format!(
                "{items}.rev().find(|(i, x)| {}).map_or(-1, |(i, _)| i as i64)",
                call(&["x.clone()", "*i as i64"])
            ),
            "some" => format!("{items}.any(|(i, x)| {})", call(&["x", "i as i64"])),
            "every" => format!("{items}.all(|(i, x)| {})", call(&["x", "i as i64"])),
            "forEach" => format!("for (i, x) in {items} {{ {}; }}", call(&["x", "i as i64"])),
            "reduce" => {
                let step = call(&["acc", "x", "i as i64"]);
                match args.get(1) {
                    Some(init) => {
                        let init = self.dyn_owned(&init.expr)?;
                        format!("{items}.fold({init}, |acc, (i, x)| {step})")
                    }
                    None => format!(
                        "{{ let mut it = {items}; let first = it.next().map(|(_, x)| x).expect(\"TypeError: Reduce of empty array with no initial value\"); it.fold(first, |acc, (i, x)| {step}) }}"
                    ),
                }
            }
            _ => {
                // `sort(compare)`: in place, like JavaScript.
                self.dyn_with_mut(obj, &[], "js_sort_by(o, &f)")?
            }
        };
        Ok(E::owned(format!("{{ let f = {f}; {code} }}"), PREC_ATOM))
    }

    /// `delete target`: always `true`, like JavaScript.
    pub(super) fn delete_expr(&mut self, target: &'a Expr) -> GResult<E> {
        let (ExprKind::Member { obj, .. } | ExprKind::Index { obj, .. }) = &target.kind else {
            return self.err("`delete` needs a property", target.span);
        };
        let ot = self.ty_of(obj);
        if self.is_dynamic(&ot) {
            let code = self.dyn_delete(target)?;
            return Ok(E::owned(code, PREC_ATOM));
        }
        match (&target.kind, self.resolve(&ot)) {
            (ExprKind::Index { index, .. }, Ty::Map(k, ..)) => {
                let m = self.expr_as(obj, Want::RefMut)?;
                let m = m.trim_start_matches("&mut ").to_string();
                let key = self.expr_to(index, &k)?;
                Ok(E::owned(
                    format!("{{ {m}.remove(&{key}); true }}"),
                    PREC_ATOM,
                ))
            }
            _ => {
                let place = self.expr_as(target, Want::RefMut)?;
                let place = place.trim_start_matches("&mut ").to_string();
                Ok(E::owned(format!("{{ {place} = None; true }}"), PREC_ATOM))
            }
        }
    }

    /// `eval(code)` with code known at run time: evaluated with the variables in scope.
    pub(super) fn eval_code(&mut self, e: &'a Expr, args: &'a [Arg]) -> GResult<E> {
        let Some(a) = args.first() else {
            return Ok(E::atom("None", super::Repr::Owned));
        };
        if !self.p.eval_scopes.contains_key(&e.id) {
            // `eval(42)`: the value itself.
            return self.expr(&a.expr);
        }
        let code = self.expr_as(&a.expr, Want::StrRef)?;
        let p = self.p;
        let mut entries = Vec::new();
        for (name, x) in &p.eval_scopes[&e.id] {
            let t = self.ty_of(x);
            if matches!(
                self.resolve(&t),
                Ty::Func(_) | Ty::Var(_) | Ty::Void | Ty::Never | Ty::Any | Ty::Gen(_)
            ) {
                continue;
            }
            // Read, not moved: the code may be run again.
            let read = self.expr(x)?;
            let read = self.coerce(read, &t, Want::Display);
            let v = if self.is_dynamic(&t) {
                format!("{}.clone()", super::exprs::paren_unless_simple(&read))
            } else {
                self.js_value_of(&read, &t)
            };
            entries.push(format!("({}, {v})", super::string_literal(name)));
        }
        let h = self.helper(Helper::Eval);
        Ok(E::owned(
            format!("{h}({code}, vec![{}])", entries.join(", ")),
            PREC_POSTFIX,
        ))
    }

    /// `new Function("a", "b", body)` with a body known at run time.
    pub(super) fn new_function(&mut self, args: &'a [Arg]) -> GResult<E> {
        let h = self.helper(Helper::Eval);
        let (body, params) = args.split_last().expect("a body");
        let names: Vec<String> = params
            .iter()
            .map(|p| match &p.expr.kind {
                ExprKind::Str(s) => s.clone(),
                _ => String::new(),
            })
            .collect();
        let body = self.expr_as(&body.expr, Want::Owned)?;
        let typed: Vec<String> = (0..names.len()).map(|i| format!("a{i}: JsValue")).collect();
        let scope: Vec<String> = names
            .iter()
            .enumerate()
            .map(|(i, n)| format!("({}, a{i})", super::string_literal(n)))
            .collect();
        self.uses_rc = true;
        Ok(E::owned(
            format!(
                "{{ let body: String = {body}; Rc::new(move |{}| {h}(&body, vec![{}])) }}",
                typed.join(", "),
                scope.join(", ")
            ),
            PREC_ATOM,
        ))
    }

    /// `new Proxy(target, handler)`.
    pub(super) fn new_proxy(&mut self, args: &'a [Arg]) -> GResult<E> {
        self.helper(Helper::Dynamic);
        let target = &args[0].expr;
        // A variable holding a value known at run time is shared with the proxy.
        let target_code = if self.is_dynamic(&self.ty_of(target))
            && matches!(
                target.kind,
                ExprKind::Ident(_) | ExprKind::Member { .. } | ExprKind::Index { .. }
            ) {
            self.dyn_with_mut(target, &[], "js_share(o)")?
        } else {
            let v = self.dyn_owned(target)?;
            format!("JsValue::Ref(std::rc::Rc::new(std::cell::RefCell::new({v})))")
        };
        let ExprKind::Object(props) = &args[1].expr.kind else {
            return self.err(
                "the handler of a `Proxy` is written in place",
                args[1].expr.span,
            );
        };
        let mut traps = [
            ("get", "None".to_string()),
            ("set", "None".to_string()),
            ("has", "None".to_string()),
            ("delete", "None".to_string()),
        ];
        for p in props {
            let name = p.key.name().unwrap_or_default();
            let (f, arity) = match &p.value {
                PropValue::Expr(x) => {
                    let ty = self.ty_of(x);
                    (
                        self.callback_code(x)?,
                        callback_arity(x, &self.resolve(&ty)),
                    )
                }
                PropValue::Method(func) => (self.closure(func, &[], true)?.code, func.params.len()),
                _ => continue,
            };
            let (slot, params, given) = match name.as_str() {
                "get" => (
                    "get",
                    "t: JsValue, k: String, r: JsValue",
                    vec!["t", "k", "r"],
                ),
                "set" => (
                    "set",
                    "t: JsValue, k: String, v: JsValue, r: JsValue",
                    vec!["t", "k", "v", "r"],
                ),
                "has" => ("has", "t: JsValue, k: String", vec!["t", "k"]),
                _ => ("delete", "t: JsValue, k: String", vec!["t", "k"]),
            };
            let used: Vec<&str> = given.iter().take(arity).copied().collect();
            let unused: Vec<String> = given
                .iter()
                .skip(arity)
                .map(|n| format!("let _ = {n};"))
                .collect();
            let code = format!(
                "Some(Box::new({{ let f = {f}; move |{params}| {{ {} f({}) }} }}))",
                unused.join(" "),
                used.join(", ")
            );
            if let Some(t) = traps.iter_mut().find(|(n, _)| *n == slot) {
                t.1 = code;
            }
        }
        let fields: Vec<String> = traps.iter().map(|(n, c)| format!("{n}: {c}")).collect();
        Ok(E::owned(
            format!(
                "JsValue::Proxy(std::rc::Rc::new(JsProxy {{ target: {target_code}, {} }}))",
                fields.join(", ")
            ),
            PREC_POSTFIX,
        ))
    }

    /// `Reflect.get(target, key)` and the others.
    pub(super) fn reflect_code(&mut self, name: &str, args: &'a [Arg]) -> GResult<E> {
        self.helper(Helper::Dynamic);
        let key = |g: &mut Self| -> GResult<String> {
            let k = &args[1].expr;
            if g.is_str(&g.ty_of(k)) {
                g.expr_as(k, Want::StrRef)
            } else {
                let k = g.dyn_ref(k)?;
                Ok(format!("&js_key_text({k})"))
            }
        };
        let code = match name {
            "Reflect.get" => {
                let t = self.dyn_ref(&args[0].expr)?;
                let k = key(self)?;
                format!("js_get({t}, {k})")
            }
            "Reflect.has" => {
                let t = self.dyn_ref(&args[0].expr)?;
                let k = key(self)?;
                format!("js_has({t}, {k})")
            }
            "Reflect.ownKeys" => {
                let t = self.dyn_ref(&args[0].expr)?;
                format!("js_keys({t})")
            }
            "Reflect.set" => {
                let k = key(self)?;
                let v = self.dyn_owned(&args[2].expr)?;
                self.dyn_with_mut(
                    &args[0].expr,
                    &[("key", format!("({k}).to_string()")), ("v", v)],
                    "{ js_set(o, &key, v); true }",
                )?
            }
            _ => {
                let k = key(self)?;
                self.dyn_with_mut(
                    &args[0].expr,
                    &[("key", format!("({k}).to_string()"))],
                    "js_delete(o, &key)",
                )?
            }
        };
        Ok(E::owned(code, PREC_ATOM))
    }
}

/// The operator of a compound assignment.
fn assign_symbol(op: AssignOp) -> &'static str {
    match op {
        AssignOp::Nullish => "??",
        AssignOp::Or => "||",
        AssignOp::And => "&&",
        _ => match op.binary() {
            Some(BinOp::Add) => "+",
            Some(BinOp::Sub) => "-",
            Some(BinOp::Mul) => "*",
            Some(BinOp::Div) => "/",
            Some(BinOp::Mod) => "%",
            Some(BinOp::Pow) => "**",
            Some(BinOp::Shl) => "<<",
            Some(BinOp::Shr) => ">>",
            Some(BinOp::UShr) => ">>>",
            Some(BinOp::BitAnd) => "&",
            Some(BinOp::BitOr) => "|",
            _ => "^",
        },
    }
}

/// How many arguments a callback takes.
fn callback_arity(e: &Expr, ty: &Ty) -> usize {
    match &e.kind {
        ExprKind::Arrow(f) | ExprKind::Func(f) => f.params.len(),
        _ => match ty {
            Ty::Func(f) => f.params.len(),
            _ => 1,
        },
    }
}

/// The methods that change an array in place.
fn is_mutating(method: &str) -> bool {
    matches!(
        method,
        "push" | "pop" | "shift" | "unshift" | "reverse" | "sort" | "splice" | "fill"
    )
}

fn is_simple_path(code: &str) -> bool {
    !code.is_empty()
        && code
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '.' || c == '&')
}
