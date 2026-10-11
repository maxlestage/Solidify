fn collatz(mut n: f64) -> i64 {
    let mut steps = 0;
    while n != 1.0 {
        n = if n % 2.0 == 0.0 { n / 2.0 } else { 3.0 * n + 1.0 };
        steps += 1;
    }
    steps
}

fn primes(limit: i64) -> Vec<i64> {
    let mut out: Vec<i64> = Vec::new();
    'outer: for i in 2..=limit {
        let mut j = 2;
        while j * j <= i {
            if i % j == 0 {
                continue 'outer;
            }
            j += 1;
        }
        out.push(i);
    }
    out
}

fn main() {
    let mut i = 0;
    loop {
        i += 3;
        if !(i < 10) {
            break;
        }
    }
    println!("{i} {} {}", collatz(27.0), primes(30).iter().map(|x| x.to_string()).collect::<Vec<_>>().join(" "));
    let mut k = 10;
    while k > 0 {
        'next: {
            if k == 4 {
                break 'next;
            }
            println!("{k}");
        }
        k -= 3;
    }
    let grid = vec![vec![1, 2], vec![3, 4]];
    let mut sum = 0;
    for r in 0..grid.len() {
        for c in 0..grid[r].len() {
            sum += grid[r][c];
        }
    }
    println!("{sum} {} {}", grid[1][0], grid.len());
    let mut count = 0;
    loop {
        count += 1;
        if count > 4 {
            break;
        }
    }
    println!("{count} {} {} {} {} {}", 7 % 3, -7 % 3, js_number_text(1024.0), 7_i64.div_euclid(2), js_number_text(7.0 / 2.0));
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

