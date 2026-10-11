//! Programs of several files: every file shares one top-level scope (names are checked
//! not to clash), imports bind their aliases, `import * as ns` members resolve to the
//! exported names.

use std::collections::{HashMap, HashSet};

use super::{IResult, Infer, Role};
#[allow(unused_imports)]
use crate::ast::NodeId;
use crate::ast::*;
use crate::error::Span;
use crate::types::Ty;

impl<'m> Infer<'m> {
    /// The file a source offset is in.
    pub(super) fn file_of(&self, offset: usize) -> usize {
        self.files
            .iter()
            .position(|f| f.contains(offset))
            .unwrap_or(0)
    }

    /// Two files cannot declare the same top-level name: they share one namespace.
    pub(super) fn check_cross_file_names(&self, body: &[Stmt]) -> IResult<()> {
        if self.files.len() < 2 {
            return Ok(());
        }
        let mut seen: HashMap<String, usize> = HashMap::new();
        for s in body {
            let file = self.file_of(s.span.start);
            let names: Vec<(String, Span)> = match &s.kind {
                StmtKind::Func(f) if f.id != usize::MAX => {
                    f.name.iter().map(|n| (n.clone(), f.span)).collect()
                }
                StmtKind::Class(c) => vec![(c.name.clone(), c.span)],
                StmtKind::Enum(e) => vec![(e.name.clone(), e.span)],
                StmtKind::Interface(i) => vec![(i.name.clone(), i.span)],
                StmtKind::TypeAlias(a) => vec![(a.name.clone(), a.span)],
                StmtKind::Var(v) => v
                    .decls
                    .iter()
                    .flat_map(|d| {
                        d.pat
                            .names()
                            .into_iter()
                            .map(|(n, _)| (n.to_string(), d.pat.span()))
                            .collect::<Vec<_>>()
                    })
                    .collect(),
                _ => continue,
            };
            for (name, span) in names {
                if let Some(&other) = seen.get(&name)
                    && other != file
                {
                    return self.err(
                        format!(
                            "`{name}` is declared in both {} and {}: rename one of them, the files share one namespace",
                            self.files[other].name, self.files[file].name
                        ),
                        span,
                    );
                }
                seen.insert(name, file);
            }
        }
        Ok(())
    }

    /// For each file, its exported names and the top-level name each stands for
    /// (re-exports may rename).
    fn exported_names(&self, body: &[Stmt]) -> Vec<HashMap<String, String>> {
        let mut out: Vec<HashMap<String, String>> = vec![HashMap::new(); self.files.len()];
        let mut from_others = Vec::new();
        for s in body {
            let file = self.file_of(s.span.start);
            let mut own = |name: &str| {
                out[file].insert(name.to_string(), name.to_string());
            };
            match &s.kind {
                StmtKind::Func(f) if f.exported => {
                    if let Some(n) = &f.name {
                        own(n);
                    }
                }
                StmtKind::Class(c) if c.exported => own(&c.name),
                StmtKind::Enum(e) if e.exported => own(&e.name),
                StmtKind::Interface(i) if i.exported => own(&i.name),
                StmtKind::TypeAlias(a) if a.exported => own(&a.name),
                StmtKind::Var(v) if v.exported => {
                    for d in &v.decls {
                        for (n, _) in d.pat.names() {
                            own(n);
                        }
                    }
                }
                StmtKind::ExportNames(names) => {
                    for (name, alias) in names {
                        out[file].insert(alias.clone().unwrap_or(name.clone()), name.clone());
                    }
                }
                StmtKind::ExportFrom { names, all, .. } => {
                    if let Some(&target) = self.files[file].imports.get(&s.span.start) {
                        from_others.push((file, target, names.clone(), *all));
                    }
                }
                _ => {}
            }
        }
        // `export … from`: what the other file exports, maybe renamed.
        loop {
            let mut changed = false;
            for (file, target, names, all) in &from_others {
                let theirs = out[*target].clone();
                let mut add = |shown: String, actual: String| {
                    if let std::collections::hash_map::Entry::Vacant(slot) = out[*file].entry(shown)
                    {
                        slot.insert(actual);
                        changed = true;
                    }
                };
                if *all {
                    for (name, actual) in theirs {
                        add(name, actual);
                    }
                } else {
                    for (name, alias) in names {
                        if let Some(actual) = theirs.get(name) {
                            add(alias.clone().unwrap_or(name.clone()), actual.clone());
                        }
                    }
                }
            }
            if !changed {
                break;
            }
        }
        out
    }

    /// Checks every import against what the named file exports and binds the aliases:
    /// the types first (`types` set), then the values.
    pub(super) fn bind_imports(&mut self, body: &'m [Stmt], types: bool) -> IResult<()> {
        if types {
            self.exports = self.exported_names(body);
        }
        for s in body {
            let StmtKind::Import(i) = &s.kind else {
                continue;
            };
            let file = self.file_of(s.span.start);
            let Some(&target) = self.files[file].imports.get(&s.span.start) else {
                return self.err(
                    format!(
                        "`{}` was not loaded: translate the entry file from the command line (`solidify main.ts`), which follows the imports",
                        i.from
                    ),
                    s.span,
                );
            };
            let target_name = self.files[target].name.clone();
            if let Some(ns) = &i.namespace
                && !types
            {
                let id = self.synth_id();
                self.declare(ns, id, Ty::Void, Role::Namespace, s.span);
                self.namespaces.insert(id, target);
            }
            if let Some(local) = &i.default {
                let Some(name) = self.files[target].default_export.clone() else {
                    return self.err(format!("{target_name} has no default export"), s.span);
                };
                if *local != name {
                    self.alias_import(local, &name, types, s.span)?;
                }
            }
            for (name, alias) in &i.names {
                let Some(actual) = self.exports[target].get(name).cloned() else {
                    return self.err(format!("{target_name} does not export `{name}`"), s.span);
                };
                let local = alias.clone().unwrap_or(name.clone());
                if local != actual {
                    self.alias_import(&local, &actual, types, s.span)?;
                }
            }
        }
        Ok(())
    }

    /// `local` means the top-level `name`: a type (`types` set), or a value.
    fn alias_import(&mut self, local: &str, name: &str, types: bool, span: Span) -> IResult<()> {
        if types {
            if self.type_exists(name) {
                let t = self
                    .aliases
                    .get(name)
                    .cloned()
                    .unwrap_or(Ty::Named(name.to_string()));
                self.aliases.insert(local.to_string(), t);
            }
            return Ok(());
        }
        if let Some(id) = self.lookup(name) {
            self.scopes[0].vars.insert(local.to_string(), id);
        } else if !self.type_exists(name) {
            return self.err(format!("unknown name `{name}`"), span);
        }
        Ok(())
    }

    /// The file behind `ns` in `ns.x`, for a namespace import.
    pub(super) fn namespace_of(&self, obj: &Expr) -> Option<usize> {
        let ExprKind::Ident(name) = &obj.kind else {
            return None;
        };
        let id = self.lookup(name)?;
        if self.vars.get(&id)?.role != Role::Namespace {
            return None;
        }
        self.namespaces.get(&id).copied()
    }

    /// The top-level name that `ns.prop` means.
    pub(super) fn namespace_member(&self, file: usize, prop: &str, span: Span) -> IResult<String> {
        match self.exports.get(file).and_then(|e| e.get(prop)) {
            Some(actual) => Ok(actual.clone()),
            None => self.err(
                format!("{} does not export `{prop}`", self.files[file].name),
                span,
            ),
        }
    }

    /// The class or enum an expression names: `C`, or `ns.C` through a namespace import,
    /// with its binding.
    pub(super) fn type_ref(&self, obj: &Expr) -> Option<(String, NodeId)> {
        let name = match &obj.kind {
            ExprKind::Ident(n) => n.clone(),
            ExprKind::Member { obj: ns, prop, .. } => {
                let file = self.namespace_of(ns)?;
                self.exports.get(file)?.get(prop)?.clone()
            }
            _ => return None,
        };
        let id = self.lookup(&name)?;
        let v = self.vars.get(&id)?;
        if !matches!(v.role, Role::Class | Role::Enum) {
            return None;
        }
        Some((v.name.clone(), id))
    }

    /// The names every file exports (for messages).
    #[allow(dead_code)]
    pub(super) fn all_exports(&self) -> HashSet<String> {
        self.exports
            .iter()
            .flat_map(|e| e.keys().cloned())
            .collect()
    }
}
