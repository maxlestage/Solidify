/// A `Symbol`: equal only to itself (or, made by `Symbol.for`, to the symbol of the same
/// key).
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[allow(dead_code)]
struct JsSymbol {
    id: u64,
    description: Option<&'static str>,
    registered: bool,
}

#[allow(dead_code)]
impl JsSymbol {
    /// `Symbol("id")`: a new symbol.
    fn new(description: Option<&str>) -> JsSymbol {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        JsSymbol {
            id: NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            description: description.map(|d| &*Box::leak(d.to_string().into_boxed_str())),
            registered: false,
        }
    }

    /// `Symbol.for(key)`: the symbol of this key, made once.
    fn for_key(key: &str) -> JsSymbol {
        static REGISTRY: std::sync::Mutex<Vec<JsSymbol>> = std::sync::Mutex::new(Vec::new());
        let mut registry = REGISTRY.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(s) = registry.iter().find(|s| s.description == Some(key)) {
            return *s;
        }
        let mut s = JsSymbol::new(Some(key));
        s.registered = true;
        registry.push(s);
        s
    }

    /// `Symbol.keyFor(s)`.
    fn key_for(self) -> Option<String> {
        if self.registered {
            self.description.map(str::to_string)
        } else {
            None
        }
    }

    /// `s.description`.
    fn description(&self) -> Option<String> {
        self.description.map(str::to_string)
    }
}

/// `String(s)`: `Symbol(id)`.
impl std::fmt::Display for JsSymbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Symbol({})", self.description.unwrap_or(""))
    }
}

impl std::fmt::Debug for JsSymbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}
