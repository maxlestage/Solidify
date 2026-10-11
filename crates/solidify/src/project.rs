//! Programs of several files: the entry file and the files it imports, translated
//! together. Every source is appended to one combined text, so that spans and node ids
//! stay unique; each imported file becomes a Rust module in the output.

use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};

use crate::ast::{Module, StmtKind};
use crate::error::{Error, Span};

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
        let mut loader = Loader {
            root,
            files: Vec::new(),
            texts: Vec::new(),
            bodies: Vec::new(),
            paths: Vec::new(),
            stack: Vec::new(),
            order: Vec::new(),
            src: String::new(),
            next_id: 0,
        };
        loader.load(entry.to_path_buf(), None)?;
        let Loader {
            mut files,
            mut bodies,
            order,
            src,
            next_id,
            ..
        } = loader;
        name_modules(&mut files);
        let mut body = Vec::new();
        for i in order {
            body.append(&mut bodies[i]);
        }
        Ok(Project {
            files,
            src,
            module: Module {
                body,
                node_count: next_id,
                default_export: None,
            },
        })
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
}

impl Loader {
    fn load(
        &mut self,
        path: PathBuf,
        importer: Option<(usize, usize)>,
    ) -> Result<usize, LoadError> {
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
            let target = self.resolve(index, at, &path, &spec)?;
            let t = self.load(target, Some((index, at)))?;
            self.files[index].imports.insert(at, t);
        }
        self.stack.pop();
        self.order.push(index);
        Ok(index)
    }

    /// The file a relative specifier names: `./x`, `./x.ts`, `./x.js`, `./dir`.
    fn resolve(
        &self,
        file: usize,
        at: usize,
        from: &Path,
        spec: &str,
    ) -> Result<PathBuf, LoadError> {
        if !(spec.starts_with("./") || spec.starts_with("../")) {
            return Err(self.at(
                file,
                at,
                &format!(
                    "`{spec}` is not one of your files: only relative imports (`./x`) are translated"
                ),
            ));
        }
        let base = from.parent().map(Path::to_path_buf).unwrap_or_default();
        let target = base.join(spec);
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
        candidates
            .into_iter()
            .find(|c| c.is_file())
            .ok_or_else(|| self.at(file, at, &format!("cannot find `{spec}`")))
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
