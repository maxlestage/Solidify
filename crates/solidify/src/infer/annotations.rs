//! From type annotations to types.

use std::collections::HashSet;
use std::rc::Rc;

use super::{
    EnumInfo, EnumKind, EnumValue, FieldInfo, IResult, Infer, StructInfo, StructKind, Variant,
};
use crate::ast::*;
use crate::error::Span;
use crate::infer::pascal;
use crate::types::{Flavor, FuncTy, Ty};

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
                Ok(Ty::Named(name))
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
                Lit::Str(s) => {
                    let span = *span;
                    self.union_ann(&[TypeAnn::Lit(Lit::Str(s.clone()), span)], span, hint)
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
            "symbol" => return self.err("symbols are not supported", span),
            "Array" | "ReadonlyArray" => Ty::Array(Box::new(arg(self, 0)?)),
            "Map" | "ReadonlyMap" => {
                Ty::Map(Box::new(arg(self, 0)?), Box::new(arg(self, 1)?), false)
            }
            "Set" | "ReadonlySet" => Ty::Set(Box::new(arg(self, 0)?)),
            "Record" => {
                let k = arg(self, 0)?;
                let v = arg(self, 1)?;
                if let Ty::Named(n) = self.types.resolve(&k)
                    && self.enum_by_name(&n).is_none()
                {
                    return self.err("Record keys must be strings, numbers or an enum", span);
                }
                Ty::Map(Box::new(k), Box::new(v), true)
            }
            "Promise" => arg(self, 0)?,
            "Partial" | "Required" | "Readonly" | "NonNullable" | "Awaited" => arg(self, 0)?,
            "Error" => Ty::Named("Error".into()),
            "Function" => {
                return self.err("`Function` is too vague: write `(x: T) => U`", span);
            }
            "Date" => return self.err("`Date` is not supported", span),
            "RegExp" => return self.err("regular expressions are not supported", span),
            "this" => match self.this_class_stack.last() {
                Some(c) => Ty::Named(c.clone()),
                None => return self.err("`this` type outside a class", span),
            },
            _ => {
                if self.is_type_param(name) {
                    return Ok(Ty::Named(name.to_string()));
                }
                if let Some(t) = self.aliases.get(name) {
                    return Ok(t.clone());
                }
                if self.struct_by_name(name).is_some() || self.enum_by_name(name).is_some() {
                    if !args.is_empty() {
                        return self.err("generic classes and interfaces are not supported", span);
                    }
                    return Ok(Ty::Named(name.to_string()));
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
            Ty::Named(name)
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
            let first = self.ann(rest[0], hint)?;
            for t in &rest[1..] {
                let other = self.ann(t, hint)?;
                if self.types.unify(&first, &other).is_err() {
                    return self.err(
                        "unions of different types are not supported: only `T | null`, unions of string literals and the same type repeated",
                        span,
                    );
                }
            }
            first
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
        let mut fields = Vec::new();
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
                    let (fty, optional) = match (self.types.resolve(&fty), *optional) {
                        (Ty::Option(inner), true) => (*inner, true),
                        (_, o) => (fty, o),
                    };
                    fields.push(FieldInfo {
                        name: name.clone(),
                        ty: fty,
                        optional,
                        access: None,
                        readonly: *readonly,
                        has_init: false,
                        span: *span,
                    });
                }
                TypeMember::Method { name, span, .. } => {
                    return self.err(
                        format!(
                            "interface methods are not supported (`{name}`): use a class, or a field of function type"
                        ),
                        *span,
                    );
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
        Ok(fields)
    }
}
