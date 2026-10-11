#[derive(Debug, Clone, PartialEq)]
struct Person {
    pub name: String,
    pub age: i64,
}

impl Person {
    pub fn new(name: String, age: i64) -> Self {
        Self { name, age }
    }

    pub fn greet(&self) -> String {
        format!("Hi {}", self.name)
    }

    pub fn birthday(&mut self) {
        self.age += 1;
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Student {
    pub name: String,
    pub age: i64,
    pub school: String,
}

impl Student {
    pub fn new(name: &str, age: i64, school: String) -> Self {
        let base = Person::new(name.to_string(), age);
        Self { name: base.name, age: base.age, school }
    }

    pub fn describe(&self) -> String {
        format!("{} from {}", self.greet(), self.school)
    }

    pub fn greet(&self) -> String {
        format!("Hi {}", self.name)
    }

    pub fn birthday(&mut self) {
        self.age += 1;
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Point {
    pub x: i64,
    pub y: i64,
}

impl Point {
    pub fn new(x: i64, y: i64) -> Self {
        Self { x, y }
    }

    pub fn norm(&self) -> f64 {
        ((self.x * self.x + self.y * self.y) as f64).sqrt()
    }
}

fn sum(arguments: &[i64]) -> i64 {
    let mut total = 0;
    for i in 0..arguments.len() {
        total += arguments[i];
    }
    total
}

fn first(a: i64, b: Option<i64>, __more: &[i64]) -> i64 {
    let arguments = [vec![a], if let Some(b) = b { vec![b] } else { Vec::new() }, __more.to_vec()].concat();
    arguments.len() as i64 + a
}

fn main() {
    let mut p = Person::new("Ann".to_string(), 30);
    p.birthday();
    let s = Student::new("Bob", 20, "MIT".to_string());
    println!("{} {} {} {}", p.greet(), p.age, s.describe(), true);
    println!("{} {} {} {}", sum(&[1, 2, 3]), sum(&[]), first(1, Some(2), &[]), first(5, None, &[]));
    println!("{}", js_number_text(Point::new(3, 4).norm()));
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

