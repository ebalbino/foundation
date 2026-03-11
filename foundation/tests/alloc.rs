#[path = "alloc/buffer.rs"]
mod buffer;
#[path = "alloc/string.rs"]
mod string;
#[path = "alloc/string_builder.rs"]
mod string_builder;
#[path = "alloc/string_pool.rs"]
mod string_pool;

use foundation::alloc::{self, arena};
use std::cell::Cell;
use std::io::{Read, Write};
use std::marker::PhantomPinned;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

#[test]
fn arena_allocates_with_alignment() {
    let arena = arena(64);

    let bytes = arena.allocate::<u8>(3).unwrap();
    assert_eq!(bytes.len(), 3);
    assert_eq!(arena.current_position(), 3);

    let words = arena.allocate::<u32>(2).unwrap();

    assert_eq!(arena.current_position(), 12);
    assert_eq!(words.as_ref().len(), 8);
}

#[test]
fn duplicate_copies_into_new_allocation() {
    let arena = arena(64);
    let values = arena
        .allocate::<u16>(3)
        .map(|mut v| {
            v.copy_from_slice(&[7, 11, 13]);
            v
        })
        .unwrap();

    let copy = alloc::duplicate(&values).unwrap();

    assert_eq!(arena.current_position(), 12);
    assert_eq!(values[..], copy[..]);
    assert_ne!(values.as_ptr(), copy.as_ptr());
}

#[test]
fn allocated_read_and_write_require_matching_sizes() {
    let arena = arena(32);
    let mut bytes = arena.allocate::<u8>(4).unwrap();

    let written = bytes.write(&[1, 2, 3, 4]).unwrap();
    assert_eq!(written, 4);
    assert_eq!(&bytes[..], &[1, 2, 3, 4]);

    let mut copy = [0; 4];
    let read = bytes.read(&mut copy).unwrap();
    assert_eq!(read, 4);
    assert_eq!(copy, [1, 2, 3, 4]);

    assert!(bytes.write(&[1, 2, 3]).is_err());
    assert!(bytes.read(&mut [0; 3]).is_err());
}

#[test]
fn reset_rewinds_and_zeroes_the_buffer() {
    let mut arena = arena(32);
    let bytes = arena
        .allocate::<u8>(4)
        .map(|mut bytes| {
            bytes.copy_from_slice(&[9, 8, 7, 6]);
            bytes
        })
        .unwrap();
    drop(bytes);

    assert_eq!(arena.current_position(), 4);

    arena.reset();

    assert_eq!(arena.current_position(), 0);
    assert!(arena.buffer().iter().all(|byte| *byte == 0));
}

#[test]
fn allocations_become_empty_after_the_arena_generation_changes() {
    let mut arena = arena(32);
    let mut bytes = arena
        .allocate::<u8>(4)
        .map(|mut bytes| {
            bytes.copy_from_slice(&[1, 2, 3, 4]);
            bytes
        })
        .unwrap();

    drop(arena.allocate::<u8>(1));
    arena.reset();

    assert!(bytes.is_empty());
    assert!(bytes.as_ref().is_empty());
    assert!(bytes.as_mut().is_empty());
    assert!(bytes.write(&[1, 2, 3, 4]).is_err());
}

#[test]
fn allocated_pin_keeps_values_alive_without_the_original_arena_handle() {
    let arena = arena(64);
    let pinned = arena.pin(123_u32).unwrap();
    drop(arena);

    assert_eq!(*pinned, 123);
}

#[test]
fn allocated_pin_drops_the_inner_value() {
    struct DropTracker(Rc<Cell<bool>>);

    impl Drop for DropTracker {
        fn drop(&mut self) {
            self.0.set(true);
        }
    }

    let dropped = Rc::new(Cell::new(false));
    let arena = arena(64);
    let pinned = arena.pin(DropTracker(dropped.clone())).unwrap();

    drop(pinned);

    assert!(dropped.get());
}

#[test]
fn allocated_pin_supports_polling_pinned_futures() {
    struct ReadyOnce {
        value: u32,
        _pin: PhantomPinned,
    }

    impl std::future::Future for ReadyOnce {
        type Output = u32;

        fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
            Poll::Ready(self.value)
        }
    }

    #[derive(Default)]
    struct NoopWake;

    impl Wake for NoopWake {
        fn wake(self: Arc<Self>) {}
    }

    let arena = arena(64);
    let mut pinned = arena.pin(
        ReadyOnce {
            value: 99,
            _pin: PhantomPinned,
        },
    )
    .unwrap();
    let waker = Waker::from(Arc::new(NoopWake));
    let mut cx = Context::from_waker(&waker);

    assert_eq!(
        std::future::Future::poll(pinned.as_mut(), &mut cx),
        Poll::Ready(99)
    );
}
