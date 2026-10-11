//! The end of inference: fixpoints across functions (throws, mutation of parameters),
//! generalization of unconstrained parameters, last uses, resolved types.

use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use super::{IResult, Infer, Program, Role};
use crate::ast::NodeId;
use crate::error::Error;
use crate::types::{Flavor, FuncTy, Ty, Types};

impl<'m> Infer<'m> {
    pub(super) fn finish(mut self) -> IResult<Program> {
        self.propagate_throws();
        self.propagate_mutation();
        self.float_json_numbers();
        // A narrowed value whose alias is changed is bound mutably.
        for (binding, test) in self.alias_of_narrow.clone() {
            if self.vars.get(&binding).is_some_and(|v| v.mutated)
                && let Some(t) = self.narrow_tests.get_mut(&test)
            {
                t.mutated = true;
            }
        }
        // A variable captured by a closure that is kept (returned, stored, given to a
        // timer) and changed somewhere is shared between the closure and its scope.
        let escaping: Vec<NodeId> = self
            .funcs
            .values()
            .filter(|f| f.escapes)
            .flat_map(|f| f.captures.iter().copied())
            .collect();
        for id in escaping {
            if let Some(v) = self.vars.get_mut(&id)
                && (v.reassigned || v.mutated || v.mutated_in_closure)
                && !matches!(v.role, Role::Func | Role::Class | Role::Enum)
            {
                v.shared = true;
            }
        }
        let boxed = self.boxed_structs();
        let drop_decl = self.assign_narrowed_decls()?;
        self.generalize()?;
        self.check_locals()?;
        let last_use = self.last_uses();
        // Resolve every type.
        let types = self.types;
        let zonk = |t: &Ty| types.zonk(t);
        let expr_ty = self.expr_ty.iter().map(|(k, v)| (*k, zonk(v))).collect();
        let mut vars = self.vars;
        for v in vars.values_mut() {
            v.ty = zonk(&v.ty);
        }
        let mut funcs = self.funcs;
        for f in funcs.values_mut() {
            f.sig = FuncTy {
                params: f.sig.params.iter().map(zonk).collect(),
                ret: zonk(&f.sig.ret),
            };
        }
        let mut structs = self.structs;
        for s in &mut structs {
            for f in &mut s.fields {
                f.ty = zonk(&f.ty);
            }
            for st in &mut s.statics {
                st.1 = zonk(&st.1);
            }
        }
        let mut narrow_tests = self.narrow_tests;
        for n in narrow_tests.values_mut() {
            n.inner = zonk(&n.inner);
        }
        let mut unions = self.unions;
        for u in &mut unions {
            for m in &mut u.members {
                m.ty = zonk(&m.ty);
            }
        }
        Ok(Program {
            types,
            node_count: self.node_count,
            expr_ty,
            vars,
            refs: self.refs,
            funcs,
            fn_bindings: self.fn_bindings,
            structs,
            enums: self.enums,
            unions,
            calls: self.calls,
            literal_struct: self.literal_struct,
            enum_literal: self.enum_literal,
            narrow_tests,
            narrowed_uses: self.narrowed_uses,
            drop_decl,
            last_use,
            calls_in_try: self.calls_in_try,
            try_returns: self.try_returns,
            main_id: self.main_id,
            has_main: self.has_main,
            boxed,
            polymorphic: self.polymorphic,
            overrides: self.overrides,
            uses_timers: self.uses_timers,
        })
    }

    /// Structs holding themselves directly (through `Option` or a tuple): a `Box` is needed.
    fn boxed_structs(&self) -> HashSet<String> {
        fn reaches(
            types: &Types,
            structs: &[super::StructInfo],
            ty: &Ty,
            target: &str,
            seen: &mut HashSet<String>,
        ) -> bool {
            match types.resolve(ty) {
                Ty::Named(n) => {
                    if n == target {
                        return true;
                    }
                    if !seen.insert(n.clone()) {
                        return false;
                    }
                    structs.iter().find(|s| s.name == n).is_some_and(|s| {
                        s.fields
                            .iter()
                            .any(|f| reaches(types, structs, &f.ty, target, seen))
                    })
                }
                Ty::Option(t) => reaches(types, structs, &t, target, seen),
                Ty::Tuple(ts) => ts.iter().any(|t| reaches(types, structs, t, target, seen)),
                _ => false,
            }
        }
        let mut out = HashSet::new();
        for s in &self.structs {
            let mut seen = HashSet::new();
            if s.fields
                .iter()
                .any(|f| reaches(&self.types, &self.structs, &f.ty, &s.name, &mut seen))
            {
                out.insert(s.name.clone());
            }
        }
        out
    }

    /// JSON numbers are doubles: a number read by `JSON.parse` that nothing in the program
    /// constrains to an integer is a float.
    fn float_json_numbers(&mut self) {
        let targets = std::mem::take(&mut self.json_targets);
        let mut seen = HashSet::new();
        for t in &targets {
            self.float_unknown_numbers(t, &mut seen);
        }
    }

    fn float_unknown_numbers(&mut self, t: &Ty, seen: &mut HashSet<String>) {
        match self.types.resolve(t) {
            Ty::Num(f) => {
                if self.types.flavor(f) == Flavor::Unknown || self.types.soft_int(f) {
                    self.types.set_flavor(f, Flavor::Float);
                }
            }
            Ty::Option(t) | Ty::Array(t) | Ty::Set(t) | Ty::Gen(t) => {
                self.float_unknown_numbers(&t, seen);
            }
            Ty::Map(k, v, _) => {
                self.float_unknown_numbers(&k, seen);
                self.float_unknown_numbers(&v, seen);
            }
            Ty::Tuple(ts) => {
                for t in &ts {
                    self.float_unknown_numbers(t, seen);
                }
            }
            Ty::Named(n) => {
                if !seen.insert(n.clone()) {
                    return;
                }
                let fields: Vec<Ty> = self
                    .structs
                    .iter()
                    .find(|s| s.name == n)
                    .map(|s| s.fields.iter().map(|f| f.ty.clone()).collect())
                    .unwrap_or_default();
                for t in &fields {
                    self.float_unknown_numbers(t, seen);
                }
                let members: Vec<Ty> = self
                    .unions
                    .iter()
                    .find(|u| u.name == n)
                    .map(|u| u.members.iter().map(|m| m.ty.clone()).collect())
                    .unwrap_or_default();
                for t in &members {
                    self.float_unknown_numbers(t, seen);
                }
            }
            _ => {}
        }
    }

    /// `(m = re.exec(s)) !== null` binds the value in the test: `m` needs no declaration
    /// when every read of it is inside such a test.
    fn assign_narrowed_decls(&self) -> IResult<HashSet<NodeId>> {
        let mut out = HashSet::new();
        for (binding, targets) in &self.assign_narrowed {
            let outside = self
                .uses
                .get(binding)
                .into_iter()
                .flatten()
                .find(|(use_id, ..)| {
                    !targets.contains(use_id) && !self.narrowed_uses.contains_key(use_id)
                });
            if outside.is_some() {
                let v = &self.vars[binding];
                return self.err(
                    format!(
                        "`{}` is assigned inside a condition and read elsewhere: assign it on its own line",
                        v.name
                    ),
                    v.span,
                );
            }
            out.insert(*binding);
        }
        Ok(out)
    }

    /// A function throws if it throws itself or calls, outside a `try`, one that does.
    fn propagate_throws(&mut self) {
        for f in self.funcs.values_mut() {
            f.throws = f.throw_direct;
        }
        loop {
            let mut changed = false;
            let ids: Vec<NodeId> = self.funcs.keys().copied().collect();
            for id in ids {
                let f = &self.funcs[&id];
                if f.throws {
                    continue;
                }
                let throws = f.calls.iter().any(|(callee, call)| {
                    !self.calls_in_try.contains(call)
                        && self.funcs.get(callee).is_some_and(|c| c.throws)
                });
                if throws {
                    self.funcs.get_mut(&id).expect("fn").throws = true;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
    }

    /// Parameters mutated or stored by a callee mark the variables passed to them; methods
    /// that mutate `self` mark their receivers and the methods calling them on `this`.
    fn propagate_mutation(&mut self) {
        loop {
            let mut changed = false;
            // Methods calling `this.m()` where `m` (or an override of it) mutates self.
            for (callee, caller) in self.this_calls.clone() {
                if self.method_mutates(callee)
                    && let Some(c) = self.funcs.get_mut(&caller)
                    && !c.mutates_self
                {
                    c.mutates_self = true;
                    changed = true;
                }
            }
            // `obj.m()` where `m` (or an override of it) mutates self: `obj` is mutated.
            for (callee, root) in self.method_receivers.clone() {
                if self.method_mutates(callee)
                    && let Some(v) = self.vars.get_mut(&root)
                    && !v.mutated
                {
                    v.mutated = true;
                    if let Some(p) = v.param_of {
                        self.param_mutated.insert(p);
                    }
                    changed = true;
                }
            }
            // A loop variable changed in place changes its collection.
            for (var, root) in self.loop_sources.clone() {
                if self.vars.get(&var).is_some_and(|v| v.mutated)
                    && let Some(v) = self.vars.get_mut(&root)
                    && !v.mutated
                {
                    v.mutated = true;
                    if let Some(p) = v.param_of {
                        self.param_mutated.insert(p);
                    }
                    changed = true;
                }
            }
            // `f(x)` where `f` mutates or stores its parameter.
            for (callee, index, binding) in self.arg_bindings.clone() {
                let key = (callee, index);
                if self.param_mutated.contains(&key)
                    && let Some(v) = self.vars.get_mut(&binding)
                    && !v.mutated
                {
                    v.mutated = true;
                    if let Some(p) = v.param_of {
                        self.param_mutated.insert(p);
                    }
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
    }

    /// Whether a method, or an override of it, changes `this`.
    fn method_mutates(&self, fn_id: NodeId) -> bool {
        self.funcs.get(&fn_id).is_some_and(|f| f.mutates_self)
            || self
                .overrides
                .get(&fn_id)
                .is_some_and(|os| os.iter().any(|o| self.method_mutates(*o)))
    }

    /// Parameters whose type is still unknown become type parameters; numbers of unknown
    /// flavor become floats.
    fn generalize(&mut self) -> IResult<()> {
        let ids: Vec<NodeId> = self.funcs.keys().copied().collect();
        for id in ids {
            let f = &self.funcs[&id];
            let mut sig = f.sig.clone();
            let mut type_params = f.type_params.clone();
            let mut next = 0;
            let existing: HashSet<String> = type_params.iter().cloned().collect();
            let mut bind_var = |types: &mut Types, t: &Ty| {
                let mut generic = Vec::new();
                collect_vars(types, t, &mut generic);
                for v in generic {
                    let name = loop {
                        let name = type_param_name(next);
                        next += 1;
                        if !existing.contains(&name) {
                            break name;
                        }
                    };
                    types.bind(v, Ty::Named(name.clone()));
                    type_params.push(name);
                }
            };
            for p in &mut sig.params {
                bind_var(&mut self.types, p);
            }
            // The return type: a variable left means the function never returns a value.
            if let Ty::Var(v) = self.types.resolve(&sig.ret) {
                self.types.bind(v, Ty::Void);
            }
            let f = self.funcs.get_mut(&id).expect("fn");
            f.type_params = type_params;
        }
        Ok(())
    }

    /// Local variables and fields must have a known type.
    fn check_locals(&mut self) -> IResult<()> {
        let mut unresolved: Vec<(String, crate::error::Span)> = Vec::new();
        for v in self.vars.values() {
            if matches!(v.role, Role::Func | Role::Class | Role::Enum) {
                continue;
            }
            if let Ty::Var(_) = self.types.resolve(&v.ty) {
                unresolved.push((v.name.clone(), v.span));
            }
        }
        unresolved.sort_by_key(|(_, s)| s.start);
        if let Some((name, span)) = unresolved.first() {
            return Err(Error::new(
                format!("cannot tell the type of `{name}`: add an annotation"),
                *span,
            ));
        }
        for s in &self.structs {
            for f in &s.fields {
                if let Ty::Var(_) = self.types.resolve(&f.ty) {
                    return Err(Error::new(
                        format!(
                            "cannot tell the type of `{}.{}`: add an annotation",
                            s.name, f.name
                        ),
                        f.span,
                    ));
                }
            }
        }
        // Unknown flavors are floats, the honest JavaScript number.
        let _ = Flavor::Float;
        Ok(())
    }

    /// The last uses of each variable: a use after which no other use can run (later uses
    /// are all in other branches of the same tests), outside loops that outlive it.
    fn last_uses(&self) -> HashSet<NodeId> {
        let mut out = HashSet::new();
        for (binding, uses) in &self.uses {
            let Some(v) = self.vars.get(binding) else {
                continue;
            };
            // A use inside a nested closure is never a move.
            if v.captured {
                continue;
            }
            let owner = v.func.unwrap_or(self.main_id);
            for (i, (use_id, depth, fn_id, path)) in uses.iter().enumerate() {
                if *depth > v.loop_depth || *fn_id != owner {
                    continue;
                }
                let later_runs = uses[i + 1..]
                    .iter()
                    .any(|(_, _, _, other)| !exclusive(path, other));
                if !later_runs {
                    out.insert(*use_id);
                }
            }
        }
        out
    }
}

/// Whether two branch paths are on different sides of the same test.
fn exclusive(a: &[(NodeId, u8)], b: &[(NodeId, u8)]) -> bool {
    a.iter()
        .zip(b)
        .any(|((ta, sa), (tb, sb))| ta == tb && sa != sb)
}

/// The next type parameter name not yet in `taken`.
pub(super) fn fresh_type_param(taken: &[String]) -> String {
    let mut n = 0;
    loop {
        let name = type_param_name(n);
        if !taken.contains(&name) {
            return name;
        }
        n += 1;
    }
}

fn type_param_name(n: usize) -> String {
    let letters = ["T", "U", "V", "W", "X", "Y", "Z"];
    match letters.get(n) {
        Some(l) => (*l).to_string(),
        None => format!("T{}", n - letters.len() + 1),
    }
}

pub(super) fn collect_vars(types: &Types, t: &Ty, out: &mut Vec<usize>) {
    match types.resolve(t) {
        Ty::Var(v) => {
            if !out.contains(&v) {
                out.push(v);
            }
        }
        Ty::Option(t) | Ty::Array(t) | Ty::Set(t) | Ty::Gen(t) => collect_vars(types, &t, out),
        Ty::Map(k, v, _) => {
            collect_vars(types, &k, out);
            collect_vars(types, &v, out);
        }
        Ty::Tuple(ts) => ts.iter().for_each(|t| collect_vars(types, t, out)),
        Ty::Func(f) => {
            f.params.iter().for_each(|t| collect_vars(types, t, out));
            collect_vars(types, &f.ret, out);
        }
        _ => {}
    }
}

/// Replaces type parameters by the given types.
pub(super) fn substitute(t: &Ty, subst: &HashMap<String, Ty>, types: &Types) -> Ty {
    match types.resolve(t) {
        Ty::Named(n) => subst.get(&n).cloned().unwrap_or(Ty::Named(n)),
        Ty::Option(t) => Ty::Option(Box::new(substitute(&t, subst, types))),
        Ty::Array(t) => Ty::Array(Box::new(substitute(&t, subst, types))),
        Ty::Set(t) => Ty::Set(Box::new(substitute(&t, subst, types))),
        Ty::Gen(t) => Ty::Gen(Box::new(substitute(&t, subst, types))),
        Ty::Map(k, v, r) => Ty::Map(
            Box::new(substitute(&k, subst, types)),
            Box::new(substitute(&v, subst, types)),
            r,
        ),
        Ty::Tuple(ts) => Ty::Tuple(ts.iter().map(|t| substitute(t, subst, types)).collect()),
        Ty::Func(f) => Ty::Func(Rc::new(FuncTy {
            params: f
                .params
                .iter()
                .map(|t| substitute(t, subst, types))
                .collect(),
            ret: substitute(&f.ret, subst, types),
        })),
        other => other,
    }
}

/// Whether two resolved types have the same shape (variables match anything).
pub fn same_shape_ty(a: &Ty, b: &Ty) -> bool {
    match (a, b) {
        (Ty::Var(_), _) | (_, Ty::Var(_)) => true,
        (Ty::Num(_), Ty::Num(_)) => true,
        (Ty::Option(x), Ty::Option(y))
        | (Ty::Array(x), Ty::Array(y))
        | (Ty::Set(x), Ty::Set(y))
        | (Ty::Gen(x), Ty::Gen(y)) => same_shape_ty(x, y),
        (Ty::Map(k1, v1, _), Ty::Map(k2, v2, _)) => same_shape_ty(k1, k2) && same_shape_ty(v1, v2),
        (Ty::Tuple(xs), Ty::Tuple(ys)) => {
            xs.len() == ys.len() && xs.iter().zip(ys).all(|(x, y)| same_shape_ty(x, y))
        }
        (Ty::Func(f), Ty::Func(g)) => {
            f.params.len() == g.params.len()
                && f.params
                    .iter()
                    .zip(&g.params)
                    .all(|(x, y)| same_shape_ty(x, y))
                && same_shape_ty(&f.ret, &g.ret)
        }
        _ => a == b,
    }
}

/// `user profile`, `user_profile`, `userProfile` → `UserProfile`.
pub fn pascal(s: &str) -> String {
    let mut out = String::new();
    let mut upper = true;
    let mut prev_lower = false;
    for c in s.chars() {
        if c.is_alphanumeric() {
            if c.is_uppercase() && prev_lower {
                upper = true;
            }
            if upper {
                out.extend(c.to_uppercase());
            } else {
                out.push(c);
            }
            upper = false;
            prev_lower = c.is_lowercase() || c.is_ascii_digit();
        } else {
            upper = true;
            prev_lower = false;
        }
    }
    out
}

/// A static string for a builtin name computed at runtime (a handful per program).
pub(super) fn leak(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}
