/// An object held in several places, like every JavaScript object: changed through one,
/// changed for all; equal only to itself.
struct Shared<T: ?Sized>(std::rc::Rc<std::cell::RefCell<T>>);

#[allow(dead_code)]
impl<T> Shared<T> {
    fn new(value: T) -> Self {
        Shared(std::rc::Rc::new(std::cell::RefCell::new(value)))
    }
}

#[allow(dead_code)]
impl<T: ?Sized> Shared<T> {
    fn borrow(&self) -> std::cell::Ref<'_, T> {
        self.0.borrow()
    }

    fn borrow_mut(&self) -> std::cell::RefMut<'_, T> {
        self.0.borrow_mut()
    }

    /// Reads the object for the time of `f` only.
    fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        f(&self.0.borrow())
    }

    /// Changes the object for the time of `f` only.
    fn with_mut<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        f(&mut self.0.borrow_mut())
    }
}

// The program runs one piece at a time (a generator's thread waits while the caller
// runs, and the other way round): its objects are never used by two threads at once.
unsafe impl<T: ?Sized> Send for Shared<T> {}
unsafe impl<T: ?Sized> Sync for Shared<T> {}

impl<T: ?Sized> Clone for Shared<T> {
    fn clone(&self) -> Self {
        Shared(self.0.clone())
    }
}

impl<T: ?Sized> PartialEq for Shared<T> {
    fn eq(&self, other: &Self) -> bool {
        std::rc::Rc::ptr_eq(&self.0, &other.0)
    }
}

impl<T: ?Sized> Eq for Shared<T> {}

impl<T: ?Sized> std::hash::Hash for Shared<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        (std::rc::Rc::as_ptr(&self.0) as *const () as usize).hash(state);
    }
}

impl<T: std::fmt::Debug + ?Sized> std::fmt::Debug for Shared<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0.try_borrow() {
            Ok(v) => v.fmt(f),
            Err(_) => f.write_str("[Circular]"),
        }
    }
}

impl<T: std::fmt::Display + ?Sized> std::fmt::Display for Shared<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.borrow().fmt(f)
    }
}

thread_local! {
    /// The objects being shown, and whether one of them was met again inside itself.
    static JS_SHOWING: std::cell::RefCell<Vec<(usize, bool)>> = const { std::cell::RefCell::new(Vec::new()) };
}

impl<T: JsInspect + ?Sized> JsInspect for Shared<T> {
    fn js_value(&self) -> JsValue {
        let ptr = std::rc::Rc::as_ptr(&self.0) as *const () as usize;
        let cycle = JS_SHOWING.with(|s| {
            let mut s = s.borrow_mut();
            match s.iter_mut().find(|(p, _)| *p == ptr) {
                Some(entry) => {
                    entry.1 = true;
                    true
                }
                None => {
                    s.push((ptr, false));
                    false
                }
            }
        });
        if cycle {
            return JsValue::Raw("[Circular *1]".into());
        }
        let v = self.0.borrow().js_value();
        let referenced = JS_SHOWING.with(|s| s.borrow_mut().pop().is_some_and(|(_, r)| r));
        match v {
            // Node marks the object a cycle comes back to.
            JsValue::Object(name, props) if referenced => {
                let marked: &'static str = Box::leak(
                    match name {
                        Some(n) => format!("<ref *1> {n}"),
                        None => "<ref *1>".to_string(),
                    }
                    .into_boxed_str(),
                );
                JsValue::Object(Some(marked), props)
            }
            other => other,
        }
    }
}
