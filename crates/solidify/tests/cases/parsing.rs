const RAW: &str = "12";

/// parseInt gives an optional integer: NaN is None, and isNaN tests it.
fn to_int(s: &str) -> i64 {
    let n = js_parse_int(s);
    let Some(n) = n else {
        return -1;
    };
    n * 2
}

fn first_digits(s: &str) -> String {
    let n = js_parse_int_radix(s, 10);
    if let Some(n) = n {
        return format!("number {n}");
    }
    "no number".to_string()
}

fn hex(s: &str) -> i64 {
    js_parse_int_radix(s, 16).unwrap_or(0)
}

/// `any` and `unknown` are inferred from their uses.
fn twice(x: i64) -> i64 {
    x + x
}

fn shout(s: &str) -> String {
    format!("{}!", s.to_uppercase())
}

fn label<T: std::fmt::Display>(value: T, times: i64) -> String {
    let mut out = "".to_string();
    for _ in 0..times {
        out.push_str(&format!("{value}"));
    }
    out
}

fn main() {
    let parsed = js_parse_int(RAW).unwrap_or(0);
    println!("{} {} {}", to_int("21"), to_int("x"), to_int("  7 apples"));
    println!("{} {} {} {} {}", first_digits("abc"), first_digits("42abc"), hex("ff"), hex("0x1A"), hex("zz"));
    println!("{} {} {} {} {}", twice(4), shout("ab"), label("na", 3), label(1.5, 2), parsed + 1);
    println!("{} {} {} {} {}", js_parse_int("3.9").as_ref().map_or("NaN".to_string(), |v| v.to_string()), 3.9_f64.trunc() as i64, (-2.5_f64).trunc() as i64, js_parse_int("8").as_ref().map_or("NaN".to_string(), |v| v.to_string()), js_parse_int("q").is_none());
    let maybe = js_parse_int("1e3");
    println!("{} {}", if let Some(maybe) = maybe { "finite".to_string() } else { "nan".to_string() }, js_parse_int("nope").is_none());
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

/// `parseInt(s, radix)`: the leading digits in the given base, or `None` for `NaN`.
fn js_parse_int_radix(s: &str, radix: i64) -> Option<i64> {
    if !(2..=36).contains(&radix) {
        return None;
    }
    let s = s.trim_start();
    let (negative, digits) = match s.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, s.strip_prefix('+').unwrap_or(s)),
    };
    let digits = if radix == 16 {
        digits
            .strip_prefix("0x")
            .or_else(|| digits.strip_prefix("0X"))
            .unwrap_or(digits)
    } else {
        digits
    };
    let end = digits
        .char_indices()
        .take_while(|&(_, c)| c.is_digit(radix as u32))
        .map(|(i, c)| i + c.len_utf8())
        .last()
        .unwrap_or(0);
    let n = i64::from_str_radix(&digits[..end], radix as u32).ok()?;
    Some(if negative { -n } else { n })
}
