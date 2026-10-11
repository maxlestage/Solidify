/// Errors of the program's own classes travel as text: `\u{2}` + the class and its
/// ancestors + `\u{2}` + the message + `\u{2}` + the object as JSON.
#[allow(dead_code)]
fn js_error_encode(v: &JsValue, classes: &str) -> String {
    let message = match v {
        JsValue::Object(_, props) => props
            .iter()
            .find(|(k, _)| k == "message")
            .map(|(_, m)| js_to_string(m))
            .unwrap_or_default(),
        _ => String::new(),
    };
    format!("\u{2}{classes}\u{2}{message}\u{2}{}", js_json_stringify(v, ""))
}

#[allow(dead_code)]
fn js_error_parts(e: &str) -> Option<(&str, &str, &str)> {
    let rest = e.strip_prefix('\u{2}')?;
    let mut parts = rest.splitn(3, '\u{2}');
    Some((parts.next()?, parts.next()?, parts.next()?))
}

/// `e.message`.
#[allow(dead_code)]
fn js_error_message(e: &str) -> String {
    js_error_parts(e).map_or_else(|| e.to_string(), |(_, m, _)| m.to_string())
}

/// `e.name`.
#[allow(dead_code)]
fn js_error_name(e: &str) -> String {
    match js_error_parts(e) {
        Some((_, _, json)) => match js_json_parse(json) {
            Ok(JsonValue::Object(props)) => props
                .iter()
                .find(|(k, _)| k == "name")
                .and_then(|(_, v)| match v {
                    JsonValue::Str(s) => Some(s.clone()),
                    _ => None,
                })
                .unwrap_or_else(|| "Error".into()),
            _ => "Error".into(),
        },
        None => "Error".into(),
    }
}

/// `e instanceof C`.
#[allow(dead_code)]
fn js_error_is(e: &str, class: &str) -> bool {
    js_error_parts(e).is_some_and(|(classes, _, _)| classes.split(',').any(|c| c == class))
}

/// `e` as an instance of `C`, when it is one.
#[allow(dead_code)]
fn js_error_as<T: JsFromJson>(e: &str, class: &str) -> Option<T> {
    if !js_error_is(e, class) {
        return None;
    }
    js_error_parts(e).and_then(|(_, _, json)| js_json_parse_as::<T>(json).ok())
}

/// `String(e)`: `Name: message`, or a thrown value that was not an error as it is.
#[allow(dead_code)]
fn js_error_text_named(e: &str) -> String {
    match e.strip_prefix('\u{3}') {
        Some(value) => value.to_string(),
        None => format!("{}: {}", js_error_name(e), js_error_message(e)),
    }
}
