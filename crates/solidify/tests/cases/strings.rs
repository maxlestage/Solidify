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
    println!("{} {} {} {} {}", format!("{:.*}", 2 as usize, N), 10.to_string(), format!("{}!", 42.to_string()), format!("7{}", 7), format!("{}7", 7));
    println!("{} {} {} {}", js_parse_int("42px"), js_parse_float("2.5kg"), js_number("12") + 1.0, js_number("x"));
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

/// `parseInt(s)`: the leading integer, or NaN.
fn js_parse_int(s: &str) -> f64 {
    let s = s.trim_start();
    let end = s
        .char_indices()
        .take_while(|&(i, c)| c.is_ascii_digit() || (i == 0 && (c == '-' || c == '+')))
        .map(|(i, c)| i + c.len_utf8())
        .last()
        .unwrap_or(0);
    s[..end].parse::<f64>().unwrap_or(f64::NAN)
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
