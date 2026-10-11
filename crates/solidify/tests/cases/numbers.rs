/// Numbers as JavaScript has them: no overflow, 32-bit bitwise operators, exact formatting.
fn fact(n: f64) -> f64 {
    if n <= 1.0 { 1.0 } else { n * fact(n - 1.0) }
}

fn fib(n: i64) -> f64 {
    let mut a: f64 = 0.0;
    let mut b: f64 = 1.0;
    for _ in 0..n {
        (a, b) = (b, a + b);
    }
    a
}

fn main() {
    let mut p: f64 = 1.0;
    for _ in 0..70 {
        p *= 3.0;
    }
    let mut h = 0;
    for c in "hello world".chars() {
        h = (((h * 31 + js_char_code_at(&c.to_string(), 0)) as i32) | 0) as i64;
    }
    let mut sum = 0;
    for x in vec![1, 2, 3] {
        sum += x;
    }
    println!("{} {} {} {h} {sum}", js_number_text(fact(25.0)), js_number_text(fib(100)), js_number_text(p));
    println!("{} {} {} {} {} {} {}", -2147483648, 1, -4, 4294967289_i64, -6, js_number_text(2.0_f64.powf(64.0)), -1);
    println!("{} {}", js_number_text(9007199254740991.0 + 2.0), 9007199254740992_i64);
    let xs: Vec<f64> = vec![1.5, 2.0, 1e21, 1e-7, -0.0];
    println!("{} {} {} {} {} {} {}", xs.iter().map(|x| js_number_text(*x)).collect::<Vec<_>>().join("-"), js_number_text(1.0 / 0.0), js_number_text(0.1 + 0.2), js_number_text(1e21), js_number_text(1.0 / 3.0), js_number_text(-1 as f64 / 0.0), js_number_text(0.0 / 0.0));
    let big = 123456789.0 * 1000000000000.0;
    println!("{} {} {} {} {}", js_number_text(big), js_number_text(2.0_f64.powf(53.0) + 1.0), 5 % 3, -5 % 3, js_number_text(7.5 % 2.0));
    let mut acc: f64 = 1.0;
    let mut i: f64 = 1.0;
    while i <= 30.0 {
        acc = acc * i;
        i += 1.0;
    }
    println!("{} {} {}", js_number_text(acc), js_number_text(acc), acc > 1e30);
    let vals: Vec<f64> = vec![2.5, -2.5, 1.005, 1.25, 0.000001234, 123.456, 1e21, 0.0, -0.0001, 99.995, 1234.5678];
    for &v in &vals {
        println!("{} {} {} {} {}", js_to_fixed(v, 0), js_to_fixed(v, 2), js_to_precision(v, 3), js_to_precision(v, 1), js_number_text(v));
    }
    println!("{} {} {} {}", js_to_fixed(25 as f64, 1), js_to_precision(7 as f64, 2), js_to_precision(123456 as f64, 2), js_to_precision(0.00000123, 2));
}

/// `s.charCodeAt(i)`: the code of a character, or -1 (NaN in JavaScript).
fn js_char_code_at(s: &str, i: i64) -> i64 {
    if i < 0 {
        return -1;
    }
    s.chars().nth(i as usize).map_or(-1, |c| c as i64)
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


/// The exact decimal digits of a finite double (no sign) and the position of the point:
/// `digits[..point]` is the integer part (`point` may be negative or past the end).
#[allow(dead_code)]
fn js_exact_digits(x: f64) -> (Vec<u8>, i64) {
    let text = format!("{:.1100}", x.abs());
    let (int, frac) = text.split_once('.').unwrap_or((&text, ""));
    let mut digits: Vec<u8> = int.bytes().chain(frac.bytes()).map(|b| b - b'0').collect();
    let mut point = int.len() as i64;
    while digits.len() > 1 && digits[0] == 0 {
        digits.remove(0);
        point -= 1;
    }
    while digits.len() > 1 && digits[digits.len() - 1] == 0 {
        digits.pop();
    }
    (digits, point)
}

/// Rounds `digits` to `keep` digits, a tie going up as JavaScript does; returns the digits
/// and whether a carry added one in front.
#[allow(dead_code)]
fn js_round_digits(digits: &[u8], keep: usize) -> (Vec<u8>, bool) {
    let mut out: Vec<u8> = digits.iter().take(keep).copied().collect();
    while out.len() < keep {
        out.push(0);
    }
    if digits.get(keep).is_some_and(|d| *d >= 5) {
        let mut i = out.len();
        loop {
            if i == 0 {
                out.insert(0, 1);
                return (out, true);
            }
            i -= 1;
            if out[i] == 9 {
                out[i] = 0;
            } else {
                out[i] += 1;
                break;
            }
        }
    }
    (out, false)
}

/// `x.toFixed(digits)`, as JavaScript writes it.
#[allow(dead_code)]
fn js_to_fixed(x: f64, digits: i64) -> String {
    if !x.is_finite() || x.abs() >= 1e21 {
        return js_number_text(x);
    }
    let f = digits.clamp(0, 100) as usize;
    let (all, point) = js_exact_digits(x);
    // The digits from the units on: zeros in front when the number is below one.
    let mut padded: Vec<u8> = Vec::new();
    if point <= 0 {
        padded.push(0);
        padded.extend(std::iter::repeat_n(0, (-point) as usize));
        padded.extend(&all);
    } else {
        padded.extend(&all);
    }
    let int_len = if point <= 0 { 1 } else { point as usize };
    let (rounded, carry) = js_round_digits(&padded, int_len + f);
    let int_len = int_len + carry as usize;
    let int: String = rounded[..int_len].iter().map(|d| (d + b'0') as char).collect();
    let frac: String = rounded[int_len..].iter().map(|d| (d + b'0') as char).collect();
    let body = if f == 0 { int } else { format!("{int}.{frac}") };
    if x < 0.0 { format!("-{body}") } else { body }
}

/// `x.toPrecision(p)`, as JavaScript writes it.
#[allow(dead_code)]
fn js_to_precision(x: f64, p: i64) -> String {
    if !x.is_finite() {
        return js_number_text(x);
    }
    let p = p.clamp(1, 100) as usize;
    let sign = if x < 0.0 { "-" } else { "" };
    if x == 0.0 {
        let zeros = "0".repeat(p - 1);
        return if p == 1 { "0".into() } else { format!("0.{zeros}") };
    }
    let (all, point) = js_exact_digits(x);
    let (rounded, carry) = js_round_digits(&all, p);
    let digits: String = rounded[..p].iter().map(|d| (d + b'0') as char).collect();
    let e = point - 1 + carry as i64;
    if e < -6 || e >= p as i64 {
        let (first, rest) = digits.split_at(1);
        let mantissa = if rest.is_empty() {
            first.to_string()
        } else {
            format!("{first}.{rest}")
        };
        let es = if e < 0 { "-" } else { "+" };
        return format!("{sign}{mantissa}e{es}{}", e.abs());
    }
    if e >= 0 {
        let (int, frac) = digits.split_at(e as usize + 1);
        if frac.is_empty() {
            format!("{sign}{int}")
        } else {
            format!("{sign}{int}.{frac}")
        }
    } else {
        format!("{sign}0.{}{digits}", "0".repeat((-e - 1) as usize))
    }
}

