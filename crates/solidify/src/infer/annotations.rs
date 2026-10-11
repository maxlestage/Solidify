//! From type annotations to types.

use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use super::{
    EnumInfo, EnumKind, EnumValue, FieldInfo, IResult, Infer, StructInfo, StructKind, Variant,
};
use crate::ast::*;
use crate::error::Span;
use crate::infer::{FnInfo, MethodInfo, pascal};
use crate::types::{Flavor, FuncTy, Ty};

/// The field of inline object types that holds a different string literal in each: the
/// discriminant of a union written in place.
fn inline_discriminant(members: &[&TypeAnn]) -> Option<String> {
    let objects: Vec<&Vec<TypeMember>> = members
        .iter()
        .filter_map(|m| match m {
            TypeAnn::Object(props, _) => Some(props),
            _ => None,
        })
        .collect();
    if objects.len() < 2 {
        return None;
    }
    let first = objects[0];
    for m in first {
        let TypeMember::Prop {
            name,
            ty: TypeAnn::Lit(Lit::Str(_), _),
            ..
        } = m
        else {
            continue;
        };
        let mut tags = HashSet::new();
        let all = objects.iter().all(|props| {
            props.iter().any(|p| match p {
                TypeMember::Prop {
                    name: n,
                    ty: TypeAnn::Lit(Lit::Str(s), _),
                    ..
                } if n == name => tags.insert(s.clone()),
                _ => false,
            })
        });
        if all && tags.len() == objects.len() {
            return Some(name.clone());
        }
    }
    None
}

impl<'m> Infer<'m> {
    /// The type an annotation denotes. `hint` names the structs made for inline object types.
    pub(super) fn ann(&mut self, ann: &TypeAnn, hint: &str) -> IResult<Ty> {
        match ann {
            TypeAnn::Named { name, args, span } => self.named_ann(name, args, *span, hint),
            TypeAnn::Array(t, _) => Ok(Ty::Array(Box::new(self.ann(t, hint)?))),
            TypeAnn::Readonly(t, _) => self.ann(t, hint),
            TypeAnn::Tuple(ts, _) => {
                let mut out = Vec::new();
                for t in ts {
                    out.push(self.ann(t, hint)?);
                }
                Ok(Ty::Tuple(out))
            }
            TypeAnn::Union(members, span) => self.union_ann(members, *span, hint),
            TypeAnn::Intersection(_, span) => {
                self.err("intersection types are not supported", *span)
            }
            TypeAnn::Object(members, span) => {
                let name = self.synth_name(hint);
                self.define_struct_from_members(&name, members, StructKind::Synth, false, *span)?;
                Ok(Ty::named(name))
            }
            TypeAnn::Func {
                params,
                ret,
                type_params,
                span,
            } => {
                if !type_params.is_empty() {
                    return self.err("generic function types are not supported", *span);
                }
                let mut ps = Vec::new();
                for p in params {
                    let mut t = match &p.ty {
                        Some(t) => self.ann(t, hint)?,
                        None => self.types.fresh(),
                    };
                    if p.optional {
                        t = Ty::Option(Box::new(t));
                    }
                    ps.push(t);
                }
                let ret = self.ann(ret, hint)?;
                Ok(Ty::Func(Rc::new(FuncTy { params: ps, ret })))
            }
            TypeAnn::Lit(lit, span) => match lit {
                Lit::Str(_) => {
                    let _ = span;
                    Ok(Ty::Str)
                }
                Lit::Num(v) => Ok(if v.fract() == 0.0 {
                    self.types.int()
                } else {
                    self.types.float()
                }),
                Lit::Bool(_) => Ok(Ty::Bool),
                Lit::Null | Lit::Undefined => Ok(Ty::Null),
            },
            TypeAnn::TypeOf(_, span) => self.err("`typeof` types are not supported", *span),
            TypeAnn::KeyOf(_, span) => self.err("`keyof` types are not supported", *span),
            TypeAnn::Indexed(_, _, span) => {
                self.err("indexed access types are not supported", *span)
            }
        }
    }

    fn named_ann(&mut self, name: &str, args: &[TypeAnn], span: Span, hint: &str) -> IResult<Ty> {
        let arg = |this: &mut Self, i: usize| -> IResult<Ty> {
            match args.get(i) {
                Some(a) => this.ann(a, hint),
                None => Ok(this.types.fresh()),
            }
        };
        Ok(match name {
            "number" => self.types.num(Flavor::Unknown),
            "string" => Ty::Str,
            "boolean" => Ty::Bool,
            "void" | "undefined" => Ty::Void,
            "null" => Ty::Null,
            // `any` and `unknown` are inferred from their uses, like a missing annotation.
            "any" | "unknown" | "object" => self.types.fresh(),
            "const" => Ty::Any,
            "never" => Ty::Never,
            "bigint" => return self.err("bigint is not supported", span),
            "symbol" => Ty::named("Symbol"),
            "Array" | "ReadonlyArray" => Ty::Array(Box::new(arg(self, 0)?)),
            "Map" | "ReadonlyMap" => {
                Ty::Map(Box::new(arg(self, 0)?), Box::new(arg(self, 1)?), false)
            }
            "Set" | "ReadonlySet" => Ty::Set(Box::new(arg(self, 0)?)),
            "Record" => {
                let k = arg(self, 0)?;
                let v = arg(self, 1)?;
                if let Ty::Named(n, ..) = self.types.resolve(&k)
                    && self.enum_by_name(&n).is_none()
                {
                    return self.err("Record keys must be strings, numbers or an enum", span);
                }
                Ty::Map(Box::new(k), Box::new(v), true)
            }
            "Promise" => arg(self, 0)?,
            "Generator" | "Iterable" | "IterableIterator" | "Iterator" => {
                Ty::Gen(Box::new(arg(self, 0)?))
            }
            "Partial" | "Required" | "Readonly" | "NonNullable" | "Awaited" => arg(self, 0)?,
            "Error" => Ty::named("Error"),
            "Function" => {
                return self.err("`Function` is too vague: write `(x: T) => U`", span);
            }
            "Date" => Ty::named("Date"),
            "RegExp" => Ty::named("RegExp"),
            "RegExpMatchArray" | "RegExpExecArray" => Ty::named("RegExpMatch"),
            "this" => match self.this_class_stack.last() {
                Some(c) => self.self_type(&c.clone()),
                None => return self.err("`this` type outside a class", span),
            },
            _ => {
                if self.is_type_param(name) {
                    return Ok(Ty::named(name.to_string()));
                }
                if let Some(t) = self.aliases.get(name) {
                    return Ok(t.clone());
                }
                if let Some((params, body)) = self.generic_aliases.get(name).cloned() {
                    if args.len() != params.len() {
                        return self.err(
                            format!(
                                "{name} takes {} type argument{}",
                                params.len(),
                                if params.len() == 1 { "" } else { "s" }
                            ),
                            span,
                        );
                    }
                    let mut subst = HashMap::new();
                    for (i, p) in params.iter().enumerate() {
                        subst.insert(p.clone(), arg(self, i)?);
                    }
                    return Ok(super::finish::substitute(&body, &subst, &self.types));
                }
                if self.enum_by_name(name).is_some() {
                    if !args.is_empty() {
                        return self.err(
                            format!("{name} is an enum: it takes no type argument"),
                            span,
                        );
                    }
                    return Ok(Ty::named(name.to_string()));
                }
                if let Some(s) = self.struct_by_name(name) {
                    let n = s.type_params.len();
                    if args.is_empty() {
                        // `Stack` alone: the arguments are inferred.
                        return Ok(self.fresh_instance(name).0);
                    }
                    if args.len() != n {
                        return self.err(
                            if n == 0 {
                                format!("{name} is not generic: it takes no type argument")
                            } else {
                                format!(
                                    "{name} takes {n} type argument{}",
                                    if n == 1 { "" } else { "s" }
                                )
                            },
                            span,
                        );
                    }
                    let mut tys = Vec::new();
                    for i in 0..n {
                        tys.push(arg(self, i)?);
                    }
                    let s = self.struct_by_name(name).expect("struct");
                    let subst: HashMap<String, Ty> = s
                        .type_params
                        .iter()
                        .cloned()
                        .zip(tys.iter().cloned())
                        .collect();
                    let bounds = s.type_bounds.clone();
                    self.record_bound_uses(&bounds, &subst, span);
                    return Ok(Ty::Named(name.to_string(), tys));
                }
                return self.err(format!("unknown type `{name}`"), span);
            }
        })
    }

    fn union_ann(&mut self, members: &[TypeAnn], span: Span, hint: &str) -> IResult<Ty> {
        let mut nullable = false;
        let mut rest: Vec<&TypeAnn> = Vec::new();
        for m in members {
            match m {
                TypeAnn::Lit(Lit::Null | Lit::Undefined, _) => nullable = true,
                TypeAnn::Named { name, .. }
                    if name == "undefined" || name == "null" || name == "void" =>
                {
                    nullable = true
                }
                other => rest.push(other),
            }
        }
        let inner = if rest.is_empty() {
            Ty::Null
        } else if rest.len() == 1 {
            self.ann(rest[0], hint)?
        } else if rest
            .iter()
            .all(|t| matches!(t, TypeAnn::Lit(Lit::Str(_), _)))
        {
            let literals: Vec<String> = rest
                .iter()
                .map(|t| match t {
                    TypeAnn::Lit(Lit::Str(s), _) => s.clone(),
                    _ => unreachable!(),
                })
                .collect();
            let name = self.synth_name(hint);
            self.define_literal_enum(&name, &literals, false, span);
            Ty::named(name)
        } else if rest
            .iter()
            .all(|t| matches!(t, TypeAnn::Lit(Lit::Num(_), _)))
        {
            self.types.num(Flavor::Unknown)
        } else if rest
            .iter()
            .all(|t| matches!(t, TypeAnn::Lit(Lit::Bool(_), _)))
        {
            Ty::Bool
        } else {
            // Different types: an enum with one variant per member.
            let mut members = Vec::new();
            let discriminant = inline_discriminant(&rest);
            for t in &rest {
                let member_hint = match (t, &discriminant) {
                    (TypeAnn::Object(props, _), Some(d)) => props
                        .iter()
                        .find_map(|m| match m {
                            TypeMember::Prop {
                                name,
                                ty: TypeAnn::Lit(Lit::Str(s), _),
                                ..
                            } if name == d => Some(s.clone()),
                            _ => None,
                        })
                        .unwrap_or_else(|| hint.to_string()),
                    _ => hint.to_string(),
                };
                members.push(self.ann(t, &member_hint)?);
            }
            self.union_of(members, span)?
        };
        Ok(if nullable {
            match inner {
                Ty::Null => Ty::Null,
                Ty::Option(t) => Ty::Option(t),
                t => Ty::Option(Box::new(t)),
            }
        } else {
            inner
        })
    }

    /// A fresh type name made from a hint (`user profile` → `UserProfile`).
    pub(super) fn synth_name(&mut self, hint: &str) -> String {
        let base = pascal(hint);
        self.synth_name_exact(&base)
    }

    /// A fresh type name from a Rust name.
    pub(super) fn synth_name_exact(&mut self, hint: &str) -> String {
        let base = hint.to_string();
        let base = if base.is_empty() {
            "Object".to_string()
        } else {
            base
        };
        let mut name = base.clone();
        let mut n = 2;
        while self.type_exists(&name) || self.synth_names.contains(&name) {
            name = format!("{base}{n}");
            n += 1;
        }
        self.synth_names.insert(name.clone());
        name
    }

    pub(super) fn define_literal_enum(
        &mut self,
        name: &str,
        literals: &[String],
        exported: bool,
        span: Span,
    ) {
        let mut variants = Vec::new();
        let mut used = HashSet::new();
        for lit in literals {
            let mut v = pascal(lit);
            if v.is_empty() {
                v = "Empty".into();
            }
            let base = v.clone();
            let mut n = 2;
            while used.contains(&v) {
                v = format!("{base}{n}");
                n += 1;
            }
            used.insert(v.clone());
            variants.push(Variant {
                name: v,
                source: lit.clone(),
                value: Some(EnumValue::Str(lit.clone())),
            });
        }
        self.enums.push(EnumInfo {
            name: name.to_string(),
            variants,
            kind: EnumKind::Literals,
            exported,
            span,
        });
    }

    pub(super) fn define_struct_from_members(
        &mut self,
        name: &str,
        members: &[TypeMember],
        kind: StructKind,
        exported: bool,
        span: Span,
    ) -> IResult<()> {
        // Registered first: a field may refer to the type itself.
        self.structs.push(StructInfo {
            name: name.to_string(),
            kind,
            fields: Vec::new(),
            methods: Vec::new(),
            ctor: None,
            exported,
            span,
            statics: Vec::new(),
            parent: None,
            is_abstract: false,
            implements: Vec::new(),
            type_params: Vec::new(),
            type_bounds: Vec::new(),
            parent_args: Vec::new(),
            implements_args: Vec::new(),
        });
        let fields = self.fields_from_members(name, members)?;
        self.struct_mut(name).expect("struct").fields = fields;
        Ok(())
    }

    pub(super) fn fields_from_members(
        &mut self,
        owner: &str,
        members: &[TypeMember],
    ) -> IResult<Vec<FieldInfo>> {
        let (fields, methods) = self.members_of(owner, members)?;
        if let Some(m) = methods.first() {
            return self.err(
                format!(
                    "`{}` has a method (`{}`): only an interface can declare methods",
                    owner, m.name
                ),
                self.funcs[&m.fn_id].span,
            );
        }
        Ok(fields)
    }

    /// The fields and the method signatures of an interface or an object type.
    pub(super) fn members_of(
        &mut self,
        owner: &str,
        members: &[TypeMember],
    ) -> IResult<(Vec<FieldInfo>, Vec<MethodInfo>)> {
        let mut fields = Vec::new();
        let mut methods = Vec::new();
        for m in members {
            match m {
                TypeMember::Prop {
                    name,
                    ty,
                    optional,
                    readonly,
                    span,
                } => {
                    let fty = self.ann(ty, &format!("{owner} {name}"))?;
                    if super::says_undefined(ty) {
                        self.undef_fields.insert((owner.to_string(), name.clone()));
                    }
                    let (fty, optional) = match (self.types.resolve(&fty), *optional) {
                        (Ty::Option(inner), true) => (*inner, true),
                        (_, o) => (fty, o),
                    };
                    let literal = match ty {
                        TypeAnn::Lit(Lit::Str(s), _) => Some(s.clone()),
                        _ => None,
                    };
                    fields.push(FieldInfo {
                        name: name.clone(),
                        ty: fty,
                        optional,
                        access: None,
                        readonly: *readonly,
                        has_init: false,
                        literal,
                        span: *span,
                    });
                }
                TypeMember::Method {
                    name,
                    type_params,
                    params,
                    ret,
                    optional,
                    span,
                } => {
                    if !type_params.is_empty() {
                        return self.err("generic interface methods are not supported", *span);
                    }
                    if *optional {
                        return self.err(
                            format!("optional methods are not supported (`{name}?`)"),
                            *span,
                        );
                    }
                    let method = self.interface_method(owner, name, params, ret, *span)?;
                    methods.push(method);
                }
                TypeMember::Index { span, .. } => {
                    return self.err(
                        "index signatures are not supported: use `Record<string, T>` or `Map`",
                        *span,
                    );
                }
                TypeMember::Call { span, .. } => {
                    return self.err("call signatures are not supported", *span);
                }
            }
        }
        Ok((fields, methods))
    }

    /// The signature of an interface method: an abstract method of the interface, which
    /// the classes implementing it provide.
    fn interface_method(
        &mut self,
        owner: &str,
        name: &str,
        params: &[Param],
        ret: &Option<TypeAnn>,
        span: Span,
    ) -> IResult<MethodInfo> {
        let mut ptys = Vec::new();
        let mut optional = Vec::new();
        let mut defaults = Vec::new();
        for p in params {
            if p.rest {
                return self.err(
                    "rest parameters in interface methods are not supported",
                    p.span,
                );
            }
            let hint = match &p.pat {
                Pattern::Ident { name, .. } => name.clone(),
                _ => String::new(),
            };
            let mut ty = match &p.ty {
                Some(t) => self.ann(t, &hint)?,
                None => self.types.fresh(),
            };
            if p.optional {
                ty = match self.types.resolve(&ty) {
                    Ty::Option(t) => Ty::Option(t),
                    t => Ty::Option(Box::new(t)),
                };
            }
            ptys.push(ty);
            optional.push(p.optional);
            defaults.push(false);
        }
        let ret_ty = match ret {
            Some(t) => self.ann(t, name)?,
            None => Ty::Void,
        };
        let id = self.synth_id();
        self.funcs.insert(
            id,
            FnInfo {
                id,
                name: Some(name.to_string()),
                params: Vec::new(),
                optional,
                defaults,
                sig: FuncTy {
                    params: ptys,
                    ret: ret_ty,
                },
                type_params: Vec::new(),
                type_bounds: Vec::new(),
                throws: false,
                throw_direct: false,
                is_async: matches!(ret, Some(TypeAnn::Named { name, .. }) if name == "Promise"),
                is_arrow: false,
                method_of: Some(owner.to_string()),
                method_kind: Some(MethodKind::Method),
                is_static: false,
                is_abstract: true,
                rest: false,
                mutates_self: false,
                captures: Vec::new(),
                escapes: false,
                is_closure: false,
                parent: None,
                calls: Vec::new(),
                uses_this: false,
                returns_this: false,
                exported: false,
                span,
            },
        );
        Ok(MethodInfo {
            name: name.to_string(),
            fn_id: id,
            kind: MethodKind::Method,
            is_static: false,
            is_abstract: true,
            access: None,
        })
    }
}
