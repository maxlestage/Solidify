/// A generator (`function*`): its body runs on its own thread and hands each `yield` over
/// a channel, one value per `next()`, so the sequence is lazy and may be infinite.
struct JsGenerator<T> {
    ask: std::sync::mpsc::SyncSender<()>,
    values: std::sync::mpsc::Receiver<Option<T>>,
}

/// What a generator body yields through.
struct JsYield<T> {
    asks: std::sync::mpsc::Receiver<()>,
    values: std::sync::mpsc::SyncSender<Option<T>>,
}

impl<T: Send + 'static> JsGenerator<T> {
    fn new(body: impl FnOnce(&JsYield<T>) + Send + 'static) -> Self {
        let (ask, asks) = std::sync::mpsc::sync_channel(0);
        let (values, received) = std::sync::mpsc::sync_channel(0);
        std::thread::spawn(move || {
            let y = JsYield { asks, values };
            if y.asks.recv().is_err() {
                return;
            }
            body(&y);
            let _ = y.values.send(None);
        });
        JsGenerator { ask, values: received }
    }
}

impl<T> JsYield<T> {
    /// `yield value`: hands the value over and waits for the next `next()`.
    fn yield_(&self, value: T) {
        if self.values.send(Some(value)).is_err() || self.asks.recv().is_err() {
            // The generator was dropped: its body never resumes.
            loop {
                std::thread::park();
            }
        }
    }
}

impl<T> Iterator for JsGenerator<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        self.ask.send(()).ok()?;
        self.values.recv().ok().flatten()
    }
}

impl<T> std::fmt::Debug for JsGenerator<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Object [Generator] {}")
    }
}
