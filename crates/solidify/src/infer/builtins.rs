//! The standard library that is understood: `console`, `Math`, `Object`, `Array`,
//! `Number`, `JSON`, and the methods of strings, arrays, maps, sets and numbers.

use std::rc::Rc;

use super::finish::leak;
use super::{Callee, IResult, Infer};
use crate::ast::*;
use crate::error::Span;
use crate::types::{Flavor, FuncTy, Ty};

impl<'m> Infer<'m> {
    /// Types a callback argument against the signature it must have.
    pub(super) fn callback(
        &mut self,
        args: &'m [Arg],
        i: usize,
        params: Vec<Ty>,
        ret: Ty,
        name: &str,
        span: Span,
    ) -> IResult<()> {
        let f = Ty::Func(Rc::new(FuncTy { params, ret }));
        self.arg_typed(args, i, Some(f), name, span)?;
        Ok(())
    }

    /// Whether an argument is a regular expression.
    fn is_regex(&mut self, arg: Option<&'m Arg>) -> bool {
        let Some(a) = arg else { return false };
        match &a.expr.kind {
            ExprKind::Regex { .. } => true,
            ExprKind::New { callee, .. } => {
                matches!(&callee.kind, ExprKind::Ident(n) if n == "RegExp")
            }
            ExprKind::Ident(name) => self
                .lookup(name)
                .map(|id| self.vars[&id].ty.clone())
                .is_some_and(
                    |t| matches!(self.types.resolve(&t), Ty::Named(n, ..) if n == "RegExp"),
                ),
            _ => false,
        }
    }

    pub(super) fn done(&mut self, e: &Expr, name: String, ty: Ty) -> IResult<Ty> {
        self.calls.insert(e.id, Callee::Builtin(leak(name)));
        Ok(ty)
    }

    pub(super) fn no_more_than(
        &self,
        args: &[Arg],
        n: usize,
        name: &str,
        span: Span,
    ) -> IResult<()> {
        if args.len() > n {
            return self.err(
                format!(
                    "`{name}` takes at most {n} argument{}",
                    if n == 1 { "" } else { "s" }
                ),
                span,
            );
        }
        Ok(())
    }

    /// `console.log(…)`, `Math.floor(…)`, `Object.keys(…)`…
    pub(super) fn namespace_call(
        &mut self,
        e: &'m Expr,
        ns: &str,
        prop: &str,
        args: &'m [Arg],
    ) -> IResult<Ty> {
        let full = format!("{ns}.{prop}");
        let span = e.span;
        match (ns, prop) {
            // `Symbol.for("key")`: one symbol per key, `Symbol.keyFor(s)`: its key.
            ("Symbol", "for") => {
                self.arg_typed(args, 0, Some(Ty::Str), &full, span)?;
                self.no_more_than(args, 1, &full, span)?;
                self.done(e, full, Ty::named("Symbol"))
            }
            ("Symbol", "keyFor") => {
                self.arg_typed(args, 0, Some(Ty::named("Symbol")), &full, span)?;
                self.no_more_than(args, 1, &full, span)?;
                self.done(e, full, Ty::Option(Box::new(Ty::Str)))
            }
            ("console", "log" | "error" | "warn" | "info" | "debug") => {
                for a in args {
                    if a.spread {
                        return self.err("spread arguments are not supported here", a.expr.span);
                    }
                    let t = self.expr(&a.expr, None)?;
                    if let Ty::Var(_) = self.types.resolve(&t) {
                        return self.err(
                            "the type of this value is not known yet: add an annotation",
                            a.expr.span,
                        );
                    }
                }
                self.done(e, full, Ty::Void)
            }
            ("console", _) => self.err(format!("`console.{prop}` is not supported"), span),
            ("Math", "floor" | "ceil" | "round" | "trunc" | "sign") => {
                let n = self.types.num(Flavor::Unknown);
                self.arg_typed(args, 0, Some(n), &full, span)?;
                self.no_more_than(args, 1, &full, span)?;
                let i = self.types.int();
                self.done(e, full, i)
            }
            ("Math", "abs") => {
                let n = self.types.num(Flavor::Unknown);
                self.arg_typed(args, 0, Some(n.clone()), &full, span)?;
                self.no_more_than(args, 1, &full, span)?;
                self.done(e, full, n)
            }
            (
                "Math",
                "sqrt" | "cbrt" | "sin" | "cos" | "tan" | "asin" | "acos" | "atan" | "sinh"
                | "cosh" | "tanh" | "exp" | "log" | "log2" | "log10" | "fround",
            ) => {
                let n = self.types.num(Flavor::Unknown);
                self.arg_typed(args, 0, Some(n), &full, span)?;
                self.no_more_than(args, 1, &full, span)?;
                let f = self.types.float();
                self.done(e, full, f)
            }
            ("Math", "atan2" | "hypot") => {
                let f = self.types.float();
                for i in 0..args.len() {
                    let n = self.types.num(Flavor::Unknown);
                    self.arg_typed(args, i, Some(n), &full, span)?;
                }
                if args.len() < 2 {
                    return self.err(format!("`{full}` needs two arguments"), span);
                }
                self.done(e, full, f)
            }
            ("Math", "pow") => {
                // A double, as `**`: `Math.pow(2, 64)` does not overflow.
                for i in 0..2 {
                    let n = self.types.num(Flavor::Unknown);
                    self.arg_typed(args, i, Some(n), &full, span)?;
                }
                self.no_more_than(args, 2, &full, span)?;
                let f = self.types.float();
                self.done(e, full, f)
            }
            ("Math", "max" | "min") => {
                let n = self.types.num(Flavor::Unknown);
                if args.len() == 1 && args[0].spread {
                    let arr = Ty::Array(Box::new(n.clone()));
                    let t = self.expr(&args[0].expr, Some(&arr))?;
                    self.expect(&t, &arr, args[0].expr.span)?;
                    return self.done(e, format!("{full}..."), n);
                }
                if args.is_empty() {
                    return self.err(format!("`{full}` needs arguments"), span);
                }
                for i in 0..args.len() {
                    self.arg_typed(args, i, Some(n.clone()), &full, span)?;
                }
                self.done(e, full, n)
            }
            ("Math", "random") => {
                self.no_more_than(args, 0, &full, span)?;
                let f = self.types.float();
                self.done(e, full, f)
            }
            ("Math", _) => self.err(format!("`Math.{prop}` is not supported"), span),
            ("Number", "isInteger" | "isSafeInteger") => {
                let n = self.types.num(Flavor::Unknown);
                self.arg_typed(args, 0, Some(n), &full, span)?;
                self.done(e, full, Ty::Bool)
            }
            ("Number", "isNaN" | "isFinite") => {
                self.nan_test_arg(args, &full, span)?;
                self.done(e, full, Ty::Bool)
            }
            ("Number", "parseInt") => {
                let t = self.parse_int_call(args, &full, span)?;
                self.done(e, "global.parseInt".into(), t)
            }
            ("Number", "parseFloat") => {
                self.arg_typed(args, 0, Some(Ty::Str), &full, span)?;
                let f = self.types.float();
                self.done(e, "global.parseFloat".into(), f)
            }
            ("String", "fromCharCode") => {
                let i = self.types.int();
                self.arg_typed(args, 0, Some(i), &full, span)?;
                self.done(e, full, Ty::Str)
            }
            ("Object", "keys" | "values" | "entries") => {
                let t = self.arg_typed(args, 0, None, &full, span)?;
                // An object known at run time, or an instance: its properties as values
                // known at run time.
                let resolved = self.types.resolve(&t);
                let object = resolved.is_dynamic()
                    || matches!(&resolved, Ty::Named(n, _) if self.struct_by_name(n).is_some());
                if object {
                    let d = Ty::dynamic();
                    let out = match prop {
                        "keys" => Ty::Str,
                        "values" => d,
                        _ => Ty::Tuple(vec![Ty::Str, d]),
                    };
                    return self.done(e, format!("Object.{prop}.dyn"), Ty::Array(Box::new(out)));
                }
                let Ty::Map(k, v, _) = resolved else {
                    return self.err(
                        format!("`{full}` works on a `Record` or `Map`; struct fields are known at compile time"),
                        span,
                    );
                };
                let out = match prop {
                    "keys" => Ty::Array(k),
                    "values" => Ty::Array(v),
                    _ => Ty::Array(Box::new(Ty::Tuple(vec![*k, *v]))),
                };
                self.done(e, full, out)
            }
            ("Object", _) => self.err(format!("`Object.{prop}` is not supported"), span),
            ("Array", "from") => {
                if args.is_empty() {
                    return self.err("`Array.from` needs an argument", span);
                }
                // `Array.from({ length: n }, (_, i) => …)`
                if let ExprKind::Object(props) = &args[0].expr.kind
                    && props.len() == 1
                    && props[0].key.name().as_deref() == Some("length")
                {
                    let PropValue::Expr(len) = &props[0].value else {
                        return self.err("`Array.from` needs `{ length: n }`", args[0].expr.span);
                    };
                    let i = self.types.int();
                    let lt = self.expr(len, Some(&i))?;
                    self.expect(&lt, &i, len.span)?;
                    self.set_ty(args[0].expr.id, Ty::Void);
                    let u = self.types.fresh();
                    if args.len() > 1 {
                        let i2 = self.types.int();
                        self.callback(args, 1, vec![Ty::Void, i2], u.clone(), &full, span)?;
                    } else {
                        self.unify(&u, &Ty::Void, span)?;
                    }
                    return self.done(e, "Array.from.length".into(), Ty::Array(Box::new(u)));
                }
                let t = self.arg_typed(args, 0, None, &full, span)?;
                let item = match self.types.resolve(&t) {
                    Ty::Array(t) | Ty::Set(t) | Ty::Gen(t) => *t,
                    Ty::Str => Ty::Str,
                    Ty::Map(k, v, _) => Ty::Tuple(vec![*k, *v]),
                    other => match self.struct_iteration(&other) {
                        Some(t) => t,
                        None => {
                            let d = self.describe(&other);
                            return self.err(format!("`Array.from` cannot read {d}"), span);
                        }
                    },
                };
                if args.len() > 1 {
                    let u = self.types.fresh();
                    let i = self.types.int();
                    self.callback(args, 1, vec![item, i], u.clone(), &full, span)?;
                    return self.done(e, "Array.from.map".into(), Ty::Array(Box::new(u)));
                }
                self.done(e, full, Ty::Array(Box::new(item)))
            }
            ("Array", "isArray") => {
                self.arg_typed(args, 0, None, &full, span)?;
                self.done(e, full, Ty::Bool)
            }
            ("Array", _) => self.err(format!("`Array.{prop}` is not supported"), span),
            ("Promise", "all") => {
                // Each value is already awaited where it is made: the array itself.
                let t = self.arg_typed(args, 0, None, &full, span)?;
                let Ty::Array(_) = self.types.resolve(&t) else {
                    return self.err("`Promise.all` needs an array", span);
                };
                self.done(e, full, t)
            }
            ("Promise", "resolve") => {
                let t = self.arg_typed(args, 0, None, &full, span)?;
                self.done(e, full, t)
            }
            ("Promise", _) => self.err(
                format!("`Promise.{prop}` is not supported: use `await` and async functions"),
                span,
            ),
            ("JSON", "stringify") => {
                self.no_more_than(args, 3, &full, span)?;
                let t = self.arg_typed(args, 0, None, &full, span)?;
                if let Ty::Var(_) = self.types.resolve(&t) {
                    return self.err(
                        "the type of this value is not known yet: add an annotation",
                        args[0].expr.span,
                    );
                }
                // A replacer: a function given each key and value, or the keys to keep.
                if let Some(a) = args.get(1)
                    && !matches!(a.expr.kind, ExprKind::Null | ExprKind::Undefined)
                {
                    let keys = Ty::Array(Box::new(Ty::Str));
                    if matches!(a.expr.kind, ExprKind::Array(_)) {
                        self.arg_typed(args, 1, Some(keys), &full, span)?;
                    } else {
                        let d = Ty::dynamic();
                        let f = Ty::Func(Rc::new(FuncTy {
                            params: vec![Ty::Str, d.clone()],
                            ret: d,
                        }));
                        let literal = matches!(a.expr.kind, ExprKind::Arrow(_) | ExprKind::Func(_));
                        let at = self.expr(&a.expr, literal.then_some(&f))?;
                        if self.types.unify(&at, &keys).is_err() {
                            self.expect(&at, &f, a.expr.span)?;
                        }
                    }
                }
                if args.len() == 3 {
                    let it = self.arg_typed(args, 2, None, &full, span)?;
                    if !matches!(self.types.resolve(&it), Ty::Str) {
                        let i = self.types.int();
                        self.expect(&it, &i, args[2].expr.span)?;
                    }
                }
                self.done(e, full, Ty::Str)
            }
            ("JSON", "parse") => {
                let expected = self.call_expected.take();
                self.no_more_than(args, 2, &full, span)?;
                self.arg_typed(args, 0, Some(Ty::Str), &full, span)?;
                // A reviver: each key and value, innermost first.
                if args.len() > 1 {
                    let d = Ty::dynamic();
                    self.callback(args, 1, vec![Ty::Str, d.clone()], d, &full, span)?;
                }
                // It throws on bad text: the function fails like one with a `throw`.
                if self.ctx().try_depth == 0 {
                    let id = self.ctx().id;
                    self.funcs.get_mut(&id).expect("fn").throw_direct = true;
                }
                // Without a type: a value known at run time; a reviver gives one too.
                let t = match expected {
                    Some(t) if !matches!(t, Ty::Var(_)) && args.len() == 1 => t,
                    _ => Ty::dynamic(),
                };
                self.json_targets.push(t.clone());
                self.done(e, full, t)
            }
            ("JSON", _) => self.err(format!("`JSON.{prop}` is not supported"), span),
            ("Date", "now") => {
                let i = self.types.int();
                self.done(e, full, i)
            }
            ("performance", "now") => {
                let f = self.types.float();
                self.done(e, full, f)
            }
            ("process", "exit") => {
                let i = self.types.int();
                if !args.is_empty() {
                    self.arg_typed(args, 0, Some(i), &full, span)?;
                }
                self.done(e, full, Ty::Never)
            }
            _ => self.err(format!("`{full}` is not supported"), span),
        }
    }

    /// A method of a string, an array, a map, a set, a number or a boolean.
    pub(super) fn builtin_method(
        &mut self,
        e: &'m Expr,
        obj: &'m Expr,
        ot: &Ty,
        prop: &str,
        args: &'m [Arg],
    ) -> IResult<Ty> {
        let span = e.span;
        match ot {
            Ty::Str => self.string_method(e, prop, args, span),
            Ty::Array(item) => {
                let item = (**item).clone();
                self.array_method(e, obj, item, prop, args, span)
            }
            Ty::Map(k, v, record) => {
                let (k, v, record) = ((**k).clone(), (**v).clone(), *record);
                let full = format!("map.{prop}");
                match prop {
                    "get" => {
                        self.arg_typed(args, 0, Some(k), &full, span)?;
                        self.done(e, full, Ty::Option(Box::new(v)))
                    }
                    "set" => {
                        self.arg_typed(args, 0, Some(k.clone()), &full, span)?;
                        self.arg_typed(args, 1, Some(v.clone()), &full, span)?;
                        self.mark_stored(&args[0].expr);
                        self.mark_stored(&args[1].expr);
                        self.mark_receiver_mutated(obj);
                        self.done(e, full, Ty::Map(Box::new(k), Box::new(v), record))
                    }
                    "has" => {
                        self.arg_typed(args, 0, Some(k), &full, span)?;
                        self.done(e, full, Ty::Bool)
                    }
                    "delete" => {
                        self.arg_typed(args, 0, Some(k), &full, span)?;
                        self.mark_receiver_mutated(obj);
                        self.done(e, full, Ty::Bool)
                    }
                    "clear" => {
                        self.mark_receiver_mutated(obj);
                        self.done(e, full, Ty::Void)
                    }
                    "keys" => self.done(e, full, Ty::Array(Box::new(k))),
                    "values" => self.done(e, full, Ty::Array(Box::new(v))),
                    "entries" => self.done(e, full, Ty::Array(Box::new(Ty::Tuple(vec![k, v])))),
                    "forEach" => {
                        self.callback(args, 0, vec![v, k], Ty::Void, &full, span)?;
                        self.done(e, full, Ty::Void)
                    }
                    _ => self.err(format!("Map has no method `{prop}`"), span),
                }
            }
            Ty::Set(item) => {
                let item = (**item).clone();
                let full = format!("set.{prop}");
                match prop {
                    "add" => {
                        self.arg_typed(args, 0, Some(item.clone()), &full, span)?;
                        self.mark_stored(&args[0].expr);
                        self.mark_receiver_mutated(obj);
                        self.done(e, full, Ty::Set(Box::new(item)))
                    }
                    "has" => {
                        self.arg_typed(args, 0, Some(item), &full, span)?;
                        self.done(e, full, Ty::Bool)
                    }
                    "delete" => {
                        self.arg_typed(args, 0, Some(item), &full, span)?;
                        self.mark_receiver_mutated(obj);
                        self.done(e, full, Ty::Bool)
                    }
                    "clear" => {
                        self.mark_receiver_mutated(obj);
                        self.done(e, full, Ty::Void)
                    }
                    "values" | "keys" => self.done(e, full, Ty::Array(Box::new(item))),
                    "forEach" => {
                        self.callback(args, 0, vec![item], Ty::Void, &full, span)?;
                        self.done(e, full, Ty::Void)
                    }
                    _ => self.err(format!("Set has no method `{prop}`"), span),
                }
            }
            Ty::Num(_) => {
                let full = format!("number.{prop}");
                match prop {
                    "toFixed" => {
                        if !args.is_empty() {
                            let i = self.types.int();
                            self.arg_typed(args, 0, Some(i), &full, span)?;
                        }
                        self.done(e, full, Ty::Str)
                    }
                    "toString" => {
                        if !args.is_empty() {
                            let i = self.types.int();
                            self.arg_typed(args, 0, Some(i), &full, span)?;
                        }
                        self.done(e, full, Ty::Str)
                    }
                    "toPrecision" => {
                        let i = self.types.int();
                        self.arg_typed(args, 0, Some(i), &full, span)?;
                        self.done(e, full, Ty::Str)
                    }
                    _ => self.err(format!("numbers have no method `{prop}`"), span),
                }
            }
            Ty::Bool => match prop {
                "toString" => self.done(e, "boolean.toString".into(), Ty::Str),
                _ => self.err(format!("booleans have no method `{prop}`"), span),
            },
            Ty::Named(n, ..) if n == "Error" => match prop {
                "toString" => self.done(e, "error.toString".into(), Ty::Str),
                _ => self.err(format!("Error has no method `{prop}`"), span),
            },
            Ty::Named(n, ..) if n == "RegExp" => {
                let full = format!("regex.{prop}");
                match prop {
                    "test" => {
                        self.arg_typed(args, 0, Some(Ty::Str), &full, span)?;
                        self.done(e, full, Ty::Bool)
                    }
                    "exec" => {
                        self.arg_typed(args, 0, Some(Ty::Str), &full, span)?;
                        self.done(e, full, Ty::Option(Box::new(Ty::named("RegExpMatch"))))
                    }
                    "toString" => self.done(e, full, Ty::Str),
                    _ => self.err(format!("RegExp has no method `{prop}`"), span),
                }
            }
            Ty::Named(n, ..) if n == "RegExpMatch" => self.err(
                format!("a match has no method `{prop}`: its groups are `m[i]`"),
                span,
            ),
            Ty::Named(n, ..) if n == "Symbol" => {
                let full = format!("symbol.{prop}");
                self.no_more_than(args, 0, &full, span)?;
                match prop {
                    "toString" => self.done(e, full, Ty::Str),
                    _ => self.err(format!("a symbol has no method `{prop}`"), span),
                }
            }
            Ty::Named(n, ..) if n == "Date" => {
                let full = format!("date.{prop}");
                self.no_more_than(args, 0, &full, span)?;
                match prop {
                    "getTime" | "valueOf" | "getFullYear" | "getMonth" | "getDate" | "getDay"
                    | "getHours" | "getMinutes" | "getSeconds" | "getMilliseconds"
                    | "getUTCFullYear" | "getUTCMonth" | "getUTCDate" | "getUTCDay"
                    | "getUTCHours" | "getUTCMinutes" | "getUTCSeconds"
                    | "getUTCMilliseconds" | "getTimezoneOffset" => {
                        let i = self.types.int();
                        self.done(e, full, i)
                    }
                    "toISOString" | "toJSON" | "toString" | "toDateString" | "toTimeString"
                    | "toUTCString" => self.done(e, full, Ty::Str),
                    _ => self.err(
                        format!("Date has no method `{prop}`: the UTC getters and the ISO text are supported"),
                        span,
                    ),
                }
            }
            Ty::Tuple(_) => self.err(
                format!("tuples have no method `{prop}`: use an array"),
                span,
            ),
            Ty::Gen(item) => {
                let item = (**item).clone();
                match prop {
                    "next" => {
                        self.no_more_than(args, 0, "next", span)?;
                        self.mark_receiver_mutated(obj);
                        self.done(e, "generator.next".into(), Ty::Option(Box::new(item)))
                    }
                    _ => self.err(
                        format!("generators have no method `{prop}`: iterate with `for … of`"),
                        span,
                    ),
                }
            }
            Ty::Any => self.err(
                format!(
                    "`{}` is `any`: give it a type to call `.{prop}()`",
                    self.text(obj)
                ),
                obj.span,
            ),
            other => {
                let d = self.describe(other);
                self.err(format!("{d} has no method `{prop}`"), span)
            }
        }
    }

    fn string_method(
        &mut self,
        e: &'m Expr,
        prop: &str,
        args: &'m [Arg],
        span: Span,
    ) -> IResult<Ty> {
        let full = format!("string.{prop}");
        let int = |this: &mut Self| this.types.int();
        match prop {
            "toUpperCase" | "toLowerCase" | "trim" | "trimStart" | "trimEnd" | "trimLeft"
            | "trimRight" | "toString" | "valueOf" | "normalize" => {
                self.no_more_than(args, 0, &full, span)?;
                self.done(e, full, Ty::Str)
            }
            "split" => {
                if self.is_regex(args.first()) {
                    self.arg_typed(args, 0, None, &full, span)?;
                    self.no_more_than(args, 1, &full, span)?;
                    return self.done(e, "string.splitRegex".into(), Ty::Array(Box::new(Ty::Str)));
                }
                self.arg_typed(args, 0, Some(Ty::Str), &full, span)?;
                self.no_more_than(args, 1, &full, span)?;
                self.done(e, full, Ty::Array(Box::new(Ty::Str)))
            }
            "includes" | "startsWith" | "endsWith" => {
                self.arg_typed(args, 0, Some(Ty::Str), &full, span)?;
                self.no_more_than(args, 1, &full, span)?;
                self.done(e, full, Ty::Bool)
            }
            "indexOf" | "lastIndexOf" => {
                self.arg_typed(args, 0, Some(Ty::Str), &full, span)?;
                self.no_more_than(args, 1, &full, span)?;
                let i = int(self);
                self.done(e, full, i)
            }
            "slice" | "substring" | "substr" => {
                for i in 0..args.len().min(2) {
                    let n = int(self);
                    self.arg_typed(args, i, Some(n), &full, span)?;
                }
                self.no_more_than(args, 2, &full, span)?;
                self.done(e, full, Ty::Str)
            }
            "charAt" | "at" => {
                let n = int(self);
                self.arg_typed(args, 0, Some(n), &full, span)?;
                self.done(e, full, Ty::Str)
            }
            "charCodeAt" | "codePointAt" => {
                let n = int(self);
                self.arg_typed(args, 0, Some(n), &full, span)?;
                let i = int(self);
                self.done(e, full, i)
            }
            "repeat" => {
                let n = int(self);
                self.arg_typed(args, 0, Some(n), &full, span)?;
                self.done(e, full, Ty::Str)
            }
            "replace" | "replaceAll" => {
                let regex = self.is_regex(args.first());
                if regex {
                    self.arg_typed(args, 0, None, &full, span)?;
                } else {
                    self.arg_typed(args, 0, Some(Ty::Str), &full, span)?;
                }
                // The replacement: a string, or a function of the match and its groups.
                let by_function = matches!(
                    args.get(1).map(|a| &a.expr.kind),
                    Some(ExprKind::Arrow(_) | ExprKind::Func(_))
                );
                if by_function {
                    let (ExprKind::Arrow(f) | ExprKind::Func(f)) = &args[1].expr.kind else {
                        unreachable!()
                    };
                    let params = vec![Ty::Str; f.params.len()];
                    self.callback(args, 1, params, Ty::Str, &full, span)?;
                } else {
                    self.arg_typed(args, 1, Some(Ty::Str), &full, span)?;
                }
                let name = match (regex, by_function) {
                    (true, true) => format!("{full}RegexWith"),
                    (true, false) => format!("{full}Regex"),
                    (false, true) => format!("{full}With"),
                    (false, false) => full,
                };
                self.done(e, name, Ty::Str)
            }
            "padStart" | "padEnd" => {
                let n = int(self);
                self.arg_typed(args, 0, Some(n), &full, span)?;
                if args.len() > 1 {
                    self.arg_typed(args, 1, Some(Ty::Str), &full, span)?;
                }
                self.done(e, full, Ty::Str)
            }
            "concat" => {
                for i in 0..args.len() {
                    self.arg_typed(args, i, Some(Ty::Str), &full, span)?;
                }
                self.done(e, full, Ty::Str)
            }
            "localeCompare" => {
                self.arg_typed(args, 0, Some(Ty::Str), &full, span)?;
                let i = int(self);
                self.done(e, full, i)
            }
            "match" => {
                if !self.is_regex(args.first()) {
                    return self.err("`match` needs a regular expression", span);
                }
                self.arg_typed(args, 0, None, &full, span)?;
                let global = self
                    .regex_flags(&args[0].expr)
                    .is_some_and(|f| f.contains('g'));
                if global {
                    self.done(
                        e,
                        "string.matchAll".into(),
                        Ty::Option(Box::new(Ty::Array(Box::new(Ty::Str)))),
                    )
                } else {
                    self.done(e, full, Ty::Option(Box::new(Ty::named("RegExpMatch"))))
                }
            }
            "matchAll" => {
                if !self.is_regex(args.first()) {
                    return self.err("`matchAll` needs a regular expression", span);
                }
                self.arg_typed(args, 0, None, &full, span)?;
                self.done(
                    e,
                    "string.matches".into(),
                    Ty::Array(Box::new(Ty::named("RegExpMatch"))),
                )
            }
            "search" => {
                if !self.is_regex(args.first()) {
                    return self.err("`search` needs a regular expression", span);
                }
                self.arg_typed(args, 0, None, &full, span)?;
                let i = int(self);
                self.done(e, full, i)
            }
            _ => self.err(format!("strings have no method `{prop}`"), span),
        }
    }

    fn array_method(
        &mut self,
        e: &'m Expr,
        obj: &'m Expr,
        item: Ty,
        prop: &str,
        args: &'m [Arg],
        span: Span,
    ) -> IResult<Ty> {
        let full = format!("array.{prop}");
        let arr = Ty::Array(Box::new(item.clone()));
        match prop {
            "push" | "unshift" => {
                if args.is_empty() {
                    return self.err(format!("`{full}` needs an argument"), span);
                }
                for a in args {
                    if a.spread {
                        let t = self.expr(&a.expr, Some(&arr))?;
                        self.expect(&t, &arr, a.expr.span)?;
                    } else {
                        let t = self.expr(&a.expr, Some(&item))?;
                        self.expect(&t, &item, a.expr.span)?;
                        self.mark_stored(&a.expr);
                    }
                }
                self.mark_receiver_mutated(obj);
                let i = self.types.int();
                self.done(e, full, i)
            }
            "pop" | "shift" => {
                self.no_more_than(args, 0, &full, span)?;
                self.mark_receiver_mutated(obj);
                self.done(e, full, Ty::Option(Box::new(item)))
            }
            "indexOf" | "lastIndexOf" => {
                self.arg_typed(args, 0, Some(item), &full, span)?;
                if args.len() > 1 {
                    let i = self.types.int();
                    self.arg_typed(args, 1, Some(i), &full, span)?;
                }
                let i = self.types.int();
                self.done(e, full, i)
            }
            "includes" => {
                self.arg_typed(args, 0, Some(item), &full, span)?;
                self.no_more_than(args, 1, &full, span)?;
                self.done(e, full, Ty::Bool)
            }
            "map" => {
                let u = self.types.fresh();
                let i = self.types.int();
                self.callback(args, 0, vec![item, i, arr], u.clone(), &full, span)?;
                self.no_more_than(args, 1, &full, span)?;
                self.done(e, full, Ty::Array(Box::new(u)))
            }
            "flatMap" => {
                let u = self.types.fresh();
                let i = self.types.int();
                let inner = Ty::Array(Box::new(u.clone()));
                self.callback(args, 0, vec![item, i], inner, &full, span)?;
                self.done(e, full, Ty::Array(Box::new(u)))
            }
            "filter" => {
                let i = self.types.int();
                self.callback(args, 0, vec![item, i, arr.clone()], Ty::Bool, &full, span)?;
                self.no_more_than(args, 1, &full, span)?;
                self.done(e, full, arr)
            }
            "forEach" => {
                let i = self.types.int();
                self.callback(args, 0, vec![item, i, arr], Ty::Void, &full, span)?;
                self.no_more_than(args, 1, &full, span)?;
                self.done(e, full, Ty::Void)
            }
            "reduce" | "reduceRight" => {
                let acc = if args.len() > 1 {
                    self.arg_typed(args, 1, None, &full, span)?
                } else {
                    item.clone()
                };
                let i = self.types.int();
                self.callback(
                    args,
                    0,
                    vec![acc.clone(), item, i],
                    acc.clone(),
                    &full,
                    span,
                )?;
                self.no_more_than(args, 2, &full, span)?;
                if args.len() > 1 {
                    self.mark_stored(&args[1].expr);
                }
                self.done(e, full, acc)
            }
            "find" | "findLast" => {
                let i = self.types.int();
                self.callback(args, 0, vec![item.clone(), i], Ty::Bool, &full, span)?;
                self.done(e, full, Ty::Option(Box::new(item)))
            }
            "findIndex" | "findLastIndex" => {
                let i = self.types.int();
                self.callback(args, 0, vec![item, i], Ty::Bool, &full, span)?;
                let r = self.types.int();
                self.done(e, full, r)
            }
            "some" | "every" => {
                let i = self.types.int();
                self.callback(args, 0, vec![item, i], Ty::Bool, &full, span)?;
                self.done(e, full, Ty::Bool)
            }
            "join" => {
                if !args.is_empty() {
                    self.arg_typed(args, 0, Some(Ty::Str), &full, span)?;
                }
                self.no_more_than(args, 1, &full, span)?;
                self.done(e, full, Ty::Str)
            }
            "slice" => {
                for i in 0..args.len().min(2) {
                    let n = self.types.int();
                    self.arg_typed(args, i, Some(n), &full, span)?;
                }
                self.no_more_than(args, 2, &full, span)?;
                self.done(e, full, arr)
            }
            "splice" => {
                let n = self.types.int();
                self.arg_typed(args, 0, Some(n), &full, span)?;
                if args.len() > 1 {
                    let n = self.types.int();
                    self.arg_typed(args, 1, Some(n), &full, span)?;
                }
                for i in 2..args.len() {
                    self.arg_typed(args, i, Some(item.clone()), &full, span)?;
                    self.mark_stored(&args[i].expr);
                }
                self.mark_receiver_mutated(obj);
                self.done(e, full, arr)
            }
            "concat" => {
                for a in args {
                    let t = self.expr(&a.expr, Some(&arr))?;
                    if self.types.unify(&t, &arr).is_err() {
                        self.expect(&t, &item, a.expr.span)?;
                    }
                }
                self.done(e, full, arr)
            }
            "reverse" | "sort" | "fill" | "toSorted" | "toReversed" => {
                match prop {
                    "sort" | "toSorted" => {
                        if !args.is_empty() {
                            let n = self.types.num(Flavor::Unknown);
                            self.callback(
                                args,
                                0,
                                vec![item.clone(), item.clone()],
                                n,
                                &full,
                                span,
                            )?;
                        }
                    }
                    "fill" => {
                        self.arg_typed(args, 0, Some(item.clone()), &full, span)?;
                        for i in 1..args.len().min(3) {
                            let n = self.types.int();
                            self.arg_typed(args, i, Some(n), &full, span)?;
                        }
                    }
                    _ => self.no_more_than(args, 0, &full, span)?,
                }
                if matches!(prop, "reverse" | "sort" | "fill") {
                    self.mark_receiver_mutated(obj);
                }
                self.done(e, full, arr)
            }
            "at" => {
                let n = self.types.int();
                self.arg_typed(args, 0, Some(n), &full, span)?;
                self.done(e, full, Ty::Option(Box::new(item)))
            }
            "flat" => {
                self.no_more_than(args, 0, &full, span)?;
                let Ty::Array(inner) = self.types.resolve(&item) else {
                    return self.err("`flat` needs an array of arrays", span);
                };
                self.done(e, full, Ty::Array(inner))
            }
            "entries" => {
                let i = self.types.int();
                self.done(e, full, Ty::Array(Box::new(Ty::Tuple(vec![i, item]))))
            }
            "keys" => {
                let i = self.types.int();
                self.done(e, full, Ty::Array(Box::new(i)))
            }
            "values" => self.done(e, full, arr),
            "toString" => self.done(e, full, Ty::Str),
            _ => self.err(format!("arrays have no method `{prop}`"), span),
        }
    }
}
