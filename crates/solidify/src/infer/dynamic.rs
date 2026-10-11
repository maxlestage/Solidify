//! Values known only at run time (`JsValue`): a `JSON.parse` result without a type, what a
//! reviver or a replacer gets. Their properties and methods are looked up when the program
//! runs, with JavaScript's rules.

use std::collections::HashSet;
use std::rc::Rc;

use super::{Callee, IResult, Infer, Role};
use crate::ast::*;
use crate::types::{FuncTy, Ty};

impl<'m> Infer<'m> {
    /// `v.name(args)` on a dynamic value.
    pub(super) fn dyn_method_call(
        &mut self,
        e: &'m Expr,
        obj: &'m Expr,
        prop: &str,
        args: &'m [Arg],
    ) -> IResult<Ty> {
        let d = Ty::dynamic();
        if matches!(
            prop,
            "push" | "pop" | "shift" | "unshift" | "reverse" | "sort" | "splice" | "fill"
        ) {
            self.mark_receiver_mutated(obj);
        }
        let full = format!("dyn.{prop}");
        let span = e.span;
        let int = self.types.int();
        let ret = match prop {
            "map" => {
                self.callback(args, 0, vec![d.clone(), int], d.clone(), &full, span)?;
                d
            }
            "filter" | "find" | "findLast" => {
                self.callback(args, 0, vec![d.clone(), int], Ty::Bool, &full, span)?;
                d
            }
            "some" | "every" => {
                self.callback(args, 0, vec![d.clone(), int], Ty::Bool, &full, span)?;
                Ty::Bool
            }
            "findIndex" | "findLastIndex" => {
                self.callback(args, 0, vec![d.clone(), int.clone()], Ty::Bool, &full, span)?;
                int
            }
            "forEach" => {
                self.callback(args, 0, vec![d.clone(), int], Ty::Void, &full, span)?;
                Ty::Void
            }
            "reduce" => {
                if args.len() > 1 {
                    self.arg_typed(args, 1, Some(d.clone()), &full, span)?;
                }
                let params = vec![d.clone(), d.clone(), int];
                self.callback(args, 0, params, d.clone(), &full, span)?;
                d
            }
            "sort" if !args.is_empty() => {
                let f = self.types.float();
                self.callback(args, 0, vec![d.clone(), d.clone()], f, &full, span)?;
                d
            }
            _ => {
                for a in args {
                    let t = self.expr(&a.expr, Some(&d))?;
                    self.dyn_cast(&a.expr, &t, &d);
                }
                d
            }
        };
        self.done(e, full, ret)
    }

    /// An object or array literal where a value known at run time is expected: built as
    /// one, each part converted.
    pub(super) fn dyn_literal(&mut self, e: &'m Expr) -> IResult<Ty> {
        let d = Ty::dynamic();
        let part = |me: &mut Self, x: &'m Expr| -> IResult<()> {
            let t = me.expr(x, Some(&d))?;
            if matches!(me.types.resolve(&t), Ty::Var(_)) {
                me.unify(&t, &d, x.span)?;
            }
            me.dyn_cast(x, &t, &d);
            Ok(())
        };
        match &e.kind {
            ExprKind::Array(elems) => {
                for el in elems {
                    match el {
                        ArrayElem::Expr(x) => part(self, x)?,
                        ArrayElem::Spread(x) => {
                            self.expr(x, None)?;
                        }
                        ArrayElem::Hole => {}
                    }
                }
            }
            ExprKind::Object(props) => {
                for p in props {
                    if let PropKey::Computed(k) = &p.key {
                        self.expr(k, None)?;
                    }
                    match &p.value {
                        PropValue::Expr(x) => part(self, x)?,
                        PropValue::Shorthand(id) => {
                            let key = p.key.name().unwrap_or_default();
                            let t = self.shorthand(&key, *id, p.span)?;
                            self.set_ty(*id, t);
                        }
                        PropValue::Spread(x) => {
                            self.expr(x, None)?;
                        }
                        PropValue::Method(_) | PropValue::Get(_) | PropValue::Set(_) => {
                            return self.err(
                                "a method in an object built at run time: declare a class or an interface for it",
                                p.span,
                            );
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(d)
    }

    /// Whether one side of a binary operator is known at run time only; a side whose type
    /// is not known yet is then one too.
    pub(super) fn dyn_operands(&mut self, left: &Expr, lt: &Ty, right: &Expr, rt: &Ty) -> bool {
        let (l, r) = (self.types.resolve(lt), self.types.resolve(rt));
        if !l.is_dynamic() && !r.is_dynamic() {
            return false;
        }
        for (x, t) in [(left, lt), (right, rt)] {
            if matches!(self.types.resolve(t), Ty::Var(_)) {
                let _ = self.unify(t, &Ty::dynamic(), x.span);
            }
        }
        true
    }

    /// `delete v.key`, `delete record[key]`, `delete obj.optionalField`.
    pub(super) fn delete(&mut self, target: &'m Expr) -> IResult<Ty> {
        let (ExprKind::Member { obj, .. } | ExprKind::Index { obj, .. }) = &target.kind else {
            return self.err("`delete` removes a property: `delete obj.key`", target.span);
        };
        let tt = self.expr(target, None)?;
        let ot = self.types.resolve(&self.expr_ty[&obj.id].clone());
        let fits = ot.is_dynamic()
            || matches!(ot, Ty::Map(..))
            || (matches!(target.kind, ExprKind::Member { .. })
                && matches!(self.types.resolve(&tt), Ty::Option(_)));
        if !fits {
            return self.err(
                "`delete` works on a property that may be missing (`key?: T`), a `Record`, or a value known at run time",
                target.span,
            );
        }
        self.mark_mutated(obj);
        Ok(Ty::Bool)
    }

    /// `eval(code)`: code written in the program is translated with it (see `desugar`);
    /// other code is evaluated when the program runs, with the variables in scope.
    pub(super) fn eval_call(&mut self, e: &'m Expr, args: &'m [Arg]) -> IResult<Ty> {
        let Some(a) = args.first() else {
            self.calls.insert(e.id, Callee::Builtin("global.eval"));
            return Ok(Ty::Null);
        };
        let t = self.expr(&a.expr, Some(&Ty::Str))?;
        match self.types.resolve(&t) {
            Ty::Str => {}
            d if d.is_dynamic() => {
                self.dyn_cast(&a.expr, &t, &Ty::Str);
            }
            // `eval(42)` is 42.
            _ => {
                self.calls.insert(e.id, Callee::Builtin("global.eval"));
                return Ok(t);
            }
        }
        let scope = self.visible_values(e);
        self.eval_scopes.insert(e.id, scope);
        self.calls.insert(e.id, Callee::Builtin("global.eval"));
        Ok(Ty::dynamic())
    }

    /// A read of every variable visible at `at` and declared before it, for code that
    /// may use any of them.
    fn visible_values(&mut self, at: &'m Expr) -> Vec<(String, Expr)> {
        let mut seen = HashSet::new();
        let mut names = Vec::new();
        for scope in self.scopes.iter().rev() {
            let mut here: Vec<(&String, &NodeId)> = scope.vars.iter().collect();
            here.sort_by_key(|(_, id)| **id);
            for (name, id) in here {
                if seen.insert(name.clone()) && *id < at.id {
                    names.push(name.clone());
                }
            }
        }
        let mut out = Vec::new();
        for name in names {
            let Some(id) = self.lookup(&name) else {
                continue;
            };
            if matches!(self.vars[&id].role, Role::Func | Role::Class | Role::Enum) {
                continue;
            }
            let read: &'m Expr = Box::leak(Box::new(Expr {
                kind: ExprKind::Ident(name.clone()),
                span: at.span,
                id: self.node_count,
            }));
            self.node_count += 1;
            if self.expr(read, None).is_ok() {
                out.push((name, read.clone()));
            }
        }
        out
    }

    /// `new Function("a", "b", body)` with a body known only at run time: a function of
    /// values known at run time.
    pub(super) fn function_call(&mut self, e: &'m Expr, args: &'m [Arg]) -> IResult<Ty> {
        let Some((body, params)) = args.split_last() else {
            return self.err("`new Function` needs a body", e.span);
        };
        for p in params {
            if !matches!(p.expr.kind, ExprKind::Str(_)) {
                return self.err(
                    "the parameter names of `new Function` are written in the program",
                    p.expr.span,
                );
            }
        }
        let t = self.expr(&body.expr, Some(&Ty::Str))?;
        if self.types.resolve(&t).is_dynamic() {
            self.dyn_cast(&body.expr, &t, &Ty::Str);
        } else {
            self.expect(&t, &Ty::Str, body.expr.span)?;
        }
        let d = Ty::dynamic();
        self.calls.insert(e.id, Callee::Builtin("new Function"));
        Ok(Ty::Func(Rc::new(FuncTy {
            params: vec![d.clone(); params.len()],
            ret: d,
        })))
    }

    /// `new Proxy(target, { get(t, key) { … }, set(t, key, value) { … } })`: a value known
    /// at run time whose reads and writes go through the traps.
    pub(super) fn new_proxy(&mut self, e: &'m Expr, args: &'m [Arg]) -> IResult<Ty> {
        let (Some(target), Some(handler)) = (args.first(), args.get(1)) else {
            return self.err("`new Proxy` needs a target and a handler", e.span);
        };
        let d = Ty::dynamic();
        let t = self.expr(&target.expr, Some(&d))?;
        if matches!(self.types.resolve(&t), Ty::Var(_)) {
            self.unify(&t, &d, target.expr.span)?;
        }
        self.dyn_cast(&target.expr, &t, &d);
        if self.types.resolve(&t).is_dynamic() {
            self.mark_mutated(&target.expr);
        }
        let ExprKind::Object(props) = &handler.expr.kind else {
            return self.err(
                "the handler of a `Proxy` is written in place: `{ get(target, key) { … } }`",
                handler.expr.span,
            );
        };
        let s = Ty::Str;
        for p in props {
            let name = p.key.name().unwrap_or_default();
            let (params, ret) = match name.as_str() {
                "get" => (vec![d.clone(), s.clone(), d.clone()], d.clone()),
                "set" => (vec![d.clone(), s.clone(), d.clone(), d.clone()], Ty::Bool),
                "has" | "deleteProperty" => (vec![d.clone(), s.clone()], Ty::Bool),
                other => {
                    return self.err(
                        format!("the `{other}` trap is not supported: use get, set, has or deleteProperty"),
                        p.span,
                    );
                }
            };
            let f = Ty::Func(Rc::new(FuncTy { params, ret }));
            // A trap lives as long as the proxy: it keeps what it uses.
            match &p.value {
                PropValue::Expr(x) => {
                    let ft = self.expr(x, Some(&f))?;
                    self.expect(&ft, &f, x.span)?;
                    self.mark_stored(x);
                }
                PropValue::Method(func) => {
                    self.closure(func, Some(&f))?;
                    if let Some(info) = self.funcs.get_mut(&func.id) {
                        info.escapes = true;
                    }
                }
                _ => return self.err("a trap is a function", p.span),
            }
        }
        self.calls.insert(e.id, Callee::Builtin("new Proxy"));
        Ok(d)
    }

    /// `Reflect.get(target, key)`, `.set`, `.has`, `.deleteProperty`, `.ownKeys`.
    pub(super) fn reflect_call(&mut self, e: &'m Expr, prop: &str, args: &'m [Arg]) -> IResult<Ty> {
        let d = Ty::dynamic();
        let full = format!("Reflect.{prop}");
        let span = e.span;
        let ret = match prop {
            "get" => d.clone(),
            "set" | "has" | "deleteProperty" => Ty::Bool,
            "ownKeys" => Ty::Array(Box::new(Ty::Str)),
            _ => return self.err(format!("`{full}` is not supported"), span),
        };
        self.arg_typed(args, 0, Some(d.clone()), &full, span)?;
        if prop != "ownKeys" {
            let k = self.arg_typed(args, 1, None, &full, span)?;
            if !matches!(self.types.resolve(&k), Ty::Str) {
                self.dyn_cast(&args[1].expr, &k, &d);
            }
        }
        if prop == "set" {
            self.arg_typed(args, 2, Some(d.clone()), &full, span)?;
        }
        if matches!(prop, "set" | "deleteProperty") {
            self.mark_mutated(&args[0].expr);
        }
        self.done(e, full, ret)
    }

    /// Two big integers (`1n + x`): `false` when neither is one. JavaScript refuses to
    /// mix them with numbers.
    pub(super) fn big_operands(
        &mut self,
        lt: &Ty,
        rt: &Ty,
        span: crate::error::Span,
    ) -> IResult<bool> {
        let big = |t: &Ty| matches!(t, Ty::Named(n, _) if n == "bigint");
        let (l, r) = (self.types.resolve(lt), self.types.resolve(rt));
        if !big(&l) && !big(&r) {
            return Ok(false);
        }
        for t in [lt, rt] {
            if matches!(self.types.resolve(t), Ty::Var(_)) {
                self.unify(t, &Ty::named("bigint"), span)?;
            }
        }
        let (l, r) = (self.types.resolve(lt), self.types.resolve(rt));
        if !(big(&l) && big(&r)) {
            return self.err(
                "Cannot mix BigInt and other types, use explicit conversions (BigInt(x), Number(x))",
                span,
            );
        }
        Ok(true)
    }
}
