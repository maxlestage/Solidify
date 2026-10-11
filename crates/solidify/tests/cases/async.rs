async fn fetch_value(x: i64) -> i64 {
    x * 2
}

async fn total(xs: &[i64]) -> i64 {
    let mut t = 0;
    for &x in xs {
        t += fetch_value(x).await;
    }
    t
}

fn main() {
    block_on(async {
        let result = total(&[1, 2, 3]).await;
        println!("total: {result}");
    })
}

/// Runs a future to completion (the async functions here never wait on I/O).
fn block_on<F: std::future::Future>(future: F) -> F::Output {
    use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
    fn noop(_: *const ()) {}
    fn clone(p: *const ()) -> RawWaker {
        RawWaker::new(p, &VTABLE)
    }
    static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, noop, noop, noop);
    let waker = unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &VTABLE)) };
    let mut cx = Context::from_waker(&waker);
    let mut future = std::pin::pin!(future);
    loop {
        if let Poll::Ready(v) = future.as_mut().poll(&mut cx) {
            return v;
        }
    }
}
