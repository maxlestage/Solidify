use std::cell::RefCell;
use std::rc::Rc;

mod node_fs {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct MkdirOptions {
        pub recursive: Option<bool>,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct RmOptions {
        pub recursive: Option<bool>,
        pub force: Option<bool>,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct Stats {
        pub size: i64,
        dir: bool,
    }

    impl Stats {
        pub fn new(size: i64, dir: bool) -> Self {
            Self { size, dir }
        }

        pub fn is_file(&self) -> bool {
            !self.dir
        }

        pub fn is_directory(&self) -> bool {
            self.dir
        }
    }

    /// Node's `fs` module: files as text, synchronous, with callbacks, and with promises.
    pub fn exists_sync(path: &str) -> bool {
        js_node_fs_exists(path)
    }

    pub fn read_file_sync(path: &str, encoding: Option<&str>) -> Result<String, String> {
        if !js_node_fs_exists(path) {
            return Err(format!("ENOENT: no such file or directory, open '{path}'"));
        }
        if js_node_fs_is_dir(path) {
            return Err("EISDIR: illegal operation on a directory, read".to_string());
        }
        Ok(js_node_fs_read(path))
    }

    pub fn write_file_sync(path: &str, data: &str) {
        js_node_fs_write(path, data);
    }

    pub fn append_file_sync(path: &str, data: &str) {
        js_node_fs_append(path, data);
    }

    pub fn readdir_sync(path: &str) -> Result<Vec<String>, String> {
        if !js_node_fs_is_dir(path) {
            return Err(format!("ENOENT: no such file or directory, scandir '{path}'"));
        }
        Ok(js_node_fs_readdir(path))
    }

    pub fn mkdir_sync(path: &str, options: Option<&MkdirOptions>) -> Result<(), String> {
        let recursive = if let Some(options) = options { (&options.recursive).as_ref() == Some(&true) } else { false };
        if !recursive && js_node_fs_exists(path) {
            return Err(format!("EEXIST: file already exists, mkdir '{path}'"));
        }
        js_node_fs_mkdir(path, recursive);
        Ok(())
    }

    pub fn rm_sync(path: &str, options: Option<&RmOptions>) -> Result<(), String> {
        let force = if let Some(options) = options { (&options.force).as_ref() == Some(&true) } else { false };
        if !js_node_fs_exists(path) {
            if force {
                return Ok(());
            }
            return Err(format!("ENOENT: no such file or directory, lstat '{path}'"));
        }
        js_node_fs_remove(path, if let Some(options) = options { (&options.recursive).as_ref() == Some(&true) } else { false });
        Ok(())
    }

    pub fn unlink_sync(path: &str) -> Result<(), String> {
        if !js_node_fs_exists(path) {
            return Err(format!("ENOENT: no such file or directory, unlink '{path}'"));
        }
        js_node_fs_remove(path, false);
        Ok(())
    }

    pub fn rename_sync(from: &str, to: &str) {
        js_node_fs_rename(from, to);
    }

    pub fn copy_file_sync(from: &str, to: &str) -> Result<(), String> {
        if !js_node_fs_exists(from) {
            return Err(format!("ENOENT: no such file or directory, copyfile '{from}' -> '{to}'"));
        }
        js_node_fs_copy(from, to);
        Ok(())
    }

    pub fn stat_sync(path: &str) -> Result<Stats, String> {
        if !js_node_fs_exists(path) {
            return Err(format!("ENOENT: no such file or directory, stat '{path}'"));
        }
        Ok(Stats::new(js_node_fs_size(path), js_node_fs_is_dir(path)))
    }

    /// Callbacks run after the current code, like Node's.
    pub fn read_file(path: String, encoding: &str, callback: Rc<dyn Fn(Option<String>, String)>) {
        js_set_timeout(move || {
            if !(js_node_fs_exists(&path)) {
                callback(Some(format!("ENOENT: no such file or directory, open '{path}'")), "".to_string());
            } else {
                callback(None, js_node_fs_read(&path));
            }
        }, 0.0);
    }

    pub fn write_file(path: String, data: String, callback: Rc<dyn Fn(Option<String>)>) {
        js_set_timeout(move || {
            js_node_fs_write(&path, &data);
            callback(None);
        }, 0.0);
    }
}

pub use node_fs::*;

mod node_path {
    use super::*;

    /// Node's `path` module (POSIX), written in the subset Solidify translates.
    pub const SEP: &str = "/";
    pub const DELIMITER: &str = ":";

    #[derive(Debug, Clone, PartialEq)]
    pub struct ParsedPath {
        pub root: String,
        pub dir: String,
        pub base: String,
        pub ext: String,
        pub name: String,
    }

    pub fn is_absolute(p: &str) -> bool {
        p.starts_with("/")
    }

    pub fn normalize(p: &str) -> String {
        if p == "" {
            return ".".to_string();
        }
        let absolute = p.starts_with("/");
        let trailing = p.ends_with("/");
        let mut out: Vec<String> = Vec::new();
        for part in p.split("/").map(String::from).collect::<Vec<String>>() {
            if &part == "" || &part == "." {
                continue;
            }
            if &part == ".." {
                if out.len() > 0 && &out[out.len() - 1] != ".." {
                    out.pop();
                } else if !absolute {
                    out.push("..".to_string());
                }
                continue;
            }
            out.push(part);
        }
        let mut result = out.join("/");
        if absolute {
            result = format!("/{result}");
        }
        if &result == "" {
            return if absolute { "/".to_string() } else { ".".to_string() };
        }
        if trailing && &result != "/" {
            result.push_str("/");
        }
        result
    }

    pub fn join(parts: &[String]) -> String {
        let kept = parts.iter().filter(|p| p.as_str() != "").cloned().collect::<Vec<_>>();
        if (kept.len() as i64) == 0 {
            return ".".to_string();
        }
        normalize(&kept.join("/"))
    }

    pub fn resolve(parts: &[String]) -> String {
        let mut result = "".to_string();
        for i in (0..parts.len()).rev() {
            let p = parts[i].clone();
            if &p == "" {
                continue;
            }
            result = if &result == "" { p.clone() } else { format!("{p}/{result}") };
            if p.starts_with("/") {
                break;
            }
        }
        if !(result.starts_with("/")) {
            result = if &result == "" { js_node_os_info("cwd") } else { format!("{}/{result}", js_node_os_info("cwd")) };
        }
        let n = normalize(&result);
        if n.chars().count() > 1 && n.ends_with("/") { js_slice(&n, 0, Some(-1)) } else { n }
    }

    pub fn dirname(p: &str) -> String {
        if p == "" {
            return ".".to_string();
        }
        let mut end = p.chars().count() as i64;
        while end > 1 && &js_char_at(p, end - 1) == "/" {
            end -= 1;
        }
        let i = js_last_index_of_from(p, "/", end - 1);
        if i < 0 {
            return ".".to_string();
        }
        if i == 0 {
            return "/".to_string();
        }
        js_slice(p, 0, Some(i))
    }

    pub fn basename(p: &str, ext: Option<&str>) -> String {
        let mut end = p.chars().count() as i64;
        while end > 1 && &js_char_at(p, end - 1) == "/" {
            end -= 1;
        }
        let trimmed = js_slice(p, 0, Some(end));
        let i = js_last_index_of(&trimmed, "/");
        let base = if i >= 0 { js_slice(&trimmed, i + 1, None) } else { trimmed };
        if let Some(ext) = ext && ext != &base && base.ends_with(ext) {
            return js_slice(&base, 0, Some((base.chars().count() - ext.chars().count()) as i64));
        }
        base
    }

    pub fn extname(p: &str) -> String {
        let base = basename(p, None);
        let i = js_last_index_of(&base, ".");
        if i <= 0 {
            return "".to_string();
        }
        js_slice(&base, i, None)
    }

    pub fn relative(from: String, to: String) -> String {
        let a = resolve(&[from]).split("/").map(String::from).collect::<Vec<String>>().into_iter().filter(|x| x.as_str() != "").collect::<Vec<_>>();
        let b = resolve(&[to]).split("/").map(String::from).collect::<Vec<String>>().into_iter().filter(|x| x.as_str() != "").collect::<Vec<_>>();
        let mut common = 0;
        while common < a.len() as i64 && common < b.len() as i64 && &a[common as usize] == &b[common as usize] {
            common += 1;
        }
        let mut up: Vec<String> = Vec::new();
        for _ in common..a.len() as i64 {
            up.push("..".to_string());
        }
        [&(&up)[..], &(&js_slice_vec(&b, common, None))[..]].concat().join("/")
    }

    pub fn parse(p: &str) -> ParsedPath {
        let base = basename(p, None);
        let ext = extname(p);
        let dir = if p.contains("/") { dirname(p) } else { "".to_string() };
        ParsedPath { root: if p.starts_with("/") { "/".to_string() } else { "".to_string() }, dir, base: base.clone(), ext: ext.clone(), name: if &ext == "" { base } else { js_slice(&base, 0, Some((base.chars().count() - ext.chars().count()) as i64)) } }
    }

    pub fn format(p: &ParsedPath) -> String {
        let base = if &p.base != "" { p.base.clone() } else { format!("{}{}", p.name, p.ext) };
        if &p.dir == "" {
            return format!("{}{base}", p.root);
        }
        if &p.dir == &p.root { format!("{}{base}", p.dir) } else { format!("{}/{base}", p.dir) }
    }

    impl JsInspect for ParsedPath {
        fn js_value(&self) -> JsValue {
            JsValue::Object(None, vec![("root".to_string(), self.root.js_value()), ("dir".to_string(), self.dir.js_value()), ("base".to_string(), self.base.js_value()), ("ext".to_string(), self.ext.js_value()), ("name".to_string(), self.name.js_value())])
        }
    }
}

pub use node_path::*;

mod node_os {
    use super::*;

    /// Node's `os` module.
    pub const EOL: &str = "\n";

    pub fn platform() -> String {
        js_node_os_info("platform")
    }

    pub fn arch() -> String {
        js_node_os_info("arch")
    }

    pub fn r#type() -> String {
        js_node_os_info("type")
    }

    pub fn homedir() -> String {
        js_node_os_info("homedir")
    }

    pub fn tmpdir() -> String {
        js_node_os_info("tmpdir")
    }

    pub fn hostname() -> String {
        js_node_os_info("hostname")
    }

    pub fn available_parallelism() -> i64 {
        js_node_os_cpus()
    }
}

pub use node_os::*;

fn main() -> Result<(), String> {
    // Node's `path` module (POSIX), written in the subset Solidify translates.
    // Node's `os` module.
    let dir = join(&[tmpdir(), "solidify-node-builtins".to_string()]);
    rm_sync(&dir, Some(&RmOptions { recursive: Some(true), force: Some(true) }))?;
    mkdir_sync(&dir, Some(&MkdirOptions { recursive: Some(true) }))?;
    let file = join(&[dir.clone(), "notes.txt".to_string()]);
    write_file_sync(&file, "hello\n");
    append_file_sync(&file, "world\n");
    println!("{} {} {} {}", exists_sync(&file), read_file_sync(&file, Some("utf8"))?.split("\n").map(String::from).collect::<Vec<String>>().len(), stat_sync(&file)?.size, stat_sync(&dir)?.is_directory());
    write_file_sync(&join(&[dir.clone(), "b.txt".to_string()]), "b");
    println!("{}", js_inspect(&readdir_sync(&dir)?.js_value()));
    println!("{} {} {} {}", basename(&file, None), extname(&file), dirname("/a/b/c.txt"), basename("/a/b/c.txt", Some(".txt")));
    println!("{} {} {} {}", join(&["a".to_string(), "b".to_string(), "../c".to_string(), "./d".to_string()]), normalize("/a//b/../c/"), is_absolute("/x"), relative("/a/b".to_string(), "/a/c/d".to_string()));
    println!("{} {SEP} {}", js_inspect(&parse("/home/u/file.tar.gz").js_value()), EOL == "\n");
    let result_: Result<(), String> = (|| {
        read_file_sync(&join(&[dir.clone(), "missing.txt".to_string()]), Some("utf8"))?;
        Ok(())
    })();
    if let Err(e) = result_ {
        println!("caught {}", e.starts_with("ENOENT"));
    }
    rm_sync(&dir, Some(&RmOptions { recursive: Some(true), force: None }))?;
    println!("{} {} {}", exists_sync(&dir), "string", &js_node_os_info("platform") == &platform());
    js_run_timers();
    Ok(())
}

/// `s.slice(start, end)`: character positions, negative ones counted from the end.
fn js_slice(s: &str, start: i64, end: Option<i64>) -> String {
    let len = s.chars().count() as i64;
    let clamp = |i: i64| if i < 0 { (len + i).max(0) } else { i.min(len) };
    let (a, b) = (clamp(start), clamp(end.unwrap_or(len)));
    if b <= a {
        return String::new();
    }
    s.chars().skip(a as usize).take((b - a) as usize).collect()
}

/// `s.charAt(i)` / `s[i]`: one character, or an empty string.
fn js_char_at(s: &str, i: i64) -> String {
    if i < 0 {
        return String::new();
    }
    s.chars().nth(i as usize).map(String::from).unwrap_or_default()
}

/// `s.lastIndexOf(needle)`: the last character position, or -1.
fn js_last_index_of(s: &str, needle: &str) -> i64 {
    match s.rfind(needle) {
        Some(byte) => s[..byte].chars().count() as i64,
        None => -1,
    }
}

/// `s.lastIndexOf(needle, from)`: the last match starting at `from` or before.
#[allow(dead_code)]
fn js_last_index_of_from(s: &str, needle: &str, from: i64) -> i64 {
    let chars: Vec<char> = s.chars().collect();
    let n: Vec<char> = needle.chars().collect();
    if n.len() > chars.len() {
        return -1;
    }
    let last = (chars.len() - n.len()) as i64;
    let mut i = from.clamp(0, last.max(0)).min(last);
    while i >= 0 {
        if chars[i as usize..].starts_with(&n) {
            return i;
        }
        i -= 1;
    }
    -1
}

/// `list.slice(start, end)`: negative positions are counted from the end.
fn js_slice_vec<T: Clone>(v: &[T], start: i64, end: Option<i64>) -> Vec<T> {
    let len = v.len() as i64;
    let clamp = |i: i64| if i < 0 { (len + i).max(0) } else { i.min(len) };
    let (a, b) = (clamp(start), clamp(end.unwrap_or(len)));
    if b <= a {
        return Vec::new();
    }
    v[a as usize..b as usize].to_vec()
}

/// A number the way JavaScript prints it.
#[allow(dead_code)]
fn js_number_text(x: f64) -> String {
    if x.is_nan() {
        return "NaN".into();
    }
    if x.is_infinite() {
        return if x > 0.0 { "Infinity" } else { "-Infinity" }.into();
    }
    if x == 0.0 {
        return "0".into();
    }
    if x.fract() == 0.0 && x.abs() < 1e15 {
        return format!("{}", x as i64);
    }
    // The shortest digits that read back as `x`, then JavaScript's placement of the point.
    let sci = format!("{:e}", x.abs());
    let (mantissa, exp) = sci.split_once('e').unwrap_or((&sci, "0"));
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let n: i64 = exp.parse::<i64>().unwrap_or(0) + 1;
    let k = digits.len() as i64;
    let body = if k <= n && n <= 21 {
        format!("{digits}{}", "0".repeat((n - k) as usize))
    } else if 0 < n && n <= 21 {
        format!("{}.{}", &digits[..n as usize], &digits[n as usize..])
    } else if -6 < n && n <= 0 {
        format!("0.{}{digits}", "0".repeat((-n) as usize))
    } else {
        let e = n - 1;
        let sign = if e < 0 { "-" } else { "+" };
        if k == 1 {
            format!("{digits}e{sign}{}", e.abs())
        } else {
            format!("{}.{}e{sign}{}", &digits[..1], &digits[1..], e.abs())
        }
    };
    if x < 0.0 { format!("-{body}") } else { body }
}


/// A value as JavaScript sees it, for `console.log`, `String(…)` and `JSON.stringify`.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
enum JsValue {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Null,
    Undefined,
    Array(Vec<JsValue>),
    /// A plain object, or an instance of the named class.
    Object(Option<&'static str>, Vec<(String, JsValue)>),
    Map(Vec<(JsValue, JsValue)>),
    Set(Vec<JsValue>),
    /// A function, by name.
    Func(&'static str),
    /// Text shown as it is (a regular expression).
    Raw(String),
    /// A date: its ISO text, and its `toString()` text.
    Date(String, String),
    /// An object held in several places: changed through one, changed for all.
    Ref(std::rc::Rc<std::cell::RefCell<JsValue>>),
    /// `new Proxy(target, handler)`.
    Proxy(std::rc::Rc<JsProxy>),
}

/// The traps of a proxy, and the object it stands for.
#[allow(dead_code, clippy::type_complexity)]
struct JsProxy {
    target: JsValue,
    get: Option<Box<dyn Fn(JsValue, String, JsValue) -> JsValue>>,
    set: Option<Box<dyn Fn(JsValue, String, JsValue, JsValue) -> bool>>,
    has: Option<Box<dyn Fn(JsValue, String) -> bool>>,
    delete: Option<Box<dyn Fn(JsValue, String) -> bool>>,
}

impl std::fmt::Debug for JsProxy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Proxy({:?})", self.target)
    }
}

impl PartialEq for JsProxy {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

#[allow(dead_code)]
impl JsValue {
    /// What a shared object or a proxy holds, to be shown.
    fn shown(&self) -> Option<JsValue> {
        match self {
            JsValue::Ref(cell) => Some(cell.borrow().clone()),
            JsValue::Proxy(p) => p.target.shown().or_else(|| Some(p.target.clone())),
            _ => None,
        }
    }
}

/// Implemented by every type that `console.log` can show.
#[allow(dead_code)]
trait JsInspect {
    fn js_value(&self) -> JsValue;
}

impl JsInspect for i64 {
    fn js_value(&self) -> JsValue {
        JsValue::Int(*self)
    }
}
impl JsInspect for f64 {
    fn js_value(&self) -> JsValue {
        JsValue::Float(*self)
    }
}
impl JsInspect for usize {
    fn js_value(&self) -> JsValue {
        JsValue::Int(*self as i64)
    }
}
impl JsInspect for i32 {
    fn js_value(&self) -> JsValue {
        JsValue::Int(*self as i64)
    }
}
impl JsInspect for u32 {
    fn js_value(&self) -> JsValue {
        JsValue::Int(*self as i64)
    }
}
impl JsInspect for u64 {
    fn js_value(&self) -> JsValue {
        JsValue::Int(*self as i64)
    }
}
impl JsInspect for f32 {
    fn js_value(&self) -> JsValue {
        JsValue::Float(*self as f64)
    }
}
impl JsInspect for bool {
    fn js_value(&self) -> JsValue {
        JsValue::Bool(*self)
    }
}
impl JsInspect for str {
    fn js_value(&self) -> JsValue {
        JsValue::Str(self.to_string())
    }
}
impl JsInspect for String {
    fn js_value(&self) -> JsValue {
        JsValue::Str(self.clone())
    }
}
impl JsInspect for char {
    fn js_value(&self) -> JsValue {
        JsValue::Str(self.to_string())
    }
}
impl JsInspect for () {
    fn js_value(&self) -> JsValue {
        JsValue::Undefined
    }
}
impl<T: JsInspect + ?Sized> JsInspect for &T {
    fn js_value(&self) -> JsValue {
        (**self).js_value()
    }
}
impl<T: JsInspect + ?Sized> JsInspect for Box<T> {
    fn js_value(&self) -> JsValue {
        (**self).js_value()
    }
}
impl<T: JsInspect> JsInspect for Option<T> {
    fn js_value(&self) -> JsValue {
        match self {
            Some(v) => v.js_value(),
            None => JsValue::Null,
        }
    }
}
impl<T: JsInspect> JsInspect for std::rc::Rc<std::cell::RefCell<T>> {
    fn js_value(&self) -> JsValue {
        self.borrow().js_value()
    }
}
impl<T: JsInspect> JsInspect for Vec<T> {
    fn js_value(&self) -> JsValue {
        JsValue::Array(self.iter().map(JsInspect::js_value).collect())
    }
}
impl<T: JsInspect> JsInspect for [T] {
    fn js_value(&self) -> JsValue {
        JsValue::Array(self.iter().map(JsInspect::js_value).collect())
    }
}
impl<A: JsInspect, B: JsInspect> JsInspect for (A, B) {
    fn js_value(&self) -> JsValue {
        JsValue::Array(vec![self.0.js_value(), self.1.js_value()])
    }
}
impl<A: JsInspect, B: JsInspect, C: JsInspect> JsInspect for (A, B, C) {
    fn js_value(&self) -> JsValue {
        JsValue::Array(vec![self.0.js_value(), self.1.js_value(), self.2.js_value()])
    }
}
impl<K: JsInspect, V: JsInspect> JsInspect for std::collections::HashMap<K, V> {
    fn js_value(&self) -> JsValue {
        JsValue::Map(self.iter().map(|(k, v)| (k.js_value(), v.js_value())).collect())
    }
}
impl<T: JsInspect> JsInspect for std::collections::HashSet<T> {
    fn js_value(&self) -> JsValue {
        JsValue::Set(self.iter().map(JsInspect::js_value).collect())
    }
}

/// `String(value)`: what a value becomes in a string.
#[allow(dead_code)]
fn js_to_string(v: &JsValue) -> String {
    if let Some(inner) = v.shown() {
        return js_to_string(&inner);
    }
    match v {
        JsValue::Int(n) => n.to_string(),
        JsValue::Float(x) => js_number_text(*x),
        JsValue::Str(s) => s.clone(),
        JsValue::Bool(b) => b.to_string(),
        JsValue::Null => "null".into(),
        JsValue::Undefined => "undefined".into(),
        JsValue::Array(items) => items
            .iter()
            .map(|i| match i {
                JsValue::Null | JsValue::Undefined => String::new(),
                other => js_to_string(other),
            })
            .collect::<Vec<_>>()
            .join(","),
        JsValue::Object(..) => "[object Object]".into(),
        JsValue::Map(_) => "[object Map]".into(),
        JsValue::Set(_) => "[object Set]".into(),
        JsValue::Func(name) => format!("function {name}() {{ [native code] }}"),
        JsValue::Raw(text) => text.clone(),
        JsValue::Date(_, text) => text.clone(),
        JsValue::Ref(_) | JsValue::Proxy(_) => unreachable!("shown above"),
    }
}

/// One argument of `console.log`: a string as it is, anything else inspected.
#[allow(dead_code)]
fn js_show(v: &JsValue) -> String {
    match v {
        JsValue::Str(s) => s.clone(),
        other => js_inspect(other),
    }
}

/// A string literal as Node's `util.inspect` quotes it.
#[allow(dead_code)]
fn js_quote(s: &str) -> String {
    let quote = if !s.contains('\'') {
        '\''
    } else if !s.contains('"') {
        '"'
    } else if !s.contains('`') && !s.contains("${") {
        '`'
    } else {
        '\''
    };
    let mut out = String::with_capacity(s.len() + 2);
    out.push(quote);
    for c in s.chars() {
        match c {
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\\' => out.push_str("\\\\"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c if (c as u32) < 0x20 || c == '\u{7f}' => out.push_str(&format!("\\x{:02X}", c as u32)),
            c => out.push(c),
        }
    }
    out.push(quote);
    out
}

#[allow(dead_code)]
fn js_key(key: &str) -> String {
    let plain = key
        .chars()
        .enumerate()
        .all(|(i, c)| c == '_' || c.is_ascii_alphabetic() || (i > 0 && c.is_ascii_digit()));
    if plain && !key.is_empty() { key.to_string() } else { js_quote(key) }
}

#[allow(dead_code)]
struct JsInspectCtx {
    indentation: usize,
    current_depth: usize,
}

/// `util.inspect(value)`: Node's layout, line breaks and grouping included.
#[allow(dead_code)]
fn js_inspect(v: &JsValue) -> String {
    let mut ctx = JsInspectCtx { indentation: 0, current_depth: 0 };
    js_format(v, 0, &mut ctx)
}

#[allow(dead_code)]
fn js_format(v: &JsValue, recurse_times: usize, ctx: &mut JsInspectCtx) -> String {
    if let Some(inner) = v.shown() {
        return js_format(&inner, recurse_times, ctx);
    }
    let (open, close) = match v {
        JsValue::Int(n) => return n.to_string(),
        JsValue::Float(x) => {
            return if *x == 0.0 && x.is_sign_negative() { "-0".into() } else { js_number_text(*x) };
        }
        JsValue::Str(s) => return js_quote(s),
        JsValue::Bool(b) => return b.to_string(),
        JsValue::Null => return "null".into(),
        JsValue::Undefined => return "undefined".into(),
        JsValue::Func(name) => {
            return if name.is_empty() {
                "[Function (anonymous)]".into()
            } else {
                format!("[Function: {name}]")
            };
        }
        JsValue::Raw(text) => return text.clone(),
        JsValue::Date(iso, _) => return iso.clone(),
        JsValue::Ref(_) | JsValue::Proxy(_) => unreachable!("shown above"),
        JsValue::Array(items) => {
            if items.is_empty() {
                return "[]".into();
            }
            if recurse_times > 2 {
                return "[Array]".into();
            }
            ("[".to_string(), "]")
        }
        JsValue::Object(name, props) => {
            if props.is_empty() {
                return match name {
                    Some(n) => format!("{n} {{}}"),
                    None => "{}".into(),
                };
            }
            if recurse_times > 2 {
                return match name {
                    Some(n) => format!("[{n}]"),
                    None => "[Object]".into(),
                };
            }
            (name.map_or("{".to_string(), |n| format!("{n} {{")), "}")
        }
        JsValue::Map(entries) => {
            if entries.is_empty() {
                return "Map(0) {}".into();
            }
            if recurse_times > 2 {
                return "[Map]".into();
            }
            (format!("Map({}) {{", entries.len()), "}")
        }
        JsValue::Set(items) => {
            if items.is_empty() {
                return "Set(0) {}".into();
            }
            if recurse_times > 2 {
                return "[Set]".into();
            }
            (format!("Set({}) {{", items.len()), "}")
        }
    };
    let recurse_times = recurse_times + 1;
    ctx.current_depth = recurse_times;
    let mut output = Vec::new();
    ctx.indentation += 2;
    match v {
        JsValue::Array(items) => {
            for item in items.iter().take(100) {
                output.push(js_format(item, recurse_times, ctx));
            }
            if items.len() > 100 {
                let rest = items.len() - 100;
                output.push(format!("... {rest} more item{}", if rest > 1 { "s" } else { "" }));
            }
        }
        JsValue::Object(_, props) => {
            for (k, val) in props {
                let text = js_format(val, recurse_times, ctx);
                output.push(format!("{}: {text}", js_key(k)));
            }
        }
        JsValue::Map(entries) => {
            for (k, val) in entries {
                let key = js_format(k, recurse_times, ctx);
                let text = js_format(val, recurse_times, ctx);
                output.push(format!("{key} => {text}"));
            }
        }
        JsValue::Set(items) => {
            for item in items {
                output.push(js_format(item, recurse_times, ctx));
            }
        }
        _ => {}
    }
    ctx.indentation -= 2;
    let entries = output.len();
    if let JsValue::Array(items) = v
        && entries > 6
    {
        output = js_group_array(ctx, output, items);
    }
    if ctx.current_depth - recurse_times < 3 && entries == output.len() {
        // Node counts a `<ref *1>` marker apart from the braces, without the space.
        let marker = usize::from(open.starts_with("<ref *"));
        let start = output.len() + ctx.indentation + open.chars().count() - marker + 10;
        let mut total = output.len() + start;
        let fits = total + output.len() <= 80 && output.iter().all(|s| {
            total += s.chars().count();
            total <= 80
        });
        if fits {
            let joined = output.join(", ");
            if !joined.contains('\n') {
                return format!("{open} {joined} {close}");
            }
        }
    }
    let indentation = format!("\n{}", " ".repeat(ctx.indentation));
    format!(
        "{open}{indentation}  {}{indentation}{close}",
        output.join(&format!(",{indentation}  "))
    )
}

/// Node's column layout for arrays of more than six short entries.
#[allow(dead_code)]
fn js_group_array(ctx: &JsInspectCtx, output: Vec<String>, items: &[JsValue]) -> Vec<String> {
    let mut total_length = 0;
    let mut max_length = 0;
    let output_length = if items.len() > 100 { output.len() - 1 } else { output.len() };
    let separator_space = 2;
    let data_len: Vec<usize> = output.iter().map(|s| s.chars().count()).collect();
    for &len in data_len.iter().take(output_length) {
        total_length += len + separator_space;
        max_length = max_length.max(len);
    }
    let actual_max = max_length + separator_space;
    if actual_max * 3 + ctx.indentation < 80
        && (total_length as f64 / actual_max as f64 > 5.0 || max_length <= 6)
    {
        let approx_char_heights = 2.5;
        let average_bias = (actual_max as f64 - total_length as f64 / output.len() as f64).sqrt();
        let biased_max = (actual_max as f64 - 3.0 - average_bias).max(1.0);
        let columns = ((approx_char_heights * biased_max * output_length as f64).sqrt() / biased_max)
            .round()
            .min(((80 - ctx.indentation) / actual_max) as f64)
            .min(12.0)
            .min(15.0) as usize;
        if columns <= 1 {
            return output;
        }
        let mut max_line_length = Vec::new();
        for i in 0..columns {
            let mut line_length = 0;
            let mut j = i;
            while j < output_length {
                if data_len[j] > line_length {
                    line_length = data_len[j];
                }
                j += columns;
            }
            max_line_length.push(line_length + separator_space);
        }
        let pad_start = items.iter().all(|v| matches!(v, JsValue::Int(_) | JsValue::Float(_)));
        let mut tmp = Vec::new();
        let mut i = 0;
        while i < output_length {
            let max = (i + columns).min(output_length);
            let mut line = String::new();
            let mut j = i;
            while j < max - 1 {
                let cell = format!("{}, ", output[j]);
                let width = max_line_length[j - i];
                let padding = width.saturating_sub(cell.chars().count());
                if pad_start {
                    line.push_str(&" ".repeat(padding));
                    line.push_str(&cell);
                } else {
                    line.push_str(&cell);
                    line.push_str(&" ".repeat(padding));
                }
                j += 1;
            }
            if pad_start {
                let width = max_line_length[j - i] - separator_space;
                let padding = width.saturating_sub(data_len[j]);
                line.push_str(&" ".repeat(padding));
            }
            line.push_str(&output[j]);
            tmp.push(line);
            i += columns;
        }
        if items.len() > 100 {
            tmp.push(output[output_length].clone());
        }
        return tmp;
    }
    output
}


/// A timer made by `setTimeout` or `setInterval`.
struct JsTimer {
    id: i64,
    due: std::time::Instant,
    every: Option<std::time::Duration>,
    callback: Box<dyn FnMut()>,
}

thread_local! {
    static JS_TIMERS: std::cell::RefCell<Vec<JsTimer>> = const { std::cell::RefCell::new(Vec::new()) };
    static JS_TIMER_ID: std::cell::Cell<i64> = const { std::cell::Cell::new(0) };
    /// The interval being run was cleared from inside its callback.
    static JS_CLEARED: std::cell::Cell<i64> = const { std::cell::Cell::new(0) };
}

/// A fresh timer id.
#[allow(dead_code)]
fn js_timer_id() -> i64 {
    JS_TIMER_ID.with(|n| {
        n.set(n.get() + 1);
        n.get()
    })
}

/// Schedules `callback` in `ms` milliseconds, again every `ms` when `repeat`.
#[allow(dead_code)]
fn js_schedule(id: i64, callback: impl FnMut() + 'static, ms: f64, repeat: bool) {
    // Like Node: a delay under 1 ms (or not a number) is 1 ms.
    let ms = if ms >= 1.0 { ms } else { 1.0 };
    let delay = std::time::Duration::from_secs_f64(ms / 1000.0);
    JS_TIMERS.with(|timers| {
        timers.borrow_mut().push(JsTimer {
            id,
            due: std::time::Instant::now() + delay,
            every: if repeat { Some(delay) } else { None },
            callback: Box::new(callback),
        })
    });
}

/// `setTimeout(callback, ms)`: the timer id.
#[allow(dead_code)]
fn js_set_timeout(callback: impl FnMut() + 'static, ms: f64) -> i64 {
    let id = js_timer_id();
    js_schedule(id, callback, ms, false);
    id
}

/// `setInterval(callback, ms)`: the timer id.
#[allow(dead_code)]
fn js_set_interval(callback: impl FnMut() + 'static, ms: f64) -> i64 {
    let id = js_timer_id();
    js_schedule(id, callback, ms, true);
    id
}

/// `clearTimeout(id)` / `clearInterval(id)`.
#[allow(dead_code)]
fn js_clear_timer(id: i64) {
    JS_TIMERS.with(|timers| timers.borrow_mut().retain(|t| t.id != id));
    JS_CLEARED.with(|c| c.set(id));
}

/// Runs the next timer due, waiting for it; `false` when none is left.
#[allow(dead_code)]
fn js_run_next_timer() -> bool {
    let next = JS_TIMERS.with(|timers| {
        let mut timers = timers.borrow_mut();
        let i = (0..timers.len()).min_by_key(|&i| timers[i].due)?;
        Some(timers.remove(i))
    });
    let Some(mut timer) = next else {
        return false;
    };
    let now = std::time::Instant::now();
    if timer.due > now {
        std::thread::sleep(timer.due - now);
    }
    JS_CLEARED.with(|c| c.set(0));
    (timer.callback)();
    if let Some(every) = timer.every
        && JS_CLEARED.with(|c| c.get()) != timer.id
    {
        timer.due += every;
        JS_TIMERS.with(|timers| timers.borrow_mut().push(timer));
    }
    true
}

/// Runs the tasks and every timer, like Node does once the script has run.
#[allow(dead_code)]
fn js_run_timers() {
    loop {
        js_run_tasks();
        js_report_unhandled();
        if !js_run_next_timer() {
            break;
        }
    }
}

type JsTask = std::pin::Pin<Box<dyn std::future::Future<Output = ()>>>;

thread_local! {
    /// The async functions waiting for a promise.
    static JS_TASKS: std::cell::RefCell<Vec<Option<JsTask>>> = const { std::cell::RefCell::new(Vec::new()) };
    /// Counts settled promises: the tasks move on when it changes.
    static JS_TICK: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    /// Rejected promises nothing waited for: (promise, message).
    static JS_UNHANDLED: std::cell::RefCell<Vec<(usize, String)>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Runs a task until it waits; kept for later when it does.
#[allow(dead_code)]
fn js_start_task(mut task: JsTask) {
    let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
    if task.as_mut().poll(&mut cx).is_pending() {
        JS_TASKS.with(|t| t.borrow_mut().push(Some(task)));
    }
}

/// Runs the waiting tasks until none can move on.
#[allow(dead_code)]
fn js_run_tasks() {
    let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
    loop {
        let before = JS_TICK.with(|t| t.get());
        let mut i = 0;
        while let Some(slot) = JS_TASKS.with(|t| t.borrow_mut().get_mut(i).map(Option::take)) {
            if let Some(mut task) = slot
                && task.as_mut().poll(&mut cx).is_pending()
            {
                JS_TASKS.with(|t| t.borrow_mut()[i] = Some(task));
            }
            i += 1;
        }
        JS_TASKS.with(|t| t.borrow_mut().retain(Option::is_some));
        if JS_TICK.with(|t| t.get()) == before {
            break;
        }
    }
}

/// A rejection nothing handled ends the program, like in Node.
#[allow(dead_code)]
fn js_report_unhandled() {
    if let Some((_, message)) = JS_UNHANDLED.with(|u| u.borrow().first().cloned()) {
        eprintln!("Uncaught {message}");
        std::process::exit(1);
    }
}


/// The native side of Node's `fs` and `os` modules.
#[allow(dead_code)]
fn js_fs_fail(op: &str, path: &str, e: std::io::Error) -> ! {
    let code = match e.kind() {
        std::io::ErrorKind::NotFound => "ENOENT: no such file or directory",
        std::io::ErrorKind::PermissionDenied => "EACCES: permission denied",
        std::io::ErrorKind::AlreadyExists => "EEXIST: file already exists",
        _ => "EIO: i/o error",
    };
    panic!("Error: {code}, {op} '{path}'")
}

#[allow(dead_code)]
fn js_node_fs_read(path: &str) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| js_fs_fail("open", path, e))
}

#[allow(dead_code)]
fn js_node_fs_write(path: &str, data: &str) {
    std::fs::write(path, data).unwrap_or_else(|e| js_fs_fail("open", path, e))
}

#[allow(dead_code)]
fn js_node_fs_append(path: &str, data: &str) {
    use std::io::Write;
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut f| f.write_all(data.as_bytes()))
        .unwrap_or_else(|e| js_fs_fail("open", path, e))
}

#[allow(dead_code)]
fn js_node_fs_exists(path: &str) -> bool {
    std::path::Path::new(path).exists()
}

#[allow(dead_code)]
fn js_node_fs_is_dir(path: &str) -> bool {
    std::path::Path::new(path).is_dir()
}

#[allow(dead_code)]
fn js_node_fs_size(path: &str) -> i64 {
    std::fs::metadata(path).map_or(0, |m| m.len() as i64)
}

/// The names in a directory, sorted like Node gives them.
#[allow(dead_code)]
fn js_node_fs_readdir(path: &str) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(path)
        .unwrap_or_else(|e| js_fs_fail("scandir", path, e))
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[allow(dead_code)]
fn js_node_fs_mkdir(path: &str, recursive: bool) {
    let r = if recursive {
        std::fs::create_dir_all(path)
    } else {
        std::fs::create_dir(path)
    };
    r.unwrap_or_else(|e| js_fs_fail("mkdir", path, e))
}

#[allow(dead_code)]
fn js_node_fs_remove(path: &str, recursive: bool) {
    let p = std::path::Path::new(path);
    let r = if p.is_dir() {
        if recursive {
            std::fs::remove_dir_all(p)
        } else {
            std::fs::remove_dir(p)
        }
    } else {
        std::fs::remove_file(p)
    };
    r.unwrap_or_else(|e| js_fs_fail("rm", path, e))
}

#[allow(dead_code)]
fn js_node_fs_rename(from: &str, to: &str) {
    std::fs::rename(from, to).unwrap_or_else(|e| js_fs_fail("rename", from, e))
}

#[allow(dead_code)]
fn js_node_fs_copy(from: &str, to: &str) {
    std::fs::copy(from, to)
        .map(|_| ())
        .unwrap_or_else(|e| js_fs_fail("copyfile", from, e))
}

/// `os.platform()`, `os.homedir()`, `process.cwd()`…
#[allow(dead_code)]
fn js_node_os_info(key: &str) -> String {
    let env = |k: &str| std::env::var(k).unwrap_or_default();
    match key {
        "platform" => match std::env::consts::OS {
            "macos" => "darwin".to_string(),
            "windows" => "win32".to_string(),
            other => other.to_string(),
        },
        "arch" => match std::env::consts::ARCH {
            "x86_64" => "x64".to_string(),
            "aarch64" => "arm64".to_string(),
            "x86" => "ia32".to_string(),
            other => other.to_string(),
        },
        "type" => match std::env::consts::OS {
            "linux" => "Linux".to_string(),
            "macos" => "Darwin".to_string(),
            "windows" => "Windows_NT".to_string(),
            other => other.to_string(),
        },
        "homedir" => env("HOME"),
        "tmpdir" => std::env::temp_dir()
            .to_string_lossy()
            .trim_end_matches('/')
            .to_string(),
        "hostname" => std::fs::read_to_string("/etc/hostname")
            .map(|h| h.trim().to_string())
            .unwrap_or_else(|_| env("HOSTNAME")),
        "cwd" => std::env::current_dir()
            .map(|d| d.to_string_lossy().into_owned())
            .unwrap_or_default(),
        _ => String::new(),
    }
}

#[allow(dead_code)]
fn js_node_os_cpus() -> i64 {
    std::thread::available_parallelism().map_or(1, |n| n.get() as i64)
}

