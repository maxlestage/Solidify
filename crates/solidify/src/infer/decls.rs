//! Declarations: interfaces, type aliases, enums, classes and function signatures are
//! registered before any body is looked at, so that they can be used in any order.

use std::rc::Rc;

use super::{
    EnumInfo, EnumKind, EnumValue, FieldInfo, FnCtx, FnInfo, IResult, Infer, MethodInfo, Role,
    StructInfo, StructKind, Variant,
};
use crate::ast::*;
use crate::error::Span;
use crate::infer::pascal;
use crate::types::{FuncTy, Ty};

impl<'m> Infer<'m> {
    pub(super) fn run(&mut self, module: &'m Module) -> IResult<()> {
        // The top level is a function: `main`.
        self.fns.push(FnCtx {
            id: self.main_id,
            ret: Ty::Void,
            this: None,
            in_ctor: false,
            is_static: false,
            try_depth: 0,
            loop_depth: 0,
            type_params: Vec::new(),
            captures: Vec::new(),
            this_fn: self.main_id,
            has_return_value: false,
            yield_ty: None,
        });
        self.funcs.insert(
            self.main_id,
            FnInfo {
                id: self.main_id,
                name: Some("main".into()),
                params: Vec::new(),
                optional: Vec::new(),
                defaults: Vec::new(),
                sig: FuncTy {
                    params: Vec::new(),
                    ret: Ty::Void,
                },
                type_params: Vec::new(),
                type_bounds: Vec::new(),
                throws: false,
                throw_direct: false,
                is_async: false,
                is_arrow: false,
                method_of: None,
                method_kind: None,
                is_static: false,
                is_abstract: false,
                mutates_self: false,
                captures: Vec::new(),
                escapes: false,
                is_closure: false,
                parent: None,
                calls: Vec::new(),
                uses_this: false,
                returns_this: false,
                exported: false,
                rest: false,
                span: Span::default(),
            },
        );
        self.check_cross_file_names(&module.body)?;
        self.collect_types(&module.body)?;
        // Imported types first: annotations of top-level values may name them.
        self.bind_imports(&module.body, true)?;
        self.collect_values(&module.body, true)?;
        self.bind_imports(&module.body, false)?;
        for s in &module.body {
            self.stmt(s)?;
            if !matches!(
                s.kind,
                StmtKind::Func(_)
                    | StmtKind::Class(_)
                    | StmtKind::Interface(_)
                    | StmtKind::TypeAlias(_)
                    | StmtKind::Enum(_)
                    | StmtKind::Empty
                    | StmtKind::Import(_)
                    | StmtKind::ExportNames(_)
                    | StmtKind::ExportFrom { .. }
            ) {
                self.has_main = true;
            }
        }
        let ctx = self.fns.pop().expect("main");
        let info = self.funcs.get_mut(&self.main_id).expect("main");
        info.captures = ctx.captures;
        // Timers run after the script: `main` exists for them.
        if self.uses_timers {
            self.has_main = true;
        }
        Ok(())
    }

    /// Registers interfaces, type aliases, enums and classes (any order).
    pub(super) fn collect_types(&mut self, stmts: &'m [Stmt]) -> IResult<()> {
        // Names first, so that types can refer to each other.
        for s in stmts {
            match &s.kind {
                StmtKind::Interface(i) => {
                    self.check_type_name(&i.name, i.span)?;
                    self.structs.push(StructInfo {
                        name: i.name.clone(),
                        kind: StructKind::Interface,
                        fields: Vec::new(),
                        methods: Vec::new(),
                        ctor: None,
                        exported: i.exported,
                        span: i.span,
                        statics: Vec::new(),
                        parent: None,
                        is_abstract: false,
                        implements: Vec::new(),
                        type_params: i.type_params.iter().map(|t| t.name.clone()).collect(),
                        type_bounds: Vec::new(),
                        parent_args: Vec::new(),
                        implements_args: Vec::new(),
                    });
                }
                StmtKind::Class(c) => {
                    self.check_type_name(&c.name, c.span)?;
                    self.structs.push(StructInfo {
                        name: c.name.clone(),
                        kind: StructKind::Class,
                        fields: Vec::new(),
                        methods: Vec::new(),
                        ctor: None,
                        exported: c.exported,
                        span: c.span,
                        statics: Vec::new(),
                        parent: None,
                        is_abstract: false,
                        implements: Vec::new(),
                        type_params: c.type_params.iter().map(|t| t.name.clone()).collect(),
                        type_bounds: Vec::new(),
                        parent_args: Vec::new(),
                        implements_args: Vec::new(),
                    });
                }
                StmtKind::Enum(e) => {
                    self.check_type_name(&e.name, e.span)?;
                    self.define_enum(e)?;
                }
                StmtKind::TypeAlias(a) => {
                    self.check_type_name(&a.name, a.span)?;
                    match &a.ty {
                        TypeAnn::Object(_, _) => self.structs.push(StructInfo {
                            name: a.name.clone(),
                            kind: StructKind::Interface,
                            fields: Vec::new(),
                            methods: Vec::new(),
                            ctor: None,
                            exported: a.exported,
                            span: a.span,
                            statics: Vec::new(),
                            parent: None,
                            is_abstract: false,
                            implements: Vec::new(),
                            type_params: a.type_params.iter().map(|t| t.name.clone()).collect(),
                            type_bounds: Vec::new(),
                            parent_args: Vec::new(),
                            implements_args: Vec::new(),
                        }),
                        TypeAnn::Union(members, span)
                            if members.len() > 1
                                && members
                                    .iter()
                                    .all(|m| matches!(m, TypeAnn::Lit(Lit::Str(_), _))) =>
                        {
                            let literals: Vec<String> = members
                                .iter()
                                .map(|m| match m {
                                    TypeAnn::Lit(Lit::Str(s), _) => s.clone(),
                                    _ => unreachable!(),
                                })
                                .collect();
                            self.define_literal_enum(&a.name, &literals, a.exported, *span);
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        // Other aliases, which may name the types above.
        for s in stmts {
            if let StmtKind::TypeAlias(a) = &s.kind
                && !self.type_exists(&a.name)
            {
                let before = self.unions.len();
                if !a.type_params.is_empty() {
                    let params: Vec<String> =
                        a.type_params.iter().map(|t| t.name.clone()).collect();
                    self.param_scope.push(params.clone());
                    let ty = self.ann(&a.ty, &a.name);
                    self.param_scope.pop();
                    let ty = ty?;
                    if self.unions.len() > before {
                        return self.err(
                            "a generic alias cannot be a union of object types: declare a generic interface per member, or give the alias no parameter",
                            a.span,
                        );
                    }
                    self.generic_aliases.insert(a.name.clone(), (params, ty));
                    continue;
                }
                let ty = self.ann(&a.ty, &a.name)?;
                // A union made for this alias takes its name.
                if let Ty::Named(n, ..) = &ty
                    && self.unions.len() > before
                    && self.unions.last().is_some_and(|u| u.name == *n)
                {
                    let u = self.unions.last_mut().expect("union");
                    u.name = a.name.clone();
                    u.exported = a.exported;
                    self.aliases
                        .insert(a.name.clone(), Ty::named(a.name.clone()));
                    continue;
                }
                self.aliases.insert(a.name.clone(), ty);
            }
        }
        // Fields and signatures.
        for s in stmts {
            if let StmtKind::Class(c) = &s.kind {
                self.class_asts.insert(c.name.clone(), c);
            }
        }
        for s in stmts {
            match &s.kind {
                StmtKind::Interface(i) => {
                    self.param_scope
                        .push(i.type_params.iter().map(|t| t.name.clone()).collect());
                    let mut fields = Vec::new();
                    let mut methods = Vec::new();
                    for base in &i.extends {
                        let TypeAnn::Named { name, args, .. } = base else {
                            return self.err(
                                "an interface can only extend a named interface",
                                base.span(),
                            );
                        };
                        let Some(parent) = self.struct_by_name(name).cloned() else {
                            return self.err(format!("unknown interface `{name}`"), base.span());
                        };
                        if parent.kind == StructKind::Class {
                            return self.err("an interface cannot extend a class", base.span());
                        }
                        // `interface Tagged extends Named<number>`: the parent's
                        // parameters given.
                        let mut given = std::collections::HashMap::new();
                        for (k, p) in parent.type_params.iter().enumerate() {
                            let t = match args.get(k) {
                                Some(a) => self.ann(a, &i.name)?,
                                None => self.types.fresh(),
                            };
                            given.insert(p.clone(), t);
                        }
                        if given.is_empty() {
                            fields.extend(parent.fields.iter().cloned());
                            methods.extend(parent.methods.iter().cloned());
                            continue;
                        }
                        for f in &parent.fields {
                            fields.push(FieldInfo {
                                ty: super::finish::substitute(&f.ty, &given, &self.types),
                                ..f.clone()
                            });
                        }
                        for m in &parent.methods {
                            // A method of its own, with the parameters given.
                            let mut info = self.funcs[&m.fn_id].clone();
                            let id = self.synth_id();
                            info.id = id;
                            info.method_of = Some(i.name.clone());
                            info.sig = FuncTy {
                                params: info
                                    .sig
                                    .params
                                    .iter()
                                    .map(|t| super::finish::substitute(t, &given, &self.types))
                                    .collect(),
                                ret: super::finish::substitute(&info.sig.ret, &given, &self.types),
                            };
                            self.funcs.insert(id, info);
                            methods.push(MethodInfo {
                                fn_id: id,
                                ..m.clone()
                            });
                        }
                    }
                    let (own_fields, own_methods) = self.members_of(&i.name, &i.members)?;
                    fields.extend(own_fields);
                    methods.extend(own_methods);
                    self.param_scope.pop();
                    let s = self.struct_mut(&i.name).expect("interface");
                    s.fields = fields;
                    s.methods = methods;
                }
                StmtKind::TypeAlias(a) => {
                    if let TypeAnn::Object(members, _) = &a.ty {
                        self.param_scope
                            .push(a.type_params.iter().map(|t| t.name.clone()).collect());
                        let fields = self.fields_from_members(&a.name, members);
                        self.param_scope.pop();
                        let fields = fields?;
                        self.struct_mut(&a.name).expect("alias").fields = fields;
                    }
                }
                StmtKind::Class(c) => self.collect_class(c)?,
                _ => {}
            }
        }
        Ok(())
    }

    fn check_type_name(&self, name: &str, span: Span) -> IResult<()> {
        if self.type_exists(name) {
            return self.err(format!("`{name}` is declared twice"), span);
        }
        Ok(())
    }

    fn define_enum(&mut self, e: &EnumDecl) -> IResult<()> {
        let mut variants = Vec::new();
        let mut next = 0.0;
        let mut kind = EnumKind::Numeric;
        for m in &e.members {
            let value = match &m.init {
                None => {
                    let v = next;
                    next += 1.0;
                    EnumValue::Num(v)
                }
                Some(Expr {
                    kind: ExprKind::Num { value, .. },
                    ..
                }) => {
                    next = value + 1.0;
                    EnumValue::Num(*value)
                }
                Some(Expr {
                    kind:
                        ExprKind::Unary {
                            op: UnaryOp::Neg,
                            arg,
                        },
                    ..
                }) if matches!(arg.kind, ExprKind::Num { .. }) => {
                    let ExprKind::Num { value, .. } = &arg.kind else {
                        unreachable!()
                    };
                    next = -value + 1.0;
                    EnumValue::Num(-value)
                }
                Some(Expr {
                    kind: ExprKind::Str(s),
                    ..
                }) => {
                    kind = EnumKind::Strings;
                    EnumValue::Str(s.clone())
                }
                Some(other) => {
                    return self.err("enum members must be numbers or strings", other.span);
                }
            };
            variants.push(Variant {
                name: pascal(&m.name),
                source: m.name.clone(),
                value: Some(value),
            });
        }
        self.enums.push(EnumInfo {
            name: e.name.clone(),
            variants,
            kind,
            exported: e.exported,
            span: e.span,
        });
        Ok(())
    }

    /// Fields, methods and the constructor of a class (signatures only).
    fn collect_class(&mut self, c: &'m Class) -> IResult<()> {
        if !self.collected_classes.insert(c.name.clone()) {
            return Ok(());
        }
        self.param_scope
            .push(c.type_params.iter().map(|t| t.name.clone()).collect());
        let r = self.collect_class_members(c);
        self.param_scope.pop();
        r
    }

    fn collect_class_members(&mut self, c: &'m Class) -> IResult<()> {
        let bounds = self.bounds_of(&c.type_params, &c.name);
        if let Some(s) = self.struct_mut(&c.name) {
            s.type_bounds = bounds;
        }
        if c.name.is_empty() {
            return self.err("classes need a name", c.span);
        }
        // The base class first: its fields come first, its methods are inherited.
        let mut parent = None;
        let mut fields = Vec::new();
        if let Some(base) = &c.extends {
            let ExprKind::Ident(base_name) = &base.kind else {
                return self.err(
                    "a class can only extend a class declared in the program",
                    base.span,
                );
            };
            if base_name == "Error" {
                return self.err(
                    "custom error classes are not supported: throw `new Error(message)`",
                    base.span,
                );
            }
            let Some(base_ast) = self.class_asts.get(base_name.as_str()).copied() else {
                return self.err(format!("unknown base class `{base_name}`"), base.span);
            };
            if self.is_subclass(base_name, &c.name) && base_name != &c.name {
                return self.err("classes cannot extend each other in a cycle", c.span);
            }
            self.collect_class(base_ast)?;
            let base_info = self.struct_by_name(base_name).expect("base").clone();
            // `extends Base<number>`: the base's fields with its parameters given.
            let n = base_info.type_params.len();
            if !c.extends_args.is_empty() && c.extends_args.len() != n {
                return self.err(
                    format!(
                        "{base_name} takes {n} type argument{}",
                        if n == 1 { "" } else { "s" }
                    ),
                    base.span,
                );
            }
            let mut args = Vec::new();
            for i in 0..n {
                args.push(match c.extends_args.get(i) {
                    Some(a) => self.ann(a, &c.name)?,
                    None => self.types.fresh(),
                });
            }
            let subst: std::collections::HashMap<String, Ty> = base_info
                .type_params
                .iter()
                .cloned()
                .zip(args.iter().cloned())
                .collect();
            fields.extend(base_info.fields.iter().map(|f| FieldInfo {
                ty: super::finish::substitute(&f.ty, &subst, &self.types),
                ..f.clone()
            }));
            if let Some(s) = self.struct_mut(&c.name) {
                s.parent_args = args;
            }
            parent = Some(base_name.clone());
        }
        self.this_class_stack.push(c.name.clone());
        let mut methods = Vec::new();
        let mut statics = Vec::new();
        let mut ctor = None;
        // Parameter properties are set before the field initializers run: they come first.
        let mut declared: Vec<FieldInfo> = Vec::new();
        for m in &c.members {
            match m {
                ClassMember::Field(f) => {
                    let ty = match &f.ty {
                        Some(t) => self.ann(t, &format!("{} {}", c.name, f.name))?,
                        None => self.types.fresh(),
                    };
                    let (ty, optional) = match (self.types.resolve(&ty), f.optional) {
                        (Ty::Option(inner), true) => (*inner, true),
                        (_, o) => (ty, o),
                    };
                    if f.is_static {
                        statics.push((f.name.clone(), ty, None));
                        continue;
                    }
                    let literal = match (&f.ty, &f.init) {
                        (Some(TypeAnn::Lit(Lit::Str(s), _)), _) => Some(s.clone()),
                        (
                            None,
                            Some(Expr {
                                kind: ExprKind::Str(s),
                                ..
                            }),
                        ) if f.readonly => Some(s.clone()),
                        _ => None,
                    };
                    if f.ty.as_ref().is_some_and(super::says_undefined) {
                        self.undef_fields.insert((c.name.clone(), f.name.clone()));
                    }
                    let field = FieldInfo {
                        name: f.name.clone(),
                        ty,
                        optional,
                        access: f.access,
                        readonly: f.readonly,
                        has_init: f.init.is_some(),
                        literal,
                        span: f.span,
                    };
                    declared.push(field);
                }
                ClassMember::Method(m) => {
                    if m.kind == MethodKind::Constructor {
                        // Parameter properties become fields.
                        for p in &m.func.params {
                            if p.access.is_some() || p.readonly {
                                let Pattern::Ident { name, span, .. } = &p.pat else {
                                    return self
                                        .err("parameter properties must be plain names", p.span);
                                };
                                let ty = match &p.ty {
                                    Some(t) => self.ann(t, &format!("{} {name}", c.name))?,
                                    None => self.types.fresh(),
                                };
                                let (ty, optional) = match (self.types.resolve(&ty), p.optional) {
                                    (Ty::Option(inner), true) => (*inner, true),
                                    (_, o) => (ty, o),
                                };
                                fields.push(FieldInfo {
                                    name: name.clone(),
                                    ty,
                                    optional,
                                    access: p.access,
                                    readonly: p.readonly,
                                    has_init: true,
                                    literal: None,
                                    span: *span,
                                });
                            }
                        }
                        ctor = Some(m.func.id);
                    }
                    let mut info =
                        self.signature(&m.func, Some(&c.name), Some(m.kind), m.is_static)?;
                    info.is_abstract = m.is_abstract;
                    // A parameter property has the type of its field.
                    if m.kind == MethodKind::Constructor {
                        for (i, p) in m.func.params.iter().enumerate() {
                            if (p.access.is_some() || p.readonly)
                                && let Pattern::Ident { name, .. } = &p.pat
                                && let Some(f) = fields.iter().find(|f| f.name == *name)
                            {
                                let fty = if f.optional {
                                    Ty::Option(Box::new(f.ty.clone()))
                                } else {
                                    f.ty.clone()
                                };
                                self.unify(&info.sig.params[i], &fty, p.span)?;
                            }
                        }
                    }
                    if m.is_abstract && !c.is_abstract {
                        return self.err("abstract methods belong in an abstract class", m.span);
                    }
                    // An override: same signature as the base method.
                    if let Some(p) = &parent
                        && m.kind != MethodKind::Constructor
                        && let Some((_, base_m)) = self.find_method(p, &m.name, m.kind)
                    {
                        let base_sig = self.funcs[&base_m.fn_id].sig.clone();
                        // A generic base: its parameters as this class gives them.
                        let given = self.ancestor_subst(&c.name, std::collections::HashMap::new());
                        let base_sig = FuncTy {
                            params: base_sig
                                .params
                                .iter()
                                .map(|t| super::finish::substitute(t, &given, &self.types))
                                .collect(),
                            ret: super::finish::substitute(&base_sig.ret, &given, &self.types),
                        };
                        if base_sig.params.len() != info.sig.params.len() {
                            return self.err(
                                format!("`{}` takes a different number of parameters than in the base class", m.name),
                                m.span,
                            );
                        }
                        for (a, b) in base_sig.params.iter().zip(&info.sig.params) {
                            self.unify(a, b, m.span)?;
                        }
                        self.unify(&base_sig.ret, &info.sig.ret, m.span)?;
                        self.overrides
                            .entry(base_m.fn_id)
                            .or_default()
                            .push(m.func.id);
                    }
                    self.funcs.insert(m.func.id, info);
                    if m.kind != MethodKind::Constructor {
                        methods.push(MethodInfo {
                            name: m.name.clone(),
                            fn_id: m.func.id,
                            kind: m.kind,
                            is_static: m.is_static,
                            is_abstract: m.is_abstract,
                            access: m.access,
                        });
                    }
                }
                ClassMember::StaticBlock(b) => {
                    return self.err("static blocks are not supported", b.span);
                }
            }
        }
        // A property's getter and setter see one type.
        for g in methods.iter().filter(|m| m.kind == MethodKind::Get) {
            if let Some(s) = methods
                .iter()
                .find(|m| m.kind == MethodKind::Set && m.name == g.name)
                && let Some(st) = self.funcs[&s.fn_id].sig.params.first().cloned()
            {
                let gt = self.funcs[&g.fn_id].sig.ret.clone();
                self.unify(&gt, &st, c.span)?;
            }
        }
        for field in declared {
            // A field declared again in a subclass keeps its place.
            match fields.iter().position(|g: &FieldInfo| g.name == field.name) {
                Some(i) => {
                    self.unify(&fields[i].ty, &field.ty, field.span)?;
                    fields[i] = field;
                }
                None => fields.push(field),
            }
        }
        self.this_class_stack.pop();
        let s = self.struct_mut(&c.name).expect("class");
        s.fields = fields;
        s.methods = methods;
        s.ctor = ctor;
        s.statics = statics;
        s.parent = parent;
        s.is_abstract = c.is_abstract;
        // `implements I`: every field and method of `I` is here, with the same type.
        let mut implements = Vec::new();
        let mut implements_args = Vec::new();
        for ann in &c.implements {
            let TypeAnn::Named {
                name: iname,
                args: iargs,
                ..
            } = ann
            else {
                return self.err("`implements` needs the name of an interface", ann.span());
            };
            let Some(iface) = self.struct_by_name(iname).cloned() else {
                return self.err(format!("unknown interface `{iname}`"), ann.span());
            };
            if iface.kind == StructKind::Class {
                return self.err(
                    format!("`{iname}` is a class: write `extends {iname}`"),
                    ann.span(),
                );
            }
            // `implements Container<number>`: the interface's parameters given.
            let mut args = Vec::new();
            for (i, _) in iface.type_params.iter().enumerate() {
                args.push(match iargs.get(i) {
                    Some(a) => self.ann(a, &c.name)?,
                    None => self.types.fresh(),
                });
            }
            let given: std::collections::HashMap<String, Ty> = iface
                .type_params
                .iter()
                .cloned()
                .zip(args.iter().cloned())
                .collect();
            let sub = |this: &Self, t: &Ty| super::finish::substitute(t, &given, &this.types);
            for f in &iface.fields {
                let Some(cf) = self
                    .struct_by_name(&c.name)
                    .and_then(|s| s.field(&f.name).cloned())
                else {
                    return self.err(
                        format!(
                            "`{}` implements `{iname}` but has no field `{}`",
                            c.name, f.name
                        ),
                        c.span,
                    );
                };
                let ft = sub(self, &f.ty);
                self.unify(&cf.ty, &ft, cf.span)?;
            }
            for m in &iface.methods {
                let Some((_, cm)) = self.find_method(&c.name, &m.name, m.kind) else {
                    return self.err(
                        format!(
                            "`{}` implements `{iname}` but has no method `{}`",
                            c.name, m.name
                        ),
                        c.span,
                    );
                };
                let (a, b) = (
                    self.funcs[&m.fn_id].sig.clone(),
                    self.funcs[&cm.fn_id].sig.clone(),
                );
                if a.params.len() != b.params.len() {
                    return self.err(
                        format!(
                            "`{}` takes a different number of parameters than in `{iname}`",
                            m.name
                        ),
                        c.span,
                    );
                }
                for (x, y) in a.params.iter().zip(&b.params) {
                    let x = sub(self, x);
                    self.unify(&x, y, c.span)?;
                }
                let ret = sub(self, &a.ret);
                self.unify(&ret, &b.ret, c.span)?;
                self.link_implementation(m.fn_id, cm.fn_id);
            }
            implements.push(iname.clone());
            if !args.is_empty() {
                implements_args.push((iname.clone(), args));
            }
        }
        let s = self.struct_mut(&c.name).expect("class");
        s.implements = implements;
        s.implements_args = implements_args;
        Ok(())
    }

    /// The signature of a function, with fresh variables where annotations are missing.
    pub(super) fn signature(
        &mut self,
        f: &'m Function,
        method_of: Option<&str>,
        method_kind: Option<MethodKind>,
        is_static: bool,
    ) -> IResult<FnInfo> {
        if f.is_generator && f.is_async {
            return self.err("async generators are not supported", f.span);
        }
        let type_params: Vec<String> = f.type_params.iter().map(|t| t.name.clone()).collect();
        // Type parameters are visible while reading the annotations.
        self.fns.push(FnCtx {
            id: f.id,
            ret: Ty::Void,
            this: None,
            in_ctor: false,
            is_static,
            try_depth: 0,
            loop_depth: 0,
            type_params: type_params.clone(),
            captures: Vec::new(),
            this_fn: f.id,
            has_return_value: false,
            yield_ty: None,
        });
        let result = self
            .signature_inner(f, method_of, method_kind, is_static, type_params)
            .map(|mut info| {
                if f.ret.as_ref().is_some_and(super::says_undefined) {
                    self.undef_fns.insert(f.id);
                }
                for (i, p) in f.params.iter().enumerate() {
                    if p.ty.as_ref().is_some_and(super::says_undefined) {
                        self.undef_params.insert((f.id, i));
                    }
                }
                let owner = f.name.clone().unwrap_or_else(|| "fn".into());
                info.type_bounds = self.bounds_of(&f.type_params, &owner);
                info
            });
        self.fns.pop();
        result
    }

    /// `<T extends HasId>`: the parameters constrained to an interface or a class. Other
    /// constraints (`extends string`, `keyof`) only check the arguments in TypeScript:
    /// they are not needed here.
    pub(super) fn bounds_of(
        &mut self,
        params: &'m [TypeParam],
        owner: &str,
    ) -> Vec<(String, String)> {
        let mut out = Vec::new();
        for tp in params {
            let Some(c) = &tp.constraint else { continue };
            // An inline object type (`T extends { id: number }`) is named after its owner.
            if let Ok(t) = self.ann(c, &format!("{owner} {}", tp.name))
                && let Ty::Named(n, _) = self.types.resolve(&t)
                && self.struct_by_name(&n).is_some()
                && n != tp.name
            {
                out.push((tp.name.clone(), n));
            }
        }
        out
    }

    fn signature_inner(
        &mut self,
        f: &'m Function,
        method_of: Option<&str>,
        method_kind: Option<MethodKind>,
        is_static: bool,
        type_params: Vec<String>,
    ) -> IResult<FnInfo> {
        let hint = f.name.clone().unwrap_or_default();
        let mut params = Vec::new();
        let mut optional = Vec::new();
        let mut defaults = Vec::new();
        let mut rest = false;
        for (i, p) in f.params.iter().enumerate() {
            if p.rest {
                if i + 1 != f.params.len() {
                    return self.err("a rest parameter comes last", p.span);
                }
                rest = true;
            }
            let name_hint = match &p.pat {
                Pattern::Ident { name, .. } => name.clone(),
                _ => format!("{hint} arg{i}"),
            };
            let mut ty = match &p.ty {
                Some(t) => self.ann(t, &name_hint)?,
                None if p.rest => Ty::Array(Box::new(self.types.fresh())),
                None => self.types.fresh(),
            };
            if p.rest && !matches!(self.types.resolve(&ty), Ty::Array(_)) {
                return self.err("a rest parameter is an array: `...xs: T[]`", p.span);
            }
            if p.optional {
                ty = match self.types.resolve(&ty) {
                    Ty::Option(t) => Ty::Option(t),
                    t => Ty::Option(Box::new(t)),
                };
            }
            params.push(ty);
            optional.push(p.optional);
            defaults.push(p.default.is_some());
        }
        let ret = match &f.ret {
            Some(t) => self.ann(t, &hint)?,
            None => {
                if matches!(method_kind, Some(MethodKind::Constructor | MethodKind::Set)) {
                    Ty::Void
                } else if f.is_generator {
                    Ty::Gen(Box::new(self.types.fresh()))
                } else {
                    self.types.fresh()
                }
            }
        };
        if f.is_generator && !matches!(self.types.resolve(&ret), Ty::Gen(_)) {
            return self.err("a generator returns `Generator<T>`", f.span);
        }
        Ok(FnInfo {
            id: f.id,
            name: f.name.clone(),
            params: Vec::new(),
            optional,
            defaults,
            sig: FuncTy { params, ret },
            type_params,
            type_bounds: Vec::new(),
            throws: false,
            throw_direct: false,
            // A function giving a `Promise` is awaited by its callers: async in Rust.
            is_async: f.is_async
                || matches!(&f.ret, Some(TypeAnn::Named { name, .. }) if name == "Promise"),
            is_arrow: f.is_arrow,
            method_of: method_of.map(str::to_string),
            method_kind,
            is_static,
            is_abstract: false,
            rest,
            mutates_self: false,
            captures: Vec::new(),
            escapes: false,
            is_closure: false,
            parent: None,
            calls: Vec::new(),
            uses_this: false,
            returns_this: false,
            exported: f.exported,
            span: f.span,
        })
    }

    /// Declares the functions of a block, so that they can be called before their
    /// declaration, and the top-level variables, so that functions declared earlier see
    /// them.
    pub(super) fn collect_values(&mut self, stmts: &'m [Stmt], top: bool) -> IResult<()> {
        for s in stmts {
            match &s.kind {
                StmtKind::Func(f) if f.id != usize::MAX => {
                    let name = f.name.clone().expect("declared function");
                    let info = self.signature(f, None, None, false)?;
                    let ty = Ty::Func(Rc::new(info.sig.clone()));
                    self.funcs.insert(f.id, info);
                    let id = self.synth_id();
                    self.declare(&name, id, ty, Role::Func, f.span);
                    self.fn_bindings.insert(id, f.id);
                }
                StmtKind::Class(c) => {
                    let id = self.synth_id();
                    self.declare(&c.name, id, Ty::named(c.name.clone()), Role::Class, c.span);
                }
                StmtKind::Enum(e) => {
                    let id = self.synth_id();
                    self.declare(&e.name, id, Ty::named(e.name.clone()), Role::Enum, e.span);
                }
                StmtKind::Var(v) if top => {
                    for d in &v.decls {
                        self.check_redeclaration(v.kind, &d.pat)?;
                        if !matches!(d.pat, Pattern::Ident { .. }) {
                            // Destructured at the top level: bound when reached.
                            continue;
                        }
                        let hint = match &d.pat {
                            Pattern::Ident { name, .. } => name.clone(),
                            _ => String::new(),
                        };
                        let ty = match &d.ty {
                            Some(t) => self.ann(t, &hint)?,
                            None => self.types.fresh(),
                        };
                        self.bind_pattern(&d.pat, &ty, Role::TopLevel)?;
                        if let (Pattern::Ident { id, .. }, Some(init)) = (&d.pat, &d.init) {
                            let literal = matches!(
                                init.kind,
                                ExprKind::Num { .. } | ExprKind::Str(_) | ExprKind::Bool(_)
                            ) || matches!(&init.kind, ExprKind::Unary { op: UnaryOp::Neg, arg } if matches!(arg.kind, ExprKind::Num { .. }));
                            if v.kind == VarKind::Const && literal {
                                self.vars.get_mut(id).expect("var").const_literal = true;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}
