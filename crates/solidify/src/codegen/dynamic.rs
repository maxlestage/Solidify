//! Values known only at run time: `JsValue`, with JavaScript's operators, property
//! lookups and methods from the `Dynamic` runtime helper.

use super::exprs::{PREC_CAST_PLUS, paren_unless_simple};
use super::{
    E, GResult, Gen, Helper, PREC_ADD, PREC_ATOM, PREC_MUL, PREC_POSTFIX, PREC_UNARY, Want,
};
use crate::ast::*;
use crate::types::Ty;

impl<'a> Gen<'a> {
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

    /// A `&mut JsValue` to the place an expression names: `data.items[0]`.
    pub(super) fn dyn_place_mut(&mut self, e: &'a Expr) -> GResult<String> {
        match &e.kind {
            ExprKind::Member { obj, prop, .. } if self.is_dynamic(&self.ty_of(obj)) => {
                let o = self.dyn_place_mut(obj)?;
                Ok(format!("js_get_mut({o}, {})", super::string_literal(prop)))
            }
            ExprKind::Index { obj, index, .. } if self.is_dynamic(&self.ty_of(obj)) => {
                let o = self.dyn_place_mut(obj)?;
                let k = if self.is_str(&self.ty_of(index)) {
                    self.expr_as(index, Want::StrRef)?
                } else {
                    let k = self.dyn_ref(index)?;
                    format!("&js_key_text({k})")
                };
                Ok(format!("js_get_mut({o}, {k})"))
            }
            _ => {
                let x = self.expr_as(e, Want::RefMut)?;
                Ok(if x.starts_with("&mut ") {
                    x
                } else {
                    format!("&mut {x}")
                })
            }
        }
    }

    /// `place op= value` on a value known at run time.
    pub(super) fn dyn_assign(
        &mut self,
        op: AssignOp,
        t: &'a Expr,
        value: &'a Expr,
    ) -> GResult<String> {
        self.helper(Helper::Dynamic);
        let place = self.dyn_place_mut(t)?;
        let v = self.dyn_owned(value)?;
        Ok(match op {
            AssignOp::Assign => format!("*{place} = {v};"),
            _ => {
                let sym = assign_symbol(op);
                format!("js_op_assign({place}, \"{sym}\", {v});")
            }
        })
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
                let o = self.dyn_place_mut(obj)?;
                return Ok(E::owned(
                    format!("js_call_method_mut({o}, {name}, vec![{list}])"),
                    PREC_POSTFIX,
                ));
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
                let o = self.dyn_place_mut(obj)?;
                format!(
                    "{{ let v = {o}; if let JsValue::Array(items) = &mut *v {{ items.sort_by(|a, b| f(a.clone(), b.clone()).partial_cmp(&0.0).unwrap_or(std::cmp::Ordering::Equal)); }} v.clone() }}"
                )
            }
        };
        Ok(E::owned(format!("{{ let f = {f}; {code} }}"), PREC_ATOM))
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
