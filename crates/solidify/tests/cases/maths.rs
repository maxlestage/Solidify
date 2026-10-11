const N: i64 = 7;

fn main() {
    let values: Vec<f64> = vec![3.5, -2.25, 10.0, 0.5];
    let mut acc: f64 = 0.0;
    for &v in &values {
        acc += v.abs();
    }
    println!("{acc} {} {} {} {}", acc.floor() as i64, 2.1_f64.ceil() as i64, 2.5_f64.round() as i64, (-2.7_f64).trunc() as i64);
    println!("{} {} {} {} {}", 1_i64.max(7).max(3), 4_i64.min(2), 16.0_f64.sqrt(), 2_i64.pow(8 as u32), 2.0_f64.powf(0.5 as f64) > 1.41);
    println!("{} {} {} {}", values.iter().copied().fold(f64::NEG_INFINITY, f64::max), values.iter().copied().fold(f64::INFINITY, f64::min), (-3_i64).signum(), 3.0_f64.hypot(4.0_f64));
    println!("{} {} {} {} {} {} {} {} {} {}", N as f64 / 2.0, N.div_euclid(2), N % 4, (N * 3) as f64 / 3.0, N & 3, N | 8, N ^ 1, N << 2, N >> 1, !N);
    println!("{} {} {} {} {}", format!("{:.3}", std::f64::consts::PI), true, 4.5_f64.fract() == 0.0, js_parse_float("x").is_nan(), js_num_to_string(255, 16));
    println!("{} {} {} {} {}", 9007199254740991_i64, 1000.0, 0.1 + 0.2 == 0.3, 5 > 3 && 2 < 1, !(1 > 2));
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
