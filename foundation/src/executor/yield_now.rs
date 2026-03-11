use core::future::Future as StdFuture;
use core::pin::Pin;
use core::task::{Context, Poll};

/// Returns a future that yields once before completing.
///
/// This is useful for cooperative scheduling when a task wants to let other
/// queued tasks run before continuing.
pub fn yield_now() -> YieldNow {
    YieldNow { yielded: false }
}

/// Future returned by [`yield_now`].
pub struct YieldNow {
    yielded: bool,
}

impl StdFuture for YieldNow {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.yielded {
            Poll::Ready(())
        } else {
            self.yielded = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}
