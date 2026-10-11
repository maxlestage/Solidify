//! Promises: `Promise<T>` is a task started at once (the `Promise` runtime helper), and
//! its methods and statics chain more of them.

use super::{E, GResult, Gen, Helper, PREC_ATOM, PREC_POSTFIX, Repr, Want, pascal};
use crate::ast::*;
use crate::types::Ty;

impl<'a> Gen<'a> {
    /// `Promise.all`, `race`, `any`, `allSettled`, `resolve` and `reject`.
    pub(super) fn promise_static(
        &mut self,
        e: &'a Expr,
        name: &str,
        args: &'a [Arg],
    ) -> GResult<E> {
        let h = self.helper(Helper::Promise);
        let code = match name {
            "Promise.resolve" => match args.first() {
                None => format!("{h}::resolved(())"),
                Some(a) if self.is_promise(&self.ty_of(&a.expr)) => {
                    return self.expr(&a.expr);
                }
                Some(a) => format!("{h}::resolved({})", self.expr_as(&a.expr, Want::Owned)?),
            },
            "Promise.reject" => {
                let t = self.promised(&self.ty_of(e));
                let t = self.settled_ty(&t, e.span)?;
                let error = self.expr_as(&args[0].expr, Want::Owned)?;
                format!("{h}::<{t}>::rejected({error})")
            }
            _ => {
                let a = &args[0].expr;
                if let Ty::Tuple(ts) = self.resolve(&self.ty_of(a)) {
                    return self.promise_all_tuple(a, &ts);
                }
                let list = self.promise_list(a)?;
                match name {
                    "Promise.all" => format!("{h}::all({list})"),
                    "Promise.race" => format!("{h}::race({list})"),
                    "Promise.any" => format!("{h}::any({list})"),
                    _ => {
                        let s = pascal(crate::infer::SETTLED);
                        format!(
                            "{h}::all_settled({list}, |r| match r {{ \
                             Ok(v) => {s} {{ status: \"fulfilled\".to_string(), value: Some(v), reason: None }}, \
                             Err(e) => {s} {{ status: \"rejected\".to_string(), value: None, reason: Some(e) }} }})"
                        )
                    }
                }
            }
        };
        Ok(E::owned(code, PREC_POSTFIX))
    }

    /// A type for Rust, `()` where nothing pins it down (a promise only ever rejected).
    fn settled_ty(&mut self, t: &Ty, span: crate::error::Span) -> GResult<String> {
        match self.resolve(t) {
            Ty::Var(_) => Ok("()".into()),
            t => self.rust_ty(&t, span),
        }
    }

    /// The promises of an array: a value that is not one is resolved already.
    fn promise_list(&mut self, a: &'a Expr) -> GResult<String> {
        let h = self.helper(Helper::Promise);
        if let ExprKind::Array(elems) = &a.kind
            && elems.iter().all(|x| matches!(x, ArrayElem::Expr(_)))
        {
            let mut items = Vec::new();
            for x in elems {
                let ArrayElem::Expr(x) = x else {
                    unreachable!()
                };
                let code = self.expr_as(x, Want::Owned)?;
                if self.is_promise(&self.ty_of(x)) {
                    items.push(code);
                } else {
                    items.push(format!("{h}::resolved({code})"));
                }
            }
            return Ok(format!("vec![{}]", items.join(", ")));
        }
        let code = self.expr_as(a, Want::Owned)?;
        match self.resolve(&self.ty_of(a)) {
            Ty::Array(item) if !self.is_promise(&item) => Ok(format!(
                "{}.into_iter().map({h}::resolved).collect::<Vec<_>>()",
                Self::paren(&E::owned(code, PREC_ATOM), PREC_POSTFIX)
            )),
            _ => Ok(code),
        }
    }

    /// `Promise.all([a, b])` of different types: a tuple of their values.
    fn promise_all_tuple(&mut self, a: &'a Expr, ts: &[Ty]) -> GResult<E> {
        let h = self.helper(Helper::Promise);
        let code = self.expr_as(a, Want::Owned)?;
        let names: Vec<String> = (0..ts.len()).map(|i| format!("p{i}")).collect();
        let values: Vec<String> = ts
            .iter()
            .zip(&names)
            .map(|(t, n)| {
                if self.is_promise(t) {
                    format!("{n}.settled().await?")
                } else {
                    n.clone()
                }
            })
            .collect();
        Ok(E::owned(
            format!(
                "{{ let ({}) = {code}; {h}::spawn_result(async move {{ Ok(({},)) }}) }}",
                names.join(", "),
                values.join(", ")
            ),
            PREC_ATOM,
        ))
    }

    /// `p.then(f)`, `p.catch(g)`, `p.finally(h)`.
    pub(super) fn promise_method(
        &mut self,
        obj: &'a Expr,
        method: &str,
        args: &'a [Arg],
    ) -> GResult<E> {
        let item = self.promised(&self.ty_of(obj));
        let p = self.expr(obj)?;
        let p = Self::paren(&p, PREC_POSTFIX);
        let gives = |g: &Self, a: &Arg| match g.resolve(&g.ty_of(&a.expr)) {
            Ty::Func(f) => g.resolve(&f.ret),
            _ => Ty::Void,
        };
        let code = match method {
            "then" => {
                let f = self.promise_callback(&args[0].expr, 1)?;
                let ret = gives(self, &args[0]);
                match args.get(1) {
                    Some(g) => {
                        let g = self.promise_callback(&g.expr, 1)?;
                        format!("{p}.then_else({f}, {g})")
                    }
                    None if self.is_promise(&ret) => format!("{p}.then_promise({f})"),
                    None => format!("{p}.then({f})"),
                }
            }
            "catch" => {
                let g = self.promise_callback(&args[0].expr, 1)?;
                let ret = gives(self, &args[0]);
                if self.is_promise(&ret) {
                    format!("{p}.catch_promise({g})")
                } else if matches!(ret, Ty::Void) && !matches!(self.resolve(&item), Ty::Void) {
                    format!("{p}.catch_none({g})")
                } else {
                    format!("{p}.catch({g})")
                }
            }
            _ => {
                let f = self.promise_callback(&args[0].expr, 0)?;
                format!("{p}.finally({f})")
            }
        };
        Ok(E::owned(code, PREC_POSTFIX))
    }

    /// A callback run later: it owns what it uses and takes `arity` values.
    fn promise_callback(&mut self, a: &'a Expr, arity: usize) -> GResult<String> {
        let (ExprKind::Arrow(f) | ExprKind::Func(f)) = &a.kind else {
            return self.expr_as(a, Want::Owned);
        };
        let reprs = vec![Repr::Owned; f.params.len()];
        let c = self.closure(f, &reprs, false)?;
        let mut code = c.code;
        if f.params.is_empty() && arity == 1 {
            code = code.replacen("||", "|_|", 1);
        }
        if !code.starts_with("move ") && !code.starts_with('{') {
            code = format!("move {code}");
        }
        Ok(code)
    }
}
