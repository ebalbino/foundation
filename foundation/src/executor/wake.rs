use std::cell::Cell;
use std::task::{RawWaker, RawWakerVTable, Waker};

#[derive(Default)]
pub(crate) struct WakerState {
    woken: Cell<bool>,
}

impl WakerState {
    pub(crate) fn reset(&self) {
        self.woken.set(false);
    }

    pub(crate) fn was_woken(&self) -> bool {
        self.woken.get()
    }

    fn wake(&self) {
        self.woken.set(true);
    }
}

pub(crate) fn waker(state: &WakerState) -> Waker {
    unsafe { Waker::from_raw(raw_waker(state)) }
}

fn raw_waker(state: &WakerState) -> RawWaker {
    RawWaker::new((state as *const WakerState).cast(), &WAKER_VTABLE)
}

static WAKER_VTABLE: RawWakerVTable = RawWakerVTable::new(
    clone_waker_state,
    wake_waker_state,
    wake_waker_state_by_ref,
    drop_waker_state,
);

unsafe fn clone_waker_state(data: *const ()) -> RawWaker {
    RawWaker::new(data, &WAKER_VTABLE)
}

unsafe fn wake_waker_state(data: *const ()) {
    let state = unsafe { &*(data.cast::<WakerState>()) };
    state.wake();
}

unsafe fn wake_waker_state_by_ref(data: *const ()) {
    let state = unsafe { &*(data.cast::<WakerState>()) };
    state.wake();
}

unsafe fn drop_waker_state(_data: *const ()) {}
