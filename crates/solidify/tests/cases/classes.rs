#[derive(Debug, Clone, PartialEq)]
struct Point {
    pub x: i64,
    pub y: i64,
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
struct Counter {
    pub name: String,
    pub count: i64,
}

impl Counter {
    pub fn new(name: String) -> Self {
        Self { name, count: 0 }
    }

    pub fn increment(&mut self) {
        self.count += 1;
    }

    pub fn label(&self) -> String {
        format!("{}: {}", self.name, self.count)
    }
}

fn describe(p: &Point) -> String {
    if let Some(label) = &p.label {
        return label.clone();
    }
    format!("({}, {})", p.x, p.y)
}

fn main() {
    let mut c = Counter::new("clicks".to_string());
    c.increment();
    c.increment();
    println!("{}", c.label());
    let points = vec![Point { x: 1, y: 2, label: None }, Point { x: 3, y: 4, label: Some("far".to_string()) }];
    for p in &points {
        println!("{}", describe(p));
    }
}
