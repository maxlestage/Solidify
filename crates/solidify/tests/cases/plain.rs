/// Plain JavaScript: no annotations at all.
fn greet<T: Clone + std::fmt::Display>(name: T) -> String {
    format!("Hello, {name}!")
}

fn average(values: &[i64]) -> f64 {
    let mut total = 0;
    for &v in values {
        total += v;
    }
    total as f64 / values.len() as f64
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
    println!("{} {} {} {}", greet("Ada"), js_number_text(average(&scores)), largest(&scores), largest(&["pear".to_string(), "zebra".to_string(), "apple".to_string()]));
    let mut count = 0;
    while count < 3 {
        count += 1;
    }
    let labels = scores.iter().map(|&s| { if s > 10 { "high".to_string() } else { "low".to_string() } }).collect::<Vec<_>>();
    println!("{count} {} {}", labels.join(","), scores.iter().filter(|&&s| s % 2 == 1).cloned().collect::<Vec<_>>().len());
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

