//! Values known only at run time (`JsValue`): a `JSON.parse` result without a type, what a
//! reviver or a replacer gets. Their properties and methods are looked up when the program
//! runs, with JavaScript's rules.

use super::{IResult, Infer};
use crate::ast::*;
use crate::types::Ty;

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
}
