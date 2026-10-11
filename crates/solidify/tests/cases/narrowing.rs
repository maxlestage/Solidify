#[derive(Debug, Clone, PartialEq)]
struct Tree {
    pub value: f64,
    pub left: Option<Box<Tree>>,
    pub right: Option<Box<Tree>>,
}

fn leaf(value: f64) -> Tree {
    Tree { value, left: None, right: None }
}

fn insert(mut root: Option<Box<Tree>>, value: f64) -> Tree {
    let Some(root) = root.as_deref_mut() else {
        return leaf(value);
    };
    if value < root.value {
        root.left = Some(Box::new(insert(root.left.clone(), value)));
    } else {
        root.right = Some(Box::new(insert(root.right.clone(), value)));
    }
    root.clone()
}

fn sum(root: Option<&Tree>) -> f64 {
    let Some(root) = root else {
        return 0.0;
    };
    root.value + sum(root.left.as_deref()) + sum(root.right.as_deref())
}

fn depth(root: Option<&Tree>) -> i64 {
    let Some(root) = root else {
        return 0;
    };
    1 + depth(root.left.as_deref()).max(depth(root.right.as_deref()))
}

fn find(root: Option<&Tree>, value: f64) -> Option<Box<Tree>> {
    let mut current = root.map(|v| Box::new(v.clone()));
    while current.is_some() && current.as_ref().unwrap().value != value {
        current = if value < current.as_ref().unwrap().value { current.as_ref().unwrap().left.clone() } else { current.as_ref().unwrap().right.clone() };
    }
    current
}

fn main() {
    let mut root: Option<Box<Tree>> = None;
    for v in vec![5, 3, 8, 1, 4, 9] {
        root = Some(Box::new(insert(root.clone(), v as f64)));
    }
    println!("{} {}", js_number_text(sum(root.as_deref())), depth(root.as_deref()));
    let found = find(root.as_deref(), 4.0);
    println!("{} {}", found.map(|v| v.value).as_ref().map_or("undefined".to_string(), |v| js_number_text(*v)), js_number_text(find(root.as_deref(), 7.0).as_ref().map(|v| v.value).unwrap_or(-1.0)));
    let name: Option<String> = None;
    println!("{} {}", name.clone().unwrap_or("anonymous".to_string()), name.map(|v| v.chars().count() as i64).unwrap_or(0));
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

