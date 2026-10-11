const N: i64 = 7;

fn main() {
    let values: Vec<f64> = vec![3.5, -2.25, 10.0, 0.5];
    let mut acc: f64 = 0.0;
    for &v in &values {
        acc += v.abs();
    }
    println!("{} {} {} {} {}", js_number_text(acc), acc.floor() as i64, 2.1_f64.ceil() as i64, 2.5_f64.round() as i64, (-2.7_f64).trunc() as i64);
    println!("{} {} {} {} {}", 1_i64.max(7).max(3), 4_i64.min(2), js_number_text(16.0_f64.sqrt()), js_number_text(2.0_f64.powf(8.0)), 2.0_f64.powf(0.5) > 1.41);
    println!("{} {} {} {}", js_number_text(values.iter().copied().fold(f64::NEG_INFINITY, f64::max)), js_number_text(values.iter().copied().fold(f64::INFINITY, f64::min)), (-3_i64).signum(), js_number_text(3.0_f64.hypot(4.0_f64)));
    println!("{} {} {} {} {} {} {} {} {} {}", js_number_text(N as f64 / 2.0), N.div_euclid(2), N % 4, js_number_text((N * 3) as f64 / 3.0), ((N as i32) & 3) as i64, ((N as i32) | 8) as i64, ((N as i32) ^ 1) as i64, (N as i32).wrapping_shl(2 as u32) as i64, (N as i32).wrapping_shr(1 as u32) as i64, (!(N as i32)) as i64);
    println!("{} {} {} {} {}", js_to_fixed(std::f64::consts::PI, 3), true, 4.5_f64.fract() == 0.0, js_parse_float("x").is_nan(), js_num_to_string(255, 16));
    println!("{} {} {} {} {}", 9007199254740991_i64, js_number_text(1000.0), 0.1 + 0.2 == 0.3, 5 > 3 && 2 < 1, !(1 > 2));
}

/// `parseFloat(s)`: the leading number, or NaN.
fn js_parse_float(s: &str) -> f64 {
    let s = s.trim_start();
    let mut end = 0;
    let (mut dot, mut exp, mut digits) = (false, false, false);
    for (i, c) in s.char_indices() {
        let ok = match c {
            '0'..='9' => {
                digits = true;
                true
            }
            '-' | '+' => i == 0 || matches!(s[..i].chars().last(), Some('e' | 'E')),
            '.' => !dot && !exp && std::mem::replace(&mut dot, true) == false,
            'e' | 'E' => digits && !exp && std::mem::replace(&mut exp, true) == false,
            _ => false,
        };
        if !ok {
            break;
        }
        end = i + c.len_utf8();
    }
    let mut text = &s[..end];
    while !text.is_empty() && text.parse::<f64>().is_err() {
        text = &text[..text.len() - 1];
    }
    text.parse::<f64>().unwrap_or(f64::NAN)
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


/// `n.toString(radix)`.
fn js_num_to_string(mut n: i64, radix: i64) -> String {
    if !(2..=36).contains(&radix) {
        return n.to_string();
    }
    if n == 0 {
        return "0".into();
    }
    let negative = n < 0;
    n = n.abs();
    let mut digits = Vec::new();
    while n > 0 {
        digits.push(std::char::from_digit((n % radix) as u32, radix as u32).unwrap_or('?'));
        n /= radix;
    }
    if negative {
        digits.push('-');
    }
    digits.iter().rev().collect()
}
