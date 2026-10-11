//! Calls: functions, methods, constructors and the standard library.

use super::exprs::format_call;
use super::{
    E, GResult, Gen, Helper, Mode, PREC_ATOM, PREC_CAST, PREC_POSTFIX, PREC_UNARY, Repr, Want,
    pascal, snake, string_literal,
};
use crate::ast::*;
use crate::error::Span;
use crate::infer::{Callee, FnInfo};
use crate::types::Ty;

impl<'a> Gen<'a> {
    pub(super) fn call(&mut self, e: &'a Expr, callee: &'a Expr, args: &'a [Arg]) -> GResult<E> {
        let resolved = self.p.calls.get(&e.id).cloned().unwrap_or(Callee::Dynamic);
        match resolved {
            Callee::Fn(binding) => {
                let fn_id = self.p.fn_bindings[&binding];
                let info = self.p.funcs[&fn_id].clone();
                let name = if info.is_closure {
                    self.name(binding)
                } else {
                    snake(&self.p.var(binding).name)
                };
                let list = self.user_args(&info, args, e.span)?;
                let code = format!("{name}({list})");
                self.finish_call(code, &info, e)
            }
            Callee::Closure(binding) => {
                let Ty::Func(f) = self.resolve(&self.p.var(binding).ty) else {
                    return self.err("not a function", callee.span);
                };
                let name = self.name(binding);
                let mut list = Vec::new();
                for (a, pt) in args.iter().zip(&f.params) {
                    list.push(self.expr_to(&a.expr, pt)?);
                }
                // Missing arguments: the defaults of the arrow function.
                if args.len() < f.params.len() {
                    let Some(fn_id) = self.p.var(binding).closure_fn else {
                        return self.err("missing argument", e.span);
                    };
                    let ast = self.fn_asts.get(&fn_id).copied().expect("closure ast");
                    let info = self.p.funcs[&fn_id].clone();
                    for i in args.len()..f.params.len() {
                        if info.optional.get(i).copied().unwrap_or(false) {
                            list.push("None".into());
                            continue;
                        }
                        if matches!(self.resolve(&f.params[i]), Ty::Void) {
                            list.push("()".into());
                            continue;
                        }
                        let Some(d) = ast.params.get(i).and_then(|p| p.default.as_ref()) else {
                            return self.err("missing argument", e.span);
                        };
                        if !is_literal(d) {
                            return self.err("the default value of this parameter is not a literal: pass the argument explicitly", e.span);
                        }
                        list.push(self.expr_to(d, &f.params[i])?);
                    }
                }
                Ok(E::owned(
                    format!("{name}({})", list.join(", ")),
                    PREC_POSTFIX,
                ))
            }
            Callee::Method {
                class,
                name,
                fn_id,
                is_static,
            } => {
                let info = self.p.funcs[&fn_id].clone();
                let list = self.user_args(&info, args, e.span)?;
                let method = snake(&name);
                let code = if is_static {
                    format!("{}::{method}({list})", pascal(&class))
                } else {
                    let ExprKind::Member { obj, .. } = &callee.kind else {
                        unreachable!()
                    };
                    let o = if info.mutates_self {
                        self.expr_mut_root(obj)?
                    } else {
                        self.expr(obj)?
                    };
                    let base = if info.mutates_self && o.repr == Repr::Ref {
                        return self.err(
                            format!("`{name}` changes the object, but it is only borrowed here"),
                            e.span,
                        );
                    } else {
                        Self::paren(&o, PREC_POSTFIX)
                    };
                    format!("{base}.{method}({list})")
                };
                self.finish_call(code, &info, e)
            }
            Callee::Ctor(_) => unreachable!("constructors go through new"),
            Callee::SuperCtor(_) => self.err(
                "`super(…)` must be the first statement of the constructor",
                e.span,
            ),
            Callee::SuperMethod { class, name, fn_id } => {
                // The base class's version, written into this class as `base_method`.
                let info = self.p.funcs[&fn_id].clone();
                let list = self.user_args(&info, args, e.span)?;
                let helper = format!("{}_{}", snake(&class), snake(&name));
                if let Some(current) = self.current_class.clone() {
                    self.super_helpers
                        .entry(current)
                        .or_default()
                        .insert((class.clone(), name.clone()));
                }
                let code = format!("self.{helper}({list})");
                self.finish_call(code, &info, e)
            }
            Callee::Builtin(name) => {
                let ExprKind::Member { obj, .. } = &callee.kind else {
                    return self.global_call(e, name, args);
                };
                if let Some(ns) = name.strip_prefix("global.") {
                    let _ = ns;
                    return self.global_call(e, name, args);
                }
                let receiver_kind = name.split('.').next().unwrap_or("");
                match receiver_kind {
                    "console" | "Math" | "Number" | "String" | "Object" | "Array" | "Date"
                    | "performance" | "process" | "Promise" => self.namespace_call(e, name, args),
                    _ => self.method_builtin(e, obj, name, args),
                }
            }
            Callee::Dynamic => {
                let ct = self.ty_of(callee);
                let Ty::Func(f) = self.resolve(&ct) else {
                    return self.err("not a function", callee.span);
                };
                let c = self.expr(callee)?;
                let base = if matches!(callee.kind, ExprKind::Member { .. }) {
                    format!("({})", c.code)
                } else {
                    Self::paren(&c, PREC_POSTFIX)
                };
                let mut list = Vec::new();
                for (a, pt) in args.iter().zip(&f.params) {
                    list.push(self.expr_to(&a.expr, pt)?);
                }
                Ok(E::owned(
                    format!("{base}({})", list.join(", ")),
                    PREC_POSTFIX,
                ))
            }
        }
    }

    /// A call to a user function: `?` when it can fail and we can propagate, `.await` when
    /// it is async.
    fn finish_call(&mut self, code: String, info: &FnInfo, _e: &'a Expr) -> GResult<E> {
        let mut code = code;
        if info.is_async {
            code.push_str(".await");
        }
        let _ = ();
        if info.throws {
            if self.fn_ctx().returns_result || !self.fn_ctx().try_ctx.is_empty() {
                code.push('?');
            } else {
                code.push_str(".unwrap()");
            }
        }
        Ok(E::owned(code, PREC_POSTFIX))
    }

    /// Arguments of a call to a user function, each passed the way the parameter wants.
    pub(super) fn user_args(
        &mut self,
        info: &FnInfo,
        args: &'a [Arg],
        span: Span,
    ) -> GResult<String> {
        let mut list = Vec::new();
        let f_ast = self.fn_asts.get(&info.id).copied();
        for (i, pt) in info.sig.params.iter().enumerate() {
            let binding = info.params.get(i).copied();
            let (stored, reassigned, mutated) = match binding {
                Some(b) => {
                    let v = self.p.var(b);
                    (v.stored, v.reassigned, v.mutated)
                }
                None => (true, false, false),
            };
            let mode = self.param_mode(pt, stored, reassigned, mutated, &info.type_params);
            let optional = info.optional.get(i).copied().unwrap_or(false);
            match args.get(i) {
                Some(a) => {
                    let at = self.ty_of(&a.expr);
                    let x = if mode == Mode::RefMut {
                        self.expr_mut_root(&a.expr)?
                    } else {
                        self.expr(&a.expr)?
                    };
                    // A member passed where a union is borrowed: the variant, borrowed.
                    if mode != Mode::Value
                        && mode != Mode::FnImpl
                        && self.union_variant(&at, pt).is_some()
                    {
                        let target = match self.resolve(pt) {
                            Ty::Option(inner) => *inner,
                            other => other,
                        };
                        let wrapped = self.owned_as(x, &at, &target);
                        list.push(match mode {
                            Mode::RefMut => format!("&mut {wrapped}"),
                            Mode::OptRef => format!("Some(&{wrapped})"),
                            _ => format!("&{wrapped}"),
                        });
                        continue;
                    }
                    let code = match mode {
                        Mode::Value => {
                            let generic = matches!(self.resolve(pt), Ty::Named(ref n) if info.type_params.contains(n));
                            if generic && x.repr == Repr::Str {
                                // A string literal for a type parameter: `&str` does.
                                x.code
                            } else {
                                self.owned_as(x, &at, pt)
                            }
                        }
                        Mode::FnImpl => {
                            if let ExprKind::Arrow(f) | ExprKind::Func(f) = &a.expr.kind {
                                let c = self.closure(f, &[], false)?;
                                c.code
                            } else {
                                self.owned(x, &at)
                            }
                        }
                        Mode::Ref => self.coerce(x, &at, Want::Ref),
                        Mode::RefMut => self.coerce(x, &at, Want::RefMut),
                        Mode::StrRef => self.coerce(x, &at, Want::StrRef),
                        Mode::Slice => self.coerce(x, &at, Want::Slice),
                        Mode::OptRef => {
                            if matches!(self.resolve(&at), Ty::Option(_)) {
                                let deref = matches!(self.resolve(pt), Ty::Option(ref i) if self.is_str(i) || self.is_boxed(i) || self.dyn_name(i).is_some());
                                match x.repr {
                                    Repr::Owned if self.opt_ref_param_e(&x) => x.code,
                                    _ => format!(
                                        "{}.{}()",
                                        Self::paren(&x, PREC_POSTFIX),
                                        if deref { "as_deref" } else { "as_ref" }
                                    ),
                                }
                            } else if matches!(self.resolve(&at), Ty::Null) {
                                "None".into()
                            } else {
                                let inner_ty = match self.resolve(pt) {
                                    Ty::Option(i) => *i,
                                    t => t,
                                };
                                let want = if self.is_str(&inner_ty) {
                                    Want::StrRef
                                } else {
                                    Want::Ref
                                };
                                format!("Some({})", self.coerce(x, &at, want))
                            }
                        }
                    };
                    list.push(code);
                }
                None => {
                    // A missing optional parameter, or a default value.
                    if optional {
                        list.push("None".into());
                        continue;
                    }
                    let Some(f) = f_ast else {
                        return self.err("missing argument", span);
                    };
                    let Some(d) = f.params.get(i).and_then(|p| p.default.as_ref()) else {
                        return self.err("missing argument", span);
                    };
                    if !is_literal(d) {
                        return self.err(
                            "the default value of this parameter is not a literal: pass the argument explicitly",
                            span,
                        );
                    }
                    let code = self.expr_to(d, pt)?;
                    let code = match mode {
                        Mode::Value | Mode::FnImpl => code,
                        Mode::StrRef => code.trim_end_matches(".to_string()").to_string(),
                        _ => format!("&{code}"),
                    };
                    list.push(code);
                }
            }
        }
        Ok(list.join(", "))
    }

    fn opt_ref_param_e(&self, x: &E) -> bool {
        if x.repr == Repr::OptRef {
            return true;
        }
        x.var.is_some_and(|(b, _)| {
            let v = self.p.var(b);
            v.role == crate::infer::Role::Param
                && !self.is_copy(&v.ty)
                && !v.stored
                && !v.mutated
                && !v.reassigned
        })
    }

    pub(super) fn new_expr(
        &mut self,
        e: &'a Expr,
        callee: &'a Expr,
        args: &'a [Arg],
    ) -> GResult<E> {
        match self.p.calls.get(&e.id).cloned() {
            Some(Callee::Ctor(class)) => {
                let name = pascal(&class);
                let ctor = self
                    .p
                    .ancestry(&class)
                    .iter()
                    .find_map(|c| self.p.struct_info(c).and_then(|s| s.ctor));
                let built = match ctor {
                    Some(ctor) => {
                        let f = self.p.funcs[&ctor].clone();
                        let list = self.user_args(&f, args, e.span)?;
                        self.finish_call(format!("{name}::new({list})"), &f, e)?
                    }
                    None => E::owned(format!("{name}::new()"), PREC_POSTFIX),
                };
                // A class held dynamically: every value of it is boxed.
                if self.p.is_dyn(&class) {
                    return Ok(E::owned(
                        format!("Box::new({}) as Box<dyn {name}Like>", built.code),
                        PREC_CAST,
                    ));
                }
                Ok(built)
            }
            Some(Callee::Builtin("new Promise")) => {
                let promise = self.helper(Helper::Promise);
                self.helper(Helper::BlockOn);
                let (ExprKind::Arrow(f) | ExprKind::Func(f)) = &args[0].expr.kind else {
                    return self.err(
                        "`new Promise` needs an inline executor function",
                        args[0].expr.span,
                    );
                };
                let c = self.closure(f, &[Repr::Owned, Repr::Owned], false)?;
                // The executor always gets `resolve` and `reject`.
                let code = match f.params.len() {
                    0 => c.code.replacen("||", "|_, _|", 1),
                    1 => c.code.replacen("| ", ", _| ", 1),
                    _ => c.code,
                };
                Ok(E::owned(
                    format!("{promise}::new({code}).await"),
                    PREC_POSTFIX,
                ))
            }
            Some(Callee::Builtin("new Error")) => match args.first() {
                Some(a) => {
                    let s = self.expr_as(&a.expr, Want::Owned)?;
                    Ok(E::owned(s, PREC_POSTFIX))
                }
                None => Ok(E::owned("String::new()", PREC_POSTFIX)),
            },
            Some(Callee::Builtin("new Map")) => {
                let name = self.map_name();
                match args.first() {
                    Some(a) => {
                        let v = self.expr_as(&a.expr, Want::Owned)?;
                        Ok(E::owned(
                            format!("{v}.into_iter().collect::<{name}<_, _>>()"),
                            PREC_POSTFIX,
                        ))
                    }
                    None => Ok(E::atom(format!("{name}::new()"), Repr::Owned)),
                }
            }
            Some(Callee::Builtin("new Set")) => {
                let name = self.set_name();
                match args.first() {
                    Some(a) => {
                        let v = self.expr_as(&a.expr, Want::Owned)?;
                        Ok(E::owned(
                            format!("{v}.into_iter().collect::<{name}<_>>()"),
                            PREC_POSTFIX,
                        ))
                    }
                    None => Ok(E::atom(format!("{name}::new()"), Repr::Owned)),
                }
            }
            Some(Callee::Builtin("new Array")) => match args.first() {
                Some(a) => {
                    let ty = self.ty_of(e);
                    let Ty::Array(item) = self.resolve(&ty) else {
                        unreachable!()
                    };
                    let n = self.expr_as(&a.expr, Want::Index)?;
                    let default = self.default_value(&item, e.span)?;
                    Ok(E::atom(format!("vec![{default}; {n}]"), Repr::Owned))
                }
                None => Ok(E::atom("Vec::new()", Repr::Owned)),
            },
            _ => self.err("unsupported `new`", callee.span),
        }
    }

    /// A zero value for `new Array(n)` and fields without initializer.
    pub(super) fn default_value(&mut self, ty: &Ty, span: Span) -> GResult<String> {
        Ok(match self.resolve(ty) {
            Ty::Num(_) => {
                if self.is_float(ty) {
                    "0.0".into()
                } else {
                    "0".into()
                }
            }
            Ty::Str => "String::new()".into(),
            Ty::Bool => "false".into(),
            Ty::Option(_) => "None".into(),
            Ty::Array(_) => "Vec::new()".into(),
            Ty::Map(..) => format!("{}::new()", self.map_name()),
            Ty::Set(_) => format!("{}::new()", self.set_name()),
            Ty::Void | Ty::Null => "()".into(),
            Ty::Named(n) if self.p.enum_info(&n).is_some() => {
                let en = self.p.enum_info(&n).expect("enum");
                format!("{}::{}", pascal(&n), en.variants[0].name)
            }
            _ => return self.err("no default value for this type", span),
        })
    }

    // ----- globals -----

    fn global_call(&mut self, e: &'a Expr, name: &str, args: &'a [Arg]) -> GResult<E> {
        let ty = self.ty_of(e);
        let arg_str =
            |g: &mut Self, i: usize| -> GResult<String> { g.expr_as(&args[i].expr, Want::StrRef) };
        Ok(match name {
            "global.parseInt" => {
                let at = self.ty_of(&args[0].expr);
                if matches!(self.resolve(&at), Ty::Num(_)) {
                    // `parseInt(3.7)`: a truncation.
                    let x = self.expr_as(&args[0].expr, Want::Owned)?;
                    if self.is_float(&at) {
                        let x = super::exprs::num_recv(&paren_num(&x), true);
                        return Ok(E::owned(format!("{x}.trunc() as i64"), PREC_CAST));
                    }
                    return Ok(E::owned(x, PREC_ATOM));
                }
                let s = arg_str(self, 0)?;
                if args.len() > 1 {
                    let radix = self.expr_as(&args[1].expr, Want::Owned)?;
                    let h = self.helper(Helper::ParseIntRadix);
                    E::owned(format!("{h}({s}, {radix})"), PREC_POSTFIX)
                } else {
                    let h = self.helper(Helper::ParseInt);
                    E::owned(format!("{h}({s})"), PREC_POSTFIX)
                }
            }
            "global.parseFloat" => {
                let s = arg_str(self, 0)?;
                let h = self.helper(Helper::ParseFloat);
                E::owned(format!("{h}({s})"), PREC_POSTFIX)
            }
            "global.Number" => {
                let at = self.ty_of(&args[0].expr);
                match self.resolve(&at) {
                    Ty::Str => {
                        let s = arg_str(self, 0)?;
                        let h = self.helper(Helper::ToNumber);
                        if self.is_int(&ty) {
                            E::owned(format!("{h}({s}) as i64"), PREC_CAST)
                        } else {
                            E::owned(format!("{h}({s})"), PREC_POSTFIX)
                        }
                    }
                    Ty::Bool => {
                        let b = self.expr_as(&args[0].expr, Want::Owned)?;
                        E::owned(format!("{b} as i64"), PREC_CAST)
                    }
                    _ => self.expr(&args[0].expr)?,
                }
            }
            "global.String" => {
                let (spec, arg) = self.format_arg(&args[0].expr)?;
                match arg {
                    Some(a) if a.starts_with("js_to_string(") => E::owned(a, PREC_POSTFIX),
                    Some(a) if spec == "{}" => E::owned(format!("{a}.to_string()"), PREC_POSTFIX),
                    _ => E::owned(
                        format_call("format!", &spec, &arg.into_iter().collect::<Vec<_>>()),
                        PREC_ATOM,
                    ),
                }
            }
            "global.Boolean" => {
                let c = self.cond(&args[0].expr)?;
                E::owned(c, PREC_ATOM)
            }
            "global.isNaN" | "global.isFinite" => {
                self.nan_test(name == "global.isNaN", &args[0].expr)?
            }
            "global.structuredClone" => {
                let x = self.expr_as(&args[0].expr, Want::Owned)?;
                E::owned(x, PREC_POSTFIX)
            }
            "global.setTimeout" | "global.setInterval" => {
                let (cb, ms) = self.timer_parts(args)?;
                self.helper(Helper::Timers);
                let f = if name == "global.setTimeout" {
                    "js_set_timeout"
                } else {
                    "js_set_interval"
                };
                E::owned(format!("{f}({cb}, {ms})"), PREC_POSTFIX)
            }
            "global.clearTimeout" | "global.clearInterval" => {
                let id = self.expr_as(&args[0].expr, Want::Owned)?;
                self.helper(Helper::Timers);
                E::owned(format!("js_clear_timer({id})"), PREC_POSTFIX)
            }
            _ => return self.err(format!("`{name}` is not supported"), e.span),
        })
    }

    fn namespace_call(&mut self, e: &'a Expr, name: &str, args: &'a [Arg]) -> GResult<E> {
        let ty = self.ty_of(e);
        Ok(match name {
            "console.log" | "console.info" | "console.debug" | "console.error" | "console.warn" => {
                let macro_name =
                    if name == "console.log" || name == "console.info" || name == "console.debug" {
                        "println!"
                    } else {
                        "eprintln!"
                    };
                if args.is_empty() {
                    return Ok(E::owned(format!("{macro_name}()"), PREC_ATOM));
                }
                let mut fmt = String::new();
                let mut list = Vec::new();
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        fmt.push(' ');
                    }
                    if let ExprKind::Str(s) = &a.expr.kind
                        && !self.p.enum_literal.contains_key(&a.expr.id)
                    {
                        fmt.push_str(&super::format_text(s));
                        continue;
                    }
                    if let ExprKind::Template { quasis, exprs } = &a.expr.kind {
                        for (j, q) in quasis.iter().enumerate() {
                            fmt.push_str(&super::format_text(q));
                            if let Some(x) = exprs.get(j) {
                                let (spec, arg) = self.format_arg(x)?;
                                fmt.push_str(&spec);
                                list.extend(arg);
                            }
                        }
                        continue;
                    }
                    let (spec, arg) = self.log_arg(&a.expr)?;
                    fmt.push_str(&spec);
                    list.extend(arg);
                }
                E::owned(format_call(macro_name, &fmt, &list), PREC_ATOM)
            }
            "Math.floor" | "Math.ceil" | "Math.round" | "Math.trunc" => {
                let at = self.ty_of(&args[0].expr);
                let method = &name[5..];
                if self.is_float(&at) {
                    // `Math.floor(a / b)` on integers: an integer division.
                    if matches!(method, "floor" | "trunc")
                        && let ExprKind::Binary {
                            op: BinOp::Div,
                            left,
                            right,
                        } = &args[0].expr.kind
                    {
                        let (lt, rt) = (self.ty_of(left), self.ty_of(right));
                        if self.is_int(&lt) && self.is_int(&rt) && self.is_int(&ty) {
                            let l = self.expr_as(left, Want::Owned)?;
                            let r = self.expr_as(right, Want::Owned)?;
                            let (l, r) =
                                (super::exprs::num_recv(&paren_num(&l), false), paren_num(&r));
                            return Ok(if method == "floor" {
                                E::owned(format!("{l}.div_euclid({r})"), PREC_POSTFIX)
                            } else {
                                E::owned(format!("{l} / {r}"), super::PREC_MUL)
                            });
                        }
                    }
                    let x = self.expr_as(&args[0].expr, Want::Owned)?;
                    let call = format!(
                        "{}.{method}()",
                        super::exprs::num_recv(&paren_num(&x), true)
                    );
                    if self.is_int(&ty) {
                        E::owned(format!("{call} as i64"), PREC_CAST)
                    } else {
                        E::owned(call, PREC_POSTFIX)
                    }
                } else {
                    self.expr(&args[0].expr)?
                }
            }
            "Math.sign" => {
                let at = self.ty_of(&args[0].expr);
                let x = self.expr_as(&args[0].expr, Want::Owned)?;
                let x = super::exprs::num_recv(&paren_num(&x), self.is_float(&at));
                if self.is_float(&at) {
                    E::owned(format!("{}.signum() as i64", paren_num(&x)), PREC_CAST)
                } else {
                    E::owned(format!("{}.signum()", paren_num(&x)), PREC_POSTFIX)
                }
            }
            "Math.abs" => {
                let at = self.ty_of(&args[0].expr);
                let x = self.expr_as(&args[0].expr, Want::Owned)?;
                E::owned(
                    format!(
                        "{}.abs()",
                        super::exprs::num_recv(&paren_num(&x), self.is_float(&at))
                    ),
                    PREC_POSTFIX,
                )
            }
            "Math.sqrt" | "Math.cbrt" | "Math.sin" | "Math.cos" | "Math.tan" | "Math.asin"
            | "Math.acos" | "Math.atan" | "Math.sinh" | "Math.cosh" | "Math.tanh" | "Math.exp"
            | "Math.log" | "Math.log2" | "Math.log10" | "Math.fround" => {
                let method = match &name[5..] {
                    "log" => "ln",
                    "fround" => "clone",
                    m => m,
                };
                let x = self.float_arg(&args[0].expr)?;
                E::owned(format!("{x}.{method}()"), PREC_POSTFIX)
            }
            "Math.atan2" => {
                let y = self.float_arg(&args[0].expr)?;
                let x = self.float_arg(&args[1].expr)?;
                E::owned(format!("{y}.atan2({x})"), PREC_POSTFIX)
            }
            "Math.hypot" => {
                let x = self.float_arg(&args[0].expr)?;
                let y = self.float_arg(&args[1].expr)?;
                E::owned(format!("{x}.hypot({y})"), PREC_POSTFIX)
            }
            "Math.pow" => {
                let x = self.expr_as(&args[0].expr, Want::Owned)?;
                let x = super::exprs::num_recv(&x, self.is_float(&ty));
                let y = self.expr_as(&args[1].expr, Want::Owned)?;
                if self.is_float(&ty) {
                    E::owned(format!("{}.powf({y})", paren_num(&x)), PREC_POSTFIX)
                } else {
                    E::owned(
                        format!("{}.pow({} as u32)", paren_num(&x), paren_num(&y)),
                        PREC_POSTFIX,
                    )
                }
            }
            "Math.max" | "Math.min" => {
                let method = &name[5..];
                let mut acc = self.expr_as(&args[0].expr, Want::Owned)?;
                acc = super::exprs::num_recv(&paren_num(&acc), self.is_float(&ty));
                for a in &args[1..] {
                    let x = self.expr_as(&a.expr, Want::Owned)?;
                    acc = format!("{acc}.{method}({x})");
                }
                E::owned(acc, PREC_POSTFIX)
            }
            "Math.max..." | "Math.min..." => {
                let method = &name[5..8];
                let v = self.recv(&args[0].expr)?;
                if self.is_float(&ty) {
                    let (init, f) = if method == "max" {
                        ("f64::NEG_INFINITY", "f64::max")
                    } else {
                        ("f64::INFINITY", "f64::min")
                    };
                    E::owned(
                        format!("{v}.iter().copied().fold({init}, {f})"),
                        PREC_POSTFIX,
                    )
                } else {
                    let init = if method == "max" {
                        "i64::MIN"
                    } else {
                        "i64::MAX"
                    };
                    E::owned(
                        format!("{v}.iter().copied().{method}().unwrap_or({init})"),
                        PREC_POSTFIX,
                    )
                }
            }
            "Math.random" => {
                let h = self.helper(Helper::Random);
                E::owned(format!("{h}()"), PREC_POSTFIX)
            }
            "Number.isInteger" | "Number.isSafeInteger" => {
                let at = self.ty_of(&args[0].expr);
                if self.is_float(&at) {
                    let x = self.expr_as(&args[0].expr, Want::Owned)?;
                    let x = super::exprs::num_recv(&paren_num(&x), true);
                    E::owned(format!("{x}.fract() == 0.0"), super::PREC_CMP)
                } else {
                    self.expr(&args[0].expr)?;
                    E::atom("true", Repr::Owned)
                }
            }
            "Number.isNaN" | "Number.isFinite" => {
                self.nan_test(name == "Number.isNaN", &args[0].expr)?
            }
            "String.fromCharCode" => {
                let x = self.expr_as(&args[0].expr, Want::Owned)?;
                E::owned(
                    format!("char::from_u32({x} as u32).map(String::from).unwrap_or_default()"),
                    PREC_POSTFIX,
                )
            }
            "Object.keys" => {
                let m = self.recv(&args[0].expr)?;
                E::owned(
                    format!("{m}.keys().cloned().collect::<Vec<_>>()"),
                    PREC_POSTFIX,
                )
            }
            "Object.values" => {
                let m = self.recv(&args[0].expr)?;
                E::owned(
                    format!("{m}.values().cloned().collect::<Vec<_>>()"),
                    PREC_POSTFIX,
                )
            }
            "Object.entries" => {
                let m = self.recv(&args[0].expr)?;
                E::owned(
                    format!("{m}.iter().map(|(k, v)| (k.clone(), v.clone())).collect::<Vec<_>>()"),
                    PREC_POSTFIX,
                )
            }
            "Array.from.length" => {
                let ExprKind::Object(props) = &args[0].expr.kind else {
                    unreachable!()
                };
                let PropValue::Expr(len) = &props[0].value else {
                    unreachable!()
                };
                let n = self.expr_as(len, Want::Owned)?;
                match args.get(1) {
                    Some(cb) => {
                        let (ExprKind::Arrow(f) | ExprKind::Func(f)) = &cb.expr.kind else {
                            return self
                                .err("`Array.from` needs an inline arrow function", cb.expr.span);
                        };
                        let c = self.closure_with_index(f)?;
                        E::owned(
                            format!("(0..{n} as i64).map({c}).collect::<Vec<_>>()"),
                            PREC_POSTFIX,
                        )
                    }
                    None => E::owned(format!("vec![(); {n} as usize]"), PREC_POSTFIX),
                }
            }
            "Array.from" | "Array.from.map" => {
                let at = self.ty_of(&args[0].expr);
                let src = match self.resolve(&at) {
                    Ty::Str => {
                        let s = self.expr_as(&args[0].expr, Want::StrRef)?;
                        format!("{s}.chars().map(String::from)")
                    }
                    Ty::Map(..) => {
                        let m = self.recv(&args[0].expr)?;
                        format!("{m}.iter().map(|(k, v)| (k.clone(), v.clone()))")
                    }
                    Ty::Gen(_) => self.expr_as(&args[0].expr, Want::Owned)?,
                    _ => {
                        let x = self.expr(&args[0].expr)?;
                        if x.repr == Repr::Owned && x.var.is_none() {
                            format!("{}.into_iter()", Self::paren(&x, PREC_POSTFIX))
                        } else {
                            let r = self.coerce(x, &at, Want::Ref);
                            format!("{r}.iter().cloned()")
                        }
                    }
                };
                match args.get(1) {
                    Some(cb) => {
                        let (ExprKind::Arrow(f) | ExprKind::Func(f)) = &cb.expr.kind else {
                            return self
                                .err("`Array.from` needs an inline arrow function", cb.expr.span);
                        };
                        let c = self.closure(f, &[Repr::Owned, Repr::Usize], false)?;
                        if f.params.len() > 1 {
                            E::owned(
                                format!(
                                    "{src}.enumerate().map(|(i, x)| ({}) (x, i)).collect::<Vec<_>>()",
                                    c.code
                                ),
                                PREC_POSTFIX,
                            )
                        } else {
                            E::owned(
                                format!("{src}.map({}).collect::<Vec<_>>()", c.code),
                                PREC_POSTFIX,
                            )
                        }
                    }
                    None => E::owned(format!("{src}.collect::<Vec<_>>()"), PREC_POSTFIX),
                }
            }
            "Promise.all" | "Promise.resolve" => self.expr(&args[0].expr)?,
            "Date.now" => E::owned(
                "std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64)",
                PREC_POSTFIX,
            ),
            "performance.now" => E::owned(
                "std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0.0, |d| d.as_secs_f64() * 1000.0)",
                PREC_POSTFIX,
            ),
            "process.exit" => {
                let code = match args.first() {
                    Some(a) => self.expr_as(&a.expr, Want::Owned)?,
                    None => "0".into(),
                };
                E::owned(format!("std::process::exit({code} as i32)"), PREC_POSTFIX)
            }
            _ => return self.err(format!("`{name}` is not supported"), e.span),
        })
    }

    /// The callback and the delay of `setTimeout(cb, ms)`, as Rust code.
    pub(super) fn timer_parts(&mut self, args: &'a [Arg]) -> GResult<(String, String)> {
        let cb = &args[0].expr;
        let code = match &cb.kind {
            ExprKind::Arrow(f) | ExprKind::Func(f) => self.closure(f, &[], false)?.code,
            _ => {
                let ct = self.ty_of(cb);
                let c = self.expr(cb)?;
                // `setTimeout(resolve, ms)`: a function of one (void) argument.
                match self.resolve(&ct) {
                    Ty::Func(f) if !f.params.is_empty() => format!("move || {}(())", c.code),
                    _ => c.code,
                }
            }
        };
        let ms = match args.get(1) {
            Some(a) => {
                let x = self.expr_as(&a.expr, Want::Owned)?;
                let at = self.ty_of(&a.expr);
                if self.is_float(&at) {
                    x
                } else {
                    self.as_f64(&E::owned(x, PREC_ATOM))
                }
            }
            None => "0.0".into(),
        };
        Ok((code, ms))
    }

    /// `isNaN(x)` / `isFinite(x)`: on a float, the test; on the `Option` of `parseInt`,
    /// `is_none` / `is_some`; on an integer, decided.
    fn nan_test(&mut self, nan: bool, arg: &'a Expr) -> GResult<E> {
        let at = self.ty_of(arg);
        if matches!(self.resolve(&at), Ty::Option(_)) {
            let x = self.expr(arg)?;
            let code = Self::paren(&x, PREC_POSTFIX);
            return Ok(E::owned(
                format!("{code}.{}()", if nan { "is_none" } else { "is_some" }),
                PREC_POSTFIX,
            ));
        }
        if self.is_float(&at) {
            let x = self.expr_as(arg, Want::Owned)?;
            let x = super::exprs::num_recv(&paren_num(&x), true);
            return Ok(E::owned(
                format!("{x}.{}()", if nan { "is_nan" } else { "is_finite" }),
                PREC_POSTFIX,
            ));
        }
        self.expr(arg)?;
        Ok(E::atom(if nan { "false" } else { "true" }, Repr::Owned))
    }

    /// The receiver of a method chain: the expression, parenthesized if needed.
    pub(super) fn recv(&mut self, e: &'a Expr) -> GResult<String> {
        let x = self.expr(e)?;
        Ok(Self::paren(&x, PREC_POSTFIX))
    }

    /// An argument as an `f64` expression.
    fn float_arg(&mut self, a: &'a Expr) -> GResult<String> {
        let at = self.ty_of(a);
        let x = self.expr_as(a, Want::Owned)?;
        Ok(if self.is_float(&at) {
            super::exprs::num_recv(&paren_num(&x), true)
        } else if super::is_int_literal(&x) {
            format!("{x}.0_f64")
        } else {
            format!("({x} as f64)")
        })
    }

    /// `(_, i) => …` as a closure of one parameter, the index.
    fn closure_with_index(&mut self, f: &'a Function) -> GResult<String> {
        if f.params.len() < 2 {
            let c = self.closure(f, &[Repr::Owned], false)?;
            return Ok(format!("|_| ({})()", c.code));
        }
        let c = self.closure(f, &[Repr::Owned, Repr::Owned], false)?;
        Ok(format!("|i| ({})((), i)", c.code))
    }

    // ----- methods of builtin types -----

    fn method_builtin(
        &mut self,
        e: &'a Expr,
        obj: &'a Expr,
        name: &str,
        args: &'a [Arg],
    ) -> GResult<E> {
        let (kind, method) = name.split_once('.').unwrap_or(("", name));
        // Methods that change their receiver borrow a shared one mutably.
        self.want_mut_root = matches!(
            method,
            "push"
                | "pop"
                | "shift"
                | "unshift"
                | "splice"
                | "sort"
                | "reverse"
                | "fill"
                | "set"
                | "delete"
                | "clear"
                | "add"
                | "next"
        );
        match kind {
            "generator" => {
                let g = self.expr_as(obj, Want::RefMut)?;
                let g = g.trim_start_matches("&mut ").to_string();
                Ok(E::owned(format!("{g}.next()"), PREC_POSTFIX))
            }
            "string" => self.string_method(e, obj, method, args),
            "array" => self.array_method(e, obj, method, args),
            "map" => self.map_method(e, obj, method, args),
            "set" => self.set_method(e, obj, method, args),
            "number" => {
                let ot = self.ty_of(obj);
                let x = self.expr_as(obj, Want::Owned)?;
                let x = paren_num(&x);
                Ok(match method {
                    "toFixed" => {
                        let d = match args.first() {
                            Some(a) => self.expr_as(&a.expr, Want::Owned)?,
                            None => "0".into(),
                        };
                        let x = if self.is_float(&ot) {
                            x
                        } else {
                            format!("({x} as f64)")
                        };
                        E::owned(
                            format!("format!(\"{{:.*}}\", {d} as usize, {x})"),
                            PREC_ATOM,
                        )
                    }
                    "toString" => match args.first() {
                        Some(a) if self.is_int(&ot) => {
                            let r = self.expr_as(&a.expr, Want::Owned)?;
                            let h = self.helper(Helper::NumToString);
                            E::owned(format!("{h}({x}, {r})"), PREC_POSTFIX)
                        }
                        _ => E::owned(format!("{x}.to_string()"), PREC_POSTFIX),
                    },
                    "toPrecision" => {
                        let d = self.expr_as(&args[0].expr, Want::Owned)?;
                        E::owned(
                            format!(
                                "format!(\"{{:.*}}\", ({d} as usize).saturating_sub(1), {x} as f64)"
                            ),
                            PREC_ATOM,
                        )
                    }
                    _ => return self.err(format!("`{name}` is not supported"), e.span),
                })
            }
            "boolean" | "error" => {
                let x = self.expr_as(obj, Want::Display)?;
                Ok(E::owned(format!("{x}.to_string()"), PREC_POSTFIX))
            }
            _ => self.err(format!("`{name}` is not supported"), e.span),
        }
    }

    fn string_method(
        &mut self,
        e: &'a Expr,
        obj: &'a Expr,
        method: &str,
        args: &'a [Arg],
    ) -> GResult<E> {
        let ot = self.ty_of(obj);
        let mut o = self.expr(obj)?;
        if o.repr == Repr::Char {
            o = E::owned(
                format!("{}.to_string()", Self::paren(&o, PREC_POSTFIX)),
                PREC_POSTFIX,
            );
        }
        let s = self.coerce(o.clone(), &ot, Want::StrRef);
        let recv = Self::paren(&o, PREC_POSTFIX);
        let arg_s =
            |g: &mut Self, i: usize| -> GResult<String> { g.expr_as(&args[i].expr, Want::StrRef) };
        let arg_n =
            |g: &mut Self, i: usize| -> GResult<String> { g.expr_as(&args[i].expr, Want::Owned) };
        Ok(match method {
            "toUpperCase" => E::owned(format!("{recv}.to_uppercase()"), PREC_POSTFIX),
            "toLowerCase" => E::owned(format!("{recv}.to_lowercase()"), PREC_POSTFIX),
            "trim" => E::owned(format!("{recv}.trim().to_string()"), PREC_POSTFIX),
            "trimStart" | "trimLeft" => {
                E::owned(format!("{recv}.trim_start().to_string()"), PREC_POSTFIX)
            }
            "trimEnd" | "trimRight" => {
                E::owned(format!("{recv}.trim_end().to_string()"), PREC_POSTFIX)
            }
            "toString" | "valueOf" | "normalize" => {
                E::owned(format!("{recv}.to_string()"), PREC_POSTFIX)
            }
            "split" => {
                if let ExprKind::Str(sep) = &args[0].expr.kind {
                    if sep.is_empty() {
                        return Ok(E::owned(
                            format!("{recv}.chars().map(String::from).collect::<Vec<String>>()"),
                            PREC_POSTFIX,
                        ));
                    }
                    let sep = string_literal(sep);
                    return Ok(E::owned(
                        format!("{recv}.split({sep}).map(String::from).collect::<Vec<String>>()"),
                        PREC_POSTFIX,
                    ));
                }
                let sep = arg_s(self, 0)?;
                let h = self.helper(Helper::StrSplit);
                E::owned(format!("{h}({s}, {sep})"), PREC_POSTFIX)
            }
            "includes" => {
                let t = arg_s(self, 0)?;
                E::owned(format!("{recv}.contains({t})"), PREC_POSTFIX)
            }
            "startsWith" => {
                let t = arg_s(self, 0)?;
                E::owned(format!("{recv}.starts_with({t})"), PREC_POSTFIX)
            }
            "endsWith" => {
                let t = arg_s(self, 0)?;
                E::owned(format!("{recv}.ends_with({t})"), PREC_POSTFIX)
            }
            "indexOf" => {
                let t = arg_s(self, 0)?;
                let h = self.helper(Helper::StrIndexOf);
                E::owned(format!("{h}({s}, {t})"), PREC_POSTFIX)
            }
            "lastIndexOf" => {
                let t = arg_s(self, 0)?;
                let h = self.helper(Helper::StrLastIndexOf);
                E::owned(format!("{h}({s}, {t})"), PREC_POSTFIX)
            }
            "slice" | "substring" | "substr" => {
                let a = match args.first() {
                    Some(_) => arg_n(self, 0)?,
                    None => "0".into(),
                };
                let b = match args.get(1) {
                    Some(_) => {
                        let n = arg_n(self, 1)?;
                        if method == "substr" {
                            format!("Some({a} + {n})")
                        } else {
                            format!("Some({n})")
                        }
                    }
                    None => "None".into(),
                };
                let h = self.helper(if method == "substring" {
                    Helper::StrSubstring
                } else {
                    Helper::StrSlice
                });
                E::owned(format!("{h}({s}, {a}, {b})"), PREC_POSTFIX)
            }
            "charAt" | "at" => {
                let i = arg_n(self, 0)?;
                let h = self.helper(Helper::StrCharAt);
                E::owned(format!("{h}({s}, {i})"), PREC_POSTFIX)
            }
            "charCodeAt" | "codePointAt" => {
                let i = match args.first() {
                    Some(_) => arg_n(self, 0)?,
                    None => "0".into(),
                };
                let h = self.helper(Helper::StrCharCodeAt);
                E::owned(format!("{h}({s}, {i})"), PREC_POSTFIX)
            }
            "repeat" => {
                let n = self.expr_as(&args[0].expr, Want::Index)?;
                E::owned(format!("{recv}.repeat({n})"), PREC_POSTFIX)
            }
            "replace" => {
                let a = arg_s(self, 0)?;
                let b = arg_s(self, 1)?;
                E::owned(format!("{recv}.replacen({a}, {b}, 1)"), PREC_POSTFIX)
            }
            "replaceAll" => {
                let a = arg_s(self, 0)?;
                let b = arg_s(self, 1)?;
                E::owned(format!("{recv}.replace({a}, {b})"), PREC_POSTFIX)
            }
            "padStart" | "padEnd" => {
                let n = arg_n(self, 0)?;
                let pad = match args.get(1) {
                    Some(_) => arg_s(self, 1)?,
                    None => "\" \"".into(),
                };
                let h = self.helper(if method == "padStart" {
                    Helper::StrPadStart
                } else {
                    Helper::StrPadEnd
                });
                E::owned(format!("{h}({s}, {n}, {pad})"), PREC_POSTFIX)
            }
            "concat" => {
                let mut fmt = String::from("{}");
                let mut list = vec![self.coerce(o, &ot, Want::Display)];
                for a in args {
                    let (spec, arg) = self.format_arg(&a.expr)?;
                    fmt.push_str(&spec);
                    list.extend(arg);
                }
                E::owned(format_call("format!", &fmt, &list), PREC_ATOM)
            }
            "localeCompare" => {
                let t = arg_s(self, 0)?;
                let h = self.helper(Helper::Compare);
                E::owned(format!("{h}({s}, {t})"), PREC_POSTFIX)
            }
            _ => return self.err(format!("`string.{method}` is not supported"), e.span),
        })
    }

    fn map_method(
        &mut self,
        e: &'a Expr,
        obj: &'a Expr,
        method: &str,
        args: &'a [Arg],
    ) -> GResult<E> {
        let ot = self.ty_of(obj);
        let Ty::Map(k, v, _) = self.resolve(&ot) else {
            unreachable!()
        };
        let key_want = if self.is_str(&k) {
            Want::StrRef
        } else {
            Want::Ref
        };
        let o = self.expr(obj)?;
        let recv = Self::paren(&o, PREC_POSTFIX);
        Ok(match method {
            "get" => {
                let key = self.expr_as(&args[0].expr, key_want)?;
                if self.is_copy(&v) {
                    E::owned(format!("{recv}.get({key}).copied()"), PREC_POSTFIX)
                } else {
                    E::owned(format!("{recv}.get({key}).cloned()"), PREC_POSTFIX)
                }
            }
            "set" => {
                let key = self.expr_to(&args[0].expr, &k)?;
                let val = self.expr_to(&args[1].expr, &v)?;
                E::owned(format!("{recv}.insert({key}, {val})"), PREC_POSTFIX)
            }
            "has" => {
                let key = self.expr_as(&args[0].expr, key_want)?;
                E::owned(format!("{recv}.contains_key({key})"), PREC_POSTFIX)
            }
            "delete" => {
                let key = self.expr_as(&args[0].expr, key_want)?;
                E::owned(format!("{recv}.remove({key}).is_some()"), PREC_POSTFIX)
            }
            "clear" => E::owned(format!("{recv}.clear()"), PREC_POSTFIX),
            "keys" => E::owned(
                format!("{recv}.keys().cloned().collect::<Vec<_>>()"),
                PREC_POSTFIX,
            ),
            "values" => E::owned(
                format!("{recv}.values().cloned().collect::<Vec<_>>()"),
                PREC_POSTFIX,
            ),
            "entries" => E::owned(
                format!("{recv}.iter().map(|(k, v)| (k.clone(), v.clone())).collect::<Vec<_>>()"),
                PREC_POSTFIX,
            ),
            "forEach" => {
                let (ExprKind::Arrow(f) | ExprKind::Func(f)) = &args[0].expr.kind else {
                    return self.err(
                        "`forEach` needs an inline arrow function",
                        args[0].expr.span,
                    );
                };
                let c = self.closure(f, &[Repr::Ref, Repr::Ref], false)?;
                E::owned(
                    format!("{recv}.iter().for_each(|(k, v)| ({})(v, k))", c.code),
                    PREC_POSTFIX,
                )
            }
            _ => return self.err(format!("`map.{method}` is not supported"), e.span),
        })
    }

    fn set_method(
        &mut self,
        e: &'a Expr,
        obj: &'a Expr,
        method: &str,
        args: &'a [Arg],
    ) -> GResult<E> {
        let ot = self.ty_of(obj);
        let Ty::Set(item) = self.resolve(&ot) else {
            unreachable!()
        };
        let want = if self.is_str(&item) {
            Want::StrRef
        } else {
            Want::Ref
        };
        let o = self.expr(obj)?;
        let recv = Self::paren(&o, PREC_POSTFIX);
        Ok(match method {
            "add" => {
                let x = self.expr_to(&args[0].expr, &item)?;
                E::owned(format!("{recv}.insert({x})"), PREC_POSTFIX)
            }
            "has" => {
                let x = self.expr_as(&args[0].expr, want)?;
                E::owned(format!("{recv}.contains({x})"), PREC_POSTFIX)
            }
            "delete" => {
                let x = self.expr_as(&args[0].expr, want)?;
                E::owned(format!("{recv}.remove({x})"), PREC_POSTFIX)
            }
            "clear" => E::owned(format!("{recv}.clear()"), PREC_POSTFIX),
            "values" | "keys" => E::owned(
                format!("{recv}.iter().cloned().collect::<Vec<_>>()"),
                PREC_POSTFIX,
            ),
            "forEach" => {
                let (ExprKind::Arrow(f) | ExprKind::Func(f)) = &args[0].expr.kind else {
                    return self.err(
                        "`forEach` needs an inline arrow function",
                        args[0].expr.span,
                    );
                };
                let c = self.closure(f, &[Repr::Ref], false)?;
                E::owned(format!("{recv}.iter().for_each({})", c.code), PREC_POSTFIX)
            }
            _ => return self.err(format!("`set.{method}` is not supported"), e.span),
        })
    }

    fn array_method(
        &mut self,
        e: &'a Expr,
        obj: &'a Expr,
        method: &str,
        args: &'a [Arg],
    ) -> GResult<E> {
        let ot = self.ty_of(obj);
        let Ty::Array(item) = self.resolve(&ot) else {
            unreachable!()
        };
        let copy = self.is_copy(&item);
        let o = self.expr(obj)?;
        let recv = Self::paren(&o, PREC_POSTFIX);
        // Iterating: by value over a temporary, by reference otherwise.
        let temp = o.repr == Repr::Owned && o.var.is_none();
        let iter = if temp {
            format!("{recv}.into_iter()")
        } else {
            format!("{recv}.iter()")
        };
        let elem_repr = if temp { Repr::Owned } else { Repr::Ref };
        let cb = |g: &mut Self, i: usize, reprs: &[Repr]| -> GResult<String> {
            match &args[i].expr.kind {
                ExprKind::Arrow(f) | ExprKind::Func(f) => Ok(g.closure(f, reprs, false)?.code),
                _ => {
                    // A named function: wrapped so that each element is passed the way the
                    // function takes it.
                    let f = g.expr(&args[i].expr)?;
                    let modes: Vec<Mode> = match g.p.refs.get(&args[i].expr.id) {
                        Some(b) if g.p.var(*b).role == crate::infer::Role::Func => {
                            let fn_id = g.p.fn_bindings[b];
                            let info = g.p.funcs[&fn_id].clone();
                            info.sig
                                .params
                                .iter()
                                .enumerate()
                                .map(|(j, pt)| {
                                    let (stored, reassigned, mutated) = info
                                        .params
                                        .get(j)
                                        .map(|b| {
                                            let v = g.p.var(*b);
                                            (v.stored, v.reassigned, v.mutated)
                                        })
                                        .unwrap_or((true, false, false));
                                    g.param_mode(pt, stored, reassigned, mutated, &info.type_params)
                                })
                                .collect()
                        }
                        _ => Vec::new(),
                    };
                    let n = reprs.len().max(1);
                    let params: Vec<String> = (0..n).map(|i| format!("a{i}")).collect();
                    let mut call_args = Vec::new();
                    for (j, (p, r)) in params.iter().zip(reprs).enumerate() {
                        let mode = modes.get(j).copied().unwrap_or(Mode::Value);
                        call_args.push(match (mode, *r) {
                            (Mode::RefMut, _) => {
                                return g.err(
                                    "this function changes its argument: call it in a loop instead",
                                    args[i].expr.span,
                                );
                            }
                            (Mode::Value, Repr::Ref | Repr::Place) if copy => format!("*{p}"),
                            (Mode::Value, Repr::Ref | Repr::Place) => format!("{p}.clone()"),
                            (Mode::Value | Mode::FnImpl, _) => p.clone(),
                            (Mode::StrRef, Repr::Owned) => format!("&{p}"),
                            (_, Repr::Owned) => format!("&{p}"),
                            (_, Repr::Usize) => format!("{p} as i64"),
                            _ => p.clone(),
                        });
                    }
                    Ok(format!(
                        "|{}| {}({})",
                        params.join(", "),
                        f.code,
                        call_args.join(", ")
                    ))
                }
            }
        };
        let uses_index = |g: &Self, i: usize| -> bool {
            match &args[i].expr.kind {
                ExprKind::Arrow(f) | ExprKind::Func(f) => f.params.len() >= 2,
                _ => {
                    let _ = g;
                    false
                }
            }
        };
        Ok(match method {
            "push" => {
                let mut parts = Vec::new();
                for a in args {
                    if a.spread {
                        let v = self.expr_as(&a.expr, Want::Owned)?;
                        parts.push(format!("{recv}.extend({v})"));
                    } else {
                        let v = self.expr_to(&a.expr, &item)?;
                        parts.push(format!("{recv}.push({v})"));
                    }
                }
                E::owned(
                    parts.join("; "),
                    if parts.len() == 1 {
                        PREC_POSTFIX
                    } else {
                        PREC_ATOM
                    },
                )
            }
            "unshift" => {
                let v = self.expr_to(&args[0].expr, &item)?;
                E::owned(format!("{recv}.insert(0, {v})"), PREC_POSTFIX)
            }
            "pop" => E::owned(format!("{recv}.pop()"), PREC_POSTFIX),
            "shift" => {
                let h = self.helper(Helper::VecShift);
                let r = self.coerce(o, &ot, Want::RefMut);
                E::owned(format!("{h}({r})"), PREC_POSTFIX)
            }
            "indexOf" | "lastIndexOf" => {
                let x = self.expr_as(&args[0].expr, Want::Ref)?;
                let h = self.helper(if method == "indexOf" {
                    Helper::VecIndexOf
                } else {
                    Helper::VecLastIndexOf
                });
                let v = self.coerce(o, &ot, Want::Slice);
                E::owned(format!("{h}({v}, {x})"), PREC_POSTFIX)
            }
            "includes" => {
                let x = self.expr_as(&args[0].expr, Want::Ref)?;
                E::owned(format!("{recv}.contains({x})"), PREC_POSTFIX)
            }
            "map" => {
                if uses_index(self, 0) {
                    let c = cb(self, 0, &[elem_repr, Repr::Usize])?;
                    E::owned(
                        format!("{iter}.enumerate().map(|(i, x)| ({c})(x, i)).collect::<Vec<_>>()"),
                        PREC_POSTFIX,
                    )
                } else {
                    let c = cb(self, 0, &[elem_repr])?;
                    E::owned(format!("{iter}.map({c}).collect::<Vec<_>>()"), PREC_POSTFIX)
                }
            }
            "flatMap" => {
                let c = cb(self, 0, &[elem_repr])?;
                E::owned(
                    format!("{iter}.flat_map({c}).collect::<Vec<_>>()"),
                    PREC_POSTFIX,
                )
            }
            "filter" => {
                let c = cb(self, 0, &[if temp { Repr::Ref } else { Repr::Place }])?;
                if temp {
                    E::owned(
                        format!("{iter}.filter({c}).collect::<Vec<_>>()"),
                        PREC_POSTFIX,
                    )
                } else {
                    E::owned(
                        format!("{iter}.filter({c}).cloned().collect::<Vec<_>>()"),
                        PREC_POSTFIX,
                    )
                }
            }
            "find" | "findLast" => {
                let c = cb(self, 0, &[if temp { Repr::Ref } else { Repr::Place }])?;
                let it = if method == "findLast" {
                    format!("{iter}.rev()")
                } else {
                    iter
                };
                if temp {
                    E::owned(format!("{it}.find({c})"), PREC_POSTFIX)
                } else {
                    E::owned(format!("{it}.find({c}).cloned()"), PREC_POSTFIX)
                }
            }
            "findIndex" | "findLastIndex" => {
                let c = cb(self, 0, &[elem_repr])?;
                let f = if method == "findIndex" {
                    "position"
                } else {
                    "rposition"
                };
                E::owned(
                    format!("{iter}.{f}({c}).map_or(-1, |i| i as i64)"),
                    PREC_POSTFIX,
                )
            }
            "some" | "every" => {
                let c = cb(self, 0, &[elem_repr])?;
                let f = if method == "some" { "any" } else { "all" };
                E::owned(format!("{iter}.{f}({c})"), PREC_POSTFIX)
            }
            "forEach" => {
                if uses_index(self, 0) {
                    let c = cb(self, 0, &[elem_repr, Repr::Usize])?;
                    E::owned(
                        format!("{iter}.enumerate().for_each(|(i, x)| ({c})(x, i))"),
                        PREC_POSTFIX,
                    )
                } else {
                    let c = cb(self, 0, &[elem_repr])?;
                    E::owned(format!("{iter}.for_each({c})"), PREC_POSTFIX)
                }
            }
            "reduce" | "reduceRight" => {
                let ty = self.ty_of(e);
                let init = match args.get(1) {
                    Some(a) => self.expr_to(&a.expr, &ty)?,
                    None => return self.err("`reduce` needs an initial value here", e.span),
                };
                let c = cb(self, 0, &[Repr::Owned, elem_repr])?;
                let it = if method == "reduceRight" {
                    format!("{iter}.rev()")
                } else {
                    iter
                };
                E::owned(format!("{it}.fold({init}, {c})"), PREC_POSTFIX)
            }
            "join" => {
                let sep = match args.first() {
                    Some(a) => self.expr_as(&a.expr, Want::StrRef)?,
                    None => "\",\"".into(),
                };
                if self.is_str(&item) {
                    E::owned(format!("{recv}.join({sep})"), PREC_POSTFIX)
                } else {
                    let shown = self.string_of("x", &item);
                    E::owned(
                        format!("{recv}.iter().map(|x| {shown}).collect::<Vec<_>>().join({sep})"),
                        PREC_POSTFIX,
                    )
                }
            }
            "slice" => {
                let v = self.coerce(o, &ot, Want::Slice);
                let a = match args.first() {
                    Some(a) => self.expr_as(&a.expr, Want::Owned)?,
                    None => "0".into(),
                };
                let b = match args.get(1) {
                    Some(b) => format!("Some({})", self.expr_as(&b.expr, Want::Owned)?),
                    None => "None".into(),
                };
                if args.len() == 2 {
                    if let (Some(x), Some(y)) = (
                        literal_usize(&a),
                        literal_usize(b.trim_start_matches("Some(").trim_end_matches(')')),
                    ) {
                        return Ok(E::owned(format!("{v}[{x}..{y}].to_vec()"), PREC_POSTFIX));
                    }
                }
                let h = self.helper(Helper::VecSlice);
                E::owned(format!("{h}({v}, {a}, {b})"), PREC_POSTFIX)
            }
            "splice" => {
                let v = self.coerce(o, &ot, Want::RefMut);
                let start = self.expr_as(&args[0].expr, Want::Owned)?;
                let count = match args.get(1) {
                    Some(c) => format!("Some({})", self.expr_as(&c.expr, Want::Owned)?),
                    None => "None".into(),
                };
                let mut items = Vec::new();
                for a in args.iter().skip(2) {
                    items.push(self.expr_to(&a.expr, &item)?);
                }
                let h = self.helper(Helper::VecSplice);
                E::owned(
                    format!("{h}({v}, {start}, {count}, vec![{}])", items.join(", ")),
                    PREC_POSTFIX,
                )
            }
            "concat" => {
                let mut parts = vec![self.coerce(o, &ot, Want::Slice)];
                for a in args {
                    let at = self.ty_of(&a.expr);
                    if matches!(self.resolve(&at), Ty::Array(_)) {
                        parts.push(self.expr_as(&a.expr, Want::Slice)?);
                    } else {
                        let x = self.expr_to(&a.expr, &item)?;
                        parts.push(format!("&[{x}][..]"));
                    }
                }
                E::owned(format!("[{}].concat()", parts.join(", ")), PREC_POSTFIX)
            }
            "reverse" => E::owned(format!("{recv}.reverse()"), PREC_POSTFIX),
            "toReversed" => E::owned(
                format!("{recv}.iter().rev().cloned().collect::<Vec<_>>()"),
                PREC_POSTFIX,
            ),
            "sort" | "toSorted" => {
                let target = if method == "sort" {
                    recv.clone()
                } else {
                    "v".to_string()
                };
                let sorting = match args.first() {
                    None => {
                        if self.is_str(&item) {
                            format!("{target}.sort()")
                        } else {
                            // JavaScript sorts numbers as strings.
                            format!("{target}.sort_by_key(|x| x.to_string())")
                        }
                    }
                    Some(a) => {
                        let (ExprKind::Arrow(f) | ExprKind::Func(f)) = &a.expr.kind else {
                            return self.err("`sort` needs an inline comparator", a.expr.span);
                        };
                        let cmp = self.comparator(f, &item)?;
                        format!("{target}.sort_by({cmp})")
                    }
                };
                if method == "sort" {
                    E::owned(sorting, PREC_POSTFIX)
                } else {
                    let v = self.coerce(o, &ot, Want::Owned);
                    E::owned(format!("{{ let mut v = {v}; {sorting}; v }}"), PREC_ATOM)
                }
            }
            "fill" => {
                let v = self.expr_to(&args[0].expr, &item)?;
                E::owned(format!("{recv}.fill({v})"), PREC_POSTFIX)
            }
            "at" => {
                let v = self.coerce(o, &ot, Want::Slice);
                let i = self.expr_as(&args[0].expr, Want::Owned)?;
                let h = self.helper(Helper::VecAt);
                E::owned(format!("{h}({v}, {i})"), PREC_POSTFIX)
            }
            "flat" => E::owned(format!("{recv}.concat()"), PREC_POSTFIX),
            "entries" => E::owned(format!("{iter}.enumerate()"), PREC_POSTFIX),
            "keys" => E::owned(format!("(0..{recv}.len())"), PREC_ATOM),
            "values" => E::owned(iter, PREC_POSTFIX),
            "toString" => {
                let shown = self.string_of("x", &item);
                E::owned(
                    format!("{recv}.iter().map(|x| {shown}).collect::<Vec<_>>().join(\",\")"),
                    PREC_POSTFIX,
                )
            }
            _ => return self.err(format!("`array.{method}` is not supported"), e.span),
        })
    }

    /// `(a, b) => a - b` as a `sort_by` closure.
    fn comparator(&mut self, f: &'a Function, item: &Ty) -> GResult<String> {
        let info = self.p.funcs[&f.id].clone();
        self.push_scope();
        let mut names = Vec::new();
        for (i, _) in f.params.iter().enumerate() {
            names.push(self.bind(info.params[i], Repr::Ref));
        }
        let (a, b) = (
            names.first().cloned().unwrap_or("a".into()),
            names.get(1).cloned().unwrap_or("b".into()),
        );
        self.fns.push(super::FnGen {
            id: f.id,
            returns_result: false,
            ret_ty: info.sig.ret.clone(),
            yield_ty: None,
            targets: Vec::new(),
            try_ctx: Vec::new(),
            in_ctor: false,
            this_name: self.fns.last().map_or("self", |c| c.this_name),
            type_params: Vec::new(),
            bounds: Default::default(),
            class: None,
            labels: 0,
        });
        let body: &Expr = match &f.body {
            FnBody::Expr(x) => x,
            FnBody::Block(b) => match b.stmts.as_slice() {
                [
                    Stmt {
                        kind: StmtKind::Return(Some(x)),
                        ..
                    },
                ] => x,
                _ => {
                    self.fns.pop();
                    self.pop_scope();
                    return self.err("comparators must be a single expression", f.span);
                }
            },
        };
        let result = (|| -> GResult<String> {
            if let ExprKind::Binary {
                op: BinOp::Sub,
                left,
                right,
            } = &body.kind
            {
                let lt = self.ty_of(left);
                let l = self.expr(left)?;
                let r = self.expr(right)?;
                let lc = Self::paren(&l, PREC_POSTFIX);
                let rc = match r.repr {
                    Repr::Ref | Repr::RefMut => r.code.clone(),
                    _ => format!("&{}", Self::paren(&r, PREC_UNARY)),
                };
                if self.is_float(&lt) {
                    return Ok(format!(
                        "|{a}, {b}| {lc}.partial_cmp({rc}).unwrap_or(std::cmp::Ordering::Equal)"
                    ));
                }
                return Ok(format!("|{a}, {b}| {lc}.cmp({rc})"));
            }
            if let ExprKind::Call { callee, args, .. } = &body.kind
                && let ExprKind::Member { obj, prop, .. } = &callee.kind
                && prop == "localeCompare"
                && args.len() == 1
            {
                let l = self.expr_as(obj, Want::StrRef)?;
                let r = self.expr_as(&args[0].expr, Want::StrRef)?;
                return Ok(format!("|{a}, {b}| {l}.cmp({r})"));
            }
            let _ = item;
            let n = self.expr_as(body, Want::Owned)?;
            let h = self.helper(Helper::Ordering);
            let bt = self.ty_of(body);
            let n = if self.is_float(&bt) {
                n
            } else {
                format!("{n} as f64")
            };
            Ok(format!("|{a}, {b}| {h}({n})"))
        })();
        self.fns.pop();
        self.pop_scope();
        result
    }
}

fn paren_num(code: &str) -> String {
    if code.starts_with('-') || code.contains(' ') {
        format!("({code})")
    } else {
        code.to_string()
    }
}

fn is_literal(e: &Expr) -> bool {
    matches!(
        e.kind,
        ExprKind::Num { .. }
            | ExprKind::Str(_)
            | ExprKind::Bool(_)
            | ExprKind::Null
            | ExprKind::Undefined
    ) || matches!(&e.kind, ExprKind::Unary { op: UnaryOp::Neg, arg } if matches!(arg.kind, ExprKind::Num { .. }))
        || matches!(&e.kind, ExprKind::Array(elems) if elems.is_empty())
}

fn literal_usize(code: &str) -> Option<&str> {
    if super::is_int_literal(code) {
        Some(code)
    } else {
        None
    }
}
