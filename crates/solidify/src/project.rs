//! Programs of several files: the entry file and the files it imports, translated
//! together. Every source is appended to one combined text, so that spans and node ids
//! stay unique; each imported file becomes a Rust module in the output.

use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};

use crate::ast::{Module, StmtKind};
use crate::error::{Error, Span};

/// Node's built-in modules, written in TypeScript on a few native functions.
fn builtin_module(spec: &str) -> Option<(&'static str, &'static str)> {
    let name = spec.strip_prefix("node:").unwrap_or(spec);
    Some(match name {
        "fs" => ("node:fs", include_str!("node/fs.ts")),
        "fs/promises" => ("node:fs/promises", include_str!("node/fs_promises.ts")),
        "path" | "path/posix" => ("node:path", include_str!("node/path.ts")),
        "os" => ("node:os", include_str!("node/os.ts")),
        "process" => ("node:process", include_str!("node/process.ts")),
        _ => return None,
    })
}

/// What an import names: a file on disk, or a built-in module.
enum Target {
    Disk(PathBuf),
    Builtin(&'static str, &'static str),
}

/// A file of the program.
#[derive(Clone, Debug)]
pub struct FileInfo {
    /// The path shown in messages, relative to the entry file's directory.
    pub name: String,
    /// The Rust module holding its items (empty for the entry file: the root).
    pub module: String,
    pub is_entry: bool,
    /// Where its text is in the combined source.
    pub start: usize,
    pub end: usize,
    /// The file named by each `import … from` and `export … from`, by the start of the
    /// statement's span.
    pub imports: HashMap<usize, usize>,
    /// The name exported with `export default`.
    pub default_export: Option<String>,
}

impl FileInfo {
    pub fn contains(&self, offset: usize) -> bool {
        self.start <= offset && offset < self.end
    }
}

/// The files of a program, loaded and parsed.
#[derive(Debug)]
pub struct Project {
    pub files: Vec<FileInfo>,
    /// Every source, one after the other: spans point into it.
    pub src: String,
    /// The statements of every file, the imported ones first.
    pub module: Module,
}

/// A file could not be loaded or parsed: the message names the file and the line.
#[derive(Clone, Debug)]
pub struct LoadError(pub String);

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for LoadError {}

impl Project {
    /// One source, as the playground or the standard input give it.
    pub fn single(name: Option<&str>, src: &str) -> Result<Project, Error> {
        let module = crate::parser::parse(src)?;
        // Built-in modules are part of Solidify: no disk needed.
        let builtin = module.body.iter().any(|s| match &s.kind {
            StmtKind::Import(i) => builtin_module(&i.from).is_some(),
            StmtKind::ExportFrom { from, .. } => builtin_module(from).is_some(),
            _ => false,
        });
        if builtin {
            let mut loader = Loader::new(PathBuf::from("."), false);
            loader
                .load_text(
                    PathBuf::from(name.unwrap_or("<stdin>")),
                    name.unwrap_or("<stdin>").to_string(),
                    src.to_string(),
                    None,
                )
                .map_err(|e| Error::new(e.0, Span::default()))?;
            return Ok(loader.finish());
        }
        let file = FileInfo {
            name: name.unwrap_or("<stdin>").to_string(),
            module: String::new(),
            is_entry: true,
            start: 0,
            end: src.len(),
            imports: HashMap::new(),
            default_export: module.default_export.clone(),
        };
        Ok(Project {
            files: vec![file],
            src: src.to_string(),
            module,
        })
    }

    /// The entry file and everything it imports, read from disk.
    pub fn load(entry: &Path) -> Result<Project, LoadError> {
        let root = entry
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        let mut loader = Loader::new(root, true);
        loader.load(Target::Disk(entry.to_path_buf()), None)?;
        Ok(loader.finish())
    }

    /// Translates the program to Rust.
    pub fn transpile(&self, options: &crate::Options) -> Result<String, Error> {
        let program = crate::infer::infer(&self.module, &self.src, &self.files)?;
        let opts = crate::codegen::Options {
            file_name: options.file_name.clone(),
            no_header: options.no_header,
            files: self.files.clone(),
        };
        crate::codegen::generate(&self.module, &program, &opts)
    }

    /// The file an offset of the combined source is in.
    pub fn file_of(&self, offset: usize) -> usize {
        self.files
            .iter()
            .position(|f| f.contains(offset))
            .unwrap_or(0)
    }

    /// An error with the offending line of its file and a caret under it.
    pub fn pretty(&self, e: &Error) -> String {
        let i = self.file_of(e.span.start);
        let f = &self.files[i];
        let local = Error::new(
            e.message.clone(),
            Span::new(
                e.span.start.saturating_sub(f.start),
                e.span.end.saturating_sub(f.start),
            ),
        );
        local
            .pretty(&self.src[f.start..f.end])
            .replacen("  --> ", &format!("  --> {}:", f.name), 1)
    }
}

struct Loader {
    root: PathBuf,
    files: Vec<FileInfo>,
    texts: Vec<String>,
    bodies: Vec<Vec<crate::ast::Stmt>>,
    /// The canonical path of each file.
    paths: Vec<PathBuf>,
    /// The files being loaded (cycle detection).
    stack: Vec<usize>,
    /// Dependency order: imported files first.
    order: Vec<usize>,
    src: String,
    next_id: usize,
    /// Files on disk can be read (not in the playground).
    disk: bool,
}

impl Loader {
    fn new(root: PathBuf, disk: bool) -> Loader {
        Loader {
            root,
            files: Vec::new(),
            texts: Vec::new(),
            bodies: Vec::new(),
            paths: Vec::new(),
            stack: Vec::new(),
            order: Vec::new(),
            src: String::new(),
            next_id: 0,
            disk,
        }
    }

    /// The program: every file's statements, the imported ones first.
    fn finish(self) -> Project {
        let Loader {
            mut files,
            mut bodies,
            order,
            src,
            next_id,
            ..
        } = self;
        name_modules(&mut files);
        // A name declared in two files: renamed in the later one.
        if files.len() > 1 {
            let words: Vec<String> = files
                .iter()
                .map(|f| {
                    f.name
                        .chars()
                        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
                        .collect::<String>()
                        .trim_matches('_')
                        .to_string()
                })
                .collect();
            let renames = crate::rename::separate_files(&mut bodies, &words);
            for (f, map) in files.iter_mut().zip(&renames) {
                if let Some(d) = &f.default_export
                    && let Some(new) = map.get(d)
                {
                    f.default_export = Some(new.clone());
                }
            }
        }
        let mut body = Vec::new();
        for i in order {
            body.append(&mut bodies[i]);
        }
        Project {
            files,
            src,
            module: Module {
                body,
                node_count: next_id,
                default_export: None,
            },
        }
    }

    fn load(
        &mut self,
        target: Target,
        importer: Option<(usize, usize)>,
    ) -> Result<usize, LoadError> {
        let path = match target {
            Target::Builtin(key, text) => {
                let key_path = PathBuf::from(format!("<{key}>"));
                if let Some(i) = self.paths.iter().position(|p| *p == key_path) {
                    return Ok(i);
                }
                return self.load_text(key_path, key.to_string(), text.to_string(), None);
            }
            Target::Disk(path) => path,
        };
        let shown = self.display(&path);
        let canonical = path.canonicalize().map_err(|_| match importer {
            Some((file, at)) => self.at(file, at, &format!("cannot find `{shown}`")),
            None => LoadError(format!("error: cannot find {shown}")),
        })?;
        if let Some(i) = self.paths.iter().position(|p| *p == canonical) {
            if self.stack.contains(&i) {
                let (file, at) = importer.expect("a cycle has an importer");
                return Err(self.at(
                    file,
                    at,
                    &format!(
                        "circular import between {} and {}",
                        self.files[file].name, self.files[i].name
                    ),
                ));
            }
            return Ok(i);
        }
        let text = std::fs::read_to_string(&path)
            .map_err(|e| LoadError(format!("error: cannot read {shown}: {e}")))?;
        self.load_text(canonical, shown, text, Some(path))
    }

    /// Parses a file's text and loads what it imports (`path`: where it is on disk).
    fn load_text(
        &mut self,
        canonical: PathBuf,
        shown: String,
        text: String,
        path: Option<PathBuf>,
    ) -> Result<usize, LoadError> {
        let index = self.files.len();
        let start = self.src.len();
        self.src.push_str(&text);
        self.src.push('\n');
        let end = start + text.len();
        self.paths.push(canonical);
        self.files.push(FileInfo {
            name: shown.clone(),
            module: String::new(),
            is_entry: index == 0,
            start,
            end,
            imports: HashMap::new(),
            default_export: None,
        });
        self.texts.push(text);
        self.bodies.push(Vec::new());
        let module = crate::parser::parse_at(&self.src, start..end, self.next_id)
            .map_err(|e| self.pretty(index, &e))?;
        self.next_id = module.node_count;
        self.files[index].default_export = module.default_export.clone();
        let mut module = module;
        // `import fs from "fs"`: the module itself, like `import * as fs`.
        for s in &mut module.body {
            if let StmtKind::Import(i) = &mut s.kind
                && builtin_module(&i.from).is_some()
                && i.namespace.is_none()
                && let Some(d) = i.default.take()
            {
                i.namespace = Some(d);
            }
        }
        let mut imports: Vec<(usize, String)> = Vec::new();
        for s in &module.body {
            match &s.kind {
                StmtKind::Import(i) => imports.push((s.span.start, i.from.clone())),
                StmtKind::ExportFrom { from, .. } => imports.push((s.span.start, from.clone())),
                _ => {}
            }
        }
        self.bodies[index] = module.body;
        self.stack.push(index);
        for (at, spec) in imports {
            let target = self.resolve(index, at, path.as_deref(), &spec)?;
            let t = self.load(target, Some((index, at)))?;
            self.files[index].imports.insert(at, t);
            // `const m = require("./m")` of a file without a default export: the module.
            if self.files[t].default_export.is_none() {
                for s in &mut self.bodies[index] {
                    if s.span.start == at
                        && let StmtKind::Import(i) = &mut s.kind
                        && i.namespace.is_none()
                        && let Some(d) = i.default.take()
                    {
                        i.namespace = Some(d);
                    }
                }
            }
        }
        self.stack.pop();
        self.order.push(index);
        Ok(index)
    }

    /// What a specifier names: a built-in module (`fs`, `node:path`), a file (`./x`,
    /// `./x.ts`, `./x.js`, `./dir`), or a package in `node_modules`.
    fn resolve(
        &self,
        file: usize,
        at: usize,
        from: Option<&Path>,
        spec: &str,
    ) -> Result<Target, LoadError> {
        if let Some((key, text)) = builtin_module(spec) {
            return Ok(Target::Builtin(key, text));
        }
        let Some(from) = from.filter(|_| self.disk) else {
            return Err(self.at(
                file,
                at,
                &format!("`{spec}` is read from disk: translate the files with the command line"),
            ));
        };
        let base = from.parent().map(Path::to_path_buf).unwrap_or_default();
        if !(spec.starts_with("./") || spec.starts_with("../") || spec.starts_with('/')) {
            return self.package(&base, spec).map(Target::Disk).ok_or_else(|| {
                self.at(
                    file,
                    at,
                    &format!("cannot find the package `{spec}` in a node_modules folder"),
                )
            });
        }
        let target = base.join(spec);
        Self::file_candidates(&target)
            .ok_or_else(|| self.at(file, at, &format!("cannot find `{spec}`")))
            .map(Target::Disk)
    }

    /// `name`, `@scope/name` or `name/sub/path` in the nearest `node_modules`: the file
    /// its `package.json` names (`exports`, `module`, `main`, `types`), or `index`.
    fn package(&self, from: &Path, spec: &str) -> Option<PathBuf> {
        let mut parts = spec.splitn(if spec.starts_with('@') { 3 } else { 2 }, '/');
        let name = if spec.starts_with('@') {
            format!("{}/{}", parts.next()?, parts.next()?)
        } else {
            parts.next()?.to_string()
        };
        let sub = parts.next();
        let mut dir = Some(from.to_path_buf());
        while let Some(d) = dir {
            let pkg = d.join("node_modules").join(&name);
            if pkg.is_dir() {
                if let Some(sub) = sub {
                    return Self::file_candidates(&pkg.join(sub));
                }
                let manifest =
                    std::fs::read_to_string(pkg.join("package.json")).unwrap_or_default();
                for entry in package_entries(&manifest) {
                    if let Some(f) = Self::file_candidates(&pkg.join(entry)) {
                        return Some(f);
                    }
                }
                return Self::file_candidates(&pkg.join("index"));
            }
            dir = d.parent().map(Path::to_path_buf);
        }
        None
    }

    /// The file a path names, with the extensions TypeScript allows.
    fn file_candidates(target: &Path) -> Option<PathBuf> {
        let target = target.to_path_buf();
        let mut candidates = vec![target.clone()];
        if let Some(ext) = target.extension().and_then(|e| e.to_str()) {
            if matches!(ext, "js" | "mjs" | "cjs" | "jsx") {
                candidates.push(target.with_extension("ts"));
                candidates.push(target.with_extension("mts"));
                candidates.push(target.with_extension("tsx"));
            }
        }
        for ext in ["ts", "mts", "tsx", "js", "mjs"] {
            let mut with = target.as_os_str().to_owned();
            with.push(format!(".{ext}"));
            candidates.push(PathBuf::from(with));
        }
        for ext in ["ts", "mts", "js"] {
            candidates.push(target.join(format!("index.{ext}")));
        }
        candidates.into_iter().find(|c| c.is_file())
    }

    /// A path as shown in messages: relative to the entry file's directory.
    fn display(&self, path: &Path) -> String {
        let rel = path.strip_prefix(&self.root).unwrap_or(path);
        let mut parts = Vec::new();
        for c in rel.components() {
            match c {
                std::path::Component::Normal(p) => parts.push(p.to_string_lossy().into_owned()),
                std::path::Component::ParentDir => parts.push("..".into()),
                _ => {}
            }
        }
        parts.join("/")
    }

    /// An error at an offset of a loaded file.
    fn at(&self, file: usize, at: usize, msg: &str) -> LoadError {
        let e = Error::new(msg, Span::new(at, at + 6));
        self.pretty(file, &e)
    }

    fn pretty(&self, file: usize, e: &Error) -> LoadError {
        let f = &self.files[file];
        let local = Error::new(
            e.message.clone(),
            Span::new(
                e.span.start.saturating_sub(f.start),
                e.span.end.saturating_sub(f.start),
            ),
        );
        LoadError(local.pretty(&self.texts[file]).replacen(
            "  --> ",
            &format!("  --> {}:", f.name),
            1,
        ))
    }
}

/// The entry points a `package.json` names, most fitting first: the TypeScript
/// sources, then the ES module, then the main file.
fn package_entries(manifest: &str) -> Vec<String> {
    let field = |key: &str| -> Option<String> {
        let at = manifest.find(&format!("\"{key}\""))?;
        let rest = manifest[at + key.len() + 2..]
            .trim_start()
            .strip_prefix(':')?
            .trim_start();
        let rest = rest.strip_prefix('"')?;
        Some(rest[..rest.find('"')?].to_string())
    };
    // `"exports": { ".": { "import": "./x.js" } }` or `"exports": "./x.js"`: the first
    // path given after the key.
    let exports = manifest.find("\"exports\"").and_then(|at| {
        let rest = &manifest[at..];
        let start = rest.find("\"./")? + 1;
        let end = rest[start..].find('"')? + start;
        Some(rest[start..end].to_string())
    });
    ["source", "types", "typings"]
        .iter()
        .filter_map(|k| field(k))
        .map(|t| t.trim_end_matches(".d.ts").to_string())
        .chain(exports)
        .chain(["module", "main"].iter().filter_map(|k| field(k)))
        .collect()
}

/// Gives every imported file a Rust module name from its path.
fn name_modules(files: &mut [FileInfo]) {
    let mut taken: Vec<String> = Vec::new();
    for f in files.iter_mut() {
        if f.is_entry {
            continue;
        }
        let parts: Vec<String> = f
            .name
            .trim_end_matches(".ts")
            .trim_end_matches(".mts")
            .trim_end_matches(".tsx")
            .trim_end_matches(".js")
            .trim_end_matches(".mjs")
            .split('/')
            .filter(|p| !p.is_empty() && *p != "..")
            .map(module_word)
            .collect();
        let parts = if parts.is_empty() {
            vec!["file".to_string()]
        } else {
            parts
        };
        // The file name, then more of the path when another file has the same name.
        let mut name = parts.last().cloned().unwrap_or_default();
        let mut depth = 1;
        while taken.contains(&name) && depth < parts.len() {
            depth += 1;
            name = parts[parts.len() - depth..].join("_");
        }
        while taken.contains(&name) {
            name.push('_');
        }
        taken.push(name.clone());
        f.module = name;
    }
}

/// A path component as a Rust identifier.
fn module_word(part: &str) -> String {
    let mut out = String::new();
    let mut prev_lower = false;
    for c in part.chars() {
        if c.is_ascii_alphanumeric() {
            if c.is_ascii_uppercase() && prev_lower {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
            prev_lower = c.is_ascii_lowercase() || c.is_ascii_digit();
        } else if !out.ends_with('_') && !out.is_empty() {
            out.push('_');
            prev_lower = false;
        }
    }
    let out = out.trim_end_matches('_').to_string();
    let out = if out.is_empty() {
        "file".to_string()
    } else if out.starts_with(|c: char| c.is_ascii_digit()) {
        format!("f{out}")
    } else {
        out
    };
    const KEYWORDS: &[&str] = &[
        "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn",
        "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
        "return", "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe",
        "use", "where", "while", "async", "await", "dyn", "main",
    ];
    if KEYWORDS.contains(&out.as_str()) {
        format!("{out}_")
    } else {
        out
    }
}
