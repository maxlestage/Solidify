#[derive(Debug, Clone, PartialEq)]
struct Shape {
    pub name: String,
}

impl Shape {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Circle {
    pub name: String,
    pub radius: f64,
}

impl Circle {
    pub fn new(radius: f64) -> Self {
        let base = Shape::new("circle".to_string());
        Self { name: base.name, radius }
    }

    pub fn area(&self) -> f64 {
        std::f64::consts::PI * self.radius * self.radius
    }

    pub fn describe(&self) -> String {
        format!("{} with area {}", self.name, format!("{:.*}", 2 as usize, self.area()))
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Rect {
    pub name: String,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub fn new(width: f64, height: f64) -> Self {
        let base = Shape::new("rect".to_string());
        Self { name: base.name, width, height }
    }

    pub fn area(&self) -> f64 {
        self.width * self.height
    }

    pub fn describe(&self) -> String {
        format!("{} ({}x{})", self.shape_describe(), self.width, self.height)
    }

    pub fn shape_describe(&self) -> String {
        format!("{} with area {}", self.name, format!("{:.*}", 2 as usize, self.area()))
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Square {
    pub name: String,
    pub width: f64,
    pub height: f64,
}

impl Square {
    pub fn new(side: f64) -> Self {
        let base = Rect::new(side, side);
        Self { name: "square".to_string(), width: base.width, height: base.height }
    }

    pub fn area(&self) -> f64 {
        self.width * self.height
    }

    pub fn describe(&self) -> String {
        format!("{} ({}x{})", self.shape_describe(), self.width, self.height)
    }

    pub fn shape_describe(&self) -> String {
        format!("{} with area {}", self.name, format!("{:.*}", 2 as usize, self.area()))
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Animal {
    pub name: String,
    pub sounds: i64,
}

impl Animal {
    pub fn new(name: String) -> Self {
        Self { name, sounds: 0 }
    }

    pub fn speak(&mut self) -> String {
        self.sounds += 1;
        format!("{} makes a sound", self.name)
    }

    pub fn kingdom() -> String {
        "animalia".to_string()
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Dog {
    pub name: String,
    pub sounds: i64,
    pub tricks: Vec<String>,
}

impl Dog {
    pub fn new(name: &str, tricks: Vec<String>) -> Self {
        let base = Animal::new(name.to_string());
        Self { name: base.name, sounds: base.sounds, tricks }
    }

    pub fn speak(&mut self) -> String {
        self.sounds += 1;
        format!("{} barks", self.name)
    }

    pub fn learn(&mut self, trick: String) {
        self.tricks.push(trick);
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Puppy {
    pub name: String,
    pub sounds: i64,
    pub tricks: Vec<String>,
}

impl Puppy {
    pub fn new(name: &str, tricks: Vec<String>) -> Self {
        let base = Dog::new(name, tricks);
        Self { name: base.name, sounds: base.sounds, tricks: base.tricks }
    }

    pub fn speak(&self) -> String {
        format!("{} yaps", self.name)
    }

    pub fn learn(&mut self, trick: String) {
        self.tricks.push(trick);
    }
}

fn loudest(animals: &[Box<dyn AnimalLike>]) -> String {
    let mut best = animals[0].clone();
    for a in animals {
        if *a.sounds() > *best.sounds() {
            best = a.clone();
        }
    }
    best.name().clone()
}

fn main() {
    let shapes = vec![Box::new(Circle::new(1.0)) as Box<dyn ShapeLike>, Box::new(Rect::new(2.0, 3.0)) as Box<dyn ShapeLike>, Box::new(Square::new(2.0)) as Box<dyn ShapeLike>];
    for s in &shapes {
        println!("{}", s.describe());
    }
    let total = shapes.iter().fold(0.0, |sum, s| sum + s.area());
    println!("{} {}", format!("{:.*}", 3 as usize, total), shapes.len());
    let mut rex = Box::new(Dog::new("Rex", Vec::new())) as Box<dyn DogLike>;
    rex.learn("sit".to_string());
    rex.learn("roll".to_string());
    println!("{} {} {} {}", rex.speak(), rex.tricks().len(), rex.tricks().join("+"), Animal::kingdom());
    let mut pets = vec![Box::new(Animal::new("Generic".to_string())) as Box<dyn AnimalLike>, rex as Box<dyn AnimalLike>, Box::new(Puppy::new("Bit", vec!["beg".to_string()])) as Box<dyn AnimalLike>];
    for p in &mut pets {
        println!("{}", p.speak());
        if let Some(p) = p.as_dog_mut() {
            p.learn("fetch".to_string());
            println!("{} knows {} tricks", p.name(), p.tricks().len());
        }
        if let Some(p) = p.as_puppy() {
            println!("a puppy!");
        }
    }
    *pets[0].name_mut() = "Gen".to_string();
    println!("{} {} {}", loudest(&pets), pets[0].name(), pets.iter().map(|p| *p.sounds()).collect::<Vec<_>>().iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","));
    println!("{} {}", js_inspect(&pets[1].js_value()), js_inspect(&shapes[0].js_value()));
}

/// `Animal` and its subclasses.
trait AnimalLike: std::fmt::Debug + JsInspect {
    fn name(&self) -> &String;
    fn name_mut(&mut self) -> &mut String;
    fn sounds(&self) -> &i64;
    fn speak(&mut self) -> String;
    fn as_dog(&self) -> Option<&dyn DogLike>;
    fn as_dog_mut(&mut self) -> Option<&mut dyn DogLike>;
    fn as_puppy(&self) -> Option<&Puppy>;
    fn clone_animal(&self) -> Box<dyn AnimalLike>;
}

impl Clone for Box<dyn AnimalLike> {
    fn clone(&self) -> Self {
        self.clone_animal()
    }
}

impl AnimalLike for Animal {
    fn name(&self) -> &String {
        &self.name
    }

    fn name_mut(&mut self) -> &mut String {
        &mut self.name
    }

    fn sounds(&self) -> &i64 {
        &self.sounds
    }

    fn speak(&mut self) -> String {
        Animal::speak(self)
    }

    fn as_dog(&self) -> Option<&dyn DogLike> {
        None
    }

    fn as_dog_mut(&mut self) -> Option<&mut dyn DogLike> {
        None
    }

    fn as_puppy(&self) -> Option<&Puppy> {
        None
    }

    fn clone_animal(&self) -> Box<dyn AnimalLike> {
        Box::new(self.clone())
    }
}

impl AnimalLike for Dog {
    fn name(&self) -> &String {
        &self.name
    }

    fn name_mut(&mut self) -> &mut String {
        &mut self.name
    }

    fn sounds(&self) -> &i64 {
        &self.sounds
    }

    fn speak(&mut self) -> String {
        Dog::speak(self)
    }

    fn as_dog(&self) -> Option<&dyn DogLike> {
        Some(self)
    }

    fn as_dog_mut(&mut self) -> Option<&mut dyn DogLike> {
        Some(self)
    }

    fn as_puppy(&self) -> Option<&Puppy> {
        None
    }

    fn clone_animal(&self) -> Box<dyn AnimalLike> {
        Box::new(self.clone())
    }
}

impl AnimalLike for Puppy {
    fn name(&self) -> &String {
        &self.name
    }

    fn name_mut(&mut self) -> &mut String {
        &mut self.name
    }

    fn sounds(&self) -> &i64 {
        &self.sounds
    }

    fn speak(&mut self) -> String {
        Puppy::speak(self)
    }

    fn as_dog(&self) -> Option<&dyn DogLike> {
        Some(self)
    }

    fn as_dog_mut(&mut self) -> Option<&mut dyn DogLike> {
        Some(self)
    }

    fn as_puppy(&self) -> Option<&Puppy> {
        Some(self)
    }

    fn clone_animal(&self) -> Box<dyn AnimalLike> {
        Box::new(self.clone())
    }
}

/// `Dog` and its subclasses.
trait DogLike: std::fmt::Debug + JsInspect + AnimalLike {
    fn tricks(&self) -> &Vec<String>;
    fn learn(&mut self, trick: String);
    fn clone_dog(&self) -> Box<dyn DogLike>;
}

impl Clone for Box<dyn DogLike> {
    fn clone(&self) -> Self {
        self.clone_dog()
    }
}

impl DogLike for Dog {
    fn tricks(&self) -> &Vec<String> {
        &self.tricks
    }

    fn learn(&mut self, trick: String) {
        Dog::learn(self, trick)
    }

    fn clone_dog(&self) -> Box<dyn DogLike> {
        Box::new(self.clone())
    }
}

impl DogLike for Puppy {
    fn tricks(&self) -> &Vec<String> {
        &self.tricks
    }

    fn learn(&mut self, trick: String) {
        Puppy::learn(self, trick)
    }

    fn clone_dog(&self) -> Box<dyn DogLike> {
        Box::new(self.clone())
    }
}

/// `Shape` and its subclasses.
trait ShapeLike: std::fmt::Debug + JsInspect {
    fn name(&self) -> &String;
    fn area(&self) -> f64;
    fn describe(&self) -> String;
    fn clone_shape(&self) -> Box<dyn ShapeLike>;
}

impl Clone for Box<dyn ShapeLike> {
    fn clone(&self) -> Self {
        self.clone_shape()
    }
}

impl ShapeLike for Circle {
    fn name(&self) -> &String {
        &self.name
    }

    fn area(&self) -> f64 {
        Circle::area(self)
    }

    fn describe(&self) -> String {
        Circle::describe(self)
    }

    fn clone_shape(&self) -> Box<dyn ShapeLike> {
        Box::new(self.clone())
    }
}

impl ShapeLike for Rect {
    fn name(&self) -> &String {
        &self.name
    }

    fn area(&self) -> f64 {
        Rect::area(self)
    }

    fn describe(&self) -> String {
        Rect::describe(self)
    }

    fn clone_shape(&self) -> Box<dyn ShapeLike> {
        Box::new(self.clone())
    }
}

impl ShapeLike for Square {
    fn name(&self) -> &String {
        &self.name
    }

    fn area(&self) -> f64 {
        Square::area(self)
    }

    fn describe(&self) -> String {
        Square::describe(self)
    }

    fn clone_shape(&self) -> Box<dyn ShapeLike> {
        Box::new(self.clone())
    }
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

/// `String(value)`: what a value becomes in a string.
#[allow(dead_code)]
fn js_to_string(v: &JsValue) -> String {
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
        let start = output.len() + ctx.indentation + open.chars().count() + 10;
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


impl JsInspect for Animal {
    fn js_value(&self) -> JsValue {
        JsValue::Object(Some("Animal"), vec![("name".to_string(), self.name.js_value()), ("sounds".to_string(), self.sounds.js_value())])
    }
}

impl JsInspect for Circle {
    fn js_value(&self) -> JsValue {
        JsValue::Object(Some("Circle"), vec![("name".to_string(), self.name.js_value()), ("radius".to_string(), self.radius.js_value())])
    }
}

impl JsInspect for Dog {
    fn js_value(&self) -> JsValue {
        JsValue::Object(Some("Dog"), vec![("name".to_string(), self.name.js_value()), ("sounds".to_string(), self.sounds.js_value()), ("tricks".to_string(), self.tricks.js_value())])
    }
}

impl JsInspect for Puppy {
    fn js_value(&self) -> JsValue {
        JsValue::Object(Some("Puppy"), vec![("name".to_string(), self.name.js_value()), ("sounds".to_string(), self.sounds.js_value()), ("tricks".to_string(), self.tricks.js_value())])
    }
}

impl JsInspect for Rect {
    fn js_value(&self) -> JsValue {
        JsValue::Object(Some("Rect"), vec![("name".to_string(), self.name.js_value()), ("width".to_string(), self.width.js_value()), ("height".to_string(), self.height.js_value())])
    }
}

impl JsInspect for Shape {
    fn js_value(&self) -> JsValue {
        JsValue::Object(Some("Shape"), vec![("name".to_string(), self.name.js_value())])
    }
}

impl JsInspect for Square {
    fn js_value(&self) -> JsValue {
        JsValue::Object(Some("Square"), vec![("name".to_string(), self.name.js_value()), ("width".to_string(), self.width.js_value()), ("height".to_string(), self.height.js_value())])
    }
}
