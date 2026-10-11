//! Statements: declarations, control flow, `try`, `switch`.

use super::exprs::narrow_path_expr;
use super::{
    E, GResult, Gen, PREC_ATOM, PREC_POSTFIX, PREC_UNARY, Repr, Target, TryCtx, Want, pascal, snake,
};
#[allow(unused_imports)]
use crate::ast::NodeId;
use crate::ast::*;
use crate::infer::{Callee, Complement, Role};
use crate::types::Ty;

impl<'a> Gen<'a> {
    /// The statements of a function body or block. With `tail`, a final `return e` becomes
    /// the tail expression, and a `Result` function ends with `Ok(())` when needed.
    pub(super) fn block_body(&mut self, stmts: &'a [Stmt], tail: bool) -> GResult<()> {
        let narrowed_before = self.narrowed.len();
        self.push_scope();
        // Hoisted function items first: Rust needs closures declared before use, but items
        // are fine anywhere; nothing to do for them.
        let last = stmts.len().saturating_sub(1);
        for (i, s) in stmts.iter().enumerate() {
            if tail && i == last {
                if let StmtKind::Return(value) = &s.kind {
                    self.comments(s);
                    match value {
                        Some(e) => {
                            let code = self.return_value(e)?;
                            self.line(&code);
                        }
                        None => {
                            if self.fn_ctx().returns_result && self.fn_ctx().try_ctx.is_empty() {
                                self.line("Ok(())");
                            }
                        }
                    }
                    self.pop_scope();
                    self.narrowed.truncate(narrowed_before);
                    return Ok(());
                }
            }
            self.stmt(s)?;
        }
        if tail
            && self.fn_ctx().try_ctx.is_empty()
            && self.fn_ctx().returns_result
            && !ends_with_exit(stmts)
        {
            self.line("Ok(())");
        }
        self.pop_scope();
        self.narrowed.truncate(narrowed_before);
        Ok(())
    }

    fn comments(&mut self, s: &Stmt) {
        for c in &s.comments {
            let text = format!("// {c}");
            self.line(text.trim_end());
        }
    }

    /// The value of `return e`, wrapped as the function needs.
    fn return_value(&mut self, e: &'a Expr) -> GResult<String> {
        let ret = self.fn_ctx().ret_ty.clone();
        let id = self.fn_ctx().id;
        let code = if matches!(e.kind, ExprKind::This)
            && self.p.funcs.get(&id).is_some_and(|f| self.fluent(f))
        {
            "self".to_string()
        } else if matches!(self.resolve(&ret), Ty::Func(_)) {
            // A returned closure is `impl Fn`, not kept behind an `Rc`.
            let x = self.expr(e)?;
            let t = self.ty_of(e);
            self.owned(x, &t)
        } else {
            self.expr_to(e, &ret)?
        };
        let in_try = self.fn_ctx().try_ctx.last().is_some_and(|t| t.returns);
        Ok(if in_try {
            format!("Ok(Some({code}))")
        } else if self.fn_ctx().returns_result {
            format!("Ok({code})")
        } else {
            code
        })
    }

    pub(super) fn stmt(&mut self, s: &'a Stmt) -> GResult<()> {
        self.comments(s);
        match &s.kind {
            StmtKind::Var(v) => self.var_decl(v),
            StmtKind::Func(f) => {
                if f.id == usize::MAX {
                    return Ok(());
                }
                let info = &self.p.funcs[&f.id];
                if info.is_closure {
                    let binding = *self
                        .p
                        .fn_bindings
                        .iter()
                        .find(|(_, id)| **id == f.id)
                        .map(|(b, _)| b)
                        .expect("binding");
                    let c = self.closure(f, &[], true)?;
                    let mutates = info
                        .captures
                        .iter()
                        .any(|c| self.p.var(*c).mutated_in_closure);
                    let name = self.bind(binding, Repr::Owned);
                    let m = if mutates { "mut " } else { "" };
                    self.line(&format!("let {m}{name} = {};", c.code));
                    return Ok(());
                }
                self.blank();
                self.function_item(f, None, None)?;
                Ok(())
            }
            StmtKind::Class(c) => self.err(
                format!("class `{}` must be declared at the top level", c.name),
                c.span,
            ),
            StmtKind::Interface(_)
            | StmtKind::TypeAlias(_)
            | StmtKind::Enum(_)
            | StmtKind::Empty
            | StmtKind::ExportNames(_)
            | StmtKind::ExportFrom { .. }
            | StmtKind::Import(_) => Ok(()),
            StmtKind::Return(value) => {
                match value {
                    Some(e) => {
                        let code = self.return_value(e)?;
                        self.line(&format!("return {code};"));
                    }
                    None => {
                        if self.fn_ctx().try_ctx.last().is_some_and(|t| t.returns) {
                            self.line("return Ok(Some(()));");
                        } else if self.fn_ctx().returns_result || !self.fn_ctx().try_ctx.is_empty()
                        {
                            self.line("return Ok(());");
                        } else {
                            self.line("return;");
                        }
                    }
                }
                Ok(())
            }
            StmtKind::If {
                test,
                then,
                otherwise,
            } => self.if_stmt(test, then, otherwise.as_deref()),
            StmtKind::While { test, body } => {
                if matches!(test.kind, ExprKind::Bool(true)) {
                    self.loop_stmt("loop", body, None)?;
                    return Ok(());
                }
                let c = self.cond(test)?;
                self.loop_stmt(&format!("while {c}"), body, None)
            }
            StmtKind::DoWhile { body, test } => {
                let text = self.capture(|g| {
                    g.indent += 1;
                    let c = g.cond(test)?;
                    g.line(&format!("if !({c}) {{"));
                    g.indent += 1;
                    g.line("break;");
                    g.indent -= 1;
                    g.line("}");
                    g.indent -= 1;
                    Ok(())
                })?;
                self.loop_stmt_with_tail("loop", body, None, &text)
            }
            StmtKind::For {
                init,
                test,
                update,
                body,
            } => self.for_stmt(init.as_deref(), test.as_ref(), update.as_ref(), body, None),
            StmtKind::ForOf {
                pat,
                iter,
                body,
                is_in,
                ..
            } => self.for_of(pat, iter, body, *is_in, None),
            StmtKind::Switch { disc, cases } => self.switch_stmt(disc, cases),
            StmtKind::Break(label) => {
                let text = self.break_text(label.as_deref(), s.span)?;
                self.line(&text);
                Ok(())
            }
            StmtKind::Continue(label) => {
                let text = self.continue_text(label.as_deref(), s.span)?;
                self.line(&text);
                Ok(())
            }
            StmtKind::Throw(e) => {
                let msg = self.error_value(e)?;
                if self.fn_ctx().returns_result || !self.fn_ctx().try_ctx.is_empty() {
                    self.line(&format!("return Err({msg});"));
                } else {
                    self.line(&format!("panic!(\"{{}}\", {msg});"));
                }
                Ok(())
            }
            StmtKind::Try {
                block,
                catch,
                finally,
            } => self.try_stmt(block, catch.as_ref(), finally.as_ref()),
            StmtKind::Block(b) => {
                self.line("{");
                self.indent += 1;
                self.block_body(&b.stmts, false)?;
                self.indent -= 1;
                self.line("}");
                Ok(())
            }
            StmtKind::Expr(e) => self.expr_stmt(e),
            StmtKind::ExportDefault(e) => self.err("unsupported export", e.span),
            StmtKind::Labeled { label, body } => match &body.kind {
                StmtKind::For {
                    init,
                    test,
                    update,
                    body,
                } => self.for_stmt(
                    init.as_deref(),
                    test.as_ref(),
                    update.as_ref(),
                    body,
                    Some(label),
                ),
                StmtKind::ForOf {
                    pat,
                    iter,
                    body,
                    is_in,
                    ..
                } => self.for_of(pat, iter, body, *is_in, Some(label)),
                StmtKind::While { test, body } => {
                    let head = if matches!(test.kind, ExprKind::Bool(true)) {
                        "loop".to_string()
                    } else {
                        format!("while {}", self.cond(test)?)
                    };
                    self.loop_stmt(&head, body, Some(label))
                }
                _ => self.err("only loops can be labeled", s.span),
            },
        }
    }

    /// The message of a thrown value, as an owned `String`.
    fn error_value(&mut self, e: &'a Expr) -> GResult<String> {
        let ty = self.ty_of(e);
        let x = self.expr(e)?;
        Ok(self.owned(x, &ty))
    }

    fn break_text(&mut self, label: Option<&str>, span: crate::error::Span) -> GResult<String> {
        if let Some(l) = label {
            for t in self.fn_ctx().targets.iter().rev() {
                if let Target::Loop {
                    label: Some(name),
                    rust_label,
                    ..
                } = t
                    && name == l
                {
                    return Ok(format!("break {};", rust_label.clone().unwrap_or_default()));
                }
            }
            return self.err(format!("unknown label `{l}`"), span);
        }
        match self.fn_ctx_mut().targets.last_mut() {
            Some(Target::Switch { rust_label, used }) => {
                *used = true;
                Ok(format!("break {rust_label};"))
            }
            Some(Target::Loop { .. }) => Ok("break;".into()),
            None => self.err("`break` outside a loop", span),
        }
    }

    fn continue_text(&mut self, label: Option<&str>, span: crate::error::Span) -> GResult<String> {
        let targets = &self.fn_ctx().targets;
        let found = targets.iter().rev().find(|t| match t {
            Target::Loop { label: name, .. } => match label {
                Some(l) => name.as_deref() == Some(l),
                None => true,
            },
            Target::Switch { .. } => false,
        });
        match found {
            Some(Target::Loop {
                rust_label,
                continue_label,
                ..
            }) => {
                if let Some(c) = continue_label {
                    return Ok(format!("break {c};"));
                }
                Ok(match (label, rust_label) {
                    (Some(_), Some(r)) => format!("continue {r};"),
                    _ => "continue;".into(),
                })
            }
            _ => self.err("`continue` outside a loop", span),
        }
    }

    // ----- declarations -----

    fn var_decl(&mut self, v: &'a VarDecl) -> GResult<()> {
        for d in &v.decls {
            match &d.pat {
                Pattern::Ident { id, span, .. } => {
                    let var = self.p.var(*id);
                    if var.role == Role::TopLevel && (var.const_literal || var.used_in_fn) {
                        // A constant or a static, written with the items.
                        continue;
                    }
                    let ty = self.resolve(&var.ty);
                    // Only assigned inside null tests, which bind the value themselves.
                    if self.p.drop_decl.contains(id)
                        && d.init
                            .as_ref()
                            .is_none_or(|i| matches!(i.kind, ExprKind::Null | ExprKind::Undefined))
                    {
                        continue;
                    }
                    let Some(init) = &d.init else {
                        let t = self.rust_ty(&ty, *span)?;
                        let name = self.bind(*id, Repr::Owned);
                        // `let x: T | undefined;` is `undefined` until set: it may be read
                        // before.
                        if matches!(self.resolve(&ty), Ty::Option(_)) {
                            let m = if self.p.var(*id).reassigned || self.p.var(*id).mutated {
                                "mut "
                            } else {
                                ""
                            };
                            self.line(&format!("let {m}{name}: {t} = None;"));
                        } else {
                            self.line(&format!("let {name}: {t};"));
                        }
                        continue;
                    };
                    self.local_from(*id, init, &ty, d.span)?;
                }
                Pattern::Array { elems, rest, span } => {
                    let Some(init) = &d.init else {
                        return self.err("destructuring needs a value", *span);
                    };
                    let it = self.ty_of(init);
                    let plain_names = elems.iter().all(|e| {
                        e.as_ref().is_some_and(|e| {
                            matches!(e.pat, Pattern::Ident { .. }) && e.default.is_none()
                        })
                    });
                    // `let [a, b] = [x, y]`: a tuple on both sides.
                    if let ExprKind::Array(items) = &init.kind
                        && plain_names
                        && rest.is_none()
                        && items.len() == elems.len()
                        && items.iter().all(|i| matches!(i, ArrayElem::Expr(_)))
                    {
                        let mut values = Vec::new();
                        let mut names = Vec::new();
                        for (el, item) in elems.iter().flatten().zip(items) {
                            let ArrayElem::Expr(x) = item else {
                                unreachable!()
                            };
                            let Pattern::Ident { id, .. } = &el.pat else {
                                unreachable!()
                            };
                            let vt = self.resolve(&self.p.var(*id).ty);
                            values.push(self.expr_to(x, &vt)?);
                            let var = self.p.var(*id);
                            let m = if var.reassigned || var.mutated {
                                "mut "
                            } else {
                                ""
                            };
                            names.push(format!("{m}{}", self.bind(*id, Repr::Owned)));
                        }
                        self.line(&format!(
                            "let ({}) = ({});",
                            names.join(", "),
                            values.join(", ")
                        ));
                        continue;
                    }
                    match self.resolve(&it) {
                        Ty::Tuple(_) if plain_names && rest.is_none() => {
                            let value = self.expr_as(init, Want::Owned)?;
                            let mut names = Vec::new();
                            for el in elems.iter().flatten() {
                                let Pattern::Ident { id, .. } = &el.pat else {
                                    unreachable!()
                                };
                                let var = self.p.var(*id);
                                let m = if var.reassigned || var.mutated {
                                    "mut "
                                } else {
                                    ""
                                };
                                names.push(format!("{m}{}", self.bind(*id, Repr::Owned)));
                            }
                            self.line(&format!("let ({}) = {value};", names.join(", ")));
                        }
                        _ => {
                            let base = self.temp_binding(init, "parts")?;
                            self.destructure_array(elems, rest, &base, &it)?;
                        }
                    }
                }
                Pattern::Object { props, span, .. } => {
                    let Some(init) = &d.init else {
                        return self.err("destructuring needs a value", *span);
                    };
                    let base = self.temp_binding(init, "obj")?;
                    let it = self.ty_of(init);
                    self.destructure_object(props, &base, &it)?;
                }
            }
        }
        Ok(())
    }

    /// `{ a, b: c = d }` taken from the variable `base` of type `it`.
    pub(super) fn destructure_object(
        &mut self,
        props: &'a [PatProp],
        base: &str,
        it: &Ty,
    ) -> GResult<()> {
        for p in props {
            let Pattern::Ident { id, .. } = &p.value else {
                return self.err("nested patterns are not supported", p.value.span());
            };
            let var = self.p.var(*id);
            let ty = self.resolve(&var.ty);
            let access = match self.resolve(it) {
                Ty::Map(..) => format!("{base}[{}]", super::string_literal(&p.key)),
                _ => format!("{base}.{}", snake(&p.key)),
            };
            let field_optional = match self.resolve(it) {
                Ty::Named(n, ..) => self
                    .p
                    .struct_info(&n)
                    .and_then(|s| s.field(&p.key))
                    .is_some_and(|f| f.optional),
                _ => false,
            };
            let value = if self.is_copy(&ty) && !field_optional {
                access.clone()
            } else {
                format!("{access}.clone()")
            };
            let value = match &p.default {
                Some(d) => {
                    let dv = self.expr_to(d, &ty)?;
                    format!("{value}.unwrap_or({dv})")
                }
                None => value,
            };
            let m = if var.reassigned || var.mutated {
                "mut "
            } else {
                ""
            };
            let name = self.bind(*id, Repr::Owned);
            self.line(&format!("let {m}{name} = {value};"));
        }
        Ok(())
    }

    /// `[a, b = c, ...rest]` taken from the variable `base` of type `it`.
    pub(super) fn destructure_array(
        &mut self,
        elems: &'a [Option<PatElem>],
        rest: &'a Option<Box<Pattern>>,
        base: &str,
        it: &Ty,
    ) -> GResult<()> {
        for (i, el) in elems.iter().enumerate() {
            let Some(el) = el else { continue };
            let Pattern::Ident { id, .. } = &el.pat else {
                return self.err("nested patterns are not supported", el.pat.span());
            };
            let var = self.p.var(*id);
            let ty = self.resolve(&var.ty);
            let access = if matches!(self.resolve(it), Ty::Tuple(_)) {
                format!("{base}.{i}")
            } else {
                format!("{base}[{i}]")
            };
            let value = if self.is_copy(&ty) {
                access
            } else {
                format!("{access}.clone()")
            };
            let value = match &el.default {
                Some(d) => {
                    let dv = self.expr_to(d, &ty)?;
                    format!("{value}.unwrap_or({dv})")
                }
                None => value,
            };
            let m = if var.reassigned || var.mutated {
                "mut "
            } else {
                ""
            };
            let name = self.bind(*id, Repr::Owned);
            self.line(&format!("let {m}{name} = {value};"));
        }
        // `[a, ...rest]`: the elements left, as a new array.
        if let Some(r) = rest {
            let Pattern::Ident { id, .. } = r.as_ref() else {
                return self.err("nested patterns are not supported", r.span());
            };
            let var = self.p.var(*id);
            let m = if var.reassigned || var.mutated {
                "mut "
            } else {
                ""
            };
            let name = self.bind(*id, Repr::Owned);
            self.line(&format!(
                "let {m}{name} = {base}[{}..].to_vec();",
                elems.len()
            ));
        }
        Ok(())
    }

    /// The value of a local: a closure stays a closure (only fields and collections keep
    /// functions behind an `Rc`).
    fn local_value(&mut self, x: E, it: &Ty, ty: &Ty) -> String {
        if matches!(self.resolve(ty), Ty::Func(_)) {
            self.owned(x, it)
        } else {
            self.owned_as(x, it, ty)
        }
    }

    /// A variable holding an expression to destructure: the variable itself, or a temporary.
    fn temp_binding(&mut self, init: &'a Expr, hint: &str) -> GResult<String> {
        if let ExprKind::Ident(_) = &init.kind {
            let x = self.expr(init)?;
            return Ok(x.code);
        }
        let x = self.expr(init)?;
        let it = self.ty_of(init);
        let value = match x.repr {
            Repr::Place => format!("&{}", Self::paren(&x, PREC_UNARY)),
            _ => self.owned(x, &it),
        };
        let name = format!("{hint}_");
        self.line(&format!("let {name} = {value};"));
        Ok(name)
    }

    /// `let name = init;` with the representation that suits the use of the variable.
    fn local_from(
        &mut self,
        id: NodeId,
        init: &'a Expr,
        ty: &Ty,
        span: crate::error::Span,
    ) -> GResult<()> {
        let var = self.p.var(id).clone();
        let it = self.ty_of(init);
        if var.shared {
            // Shared with a closure that outlives this scope.
            let x = match &init.kind {
                ExprKind::Arrow(f) | ExprKind::Func(f) => self.closure(f, &[], true)?,
                _ => self.expr(init)?,
            };
            let value = self.local_value(x, &it, ty);
            let needs_type = matches!(
                value.as_str(),
                "Vec::new()"
                    | "HashMap::new()"
                    | "HashSet::new()"
                    | "JsMap::new()"
                    | "JsSet::new()"
                    | "None"
                    | "String::new()"
            ) || matches!(self.resolve(&it), Ty::Null)
                || matches!(init.kind, ExprKind::Num { .. }) && self.is_float(ty)
                || self.generic_instance(ty);
            let name = self.bind(id, Repr::Owned);
            self.shared.insert(id);
            self.uses_rc = true;
            if needs_type {
                let t = self.rust_ty(ty, span)?;
                self.line(&format!(
                    "let {name}: Rc<RefCell<{t}>> = Rc::new(RefCell::new({value}));"
                ));
            } else {
                self.line(&format!("let {name} = Rc::new(RefCell::new({value}));"));
            }
            return Ok(());
        }
        let copy = self.is_copy(ty);
        let fixed = !var.reassigned;
        let plain_place = matches!(
            init.kind,
            ExprKind::Index { .. }
                | ExprKind::Member {
                    optional: false,
                    ..
                }
        ) && !matches!(self.resolve(&it), Ty::Named(ref n, ..) if n == "Error");
        // A string literal kept as a `&str`.
        if let ExprKind::Str(s) = &init.kind
            && fixed
            && !var.mutated
            && !var.stored
            && !self.p.enum_literal.contains_key(&init.id)
        {
            let name = self.bind(id, Repr::Str);
            self.line(&format!("let {name} = {};", super::string_literal(s)));
            return Ok(());
        }
        // A reference to a field or an element, when the variable only ever holds such
        // places (`let current = this.head; … current = current.next`).
        if plain_place
            && !copy
            && var.alias
            && !var.stored
            && !var.captured
            && (fixed || !matches!(self.resolve(&it), Ty::Option(_)) || true)
        {
            let x = self.expr(init)?;
            if matches!(x.repr, Repr::Place | Repr::Ref | Repr::RefMut) {
                let m = if var.reassigned { "mut " } else { "" };
                let (prefix, repr) = if var.mutated || x.repr == Repr::RefMut {
                    (
                        if x.repr == Repr::RefMut { "" } else { "&mut " },
                        Repr::RefMut,
                    )
                } else {
                    (if x.repr == Repr::Ref { "" } else { "&" }, Repr::Ref)
                };
                let name = self.bind(id, repr);
                self.line(&format!(
                    "let {m}{name} = {prefix}{};",
                    Self::paren(&x, PREC_UNARY)
                ));
                return Ok(());
            }
            let value = self.local_value(x, &it, ty);
            let m = if var.mutated { "mut " } else { "" };
            let name = self.bind(id, Repr::Owned);
            self.line(&format!("let {m}{name} = {value};"));
            return Ok(());
        }
        // `const id = setInterval(() => { … clearInterval(id) … })`: the id exists first.
        if let ExprKind::Call { args, .. } = &init.kind
            && let Some(Callee::Builtin(name @ ("global.setTimeout" | "global.setInterval"))) =
                self.p.calls.get(&init.id)
            && self.mentions(&args[0].expr, id)
        {
            let repeat = *name == "global.setInterval";
            let name = self.bind(id, Repr::Owned);
            self.helper(super::Helper::Timers);
            self.line(&format!("let {name} = js_timer_id();"));
            let (cb, ms) = self.timer_parts(args)?;
            self.line(&format!("js_schedule({name}, {cb}, {ms}, {repeat});"));
            return Ok(());
        }
        let x = match &init.kind {
            ExprKind::Arrow(f) | ExprKind::Func(f) => self.closure(f, &[], true)?,
            _ => self.expr(init)?,
        };
        if x.repr == Repr::OptRef && !var.stored && !var.captured {
            let m = if var.reassigned { "mut " } else { "" };
            let name = self.bind(id, Repr::OptRef);
            self.line(&format!("let {m}{name} = {};", x.code));
            return Ok(());
        }
        let usize_value = x.repr == Repr::Usize && fixed && !var.stored;
        let value = if usize_value {
            x.code.clone()
        } else {
            self.local_value(x, &it, ty)
        };
        let float_literal = match &init.kind {
            ExprKind::Num { .. } => self.is_float(ty),
            ExprKind::Unary { op: UnaryOp::Neg, arg } => matches!(arg.kind, ExprKind::Num { .. }) && self.is_float(ty),
            ExprKind::Array(elems) => {
                matches!(self.resolve(ty), Ty::Array(ref item) if self.is_float(item))
                    && elems.iter().all(|el| {
                        matches!(el, ArrayElem::Expr(x) if matches!(&x.kind, ExprKind::Num { .. })
                            || matches!(&x.kind, ExprKind::Unary { op: UnaryOp::Neg, arg } if matches!(arg.kind, ExprKind::Num { .. })))
                    })
            }
            _ => false,
        };
        let needs_type = matches!(
            value.as_str(),
            "Vec::new()"
                | "HashMap::new()"
                | "HashSet::new()"
                | "JsMap::new()"
                | "JsSet::new()"
                | "None"
                | "String::new()"
        ) || matches!(self.resolve(&it), Ty::Null)
            || float_literal
            || self.generic_instance(ty);
        let closure_mut = match &init.kind {
            ExprKind::Arrow(f) | ExprKind::Func(f) => self.p.funcs[&f.id]
                .captures
                .iter()
                .any(|c| self.p.var(*c).mutated_in_closure),
            ExprKind::Call { .. } => matches!(self.resolve(&it), Ty::Func(_)),
            _ => false,
        };
        let m = if var.reassigned || var.mutated || closure_mut {
            "mut "
        } else {
            ""
        };
        let name = self.bind(
            id,
            if usize_value {
                Repr::Usize
            } else {
                Repr::Owned
            },
        );
        if needs_type {
            let t = self.rust_ty(ty, span)?;
            self.line(&format!("let {m}{name}: {t} = {value};"));
        } else {
            self.line(&format!("let {m}{name} = {value};"));
        }
        Ok(())
    }

    // ----- expression statements -----

    pub(super) fn expr_stmt(&mut self, e: &'a Expr) -> GResult<()> {
        match &e.kind {
            ExprKind::Assign { op, target, value } => self.assign_stmt(*op, target, value, e),
            ExprKind::Update { inc, arg, .. } => {
                let text = self.update_text(*inc, arg)?;
                self.line(&format!("{text};"));
                Ok(())
            }
            ExprKind::Call { .. } | ExprKind::New { .. } | ExprKind::Await(_) => {
                let outer = self.stmt_call.replace(e.id);
                let x = self.expr(e);
                self.stmt_call = outer;
                let x = x?;
                // `map.delete(k);` as a statement: the result is not wanted.
                let code = match self.p.calls.get(&e.id) {
                    Some(Callee::Builtin("map.delete" | "set.delete")) => x
                        .code
                        .strip_suffix(".is_some()")
                        .map_or(x.code.clone(), str::to_string),
                    _ => x.code,
                };
                self.line(&format!("{code};"));
                Ok(())
            }
            ExprKind::Seq(items) => {
                for i in items {
                    self.expr_stmt(i)?;
                }
                Ok(())
            }
            ExprKind::Yield { value, delegate } => {
                let Some(v) = value else {
                    return self.err("`yield` needs a value", e.span);
                };
                if *delegate {
                    // `yield* inner`: every value of the inner sequence.
                    let x = self.expr_as(v, Want::Owned)?;
                    self.line(&format!("for item in {x} {{"));
                    self.indent += 1;
                    self.line("y.yield_(item);");
                    self.indent -= 1;
                    self.line("}");
                    return Ok(());
                }
                let item = match self.fns.iter().rev().find_map(|c| c.yield_ty.clone()) {
                    Some(t) => t,
                    None => return self.err("`yield` outside a generator", e.span),
                };
                let code = self.expr_to(v, &item)?;
                self.line(&format!("y.yield_({code});"));
                Ok(())
            }
            _ => {
                let x = self.expr(e)?;
                self.line(&format!("let _ = {};", x.code));
                Ok(())
            }
        }
    }

    /// `x++` as `x += 1`.
    pub(super) fn update_text(&mut self, inc: bool, arg: &'a Expr) -> GResult<String> {
        let ty = self.ty_of(arg);
        if self.is_dynamic(&ty) {
            let place = self.dyn_place_mut(arg)?;
            let sym = if inc { "+" } else { "-" };
            return Ok(format!("js_op_assign({place}, \"{sym}\", JsValue::Int(1))"));
        }
        let x = self.expr_mut_root(arg)?;
        let target = self.place(x);
        let one = if self.is_float(&ty) { "1.0" } else { "1" };
        Ok(format!("{target} {}= {one}", if inc { "+" } else { "-" }))
    }

    /// A nullable value as an `Option<&T>`: borrowed from where it lives.
    fn borrowed_option(&mut self, value: &'a Expr) -> GResult<String> {
        if let ExprKind::Cond { test, cons, alt } = &value.kind
            && self.negative_narrow(test).is_none()
        {
            let c = self.cond(test)?;
            let a = self.borrowed_option(cons)?;
            let b = self.borrowed_option(alt)?;
            return Ok(format!("if {c} {{ {a} }} else {{ {b} }}"));
        }
        let ty = self.ty_of(value);
        let x = self.expr(value)?;
        let inner = match self.resolve(&ty) {
            Ty::Option(inner) => self.resolve(&inner),
            Ty::Null => return Ok("None".into()),
            other => other,
        };
        Ok(match x.repr {
            Repr::OptRef => x.code,
            _ if self.is_str(&inner) || self.is_boxed(&inner) => {
                format!("{}.as_deref()", Self::paren(&x, PREC_POSTFIX))
            }
            _ => format!("{}.as_ref()", Self::paren(&x, PREC_POSTFIX)),
        })
    }

    /// The code of an assignment target.
    fn place(&mut self, x: E) -> String {
        match x.repr {
            Repr::Ref | Repr::RefMut => format!("*{}", Self::paren(&x, PREC_UNARY)),
            _ => x.code,
        }
    }

    fn assign_stmt(
        &mut self,
        op: AssignOp,
        target: &'a AssignTarget,
        value: &'a Expr,
        e: &'a Expr,
    ) -> GResult<()> {
        let t = match target {
            AssignTarget::Pat(p) => return self.assign_pattern(p, value, e),
            AssignTarget::Expr(t) => t,
        };
        let tt = self.ty_of(t);
        let vt = self.ty_of(value);
        // A property of a value known at run time, or such a value changed by an operator.
        let in_dynamic = match &t.kind {
            ExprKind::Member { obj, .. } | ExprKind::Index { obj, .. } => {
                self.is_dynamic(&self.ty_of(obj))
            }
            _ => self.is_dynamic(&tt) && op != AssignOp::Assign,
        };
        if in_dynamic {
            let line = self.dyn_assign(op, t, value)?;
            self.line(&line);
            return Ok(());
        }
        // `re.lastIndex = i`
        if let ExprKind::Member { obj, prop, .. } = &t.kind
            && prop == "lastIndex"
            && matches!(self.ty_of(obj), Ty::Named(ref n, ..) if n == "RegExp")
            && op == AssignOp::Assign
        {
            let r = self.expr(obj)?;
            let r = Self::paren(&r, PREC_POSTFIX);
            let i = self.expr_as(value, Want::Owned)?;
            self.line(&format!("{r}.set_last_index({i});"));
            return Ok(());
        }
        // `map[key] = value`
        if let ExprKind::Index { obj, index, .. } = &t.kind
            && let Ty::Map(k, v, _) = self.ty_of(obj)
            && op == AssignOp::Assign
        {
            let m = self.expr_as(obj, Want::RefMut)?;
            let m = m.trim_start_matches("&mut ").to_string();
            let key = self.expr_to(index, &k)?;
            let val = self.expr_to(value, &v)?;
            self.line(&format!("{m}.insert({key}, {val});"));
            return Ok(());
        }
        // A field of a union value: through its mutable accessor.
        if let ExprKind::Member { obj, prop, .. } = &t.kind
            && let Ty::Named(n, ..) = self.ty_of(obj)
            && self.p.union_info(&n).is_some()
            && op == AssignOp::Assign
        {
            self.union_accessors
                .insert((n.clone(), prop.to_string(), true));
            let o = self.expr(obj)?;
            let base = Self::paren(&o, PREC_POSTFIX);
            let val = self.expr_to(value, &tt)?;
            self.line(&format!("*{base}.{}_mut() = {val};", snake(prop)));
            return Ok(());
        }
        // A field of a constrained type parameter: through the trait's mutable accessor.
        if let ExprKind::Member { obj, prop, .. } = &t.kind
            && let Some((_, c)) = self.constrained(&self.ty_of(obj))
        {
            self.bound_setters.insert((c, prop.to_string()));
            let o = self.expr_mut_root(obj)?;
            let base = Self::paren(&o, PREC_POSTFIX);
            let place = format!("*{base}.{}_mut()", snake(prop));
            let x = E::new(place, PREC_UNARY, Repr::Owned);
            return self.assign_to(op, x, value, &tt, &vt, e);
        }
        // A field of a value held dynamically: through its mutable accessor.
        if let ExprKind::Member { obj, prop, .. } = &t.kind
            && !matches!(obj.kind, ExprKind::This)
            && let Ty::Named(n, ..) = self.ty_of(obj)
            && self.p.is_dyn(&n)
            && self
                .p
                .struct_info(&n)
                .is_some_and(|s| s.field(prop).is_some())
        {
            self.dyn_setters.insert((n.clone(), prop.to_string()));
            let o = self.expr(obj)?;
            let base = Self::paren(&o, PREC_POSTFIX);
            let place = format!("*{base}.{}_mut()", snake(prop));
            let x = E::new(place, PREC_UNARY, Repr::Owned);
            return self.assign_to(op, x, value, &tt, &vt, e);
        }
        // A setter.
        if let ExprKind::Member { obj, prop, .. } = &t.kind
            && let Ty::Named(n, ..) = self.ty_of(obj)
            && let Some(info) = self.p.struct_info(&n)
            && info.field(prop).is_none()
            && self.p.find_method(&n, prop, MethodKind::Set).is_some()
        {
            let o = self.expr(obj)?;
            let base = Self::paren(&o, PREC_POSTFIX);
            let val = self.expr_to(value, &tt)?;
            self.line(&format!("{base}.set_{}({val});", snake(prop)));
            return Ok(());
        }
        // `arr.length = 0`
        if let ExprKind::Member { obj, prop, .. } = &t.kind
            && prop == "length"
            && matches!(self.ty_of(obj), Ty::Array(_))
        {
            let a = self.expr_as(obj, Want::RefMut)?;
            let a = a.trim_start_matches("&mut ").to_string();
            let n = self.expr_as(value, Want::Index)?;
            if n == "0" {
                self.line(&format!("{a}.clear();"));
            } else {
                self.line(&format!("{a}.truncate({n});"));
            }
            return Ok(());
        }
        // Rebinding a reference variable.
        if let ExprKind::Ident(_) = &t.kind
            && let Some(&b) = self.p.refs.get(&t.id)
            && self.is_alias(b)
            && op == AssignOp::Assign
        {
            let name = self.name(b);
            if self.repr_of(b) == Repr::OptRef {
                let v = self.borrowed_option(value)?;
                self.line(&format!("{name} = {v};"));
                return Ok(());
            }
            let want = if self.repr_of(b) == Repr::RefMut {
                Want::RefMut
            } else {
                Want::Ref
            };
            let v = self.expr_as(value, want)?;
            self.line(&format!("{name} = {v};"));
            return Ok(());
        }
        // The variable itself, even inside a region where it is narrowed.
        let x = match (&t.kind, self.p.refs.get(&t.id)) {
            (ExprKind::Ident(_), Some(&b))
                if self.shared.contains(&b) || self.statics.contains_key(&b) =>
            {
                // The value first, then the cell: the value may read the variable.
                let place = if self.shared.contains(&b) {
                    format!("(*{}.borrow_mut())", self.name(b))
                } else {
                    format!("(*{}.lock().unwrap())", self.name(b))
                };
                if matches!(op, AssignOp::Assign) {
                    let val = self.expr_to(value, &tt)?;
                    self.line(&format!("let value_ = {val};"));
                    self.line(&format!("{place} = value_;"));
                    return Ok(());
                }
                E::new(place, PREC_ATOM, Repr::Place)
            }
            (ExprKind::Ident(_), Some(&b)) => {
                let mut x = E::atom(self.name(b), self.repr_of(b));
                x.var = Some((b, t.id));
                x
            }
            _ => self.expr_mut_root(t)?,
        };
        self.assign_to(op, x, value, &tt, &vt, e)
    }

    /// `place op= value`.
    fn assign_to(
        &mut self,
        op: AssignOp,
        x: E,
        value: &'a Expr,
        tt: &Ty,
        vt: &Ty,
        e: &'a Expr,
    ) -> GResult<()> {
        let (tt, vt) = (tt.clone(), vt.clone());
        let place = self.place(x);
        match op {
            AssignOp::Assign => {
                let val = self.expr_to(value, &tt)?;
                self.line(&format!("{place} = {val};"));
            }
            AssignOp::Add if self.is_str(&tt) => {
                if let ExprKind::Str(s) = &value.kind {
                    self.line(&format!("{place}.push_str({});", super::string_literal(s)));
                } else if self.is_str(&vt) {
                    let v = self.expr_as(value, Want::StrRef)?;
                    self.line(&format!("{place}.push_str({v});"));
                } else {
                    let (spec, arg) = self.format_arg(value)?;
                    let call = super::exprs::format_call(
                        "format!",
                        &spec,
                        &arg.into_iter().collect::<Vec<_>>(),
                    );
                    self.line(&format!("{place}.push_str(&{call});"));
                }
            }
            AssignOp::Nullish => {
                let Ty::Option(inner) = self.resolve(&tt) else {
                    return self.err("`??=` needs a nullable variable", e.span);
                };
                let v = self.expr_to(value, &inner)?;
                self.line(&format!("{place}.get_or_insert({v});"));
            }
            AssignOp::Or => {
                let v = self.expr_as(value, Want::Owned)?;
                self.line(&format!("{place} = {place} || {v};"));
            }
            AssignOp::And => {
                let v = self.expr_as(value, Want::Owned)?;
                self.line(&format!("{place} = {place} && {v};"));
            }
            AssignOp::Div => {
                let v = self.expr_as(value, Want::Owned)?;
                self.line(&format!("{place} /= {v};"));
            }
            AssignOp::Pow => {
                let v = self.float_operand(value)?;
                self.line(&format!("{place} = {place}.powf({v});"));
            }
            AssignOp::Shl
            | AssignOp::Shr
            | AssignOp::UShr
            | AssignOp::BitAnd
            | AssignOp::BitOr
            | AssignOp::BitXor => {
                // 32-bit integers, as in JavaScript.
                let b = self.int32_operand(value)?;
                let bin = op.binary().expect("bitwise");
                let code = self.bitwise_code(bin, &format!("({place} as i32)"), &b);
                self.line(&format!("{place} = {code};"));
            }
            other => {
                let sym = match other {
                    AssignOp::Add => "+",
                    AssignOp::Sub => "-",
                    AssignOp::Mul => "*",
                    AssignOp::Mod => "%",
                    _ => unreachable!(),
                };
                let v = self.expr_as(value, Want::Owned)?;
                self.line(&format!("{place} {sym}= {v};"));
            }
        }
        Ok(())
    }

    fn assign_pattern(&mut self, p: &'a Pattern, value: &'a Expr, e: &'a Expr) -> GResult<()> {
        let Pattern::Array { elems, .. } = p else {
            return self.err("unsupported assignment pattern", e.span);
        };
        // `[a, b] = [b, a]`
        if let ExprKind::Array(items) = &value.kind
            && elems.len() == 2
            && items.len() == 2
            && let [Some(a), Some(b)] = elems.as_slice()
            && let (Pattern::Ident { name: an, .. }, Pattern::Ident { name: bn, .. }) =
                (&a.pat, &b.pat)
            && let (ArrayElem::Expr(x), ArrayElem::Expr(y)) = (&items[0], &items[1])
            && let (ExprKind::Ident(xn), ExprKind::Ident(yn)) = (&x.kind, &y.kind)
            && an == yn
            && bn == xn
        {
            let (xa, xb) = (self.expr(x)?, self.expr(y)?);
            self.line(&format!(
                "std::mem::swap(&mut {}, &mut {});",
                xb.code, xa.code
            ));
            return Ok(());
        }
        // `[a, b] = [x, y]`: a tuple on both sides.
        if let ExprKind::Array(items) = &value.kind
            && items.len() == elems.len()
            && items.iter().all(|i| matches!(i, ArrayElem::Expr(_)))
            && elems.iter().all(|e| {
                e.as_ref()
                    .is_some_and(|e| matches!(e.pat, Pattern::Ident { .. }))
            })
        {
            let mut values = Vec::new();
            let mut names = Vec::new();
            for (el, item) in elems.iter().flatten().zip(items) {
                let (ArrayElem::Expr(x), Pattern::Ident { id, .. }) = (item, &el.pat) else {
                    unreachable!()
                };
                let binding = self.p.refs[id];
                let vt = self.resolve(&self.p.var(binding).ty);
                values.push(self.expr_to(x, &vt)?);
                names.push(self.name(binding));
            }
            self.line(&format!(
                "({}) = ({});",
                names.join(", "),
                values.join(", ")
            ));
            return Ok(());
        }
        let v = self.expr_as(value, Want::Owned)?;
        let it = self.ty_of(value);
        self.line(&format!("let parts_ = {v};"));
        for (i, el) in elems.iter().enumerate() {
            let Some(el) = el else { continue };
            let Pattern::Ident { id, .. } = &el.pat else {
                return self.err("nested patterns are not supported", el.pat.span());
            };
            let binding = self.p.refs[id];
            let name = self.name(binding);
            let access = if matches!(self.resolve(&it), Ty::Tuple(_)) {
                format!("parts_.{i}")
            } else {
                format!("parts_[{i}].clone()")
            };
            self.line(&format!("{name} = {access};"));
        }
        Ok(())
    }

    // ----- control flow -----

    fn if_stmt(
        &mut self,
        test: &'a Expr,
        then: &'a Stmt,
        otherwise: Option<&'a Stmt>,
    ) -> GResult<()> {
        if self.if_union(test, then, otherwise)? {
            return Ok(());
        }
        // `if (x === null) return;` narrows the rest of the block: `let Some(x) = x else {…};`
        if let Some((n, path_expr)) = self.negative_narrow(test)
            && n.let_else
        {
            let pt = self.ty_of(path_expr);
            let x = self.expr(path_expr)?;
            let (pattern, source) = self.narrow_binding(&n, x, &pt);
            self.line(&format!("let {pattern} = {source} else {{"));
            self.indent += 1;
            let saved: Vec<(String, Repr)> =
                self.narrowed.drain(self.narrowed.len() - 1..).collect();
            self.stmt_or_block(then)?;
            self.narrowed.extend(saved);
            self.indent -= 1;
            self.line("};");
            if let Some(o) = otherwise {
                self.stmt_or_block(o)?;
            }
            return Ok(());
        }
        // `if (x === null) {…} else {…}`: the else branch sees `x`: branches are swapped.
        if let Some((n, path_expr)) = self.negative_narrow(test)
            && let Some(o) = otherwise
            && !n.unwrap
        {
            let pt = self.ty_of(path_expr);
            let x = self.expr(path_expr)?;
            let before = self.narrowed.len();
            let (pattern, source) = self.narrow_binding(&n, x, &pt);
            self.line(&format!("if let {pattern} = {source} {{"));
            self.indent += 1;
            self.stmt_or_block(o)?;
            self.indent -= 1;
            self.narrowed.truncate(before);
            self.line("} else {");
            self.indent += 1;
            self.stmt_or_block(then)?;
            self.indent -= 1;
            self.line("}");
            return Ok(());
        }
        let before = self.narrowed.len();
        let c = self.cond(test)?;
        self.line(&format!("if {c} {{"));
        self.indent += 1;
        self.stmt_or_block(then)?;
        self.indent -= 1;
        self.narrowed.truncate(before);
        match otherwise {
            None => self.line("}"),
            Some(Stmt {
                kind:
                    StmtKind::If {
                        test,
                        then,
                        otherwise,
                    },
                ..
            }) if !self.negative_narrow(test).is_some_and(|(n, _)| n.let_else) => {
                let text = self.capture(|g| g.if_stmt(test, then, otherwise.as_deref()))?;
                let text = text.trim_start();
                let indent = "    ".repeat(self.indent);
                self.out.push_str(&format!("{indent}}} else {text}"));
            }
            Some(o) => {
                self.line("} else {");
                self.indent += 1;
                self.stmt_or_block(o)?;
                self.indent -= 1;
                self.line("}");
            }
        }
        Ok(())
    }

    /// The test of a union member in `test` (through a `!`), with what the value is when
    /// the test is true and when it is false.
    pub(super) fn union_test_of(
        &self,
        test: &'a Expr,
    ) -> Option<(crate::infer::NarrowTest, &'a Expr, UnionSides)> {
        let (inner, negated) = match &test.kind {
            ExprKind::Unary {
                op: UnaryOp::Not,
                arg,
            } => (&**arg, true),
            _ => (test, false),
        };
        let n = self.p.narrow_tests.get(&inner.id)?;
        let (en, variant) = n.union.clone()?;
        let positive = n.positive != negated;
        let tested = Side::Variant(variant);
        let other = n.complement.clone().map(|c| match c {
            Complement::Variant(v, _) => Side::Variant(v),
            Complement::Union(t) => Side::Union(t),
        });
        let (when_true, when_false) = if positive {
            (Some(tested), other)
        } else {
            (other, Some(tested))
        };
        Some((
            n.clone(),
            narrow_path_expr(inner),
            UnionSides {
                union: en,
                when_true,
                when_false,
                test_id: inner.id,
            },
        ))
    }

    /// Whether an expression (a callback) refers to the variable.
    fn mentions(&self, e: &Expr, binding: NodeId) -> bool {
        let mut found = false;
        let mut on_stmt = |_: &Stmt| {};
        let mut on_expr = |x: &Expr| {
            if self.p.refs.get(&x.id) == Some(&binding) {
                found = true;
            }
        };
        super::visit_expr_deep(e, &mut on_stmt, &mut on_expr);
        found
    }

    /// Whether a branch reads the value a test narrowed.
    pub(super) fn branch_uses_narrow(&self, body: &Stmt, test_id: NodeId) -> bool {
        let mut used = false;
        let mut on_stmt = |_: &Stmt| {};
        let mut on_expr = |e: &Expr| {
            if self
                .p
                .narrowed_uses
                .get(&e.id)
                .is_some_and(|(_, t)| *t == test_id)
            {
                used = true;
            }
        };
        super::visit_all(std::slice::from_ref(body), &mut on_stmt, &mut on_expr);
        used
    }

    /// The source of a `match` on a union value, and how the bound members are held.
    fn union_source(&mut self, n: &crate::infer::NarrowTest, x: E, pt: &Ty) -> (String, Repr) {
        let before = self.narrowed.len();
        let (_, source) = self.narrow_binding(n, x, pt);
        let repr = self.narrowed.last().map_or(Repr::Ref, |(_, r)| *r);
        self.narrowed.truncate(before);
        (source, repr)
    }

    /// `match other { U::X(v) => U2::X(v), … }`: a value of union `from`, known not to be
    /// the tested member, as the union of the remaining members.
    pub(super) fn union_conversion(
        &mut self,
        other: &str,
        from: &str,
        to: &Ty,
        repr: Repr,
    ) -> GResult<String> {
        let Some(target) = self.p.union_of(to).cloned() else {
            return self.err("not a union", crate::error::Span::default());
        };
        let from_name = pascal(from);
        let to_name = pascal(&target.name);
        let mut arms = Vec::new();
        for m in &target.members {
            let Some((_, from_variant)) = self.union_variant(&m.ty, &Ty::named(from.to_string()))
            else {
                continue;
            };
            let value = if repr == Repr::Owned {
                "v".to_string()
            } else if self.is_copy(&m.ty) {
                "*v".to_string()
            } else {
                "v.clone()".to_string()
            };
            arms.push(format!(
                "{from_name}::{from_variant}(v) => {to_name}::{}({value})",
                m.name
            ));
        }
        arms.push("_ => unreachable!()".into());
        Ok(format!("match {other} {{ {} }}", arms.join(", ")))
    }

    /// The pattern of a side, and the lines binding `name` in its arm (a conversion for a
    /// union side).
    fn side_arm(
        &mut self,
        side: &Side,
        name: &str,
        sides: &UnionSides,
        repr: Repr,
    ) -> GResult<(String, Option<String>)> {
        Ok(match side {
            Side::Variant(v) => (format!("{}::{v}({name})", pascal(&sides.union)), None),
            Side::Union(t) => {
                let conv = self.union_conversion("other", &sides.union, t, repr)?;
                ("other".to_string(), Some(format!("let {name} = {conv};")))
            }
        })
    }

    /// `if (t.type === "a") … else if (t.type === "b") … else …`: one `match` on the
    /// union with an arm per member. Returns whether the chain was handled.
    fn if_union_chain(
        &mut self,
        test: &'a Expr,
        then: &'a Stmt,
        otherwise: Option<&'a Stmt>,
    ) -> GResult<bool> {
        let Some((n0, path_expr, sides0)) = self.union_test_of(test) else {
            return Ok(false);
        };
        let Some(o) = otherwise else {
            return Ok(false);
        };
        // The chain: (variant of the root union, body) per test, and the final `else`.
        let root = sides0.union.clone();
        let root_ty = Ty::named(root.clone());
        let mut arms: Vec<(String, &'a Stmt)> = Vec::new();
        let mut last_side: Option<Side> = None;
        let mut current: Option<(&'a Expr, &'a Stmt, Option<&'a Stmt>)> =
            Some((test, then, Some(o)));
        let mut final_else: Option<&'a Stmt> = None;
        while let Some((t, body, next)) = current.take() {
            let Some((n, p, sides)) = self.union_test_of(t) else {
                return Ok(false);
            };
            if n.root != n0.root
                || n.path != n0.path
                || p.span != path_expr.span && n.name != n0.name
            {
                return Ok(false);
            }
            let Some(Side::Variant(v)) = &sides.when_true else {
                return Ok(false);
            };
            let Some(member_ty) = self
                .p
                .union_info(&sides.union)
                .and_then(|u| u.variant(v))
                .map(|m| m.ty.clone())
            else {
                return Ok(false);
            };
            let Some((_, root_variant)) = self.union_variant(&member_ty, &root_ty) else {
                return Ok(false);
            };
            arms.push((root_variant, body));
            last_side = sides.when_false.clone();
            match next {
                Some(Stmt {
                    kind:
                        StmtKind::If {
                            test: t2,
                            then: then2,
                            otherwise: o2,
                        },
                    ..
                }) if self.union_test_of(t2).is_some() => {
                    current = Some((t2, then2, o2.as_deref()));
                }
                other => final_else = other,
            }
        }
        if arms.len() < 2 {
            return Ok(false);
        }
        // The last `else` binds the one member left, or sees the remaining union.
        let final_arm: Option<(String, Option<String>)> = match (&last_side, final_else) {
            (Some(Side::Variant(v)), Some(_)) => {
                let last_union = self.union_test_of(test).map(|(_, _, s)| s.union);
                let _ = last_union;
                // `v` names a variant of the union the last test was made on.
                let member_ty = self
                    .p
                    .unions
                    .iter()
                    .find_map(|u| u.variant(v).map(|m| m.ty.clone()));
                match member_ty.and_then(|t| self.union_variant(&t, &root_ty)) {
                    Some((_, rv)) => Some((rv, None)),
                    None => return Ok(false),
                }
            }
            (Some(Side::Union(_)), Some(_)) => return Ok(false),
            (_, None) => None,
            (None, Some(_)) => return Ok(false),
        };
        let pt = self.ty_of(path_expr);
        let x = self.expr(path_expr)?;
        let before = self.narrowed.len();
        let name = snake(&n0.name);
        let (source, repr) = self.union_source(&n0, x, &pt);
        let en = pascal(&root);
        self.line(&format!("match {source} {{"));
        self.indent += 1;
        let mut covered = Vec::new();
        for (variant, body) in arms {
            self.line(&format!("{en}::{variant}({name}) => {{"));
            self.indent += 1;
            self.narrowed.push((name.clone(), repr));
            self.stmt_or_block(body)?;
            self.narrowed.truncate(before);
            self.indent -= 1;
            self.line("}");
            covered.push(variant);
        }
        if let (Some((variant, _)), Some(body)) = (&final_arm, final_else) {
            self.line(&format!("{en}::{variant}({name}) => {{"));
            self.indent += 1;
            self.narrowed.push((name.clone(), repr));
            self.stmt_or_block(body)?;
            self.narrowed.truncate(before);
            self.indent -= 1;
            self.line("}");
            covered.push(variant.clone());
        }
        let all = self
            .p
            .union_info(&root)
            .is_some_and(|u| u.members.iter().all(|m| covered.contains(&m.name)));
        if !all {
            self.line("_ => {}");
        }
        self.indent -= 1;
        self.line("}");
        Ok(true)
    }

    /// `if` on a test of a union member: `if let` or `match`, binding the member on each
    /// side. Returns whether the statement was handled.
    fn if_union(
        &mut self,
        test: &'a Expr,
        then: &'a Stmt,
        otherwise: Option<&'a Stmt>,
    ) -> GResult<bool> {
        let Some((n, path_expr, sides)) = self.union_test_of(test) else {
            return Ok(false);
        };
        if self.if_union_chain(test, then, otherwise)? {
            return Ok(true);
        }
        let pt = self.ty_of(path_expr);
        let x = self.expr(path_expr)?;
        let before = self.narrowed.len();
        let name = snake(&n.name);
        let (source, repr) = self.union_source(&n, x, &pt);
        // A side made of the remaining members is held by value after its conversion.
        let repr_of = |side: &Side| match side {
            Side::Variant(..) => repr,
            Side::Union(_) => Repr::Owned,
        };
        match (&sides.when_true, &sides.when_false, otherwise) {
            (Some(t), Some(f), Some(o)) => {
                let (pt_, prelude_t) = self.side_arm(t, &name, &sides, repr)?;
                let (pf, prelude_f) = self.side_arm(f, &name, &sides, repr)?;
                self.line(&format!("match {source} {{"));
                self.indent += 1;
                // A catch-all `other` arm must come last.
                let mut arms: Vec<(&str, &Option<String>, &Side, &'a Stmt)> =
                    vec![(&pt_, &prelude_t, t, then), (&pf, &prelude_f, f, o)];
                if pt_ == "other" {
                    arms.swap(0, 1);
                }
                for (pattern, prelude, side, body) in arms {
                    let uses = self.branch_uses_narrow(body, sides.test_id);
                    let pattern = if pattern == "other" && !uses {
                        "_"
                    } else {
                        pattern
                    };
                    self.line(&format!("{pattern} => {{"));
                    self.indent += 1;
                    if let Some(p) = prelude
                        && uses
                    {
                        self.line(p);
                    }
                    self.narrowed.push((name.clone(), repr_of(side)));
                    self.stmt_or_block(body)?;
                    self.narrowed.truncate(before);
                    self.indent -= 1;
                    self.line("}");
                }
                self.indent -= 1;
                self.line("}");
            }
            (Some(t), Some(f), None) if n.let_else => {
                // The `then` branch leaves: the rest of the block sees the other side.
                let (pt_, prelude_t) = self.side_arm(t, &name, &sides, repr)?;
                let rest_arm = match f {
                    Side::Variant(v) => format!("{}::{v}({name}) => {name},", pascal(&sides.union)),
                    Side::Union(ty) => {
                        let conv = self.union_conversion("other", &sides.union, ty, repr)?;
                        format!("other => {conv},")
                    }
                };
                self.line(&format!("let {name} = match {source} {{"));
                self.indent += 1;
                if pt_ == "other" {
                    self.line(&rest_arm);
                }
                let uses = self.branch_uses_narrow(then, sides.test_id);
                self.line(&format!(
                    "{} => {{",
                    if pt_ == "other" && !uses { "_" } else { &pt_ }
                ));
                self.indent += 1;
                if let Some(p) = prelude_t
                    && uses
                {
                    self.line(&p);
                }
                self.narrowed.push((name.clone(), repr_of(t)));
                self.stmt_or_block(then)?;
                self.narrowed.truncate(before);
                self.indent -= 1;
                self.line("}");
                if pt_ != "other" {
                    self.line(&rest_arm);
                }
                self.indent -= 1;
                self.line("};");
                self.narrowed.push((name, repr_of(f)));
            }
            (Some(Side::Variant(tv)), _, _) => {
                let pattern = format!("{}::{tv}({name})", pascal(&sides.union));
                self.line(&format!("if let {pattern} = {source} {{"));
                self.indent += 1;
                self.narrowed.push((name.clone(), repr));
                self.stmt_or_block(then)?;
                self.narrowed.truncate(before);
                self.indent -= 1;
                match otherwise {
                    Some(o) => {
                        self.line("} else {");
                        self.indent += 1;
                        self.stmt_or_block(o)?;
                        self.indent -= 1;
                        self.line("}");
                    }
                    None => self.line("}"),
                }
            }
            (Some(Side::Union(_)), None, _) => return Ok(false),
            (Some(Side::Union(ty)), f, None) => {
                // The tested member is not in `then`: everything else is.
                let (pf, _) = match f {
                    Some(f) => self.side_arm(f, &name, &sides, repr)?,
                    None => return Ok(false),
                };
                let pattern_wild = pf.replace(&format!("({name})"), "(_)");
                self.line(&format!("if !matches!({source}, {pattern_wild}) {{"));
                self.indent += 1;
                if self.branch_uses_narrow(then, sides.test_id) {
                    let conv = self.union_conversion(&source, &sides.union, ty, repr)?;
                    self.line(&format!("let {name} = {conv};"));
                }
                self.narrowed.push((name.clone(), Repr::Owned));
                self.stmt_or_block(then)?;
                self.narrowed.truncate(before);
                self.indent -= 1;
                self.line("}");
            }
            (None, Some(f), Some(o)) => {
                let (pf, prelude_f) = self.side_arm(f, &name, &sides, repr)?;
                self.line(&format!("match {source} {{"));
                self.indent += 1;
                let uses = self.branch_uses_narrow(o, sides.test_id);
                self.line(&format!(
                    "{} => {{",
                    if pf == "other" && !uses { "_" } else { &pf }
                ));
                self.indent += 1;
                if let Some(p) = prelude_f
                    && uses
                {
                    self.line(&p);
                }
                self.narrowed.push((name.clone(), repr_of(f)));
                self.stmt_or_block(o)?;
                self.narrowed.truncate(before);
                self.indent -= 1;
                self.line("}");
                self.line("_ => {");
                self.indent += 1;
                self.stmt_or_block(then)?;
                self.indent -= 1;
                self.line("}");
                self.indent -= 1;
                self.line("}");
            }
            (None, Some(Side::Variant(ev)), None) if n.let_else => {
                let pattern = format!("{}::{ev}({name})", pascal(&sides.union));
                self.line(&format!("let {pattern} = {source} else {{"));
                self.indent += 1;
                self.stmt_or_block(then)?;
                self.indent -= 1;
                self.line("};");
                self.narrowed.push((name, repr));
            }
            (None, Some(Side::Union(ty)), None) if n.let_else => {
                let conv = self.union_conversion("other", &sides.union, ty, repr)?;
                self.line(&format!("let {name} = match {source} {{"));
                self.indent += 1;
                self.line(&format!("other => {conv},"));
                self.indent -= 1;
                self.line("};");
                let _ = then;
                self.narrowed.push((name, Repr::Owned));
            }
            (None, Some(f), None) => {
                let (pf, _) = self.side_arm(f, &name, &sides, repr)?;
                let pattern_wild = pf.replace(&format!("({name})"), "(_)");
                self.line(&format!("if !matches!({source}, {pattern_wild}) {{"));
                self.indent += 1;
                self.stmt_or_block(then)?;
                self.indent -= 1;
                self.line("}");
            }
            (None, None, _) => return Ok(false),
        }
        Ok(true)
    }

    /// A test of the form `x === null` or `!x` on a nullable path.
    pub(super) fn negative_narrow(
        &self,
        test: &'a Expr,
    ) -> Option<(crate::infer::NarrowTest, &'a Expr)> {
        if let Some(n) = self.p.narrow_tests.get(&test.id)
            && !n.positive
        {
            return Some((n.clone(), narrow_path_expr(test)));
        }
        if let ExprKind::Unary {
            op: UnaryOp::Not,
            arg,
        } = &test.kind
            && let Some(n) = self.p.narrow_tests.get(&arg.id)
            && n.positive
        {
            let mut n = n.clone();
            n.positive = false;
            return Some((n, narrow_path_expr(arg)));
        }
        None
    }

    /// The statements of a branch, without an extra pair of braces.
    fn stmt_or_block(&mut self, s: &'a Stmt) -> GResult<()> {
        match &s.kind {
            StmtKind::Block(b) => {
                self.comments(s);
                self.block_body(&b.stmts, false)
            }
            _ => self.block_body(std::slice::from_ref(s), false),
        }
    }

    fn loop_stmt(&mut self, head: &str, body: &'a Stmt, label: Option<&str>) -> GResult<()> {
        self.loop_stmt_with_tail(head, body, label, "")
    }

    /// A loop whose body is followed by `tail` (the update of a `for`, the test of a
    /// `do … while`). A `continue` in the body must still run the tail: it becomes a break
    /// out of a labeled block.
    fn loop_stmt_with_tail(
        &mut self,
        head: &str,
        body: &'a Stmt,
        label: Option<&str>,
        tail: &str,
    ) -> GResult<()> {
        let rust_label = label.map(|l| format!("'{}", snake(l).trim_start_matches("r#")));
        let has_continue = !tail.is_empty() && contains_continue(body, label);
        let continue_label = if has_continue {
            Some(self.fresh_label("next"))
        } else {
            None
        };
        self.fn_ctx_mut().targets.push(Target::Loop {
            label: label.map(str::to_string),
            rust_label: rust_label.clone(),
            continue_label: continue_label.clone(),
        });
        let prefix = rust_label.map(|l| format!("{l}: ")).unwrap_or_default();
        self.line(&format!("{prefix}{head} {{"));
        self.indent += 1;
        if let Some(c) = &continue_label {
            self.line(&format!("{c}: {{"));
            self.indent += 1;
            self.stmt_or_block(body)?;
            self.indent -= 1;
            self.line("}");
        } else {
            self.stmt_or_block(body)?;
        }
        self.out.push_str(tail);
        self.indent -= 1;
        self.line("}");
        self.fn_ctx_mut().targets.pop();
        Ok(())
    }

    fn for_stmt(
        &mut self,
        init: Option<&'a Stmt>,
        test: Option<&'a Expr>,
        update: Option<&'a Expr>,
        body: &'a Stmt,
        label: Option<&str>,
    ) -> GResult<()> {
        if let Some(range) = self.counted_loop(init, test, update, body)? {
            self.loop_stmt(&range, body, label)?;
            return Ok(());
        }
        self.push_scope();
        if let Some(i) = init {
            self.stmt(i)?;
        }
        let head = match test {
            Some(t) => format!("while {}", self.cond(t)?),
            None => "loop".into(),
        };
        let tail = match update {
            Some(u) => self.capture(|g| {
                g.indent += 1;
                g.expr_stmt(u)?;
                g.indent -= 1;
                Ok(())
            })?,
            None => String::new(),
        };
        self.loop_stmt_with_tail(&head, body, label, &tail)?;
        self.pop_scope();
        Ok(())
    }

    /// `for (let i = a; i < b; i++)` as `for i in a..b`, when the body leaves `i` alone.
    fn counted_loop(
        &mut self,
        init: Option<&'a Stmt>,
        test: Option<&'a Expr>,
        update: Option<&'a Expr>,
        body: &'a Stmt,
    ) -> GResult<Option<String>> {
        let (Some(init), Some(test), Some(update)) = (init, test, update) else {
            return Ok(None);
        };
        let StmtKind::Var(v) = &init.kind else {
            return Ok(None);
        };
        let [d] = v.decls.as_slice() else {
            return Ok(None);
        };
        let (Pattern::Ident { name, id, .. }, Some(start)) = (&d.pat, &d.init) else {
            return Ok(None);
        };
        let ExprKind::Binary { op, left, right } = &test.kind else {
            return Ok(None);
        };
        if !matches!(left.kind, ExprKind::Ident(ref n) if n == name) {
            return Ok(None);
        }
        let step: i64 = match &update.kind {
            ExprKind::Update { inc, arg, .. } if matches!(arg.kind, ExprKind::Ident(ref n) if n == name) => {
                if *inc {
                    1
                } else {
                    -1
                }
            }
            ExprKind::Assign {
                op: aop,
                target: AssignTarget::Expr(t),
                value,
            } if matches!(t.kind, ExprKind::Ident(ref n) if n == name) => {
                let ExprKind::Num {
                    value: k,
                    int: true,
                    ..
                } = &value.kind
                else {
                    return Ok(None);
                };
                match aop {
                    AssignOp::Add => *k as i64,
                    AssignOp::Sub => -(*k as i64),
                    _ => return Ok(None),
                }
            }
            _ => return Ok(None),
        };
        if step == 0 || assigns_var(body, name) || self.is_float(&self.p.var(*id).ty) {
            return Ok(None);
        }
        let ascending = step > 0;
        if !matches!(
            (op, ascending),
            (BinOp::Lt | BinOp::LtEq, true) | (BinOp::Gt | BinOp::GtEq, false)
        ) {
            return Ok(None);
        }
        let (s, e, usize_loop) = {
            let st = self.expr(start)?;
            let en = self.expr(right)?;
            let usize_loop = (en.repr == Repr::Usize
                && (st.repr == Repr::Usize || super::is_int_literal(&st.code)))
                || (st.repr == Repr::Usize && super::is_int_literal(&en.code));
            let (start_ty, end_ty) = (self.ty_of(start), self.ty_of(right));
            if usize_loop {
                (st.code, en.code, true)
            } else {
                let s = self.coerce(st, &start_ty, Want::Owned);
                let e = self.coerce(en, &end_ty, Want::Owned);
                (s, e, false)
            }
        };
        let var_name = self.bind(*id, if usize_loop { Repr::Usize } else { Repr::Owned });
        // A counter the body never reads: `_`.
        let mut read = false;
        let mut on_stmt = |_: &Stmt| {};
        let mut on_expr = |x: &Expr| {
            if self.p.refs.get(&x.id) == Some(id) {
                read = true;
            }
        };
        super::visit_stmt(body, &mut on_stmt, &mut on_expr);
        let var_name = if read { var_name } else { "_".to_string() };
        let head = if ascending {
            let r = if *op == BinOp::Lt {
                format!("{s}..{e}")
            } else {
                format!("{s}..={e}")
            };
            if step == 1 {
                format!("for {var_name} in {r}")
            } else {
                format!("for {var_name} in ({r}).step_by({step})")
            }
        } else {
            if step != -1 {
                return Ok(None);
            }
            let r = if *op == BinOp::GtEq {
                // `for (let i = n - 1; i >= 0; i--)` → `(0..n).rev()`
                if let Some(upper) = s.strip_suffix(" - 1") {
                    format!("({e}..{upper}).rev()")
                } else {
                    format!("({e}..={s}).rev()")
                }
            } else {
                format!("({e} + 1..={s}).rev()")
            };
            format!("for {var_name} in {r}")
        };
        Ok(Some(head))
    }

    fn for_of(
        &mut self,
        pat: &'a Pattern,
        iter: &'a Expr,
        body: &'a Stmt,
        is_in: bool,
        label: Option<&str>,
    ) -> GResult<()> {
        let it = self.ty_of(iter);
        self.push_scope();
        // Iterators of maps, arrays and sets used directly.
        let direct = match &iter.kind {
            ExprKind::Call { callee, .. } => match self.p.calls.get(&iter.id) {
                Some(Callee::Builtin(name)) => {
                    let ExprKind::Member { obj, .. } = &callee.kind else {
                        unreachable!()
                    };
                    match *name {
                        "array.entries" => Some((obj, "iter().enumerate()", true)),
                        "array.keys" => Some((obj, "keys", false)),
                        "array.values" | "set.values" | "set.keys" => Some((obj, "iter()", false)),
                        "map.keys" => Some((obj, "keys()", false)),
                        "map.values" => Some((obj, "values()", false)),
                        "map.entries" => Some((obj, "iter()", false)),
                        _ => None,
                    }
                }
                _ => None,
            },
            _ => None,
        };
        let (source, elem_repr, enumerate) = if let Some((obj, method, enumerate)) = direct {
            let o = self.expr(obj)?;
            let base = Self::paren(&o, PREC_POSTFIX);
            let changed_in_loop =
                o.var.is_some_and(|(b, _)| self.p.var(b).mutated) && body_mutates(body, obj);
            if method == "keys" {
                (format!("0..{base}.len()"), Repr::Usize, false)
            } else if changed_in_loop && matches!(method, "keys()" | "values()") {
                (
                    format!("{base}.{method}.cloned().collect::<Vec<_>>()"),
                    Repr::Owned,
                    enumerate,
                )
            } else {
                (format!("{base}.{method}"), Repr::Ref, enumerate)
            }
        } else if is_in {
            let o = self.expr(iter)?;
            let base = Self::paren(&o, PREC_POSTFIX);
            match self.resolve(&it) {
                Ty::Map(..) => (format!("{base}.keys()"), Repr::Ref, false),
                ref d if d.is_dynamic() => (format!("js_keys(&{base})"), Repr::Owned, false),
                _ => (format!("0..{base}.len()"), Repr::Usize, false),
            }
        } else {
            let o = self.expr(iter)?;
            let base = Self::paren(&o, PREC_POSTFIX);
            match self.resolve(&it) {
                Ty::Str => (format!("{base}.chars()"), Repr::Char, false),
                ref d if d.is_dynamic() => (format!("js_items(&{base})"), Repr::Owned, false),
                // A generator is consumed.
                Ty::Gen(_) => (o.code.clone(), Repr::Owned, false),
                Ty::Array(ref item) | Ty::Set(ref item) => {
                    let copy = self.is_copy(item);
                    let mutated =
                        pat.names().iter().any(|(_, id)| self.p.var(*id).mutated) && !copy;
                    match o.repr {
                        Repr::Owned if o.var.is_none() => (o.code.clone(), Repr::Owned, false),
                        Repr::Owned | Repr::Place => {
                            if mutated {
                                (
                                    format!("&mut {}", Self::paren(&o, PREC_UNARY)),
                                    Repr::RefMut,
                                    false,
                                )
                            } else {
                                (
                                    format!("&{}", Self::paren(&o, PREC_UNARY)),
                                    Repr::Ref,
                                    false,
                                )
                            }
                        }
                        Repr::RefMut => {
                            if mutated {
                                (format!("{base}.iter_mut()"), Repr::RefMut, false)
                            } else {
                                (format!("{base}.iter()"), Repr::Ref, false)
                            }
                        }
                        _ => (o.code.clone(), Repr::Ref, false),
                    }
                }
                Ty::Map(..) => match o.repr {
                    Repr::Owned if o.var.is_none() => (o.code.clone(), Repr::Owned, false),
                    Repr::Owned | Repr::Place => (
                        format!("&{}", Self::paren(&o, PREC_UNARY)),
                        Repr::Ref,
                        false,
                    ),
                    _ => (format!("{base}.iter()"), Repr::Ref, false),
                },
                // `[Symbol.iterator]()`.
                ref other => match self.struct_iteration(other) {
                    Some((_, generator)) => {
                        (self.iteration_source(&base, generator), Repr::Owned, false)
                    }
                    None => return self.err("cannot iterate over this value", iter.span),
                },
            }
        };
        let item_ty = match self.resolve(&it) {
            Ty::Array(t) | Ty::Set(t) | Ty::Gen(t) if !enumerate => *t,
            Ty::Array(t) => Ty::Tuple(vec![Ty::Any, *t]),
            Ty::Map(k, v, _) => Ty::Tuple(vec![*k, *v]),
            Ty::Str => Ty::Str,
            ref other => self.struct_iteration(other).map_or(Ty::Any, |(t, _)| t),
        };
        let pattern = self.loop_pattern(pat, &item_ty, elem_repr, enumerate)?;
        let head = format!("for {pattern} in {source}");
        self.loop_stmt(&head, body, label)?;
        self.pop_scope();
        Ok(())
    }

    /// The pattern binding the loop variable(s).
    fn loop_pattern(
        &mut self,
        pat: &'a Pattern,
        item_ty: &Ty,
        repr: Repr,
        enumerate: bool,
    ) -> GResult<String> {
        match pat {
            Pattern::Ident { id, .. } => {
                let var_ty = self.resolve(&self.p.var(*id).ty);
                let copy = self.is_copy(&var_ty);
                let var = self.p.var(*id);
                let m = if var.reassigned { "mut " } else { "" };
                let (prefix, r) = match repr {
                    Repr::Ref if copy => ("&", Repr::Owned),
                    Repr::Ref => ("", Repr::Ref),
                    Repr::RefMut => ("", Repr::RefMut),
                    other => ("", other),
                };
                let name = self.bind(*id, r);
                Ok(format!("{prefix}{m}{name}"))
            }
            Pattern::Array { elems, .. } => {
                let Ty::Tuple(parts) = self.resolve(item_ty) else {
                    return self.err("only pairs can be destructured in a loop", pat.span());
                };
                let mut names = Vec::new();
                for (i, el) in elems.iter().enumerate() {
                    let Some(el) = el else {
                        names.push("_".to_string());
                        continue;
                    };
                    let Pattern::Ident { id, .. } = &el.pat else {
                        return self.err("nested patterns are not supported", el.pat.span());
                    };
                    let part_ty = parts.get(i).cloned().unwrap_or(Ty::Any);
                    let copy = self.is_copy(&part_ty);
                    let (prefix, r) = if enumerate && i == 0 {
                        ("", Repr::Usize)
                    } else {
                        match repr {
                            Repr::Ref if copy => ("&", Repr::Owned),
                            Repr::Ref => ("", Repr::Ref),
                            Repr::RefMut => ("", Repr::RefMut),
                            other => ("", other),
                        }
                    };
                    let name = self.bind(*id, r);
                    names.push(format!("{prefix}{name}"));
                }
                Ok(format!("({})", names.join(", ")))
            }
            Pattern::Object { span, .. } => self.err(
                "object patterns are not supported in loops: use `item.field`",
                *span,
            ),
        }
    }

    fn switch_stmt(&mut self, disc: &'a Expr, cases: &'a [Case]) -> GResult<()> {
        if let Some(text) = self.switch_on_union(disc, cases)? {
            self.out.push_str(&text);
            return Ok(());
        }
        let dt = self.ty_of(disc);
        // Patterns must be literals or enum variants.
        let mut patterns: Vec<Option<String>> = Vec::new();
        let mut all_literal = !self.is_float(&dt);
        for c in cases {
            match &c.test {
                None => patterns.push(None),
                Some(t) => match self.pattern_text(t) {
                    Some(p) => patterns.push(Some(p)),
                    None => {
                        all_literal = false;
                        patterns.push(None);
                    }
                },
            }
        }
        if !all_literal {
            return self.switch_as_ifs(disc, cases);
        }
        let d = self.expr(disc)?;
        let head = if self.is_str(&dt) {
            match d.repr {
                Repr::Str => d.code.clone(),
                _ => format!("{}.as_str()", Self::paren(&d, PREC_POSTFIX)),
            }
        } else {
            self.coerce(d, &dt, Want::Owned)
        };
        let label = self.fresh_label("switch");
        self.fn_ctx_mut().targets.push(Target::Switch {
            rust_label: label.clone(),
            used: false,
        });
        let mut has_default = false;
        let text = self.capture(|g| {
            g.indent += 1;
            let mut pending: Vec<String> = Vec::new();
            for (i, c) in cases.iter().enumerate() {
                match &patterns[i] {
                    Some(p) => pending.push(p.clone()),
                    None => {
                        has_default = true;
                        pending.push("_".into());
                    }
                }
                if c.body.is_empty() && i + 1 < cases.len() {
                    continue;
                }
                let mut body: &[Stmt] = &c.body;
                let falls_through = !body.last().is_some_and(always_exits) && i + 1 < cases.len();
                if falls_through && !body.is_empty() {
                    return g.err("a `case` must end with `break` or `return`: falling through is not supported", c.span);
                }
                // `case 1: { … break; }`: the block is the body.
                if let [Stmt { kind: StmtKind::Block(b), .. }] = body {
                    body = &b.stmts;
                }
                if let Some(Stmt { kind: StmtKind::Break(None), .. }) = body.last() {
                    body = &body[..body.len() - 1];
                }
                g.line(&format!("{} => {{", pending.join(" | ")));
                pending.clear();
                g.indent += 1;
                g.block_body(body, false)?;
                g.indent -= 1;
                g.line("}");
            }
            g.indent -= 1;
            Ok(())
        })?;
        let Some(Target::Switch { used, .. }) = self.fn_ctx_mut().targets.pop() else {
            unreachable!()
        };
        let covered = match self.resolve(&dt) {
            Ty::Named(n, ..) => self.p.enum_info(&n).is_some_and(|en| {
                en.variants.iter().all(|v| {
                    patterns
                        .iter()
                        .flatten()
                        .any(|p| p.ends_with(&format!("::{}", v.name)))
                })
            }),
            Ty::Bool => patterns.iter().flatten().count() == 2,
            _ => false,
        };
        if used {
            self.line(&format!("{label}: {{"));
            self.indent += 1;
        }
        self.line(&format!("match {head} {{"));
        self.out
            .push_str(&if used { indent_text(&text, 1) } else { text });
        if !has_default && !covered {
            self.indent += 1;
            self.line("_ => {}");
            self.indent -= 1;
        }
        self.line("}");
        if used {
            self.indent -= 1;
            self.line("}");
        }
        Ok(())
    }

    /// `switch (shape.kind)` on a union: a `match` on the union, each case binding its
    /// member.
    fn switch_on_union(&mut self, disc: &'a Expr, cases: &'a [Case]) -> GResult<Option<String>> {
        let ExprKind::Member { obj, .. } = &disc.kind else {
            return Ok(None);
        };
        let first_test = cases.iter().find_map(|c| c.test.as_ref());
        let Some(first) = first_test else {
            return Ok(None);
        };
        let Some(n0) = self.p.narrow_tests.get(&first.id).cloned() else {
            return Ok(None);
        };
        let Some((en, _)) = &n0.union else {
            return Ok(None);
        };
        let en = pascal(en);
        let ot = self.ty_of(obj);
        let o = self.expr(obj)?;
        let mutated = cases
            .iter()
            .filter_map(|c| c.test.as_ref())
            .any(|t| self.p.narrow_tests.get(&t.id).is_some_and(|n| n.mutated));
        let source = if mutated {
            match o.repr {
                Repr::RefMut => o.code.clone(),
                _ => format!("&mut {}", Self::paren(&o, PREC_UNARY)),
            }
        } else {
            match o.repr {
                Repr::Ref | Repr::RefMut => o.code.clone(),
                Repr::Owned if self.can_move(&o) => o.code.clone(),
                _ => format!("&{}", Self::paren(&o, PREC_UNARY)),
            }
        };
        let by_value = !mutated && o.repr == Repr::Owned && self.can_move(&o);
        let _ = ot;
        let label = self.fresh_label("switch");
        self.fn_ctx_mut().targets.push(Target::Switch {
            rust_label: label.clone(),
            used: false,
        });
        let mut has_default = false;
        let text = self.capture(|g| {
            g.indent += 1;
            let mut pending: Vec<String> = Vec::new();
            let mut bound: Vec<(String, Repr)> = Vec::new();
            for (i, c) in cases.iter().enumerate() {
                match &c.test {
                    Some(t) => {
                        let Some(n) = g.p.narrow_tests.get(&t.id).cloned() else {
                            return g.err("every `case` of a union must be one of its tags", t.span);
                        };
                        let (_, variant) = n.union.clone().expect("union");
                        let name = snake(&n.name);
                        let repr = if by_value {
                            Repr::Owned
                        } else if mutated {
                            Repr::RefMut
                        } else {
                            Repr::Ref
                        };
                        pending.push(format!("{en}::{variant}({name})"));
                        bound.push((name, repr));
                    }
                    None => {
                        has_default = true;
                        pending.push("_".into());
                    }
                }
                if c.body.is_empty() && i + 1 < cases.len() {
                    continue;
                }
                let mut body: &[Stmt] = &c.body;
                let falls_through = !body.last().is_some_and(always_exits) && i + 1 < cases.len();
                if falls_through && !body.is_empty() {
                    return g.err("a `case` must end with `break` or `return`: falling through is not supported", c.span);
                }
                if let [Stmt { kind: StmtKind::Block(b), .. }] = body {
                    body = &b.stmts;
                }
                if let Some(Stmt { kind: StmtKind::Break(None), .. }) = body.last() {
                    body = &body[..body.len() - 1];
                }
                // Several tags for one body: the binding must have the same name.
                let names: std::collections::BTreeSet<&str> = bound.iter().map(|(n, _)| n.as_str()).collect();
                if names.len() > 1 {
                    return g.err("cases sharing a body must test the same value", c.span);
                }
                g.line(&format!("{} => {{", pending.join(" | ")));
                pending.clear();
                let before = g.narrowed.len();
                if let Some((name, repr)) = bound.first() {
                    g.narrowed.push((name.clone(), *repr));
                }
                bound.clear();
                g.indent += 1;
                g.block_body(body, false)?;
                g.indent -= 1;
                g.narrowed.truncate(before);
                g.line("}");
            }
            g.indent -= 1;
            Ok(())
        })?;
        let Some(Target::Switch { used, .. }) = self.fn_ctx_mut().targets.pop() else {
            unreachable!()
        };
        let covered = self.p.union_info(&en).is_some_and(|u| {
            u.members
                .iter()
                .all(|m| text.contains(&format!("{en}::{}(", m.name)))
        }) || self.p.unions.iter().any(|u| {
            pascal(&u.name) == en
                && u.members
                    .iter()
                    .all(|m| text.contains(&format!("{en}::{}(", m.name)))
        });
        let mut out = String::new();
        let indent = "    ".repeat(self.indent);
        if used {
            out.push_str(&format!("{indent}{label}: {{\n"));
        }
        let inner_indent = if used {
            format!("{indent}    ")
        } else {
            indent.clone()
        };
        out.push_str(&format!("{inner_indent}match {source} {{\n"));
        out.push_str(&if used { indent_text(&text, 1) } else { text });
        if !has_default && !covered {
            out.push_str(&format!("{inner_indent}    _ => {{}}\n"));
        }
        out.push_str(&format!("{inner_indent}}}\n"));
        if used {
            out.push_str(&format!("{indent}}}\n"));
        }
        Ok(Some(out))
    }

    fn pattern_text(&mut self, t: &'a Expr) -> Option<String> {
        let ty = self.ty_of(t);
        match &t.kind {
            ExprKind::Num { value, .. } => Some(if self.is_float(&ty) {
                format!("{value:?}")
            } else {
                format!("{}", *value as i64)
            }),
            ExprKind::Unary {
                op: UnaryOp::Neg,
                arg,
            } => match &arg.kind {
                ExprKind::Num { value, .. } => Some(format!("-{}", *value as i64)),
                _ => None,
            },
            ExprKind::Str(s) => {
                if let Some((en, v)) = self.p.enum_literal.get(&t.id) {
                    return Some(format!("{}::{v}", super::pascal(en)));
                }
                Some(super::string_literal(s))
            }
            ExprKind::Bool(b) => Some(b.to_string()),
            ExprKind::Member { obj, prop, .. } => {
                if let ExprKind::Ident(_) = &obj.kind
                    && let Some(&b) = self.p.refs.get(&obj.id)
                    && self.p.var(b).role == Role::Enum
                {
                    let en = self.p.enum_info(&self.p.var(b).name)?;
                    let v = en.variants.iter().find(|v| v.source == *prop)?;
                    return Some(format!("{}::{}", super::pascal(&en.name), v.name));
                }
                None
            }
            _ => None,
        }
    }

    /// A `switch` whose cases are not constants: a chain of `if`.
    fn switch_as_ifs(&mut self, disc: &'a Expr, cases: &'a [Case]) -> GResult<()> {
        let dt = self.ty_of(disc);
        let d = self.expr_as(disc, Want::Owned)?;
        self.line(&format!("let switch_ = {d};"));
        let label = self.fresh_label("switch");
        self.fn_ctx_mut().targets.push(Target::Switch {
            rust_label: label.clone(),
            used: false,
        });
        let text = self.capture(|g| {
            let mut first = true;
            let mut pending: Vec<String> = Vec::new();
            for (i, c) in cases.iter().enumerate() {
                if let Some(t) = &c.test {
                    let v = g.expr_as(
                        t,
                        if g.is_str(&dt) {
                            Want::StrRef
                        } else {
                            Want::Owned
                        },
                    )?;
                    pending.push(format!("switch_ == {v}"));
                } else {
                    pending.push("true".into());
                }
                if c.body.is_empty() && i + 1 < cases.len() {
                    continue;
                }
                let mut body: &[Stmt] = &c.body;
                if let Some(Stmt {
                    kind: StmtKind::Break(None),
                    ..
                }) = body.last()
                {
                    body = &body[..body.len() - 1];
                }
                let test = if pending.iter().any(|p| p == "true") {
                    "true".to_string()
                } else {
                    pending.join(" || ")
                };
                pending.clear();
                let keyword = if first { "if" } else { "} else if" };
                first = false;
                if test == "true" {
                    g.line(if keyword == "if" {
                        "if true {"
                    } else {
                        "} else {"
                    });
                } else {
                    g.line(&format!("{keyword} {test} {{"));
                }
                g.indent += 1;
                g.block_body(body, false)?;
                g.indent -= 1;
            }
            if !first {
                g.line("}");
            }
            Ok(())
        })?;
        let Some(Target::Switch { used, .. }) = self.fn_ctx_mut().targets.pop() else {
            unreachable!()
        };
        if used {
            self.line(&format!("{label}: {{"));
            self.indent += 1;
            self.out.push_str(&indent_text(&text, 1));
            self.indent -= 1;
            self.line("}");
        } else {
            self.out.push_str(&text);
        }
        Ok(())
    }

    fn try_stmt(
        &mut self,
        block: &'a Block,
        catch: Option<&'a Catch>,
        finally: Option<&'a Block>,
    ) -> GResult<()> {
        // `return` inside the block: the closure gives back `Some(value)`.
        let returns = block
            .stmts
            .iter()
            .any(|s| walk(s, &mut |st| matches!(st.kind, StmtKind::Return(_)), false));
        // Every path of the block returns or throws: nothing runs after the closure.
        let always = self.p.try_returns.contains(&block.span.start);
        if contains_break_or_continue(&block.stmts) {
            return self.err(
                "`break` and `continue` inside `try` are not supported",
                block.span,
            );
        }
        let ret_ty = self.fn_ctx().ret_ty.clone();
        let result_ty = if returns {
            let t = self.rust_ty(&ret_ty, block.span)?;
            format!("Result<Option<{t}>, String>")
        } else {
            "Result<(), String>".into()
        };
        self.fn_ctx_mut().try_ctx.push(TryCtx { returns });
        let body = self.capture(|g| {
            g.indent += 1;
            g.block_body(&block.stmts, false)?;
            if !always {
                g.line(if returns { "Ok(None)" } else { "Ok(())" });
            }
            g.indent -= 1;
            Ok(())
        });
        self.fn_ctx_mut().try_ctx.pop();
        let body = body?;
        self.line(&format!("let result_: {result_ty} = (|| {{"));
        self.out.push_str(&body);
        self.line("})();");
        if let Some(f) = finally {
            self.block_body(&f.stmts, false)?;
        }
        let ret = if self.fn_ctx().returns_result {
            "return Ok(v)"
        } else {
            "return v"
        };
        match catch {
            Some(c) => {
                let param = match &c.param {
                    Some(Pattern::Ident { id, .. }) => self.bind(*id, Repr::Owned),
                    Some(other) => return self.err("unsupported catch parameter", other.span()),
                    None => "_".into(),
                };
                if returns {
                    self.line("match result_ {");
                    self.indent += 1;
                    self.line(&format!("Ok(Some(v)) => {ret},"));
                    self.line(if always {
                        "Ok(None) => unreachable!(),"
                    } else {
                        "Ok(None) => {}"
                    });
                    self.line(&format!("Err({param}) => {{"));
                    self.indent += 1;
                    self.block_body(&c.body.stmts, false)?;
                    self.indent -= 1;
                    self.line("}");
                    self.indent -= 1;
                    self.line("}");
                } else {
                    self.line(&format!("if let Err({param}) = result_ {{"));
                    self.indent += 1;
                    self.block_body(&c.body.stmts, false)?;
                    self.indent -= 1;
                    self.line("}");
                }
            }
            None => {
                // `try … finally`: errors go on.
                let outcome = if self.fn_ctx().returns_result {
                    "result_?"
                } else {
                    "result_.expect(\"unhandled error\")"
                };
                if !returns {
                    self.line(&format!("{outcome};"));
                } else if always {
                    self.line(&format!(
                        "let Some(v) = {outcome} else {{ unreachable!() }};"
                    ));
                    self.line(&format!("{ret};"));
                } else {
                    self.line(&format!("if let Some(v) = {outcome} {{"));
                    self.indent += 1;
                    self.line(&format!("{ret};"));
                    self.indent -= 1;
                    self.line("}");
                }
            }
        }
        Ok(())
    }
}

/// What a union value is on one side of a test.
#[derive(Clone, Debug)]
pub(super) enum Side {
    /// One member: its variant.
    Variant(String),
    /// The union of the remaining members.
    Union(Ty),
}

/// What a union test binds when true and when false.
pub(super) struct UnionSides {
    /// The union tested (TypeScript name).
    pub union: String,
    pub when_true: Option<Side>,
    pub when_false: Option<Side>,
    pub test_id: NodeId,
}

/// Whether a block ends by leaving (return, throw, break, continue, infinite loop).
fn ends_with_exit(stmts: &[Stmt]) -> bool {
    stmts.last().is_some_and(always_exits)
}

fn always_exits(s: &Stmt) -> bool {
    match &s.kind {
        StmtKind::Return(_) | StmtKind::Throw(_) | StmtKind::Break(_) | StmtKind::Continue(_) => {
            true
        }
        StmtKind::Block(b) => ends_with_exit(&b.stmts),
        StmtKind::If {
            then,
            otherwise: Some(o),
            ..
        } => always_exits(then) && always_exits(o),
        StmtKind::While { test, body } => {
            matches!(test.kind, ExprKind::Bool(true)) && !contains_break(body, None)
        }
        StmtKind::Switch { cases, .. } => {
            cases.iter().any(|c| c.test.is_none())
                && cases.iter().all(|c| {
                    ends_with_exit(&c.body)
                        && !matches!(c.body.last().map(|s| &s.kind), Some(StmtKind::Break(_)))
                })
        }
        StmtKind::Try { block, catch, .. } => {
            ends_with_exit(&block.stmts)
                && catch.as_ref().is_none_or(|c| ends_with_exit(&c.body.stmts))
        }
        _ => false,
    }
}

/// Whether the body of a loop contains a `continue` aimed at it.
fn contains_continue(s: &Stmt, label: Option<&str>) -> bool {
    walk(
        s,
        &mut |st| match &st.kind {
            StmtKind::Continue(None) => true,
            StmtKind::Continue(Some(l)) => label == Some(l.as_str()),
            _ => false,
        },
        true,
    )
}

fn contains_break(s: &Stmt, label: Option<&str>) -> bool {
    walk(
        s,
        &mut |st| match &st.kind {
            StmtKind::Break(None) => true,
            StmtKind::Break(Some(l)) => label == Some(l.as_str()),
            _ => false,
        },
        true,
    )
}

fn contains_break_or_continue(stmts: &[Stmt]) -> bool {
    stmts.iter().any(|s| {
        walk(
            s,
            &mut |st| matches!(st.kind, StmtKind::Break(_) | StmtKind::Continue(_)),
            true,
        )
    })
}

/// Visits the statements of `s` that belong to the same loop (not nested loops when
/// `stop_at_loops`), until `f` returns true.
fn walk(s: &Stmt, f: &mut dyn FnMut(&Stmt) -> bool, stop_at_loops: bool) -> bool {
    if f(s) {
        return true;
    }
    match &s.kind {
        StmtKind::Block(b) => b.stmts.iter().any(|x| walk(x, f, stop_at_loops)),
        StmtKind::If {
            then, otherwise, ..
        } => {
            walk(then, f, stop_at_loops)
                || otherwise
                    .as_ref()
                    .is_some_and(|o| walk(o, f, stop_at_loops))
        }
        StmtKind::While { body, .. }
        | StmtKind::DoWhile { body, .. }
        | StmtKind::For { body, .. }
        | StmtKind::ForOf { body, .. } => {
            if stop_at_loops {
                // A labeled continue could still target us; approximate.
                walk(
                    body,
                    &mut |st| {
                        matches!(
                            st.kind,
                            StmtKind::Continue(Some(_)) | StmtKind::Break(Some(_))
                        ) && f(st)
                    },
                    true,
                )
            } else {
                walk(body, f, stop_at_loops)
            }
        }
        StmtKind::Switch { cases, .. } => cases
            .iter()
            .any(|c| c.body.iter().any(|x| walk(x, f, stop_at_loops))),
        StmtKind::Try {
            block,
            catch,
            finally,
        } => {
            block.stmts.iter().any(|x| walk(x, f, stop_at_loops))
                || catch
                    .as_ref()
                    .is_some_and(|c| c.body.stmts.iter().any(|x| walk(x, f, stop_at_loops)))
                || finally
                    .as_ref()
                    .is_some_and(|b| b.stmts.iter().any(|x| walk(x, f, stop_at_loops)))
        }
        StmtKind::Labeled { body, .. } => walk(body, f, stop_at_loops),
        _ => false,
    }
}

/// Whether the loop body changes the collection it iterates (a method call on it).
fn body_mutates(body: &Stmt, obj: &Expr) -> bool {
    let ExprKind::Ident(name) = &obj.kind else {
        return false;
    };
    walk(
        body,
        &mut |st| match &st.kind {
            StmtKind::Expr(Expr {
                kind: ExprKind::Call { callee, .. },
                ..
            }) => {
                matches!(&callee.kind, ExprKind::Member { obj, prop, .. }
                if matches!(&obj.kind, ExprKind::Ident(n) if n == name)
                    && matches!(prop.as_str(), "set" | "delete" | "clear" | "add" | "push" | "pop" | "splice" | "shift" | "unshift"))
            }
            _ => false,
        },
        false,
    )
}

/// Whether the body assigns to the variable (the loop variable of a counted loop).
fn assigns_var(s: &Stmt, name: &str) -> bool {
    walk(
        s,
        &mut |st| match &st.kind {
            StmtKind::Expr(e) => expr_assigns(e, name),
            StmtKind::For {
                update: Some(u), ..
            } => expr_assigns(u, name),
            _ => false,
        },
        false,
    ) || walk(
        s,
        &mut |st| matches!(&st.kind, StmtKind::If { test, .. } if expr_assigns(test, name)),
        false,
    )
}

fn expr_assigns(e: &Expr, name: &str) -> bool {
    match &e.kind {
        ExprKind::Assign {
            target: AssignTarget::Expr(t),
            value,
            ..
        } => matches!(t.kind, ExprKind::Ident(ref n) if n == name) || expr_assigns(value, name),
        ExprKind::Update { arg, .. } => matches!(arg.kind, ExprKind::Ident(ref n) if n == name),
        ExprKind::Seq(items) => items.iter().any(|i| expr_assigns(i, name)),
        ExprKind::Call { args, .. } => args.iter().any(|a| expr_assigns(&a.expr, name)),
        ExprKind::Arrow(f) | ExprKind::Func(f) => match &f.body {
            FnBody::Block(b) => b.stmts.iter().any(|s| assigns_var(s, name)),
            FnBody::Expr(x) => expr_assigns(x, name),
        },
        _ => false,
    }
}

fn indent_text(text: &str, levels: usize) -> String {
    let pad = "    ".repeat(levels);
    text.lines()
        .map(|l| {
            if l.is_empty() {
                String::new()
            } else {
                format!("{pad}{l}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

#[allow(dead_code)]
const _ATOM: u8 = PREC_ATOM;
