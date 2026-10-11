//! Unions of different types (`string | number`, `Circle | Square`): one Rust enum per
//! union, a variant per member, and the tests that tell the members apart.

use super::finish::same_shape_ty;
use super::{
    ActiveNarrow, Complement, IResult, Infer, NarrowTest, Role, Root, UnionInfo, UnionMember,
};
use crate::ast::*;
use crate::error::Span;
use crate::infer::pascal;
use crate::types::Ty;

impl<'m> Infer<'m> {
    /// The enum of a union of different types; the same set of members gives the same
    /// enum. A single distinct member is just that type.
    pub(super) fn union_of(&mut self, members: Vec<Ty>, span: Span) -> IResult<Ty> {
        let mut distinct: Vec<Ty> = Vec::new();
        for m in members {
            let zm = self.types.zonk(&m);
            if !distinct
                .iter()
                .any(|d| same_shape_ty(&self.types.zonk(d), &zm))
            {
                distinct.push(m);
            }
        }
        if distinct.len() == 1 {
            return Ok(distinct.pop().expect("member"));
        }
        // An existing union with the same members is the same union: its member types
        // are shared, so that a number is an integer or a float in both.
        let mut reuse: Option<(String, Vec<(Ty, Ty)>)> = None;
        for u in &self.unions {
            if u.members.len() != distinct.len() {
                continue;
            }
            let mut pairs = Vec::new();
            let all = distinct.iter().all(|t| {
                let zt = self.types.zonk(t);
                match u
                    .members
                    .iter()
                    .find(|m| same_shape_ty(&self.types.zonk(&m.ty), &zt))
                {
                    Some(m) => {
                        pairs.push((m.ty.clone(), t.clone()));
                        true
                    }
                    None => false,
                }
            });
            if all {
                reuse = Some((u.name.clone(), pairs));
                break;
            }
        }
        if let Some((name, pairs)) = reuse {
            for (a, b) in pairs {
                let _ = self.types.unify(&a, &b);
            }
            return Ok(Ty::Named(name));
        }
        let mut names: Vec<String> = Vec::new();
        for t in &distinct {
            let base = self.variant_name(t);
            let mut name = base.clone();
            let mut n = 2;
            while names.contains(&name) {
                name = format!("{base}{n}");
                n += 1;
            }
            names.push(name);
        }
        let discriminant = self.find_discriminant(&distinct);
        let mut union_members = Vec::new();
        for (t, name) in distinct.into_iter().zip(names.iter()) {
            let tag = discriminant
                .as_ref()
                .and_then(|d| match self.types.resolve(&t) {
                    Ty::Named(n) => self
                        .struct_by_name(&n)
                        .and_then(|s| s.field(d))
                        .and_then(|f| f.literal.clone()),
                    _ => None,
                });
            union_members.push(UnionMember {
                name: name.clone(),
                ty: t,
                tag,
            });
        }
        let name = self.synth_name_exact(&names.join("Or"));
        self.unions.push(UnionInfo {
            name: name.clone(),
            members: union_members,
            discriminant,
            exported: false,
            span,
        });
        Ok(Ty::Named(name))
    }

    /// The variant name for a member type: `String`, `Number`, `Circle`, `StringArray`…
    fn variant_name(&self, t: &Ty) -> String {
        match self.types.resolve(t) {
            Ty::Str => "String".into(),
            Ty::Num(_) => "Number".into(),
            Ty::Bool => "Boolean".into(),
            Ty::Array(item) => format!("{}Array", self.variant_name(&item)),
            Ty::Tuple(_) => "Tuple".into(),
            Ty::Map(..) => "Map".into(),
            Ty::Set(_) => "Set".into(),
            Ty::Func(_) => "Function".into(),
            Ty::Named(n) => pascal(&n),
            Ty::Option(t) => format!("Optional{}", self.variant_name(&t)),
            _ => "Value".into(),
        }
    }

    /// A field every struct member has, with a different string literal type in each.
    fn find_discriminant(&self, members: &[Ty]) -> Option<String> {
        let mut structs = Vec::new();
        for m in members {
            let Ty::Named(n) = self.types.resolve(m) else {
                return None;
            };
            structs.push(self.struct_by_name(&n)?);
        }
        let first = structs.first()?;
        for f in &first.fields {
            if f.literal.is_none() {
                continue;
            }
            let mut tags = std::collections::HashSet::new();
            let all = structs.iter().all(|s| {
                s.field(&f.name)
                    .and_then(|g| g.literal.clone())
                    .is_some_and(|lit| tags.insert(lit))
            });
            if all && tags.len() == structs.len() {
                return Some(f.name.clone());
            }
        }
        None
    }

    /// The member of a union that a value of type `ty` fits, by index.
    pub(super) fn union_member_for(&self, union: &UnionInfo, ty: &Ty) -> Option<usize> {
        let zt = self.types.zonk(ty);
        union
            .members
            .iter()
            .position(|m| same_shape_ty(&self.types.zonk(&m.ty), &zt))
    }

    /// Whether a value of type `found` can go where `expected` is wanted because
    /// `expected` is a union (or a nullable union) with `found` among its members.
    pub(super) fn union_accepts(&mut self, expected: &Ty, found: &Ty) -> bool {
        let target = match self.types.resolve(expected) {
            Ty::Option(inner) => self.types.resolve(&inner),
            other => other,
        };
        let Ty::Named(n) = target else {
            return false;
        };
        let Some(u) = self.union_by_name(&n) else {
            return false;
        };
        let found = match self.types.resolve(found) {
            Ty::Option(inner) => self.types.resolve(&inner),
            other => other,
        };
        if let Ty::Named(f) = &found
            && *f == n
        {
            return true;
        }
        if let Some(i) = self.union_member_for(u, &found) {
            // Settle the numbers' flavor and such.
            let member = u.members[i].ty.clone();
            let _ = self.types.unify(&member, &found);
            return true;
        }
        false
    }

    /// The member an object literal builds, from its discriminant or its keys.
    pub(super) fn union_literal_member(&self, union: &UnionInfo, props: &[Prop]) -> Option<String> {
        if let Some(d) = &union.discriminant {
            let tag = props.iter().find_map(|p| match (&p.key, &p.value) {
                (PropKey::Ident(k) | PropKey::Str(k), PropValue::Expr(v)) if k == d => {
                    match &v.kind {
                        ExprKind::Str(s) => Some(s.clone()),
                        _ => None,
                    }
                }
                _ => None,
            })?;
            return union.member_with_tag(&tag).and_then(|m| match &m.ty {
                Ty::Named(n) => Some(n.clone()),
                _ => None,
            });
        }
        let keys: Vec<String> = props.iter().filter_map(|p| p.key.name()).collect();
        for m in &union.members {
            let Ty::Named(n) = self.types.resolve(&m.ty) else {
                continue;
            };
            let Some(s) = self.struct_by_name(&n) else {
                continue;
            };
            let fits = keys.iter().all(|k| s.field(k).is_some())
                && s.fields
                    .iter()
                    .all(|f| f.optional || keys.contains(&f.name));
            if fits {
                return Some(n);
            }
        }
        None
    }

    /// The member `typeof x === "<kind>"` selects.
    fn typeof_member(&self, union: &UnionInfo, kind: &str) -> Option<usize> {
        let class = |t: &Ty| -> &'static str {
            match self.types.resolve(t) {
                Ty::Str => "string",
                Ty::Num(_) => "number",
                Ty::Bool => "boolean",
                Ty::Func(_) => "function",
                _ => "object",
            }
        };
        let matching: Vec<usize> = union
            .members
            .iter()
            .enumerate()
            .filter(|(_, m)| class(&m.ty) == kind)
            .map(|(i, _)| i)
            .collect();
        match matching.as_slice() {
            [i] => Some(*i),
            _ => None,
        }
    }

    /// If `e` tests which member of a union a value is, the narrowing it makes: the tested
    /// path, the enum, the variant, and whether the member is there when the test is true.
    pub(super) fn union_test(&mut self, e: &'m Expr) -> IResult<Option<UnionTest<'m>>> {
        let (value, pick, positive): (&'m Expr, Pick, bool) = match &e.kind {
            ExprKind::Binary { op, left, right } if op.is_equality() => {
                let positive = matches!(op, BinOp::Eq | BinOp::StrictEq);
                // `typeof x === "string"`
                let typeof_pair = match (&left.kind, &right.kind) {
                    (ExprKind::TypeOf(x), ExprKind::Str(s))
                    | (ExprKind::Str(s), ExprKind::TypeOf(x)) => Some((&**x, s.clone())),
                    _ => None,
                };
                if let Some((x, s)) = typeof_pair {
                    (x, Pick::TypeOf(s), positive)
                } else {
                    // `x.kind === "circle"`
                    let tag_pair = match (&left.kind, &right.kind) {
                        (
                            ExprKind::Member {
                                obj,
                                prop,
                                optional: false,
                            },
                            ExprKind::Str(s),
                        )
                        | (
                            ExprKind::Str(s),
                            ExprKind::Member {
                                obj,
                                prop,
                                optional: false,
                            },
                        ) => Some((&**obj, prop.clone(), s.clone())),
                        _ => None,
                    };
                    let Some((obj, prop, s)) = tag_pair else {
                        return Ok(None);
                    };
                    (obj, Pick::Tag(prop, s), positive)
                }
            }
            ExprKind::Binary {
                op: BinOp::InstanceOf,
                left,
                right,
            } => {
                let ExprKind::Ident(class) = &right.kind else {
                    return Ok(None);
                };
                (&**left, Pick::Class(class.clone()), true)
            }
            ExprKind::Binary {
                op: BinOp::In,
                left,
                right,
            } => {
                let ExprKind::Str(field) = &left.kind else {
                    return Ok(None);
                };
                (&**right, Pick::Field(field.clone()), true)
            }
            ExprKind::Call { callee, args, .. } if args.len() == 1 => {
                let ExprKind::Member { obj, prop, .. } = &callee.kind else {
                    return Ok(None);
                };
                if !matches!(&obj.kind, ExprKind::Ident(n) if n == "Array") || prop != "isArray" {
                    return Ok(None);
                }
                (&args[0].expr, Pick::Array, true)
            }
            _ => return Ok(None),
        };
        if self.path_of(value).is_none() {
            return Ok(None);
        }
        // The tested value must be a union (what a narrowing bound it to, if any).
        let vt = self.expr(value, None)?;
        let Ty::Named(n) = self.types.resolve(&vt) else {
            return Ok(None);
        };
        let Some(union) = self.union_by_name(&n).cloned() else {
            return Ok(None);
        };
        let index = match &pick {
            Pick::TypeOf(s) => self.typeof_member(&union, s),
            Pick::Tag(prop, s) => {
                if union.discriminant.as_deref() != Some(prop.as_str()) {
                    return Ok(None);
                }
                union
                    .members
                    .iter()
                    .position(|m| m.tag.as_deref() == Some(s.as_str()))
            }
            Pick::Class(c) => union
                .members
                .iter()
                .position(|m| matches!(&m.ty, Ty::Named(x) if x == c)),
            Pick::Field(f) => {
                let with: Vec<usize> = union
                    .members
                    .iter()
                    .enumerate()
                    .filter(|(_, m)| match &m.ty {
                        Ty::Named(x) => {
                            self.struct_by_name(x).is_some_and(|s| s.field(f).is_some())
                        }
                        _ => false,
                    })
                    .map(|(i, _)| i)
                    .collect();
                match with.as_slice() {
                    [i] => Some(*i),
                    _ => None,
                }
            }
            Pick::Array => union
                .members
                .iter()
                .position(|m| matches!(self.types.resolve(&m.ty), Ty::Array(_))),
        };
        let Some(index) = index else {
            return self.err(
                format!("this test does not select one member of {}", union.name),
                e.span,
            );
        };
        let member = &union.members[index];
        Ok(Some(UnionTest {
            value,
            union: union.name.clone(),
            variant: member.name.clone(),
            inner: member.ty.clone(),
            positive,
        }))
    }

    /// Registers the narrowing of a union test: the member on the side the test selects,
    /// and, for a union of two members, the other member on the other side.
    pub(super) fn narrow_union(
        &mut self,
        t: &UnionTest<'m>,
        test_id: NodeId,
    ) -> IResult<(ActiveNarrow, Option<ActiveNarrow>)> {
        let path = self.path_of(t.value).expect("path");
        let name = match (&path.root, path.fields.last()) {
            (Root::Var(id), None) => self.vars[id].name.clone(),
            (_, Some(last)) => last.clone(),
            (Root::This, None) => "this".into(),
        };
        let complement = match self.union_by_name(&t.union).cloned() {
            Some(u) if u.members.len() == 2 => u
                .members
                .iter()
                .find(|m| m.name != t.variant)
                .map(|m| Complement::Variant(m.name.clone(), m.ty.clone())),
            Some(u) => {
                let rest: Vec<Ty> = u
                    .members
                    .iter()
                    .filter(|m| m.name != t.variant)
                    .map(|m| m.ty.clone())
                    .collect();
                Some(Complement::Union(self.union_of(rest, t.value.span)?))
            }
            None => None,
        };
        self.set_ty(test_id, Ty::Bool);
        self.narrow_tests.insert(
            test_id,
            NarrowTest {
                root: path.root.clone(),
                path: path.fields.clone(),
                name: name.clone(),
                positive: t.positive,
                union: Some((t.union.clone(), t.variant.clone())),
                complement: complement.clone(),
                instance_of: None,
                mutated: false,
                inner: t.inner.clone(),
                let_else: false,
                unwrap: false,
            },
        );
        let other = complement.map(|c| ActiveNarrow {
            root: path.root.clone(),
            path: path.fields.clone(),
            name: name.clone(),
            inner: c.ty().clone(),
            test_id,
        });
        Ok((
            ActiveNarrow {
                root: path.root,
                path: path.fields,
                name,
                inner: t.inner.clone(),
                test_id,
            },
            other,
        ))
    }

    /// A field of a union: it must exist, with one type, in every member.
    pub(super) fn union_field(&self, union: &UnionInfo, prop: &str) -> Option<Ty> {
        let mut found: Option<Ty> = None;
        for m in &union.members {
            let Ty::Named(n) = self.types.resolve(&m.ty) else {
                return None;
            };
            let f = self.struct_by_name(&n)?.field(prop)?;
            let fty = if f.optional {
                Ty::Option(Box::new(f.ty.clone()))
            } else {
                f.ty.clone()
            };
            match &found {
                None => found = Some(fty),
                Some(t) => {
                    if !same_shape_ty(&self.types.zonk(t), &self.types.zonk(&fty)) {
                        return None;
                    }
                }
            }
        }
        found
    }

    /// Unused role helper kept for symmetry with the other modules.
    #[allow(dead_code)]
    fn role(&self) -> Role {
        Role::Local
    }
}

/// How a test picks a member.
enum Pick {
    TypeOf(String),
    Tag(String, String),
    Class(String),
    Field(String),
    Array,
}

/// A test of a union member, classified.
pub(super) struct UnionTest<'m> {
    pub(super) value: &'m Expr,
    pub(super) union: String,
    pub(super) variant: String,
    pub(super) inner: Ty,
    pub(super) positive: bool,
}
