/// Plain JavaScript: no annotations at all.
fn greet<T: std::fmt::Display>(name: T) -> String {
    format!("Hello, {name}!")
}

fn average(values: &[i64]) -> f64 {
    let mut total = 0;
    for &v in values {
        total += v;
    }
    total as f64 / values.len() as i64 as f64
}

fn largest<T: Clone + PartialOrd>(items: &[T]) -> T {
    let mut best = items[0].clone();
    for item in items {
        if item > &best {
            best = item.clone();
        }
    }
    best
}

fn main() {
    let scores = vec![12, 7, 19, 3];
    println!("{} {} {} {}", greet("Ada".to_string()), average(&scores), largest(&scores), largest(&vec!["pear".to_string(), "zebra".to_string(), "apple".to_string()]));
    let mut count = 0;
    while count < 3 {
        count += 1;
    }
    let labels = scores.iter().map(|&s| { if s > 10 { "high".to_string() } else { "low".to_string() } }).collect::<Vec<_>>();
    println!("{count} {} {}", labels.join(","), scores.iter().filter(|&&s| s % 2 == 1).cloned().collect::<Vec<_>>().len());
}
