use super::wake::WakerState;
use core::future::Future as StdFuture;
use core::mem;
use core::pin::Pin;
use core::task::{Context, Poll};

pub(crate) struct Task {
    future: *mut (),
    poll: unsafe fn(*mut (), &mut Context<'_>) -> Poll<()>,
    drop: unsafe fn(*mut ()),
    pub(crate) waker_state: WakerState,
}

impl Task {
    pub(crate) fn new<Fut>(future: Pin<crate::alloc::Pinned<Fut>>) -> Self
    where
        Fut: StdFuture<Output = ()> + 'static,
    {
        let future_ptr = (&*future as *const Fut).cast_mut().cast();
        mem::forget(future);

        Self {
            future: future_ptr,
            poll: poll_future::<Fut>,
            drop: drop_future::<Fut>,
            waker_state: WakerState::default(),
        }
    }

    pub(crate) unsafe fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<()> {
        let task = unsafe { self.get_unchecked_mut() };
        unsafe { (task.poll)(task.future, context) }
    }
}

impl Drop for Task {
    fn drop(&mut self) {
        unsafe {
            (self.drop)(self.future);
        }
    }
}

unsafe fn poll_future<Fut>(future: *mut (), context: &mut Context<'_>) -> Poll<()>
where
    Fut: StdFuture<Output = ()> + 'static,
{
    let future = unsafe { &mut *(future.cast::<Fut>()) };
    unsafe { Pin::new_unchecked(future) }.poll(context)
}

unsafe fn drop_future<Fut>(future: *mut ())
where
    Fut: 'static,
{
    unsafe {
        core::ptr::drop_in_place(future.cast::<Fut>());
    }
}
