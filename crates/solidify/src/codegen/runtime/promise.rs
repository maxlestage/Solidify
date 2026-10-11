/// A promise: settled by its `resolve` / `reject` callbacks, awaited by the event loop.
struct JsPromise<T> {
    state: std::rc::Rc<std::cell::RefCell<Option<Result<T, String>>>>,
}

impl<T: 'static> JsPromise<T> {
    #[allow(dead_code)]
    fn new(executor: impl FnOnce(Box<dyn Fn(T)>, Box<dyn Fn(String)>)) -> Self {
        let state = std::rc::Rc::new(std::cell::RefCell::new(None));
        let (resolve_state, reject_state) = (state.clone(), state.clone());
        executor(
            Box::new(move |value| {
                let mut slot = resolve_state.borrow_mut();
                if slot.is_none() {
                    *slot = Some(Ok(value));
                }
            }),
            Box::new(move |error| {
                let mut slot = reject_state.borrow_mut();
                if slot.is_none() {
                    *slot = Some(Err(error));
                }
            }),
        );
        JsPromise { state }
    }
}

impl<T> std::future::Future for JsPromise<T> {
    type Output = T;
    fn poll(
        self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<T> {
        match self.state.borrow_mut().take() {
            Some(Ok(value)) => std::task::Poll::Ready(value),
            Some(Err(error)) => panic!("{error}"),
            None => std::task::Poll::Pending,
        }
    }
}
