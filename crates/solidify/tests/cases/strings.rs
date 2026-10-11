const S: &str = "  Hello, World  ";
const N: f64 = 3.14159;

fn main() {
    let t = S.trim().to_string();
    println!("{} {} {}", t.chars().count(), t.to_uppercase(), t.to_lowercase());
    println!("{} {} {} {} {}", js_slice(&t, 0, Some(5)), js_slice(&t, -5, None), js_substring(&t, 7, None), js_char_at(&t, 1), js_char_at(&t, 4));
    println!("{} {} {} {} {}", js_index_of(&t, "World"), js_index_of(&t, "x"), t.contains("lo,"), t.starts_with("He"), t.ends_with("!"));
    println!("{} {} {}", t.split(", ").map(String::from).collect::<Vec<String>>().join(" & "), "a-b-c".split("-").map(String::from).collect::<Vec<String>>().len(), "abc".chars().map(String::from).collect::<Vec<String>>().join("|"));
    println!("{} {} {} {} {}", "ab".repeat(3), js_pad_start("x", 3, "0"), js_pad_end("5", 3, "."), "a.b.c".replacen(".", "/", 1), "a.b.c".replace(".", "/"));
    let mut acc = "".to_string();
    for ch in "abc".chars() {
        acc.push_str(&ch.to_string().to_uppercase());
        if ch == 'b' {
            acc.push_str("!");
        }
    }
    println!("{acc} {}", acc.chars().count());
    println!("{} {} {} {} {}", js_to_fixed(N, 2), 10.to_string(), format!("{}!", 42.to_string()), format!("7{}", 7), format!("{}7", 7));
    println!("{} {} {} {}", js_parse_int("42px").as_ref().map_or("NaN".to_string(), |v| v.to_string()), js_number_text(js_parse_float("2.5kg")), js_number_text(js_number("12") + 1.0), js_number_text(js_number("x")));
    let mut words = vec!["b".to_string(), "a".to_string(), "c".to_string()];
    words.sort();
    println!("{} {} {}", words.join(""), "b" < "c", "Zed" > "apple");
}

/// `s.slice(start, end)`: character positions, negative ones counted from the end.
fn js_slice(s: &str, start: i64, end: Option<i64>) -> String {
    let len = s.chars().count() as i64;
    let clamp = |i: i64| if i < 0 { (len + i).max(0) } else { i.min(len) };
    let (a, b) = (clamp(start), clamp(end.unwrap_or(len)));
    if b <= a {
        return String::new();
    }
    s.chars().skip(a as usize).take((b - a) as usize).collect()
}

/// `s.substring(start, end)`: character positions, swapped when out of order.
fn js_substring(s: &str, start: i64, end: Option<i64>) -> String {
    let len = s.chars().count() as i64;
    let clamp = |i: i64| i.clamp(0, len);
    let (mut a, mut b) = (clamp(start), clamp(end.unwrap_or(len)));
    if a > b {
        std::mem::swap(&mut a, &mut b);
    }
    s.chars().skip(a as usize).take((b - a) as usize).collect()
}

/// `s.charAt(i)` / `s[i]`: one character, or an empty string.
fn js_char_at(s: &str, i: i64) -> String {
    if i < 0 {
        return String::new();
    }
    s.chars().nth(i as usize).map(String::from).unwrap_or_default()
}

/// `s.indexOf(needle)`: the character position, or -1.
fn js_index_of(s: &str, needle: &str) -> i64 {
    match s.find(needle) {
        Some(byte) => s[..byte].chars().count() as i64,
        None => -1,
    }
}

/// `s.indexOf(needle, from)`.
#[allow(dead_code)]
fn js_index_of_from(s: &str, needle: &str, from: i64) -> i64 {
    let from = from.max(0) as usize;
    let Some((start, _)) = s.char_indices().nth(from).or((from == s.chars().count()).then_some((s.len(), ' '))) else {
        return if needle.is_empty() { s.chars().count() as i64 } else { -1 };
    };
    match s[start..].find(needle) {
        Some(byte) => s[..start + byte].chars().count() as i64,
        None => -1,
    }
}

/// `s.padStart(width, pad)`.
fn js_pad_start(s: &str, width: i64, pad: &str) -> String {
    let len = s.chars().count() as i64;
    if width <= len || pad.is_empty() {
        return s.to_string();
    }
    let fill: String = pad.chars().cycle().take((width - len) as usize).collect();
    format!("{fill}{s}")
}

/// `s.padEnd(width, pad)`.
fn js_pad_end(s: &str, width: i64, pad: &str) -> String {
    let len = s.chars().count() as i64;
    if width <= len || pad.is_empty() {
        return s.to_string();
    }
    let fill: String = pad.chars().cycle().take((width - len) as usize).collect();
    format!("{s}{fill}")
}

/// `parseInt(s)`: the leading integer, or `None` for `NaN`.
fn js_parse_int(s: &str) -> Option<i64> {
    let s = s.trim_start();
    let end = s
        .char_indices()
        .take_while(|&(i, c)| c.is_ascii_digit() || (i == 0 && (c == '-' || c == '+')))
        .map(|(i, c)| i + c.len_utf8())
        .last()
        .unwrap_or(0);
    s[..end].parse::<i64>().ok()
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


/// `Number(s)`: the whole string as a number, 0 when blank, NaN otherwise.
fn js_number(s: &str) -> f64 {
    let s = s.trim();
    if s.is_empty() {
        return 0.0;
    }
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        return i64::from_str_radix(hex, 16).map_or(f64::NAN, |n| n as f64);
    }
    match s {
        "Infinity" | "+Infinity" => f64::INFINITY,
        "-Infinity" => f64::NEG_INFINITY,
        _ => s.parse::<f64>().unwrap_or(f64::NAN),
    }
}
