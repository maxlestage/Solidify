fn parse_positive(s: &str) -> Result<f64, String> {
    let n = js_parse_float(s);
    if n.is_nan() {
        return Err(format!("not a number: {s}"));
    }
    if n < 0.0 {
        return Err("negative".to_string());
    }
    Ok(n)
}

fn sum(items: &[String]) -> Result<f64, String> {
    let mut t: f64 = 0.0;
    for item in items {
        t += parse_positive(item.as_str())?;
    }
    Ok(t)
}

fn main() {
    let result_: Result<(), String> = (|| {
        println!("{}", sum(&vec!["1".to_string(), "2.5".to_string(), "3".to_string()])?);
        println!("{}", sum(&vec!["1".to_string(), "x".to_string()])?);
        Ok(())
    })();
    if let Err(e) = result_ {
        println!("failed: {e}");
    }
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
