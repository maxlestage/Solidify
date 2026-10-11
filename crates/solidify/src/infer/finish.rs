//! The end of inference: fixpoints across functions (throws, mutation of parameters),
//! generalization of unconstrained parameters, last uses, resolved types.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::rc::Rc;

use super::{IResult, Infer, Program, Role};
use crate::ast::NodeId;
use crate::error::Error;
use crate::types::{Flavor, FuncTy, Ty, Types};

impl<'m> Infer<'m> {
    pub(super) fn finish(mut self) -> IResult<Program> {
        self.types.settle_flows();
        self.float_growing_numbers();
        self.types.settle_flows();
        let var_inits = std::mem::take(&mut self.var_inits);
        let fn_returns = std::mem::take(&mut self.fn_returns);
        let mut undef_vars = std::mem::take(&mut self.undef_vars);
        let undef_fns = std::mem::take(&mut self.undef_fns);
        let mut undef_fields = std::mem::take(&mut self.undef_fields);
        // Optional parameters and fields (`x?: T`) are `undefined` when not given.
        for (&id, f) in &self.funcs {
            for (i, b) in f.params.iter().enumerate() {
                if f.optional.get(i).copied().unwrap_or(false)
                    || self.undef_params.contains(&(id, i))
                {
                    undef_vars.insert(*b);
                }
            }
        }
        for s in &self.structs {
            for fi in &s.fields {
                if fi.optional {
                    undef_fields.insert((s.name.clone(), fi.name.clone()));
                }
            }
        }
        self.propagate_throws();
        // A method is mutating when any implementation or override of it is: known
        // before what calls it is settled, and again after.
        let ids: Vec<NodeId> = self.funcs.keys().copied().collect();
        for id in &ids {
            if self.method_mutates(*id) {
                self.funcs.get_mut(id).expect("fn").mutates_self = true;
            }
        }
        self.propagate_mutation();
        for id in ids {
            if self.method_mutates(id) {
                self.funcs.get_mut(&id).expect("fn").mutates_self = true;
            }
        }
        self.float_json_numbers();
        self.types.settle_flows();
        let bound_impls = self.check_bound_uses()?;
        // Methods that return `this` on every path.
        for (id, (this, other)) in std::mem::take(&mut self.this_returns) {
            if this > 0
                && other == 0
                && let Some(f) = self.funcs.get_mut(&id)
                && f.method_kind == Some(crate::ast::MethodKind::Method)
                && !f.is_static
                && !f.is_async
                && !self.overrides.contains_key(&id)
            {
                f.returns_this = true;
            }
        }
        let promise_rejects = self.promise_rejects();
        let shared_types = self.shared_types();
        // A narrowed value whose alias is changed is bound mutably.
        for (binding, test) in self.alias_of_narrow.clone() {
            if self.vars.get(&binding).is_some_and(|v| v.mutated)
                && let Some(t) = self.narrow_tests.get_mut(&test)
            {
                t.mutated = true;
            }
        }
        // `const b = list; b.push(x)`: one array (map, set) for both variables.
        loop {
            let mut changed = false;
            for (b, a) in self.var_aliases.clone() {
                let (Some(vb), Some(va)) = (self.vars.get(&b), self.vars.get(&a)) else {
                    continue;
                };
                let collection = matches!(
                    self.types.resolve(&va.ty),
                    Ty::Array(_) | Ty::Map(..) | Ty::Set(_)
                ) || matches!(self.types.resolve(&va.ty), Ty::Named(n, _) if self.struct_by_name(&n).is_some() && !shared_types.contains(&n));
                let changed_somewhere = va.mutated || vb.mutated || va.shared || vb.shared;
                if collection
                    && changed_somewhere
                    && !va.reassigned
                    && !vb.reassigned
                    && !(va.role == Role::TopLevel && va.used_in_fn)
                    && !(vb.role == Role::TopLevel && vb.used_in_fn)
                    && !(va.shared && vb.shared && vb.shares_with == Some(a))
                {
                    self.vars.get_mut(&a).expect("var").shared = true;
                    let vb = self.vars.get_mut(&b).expect("var");
                    vb.shared = true;
                    vb.shares_with = Some(a);
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        // `{ items: list }` with the list or the field changed afterwards: one collection.
        let mut shared_fields = HashSet::new();
        for (s, f, b) in self.field_sources.clone() {
            let Some(v) = self.vars.get(&b) else { continue };
            let collection = matches!(
                self.types.resolve(&v.ty),
                Ty::Array(_) | Ty::Map(..) | Ty::Set(_)
            );
            if collection
                && (v.mutated || v.shared || self.changed_fields.contains(&(s.clone(), f.clone())))
                && !v.reassigned
                && !(v.role == Role::TopLevel && v.used_in_fn)
                && !matches!(v.role, Role::Param)
            {
                self.vars.get_mut(&b).expect("var").shared = true;
                shared_fields.insert((s, f));
            }
        }
        // A variable captured by a closure that is kept (returned, stored, given to a
        // timer) and changed somewhere is shared between the closure and its scope.
        let escaping: Vec<NodeId> = self
            .funcs
            .values()
            .filter(|f| f.escapes || (f.is_async && f.is_closure))
            .flat_map(|f| f.captures.iter().copied())
            .collect();
        for id in escaping {
            // A parameter kept by such a closure is taken by value.
            if let Some(v) = self.vars.get_mut(&id)
                && v.role == Role::Param
            {
                v.stored = true;
                if let Some(p) = v.param_of {
                    self.param_stored.insert(p);
                }
            }
            if let Some(v) = self.vars.get_mut(&id)
                && (v.reassigned || v.mutated || v.mutated_in_closure)
                && !matches!(v.role, Role::Func | Role::Class | Role::Enum)
            {
                v.shared = true;
            }
        }
        let mut boxed = self.boxed_structs();
        // A shared object is already behind a pointer.
        boxed.retain(|n| !shared_types.contains(n));
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
        let error_classes = {
            let mut set = self.error_classes.clone();
            loop {
                let before = set.len();
                for s in &self.structs {
                    if s.parent.as_ref().is_some_and(|p| set.contains(p)) {
                        set.insert(s.name.clone());
                    }
                }
                if set.len() == before {
                    break;
                }
            }
            set
        };
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
        let dyn_casts = self
            .dyn_casts
            .into_iter()
            .map(|(id, (a, b))| (id, (zonk(&a), zonk(&b))))
            .collect();
        let mut program = Program {
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
            dyn_casts,
            dyn_literals: self.dyn_literals,
            shared_types,
            error_classes,
            shared_fields,
            eval_scopes: self.eval_scopes,
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
            promise_rejects,
            auto_awaits: std::mem::take(&mut self.auto_awaits),
            bound_impls,
            undef_vars,
            undef_fns,
            undef_fields,
            undef_elem_vars: HashSet::new(),
            enum_names: std::mem::take(&mut self.enum_names),
        };
        program.settle_undefined(&var_inits, &fn_returns);
        Ok(program)
    }

    /// Each type given for a constrained parameter (`T extends HasId`) must have the
    /// constraint's fields and methods: it gets the constraint's trait. Returns the
    /// constraints and the (constraint, type) pairs to implement.
    fn check_bound_uses(&mut self) -> IResult<BTreeSet<(String, String)>> {
        let uses = std::mem::take(&mut self.bound_uses);
        let mut impls = BTreeSet::new();
        for (c, t, span) in uses {
            let n = match self.types.resolve(&t) {
                Ty::Named(n, _) if self.struct_by_name(&n).is_some() => n,
                // Another type parameter (checked where it is given), or never known.
                Ty::Named(n, _)
                    if self.enum_by_name(&n).is_none() && self.union_by_name(&n).is_none() =>
                {
                    continue;
                }
                Ty::Var(_) => continue,
                other => {
                    let d = self.describe(&other);
                    return self.err(format!("{d} does not have the fields of {c}"), span);
                }
            };
            let cs = self.struct_by_name(&c).expect("constraint").clone();
            for f in &cs.fields {
                let Some(g) = self
                    .struct_by_name(&n)
                    .and_then(|s| s.field(&f.name))
                    .cloned()
                else {
                    return self.err(
                        format!("{n} has no field `{}`, which {c} requires", f.name),
                        span,
                    );
                };
                if f.optional != g.optional {
                    return self.err(
                        format!("`{}` is optional in only one of {n} and {c}", f.name),
                        span,
                    );
                }
                self.unify(&f.ty, &g.ty, span)?;
            }
            for m in &cs.methods {
                if self.find_method(&n, &m.name, m.kind).is_none() {
                    return self.err(
                        format!("{n} has no method `{}`, which {c} requires", m.name),
                        span,
                    );
                }
            }
            impls.insert((c, n));
        }
        Ok(impls)
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
                Ty::Named(n, ..) => {
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
    /// The structs whose instances are held in several places and changed, or compared
    /// with `===`: shared between the places, like JavaScript objects.
    fn shared_types(&self) -> HashSet<String> {
        let mut changed = self.changed_types.clone();
        for (fn_id, class) in &self.instance_calls {
            if self.method_mutates(*fn_id) {
                changed.insert(class.clone());
            }
        }
        // Copies: variables made from a place or kept by something else, and values
        // copied out of a place.
        let mut copied: HashSet<String> = HashSet::new();
        let mut seen = HashSet::new();
        for v in self.vars.values() {
            if (v.alias || v.stored || v.captured)
                && !matches!(v.role, Role::Func | Role::Class | Role::Enum)
            {
                self.structs_in(&v.ty, &mut copied, &mut seen);
            }
        }
        for t in &self.copied_types {
            self.structs_in(t, &mut copied, &mut seen);
        }
        let mut shared: HashSet<String> = changed.intersection(&copied).cloned().collect();
        // An async method that changes `this` runs later, on the object itself.
        for f in self.funcs.values() {
            if f.is_async
                && f.mutates_self
                && let Some(c) = &f.method_of
            {
                shared.insert(c.clone());
            }
        }
        shared.extend(self.identity_types.iter().cloned());
        // A class hierarchy (and the interfaces it implements) is shared as a whole: a
        // value of one class may be held as another.
        loop {
            let before = shared.len();
            for s in &self.structs {
                let related: Vec<&str> = s
                    .parent
                    .iter()
                    .map(String::as_str)
                    .chain(s.implements.iter().map(String::as_str))
                    .collect();
                if related.iter().any(|r| shared.contains(*r)) || shared.contains(&s.name) {
                    shared.insert(s.name.clone());
                    for r in related {
                        if self.struct_by_name(r).is_some() {
                            shared.insert(r.to_string());
                        }
                    }
                }
            }
            if shared.len() == before {
                break;
            }
        }
        shared
    }

    /// The structs a type holds, directly or through fields.
    fn structs_in(&self, t: &Ty, out: &mut HashSet<String>, seen: &mut HashSet<String>) {
        match self.types.resolve(t) {
            Ty::Named(n, args) => {
                for a in &args {
                    self.structs_in(a, out, seen);
                }
                if let Some(s) = self.struct_by_name(&n)
                    && seen.insert(n.clone())
                {
                    out.insert(n);
                    for f in &s.fields {
                        self.structs_in(&f.ty, out, seen);
                    }
                }
            }
            Ty::Option(t) | Ty::Array(t) | Ty::Set(t) | Ty::Gen(t) => {
                self.structs_in(&t, out, seen)
            }
            Ty::Map(k, v, _) => {
                self.structs_in(&k, out, seen);
                self.structs_in(&v, out, seen);
            }
            Ty::Tuple(ts) => {
                for t in &ts {
                    self.structs_in(t, out, seen);
                }
            }
            _ => {}
        }
    }

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
            Ty::Named(n, args) => {
                for a in &args {
                    self.float_unknown_numbers(a, seen);
                }
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

    /// Whether some promise may be rejected, and awaiting one may throw.
    pub(super) fn promise_rejects(&self) -> bool {
        self.rejects || self.funcs.values().any(|f| f.is_async && f.throws)
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
                // An async callee rejects its promise: only waiting for it throws.
                let throws = f.calls.iter().any(|(callee, call)| {
                    !self.calls_in_try.contains(call)
                        && self.funcs.get(callee).is_some_and(|c| {
                            c.throws && (!c.is_async || self.awaited_calls.contains(call))
                        })
                }) || (self.promise_rejects()
                    && self
                        .promise_awaits
                        .iter()
                        .any(|(f, a)| *f == id && !self.calls_in_try.contains(a)));
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
            // A loop variable over a field of `this`, changed: the method changes `this`.
            for (var, method) in self.loop_this_sources.clone() {
                if self.vars.get(&var).is_some_and(|v| v.mutated)
                    && let Some(f) = self.funcs.get_mut(&method)
                    && !f.mutates_self
                {
                    f.mutates_self = true;
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
                    types.bind(v, Ty::named(name.clone()));
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
        Ty::Tuple(ts) | Ty::Named(_, ts) => ts.iter().for_each(|t| collect_vars(types, t, out)),
        Ty::Func(f) => {
            f.params.iter().for_each(|t| collect_vars(types, t, out));
            collect_vars(types, &f.ret, out);
        }
        _ => {}
    }
}

/// Replaces type parameters by the given types.
pub fn substitute(t: &Ty, subst: &HashMap<String, Ty>, types: &Types) -> Ty {
    match types.resolve(t) {
        Ty::Named(n, args) if args.is_empty() => subst.get(&n).cloned().unwrap_or(Ty::named(n)),
        Ty::Named(n, args) => Ty::Named(
            n,
            args.iter().map(|t| substitute(t, subst, types)).collect(),
        ),
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
