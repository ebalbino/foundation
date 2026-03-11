use foundation::alloc::arena;
use foundation::executor::{self, ExecutorError, Step};

#[test]
fn resolves_initial_value_without_tasks() {
    let arena = arena(1024);
    let executor = executor::executor(arena, 7_u32);

    assert_eq!(executor.resolve().unwrap(), 7);
}

#[test]
fn spawned_async_tasks_mutate_the_shared_value() {
    let arena = arena(1024);
    let mut executor = executor::executor(arena, std::string::String::from("seed"));

    executor
        .spawn(|value| async move {
            value.update(|value| value.push_str("-alpha"));
        })
        .unwrap();

    executor
        .spawn(|value| async move {
            value.update(|value| value.push_str("-beta"));
        })
        .unwrap();

    assert_eq!(executor.resolve().unwrap(), "seed-alpha-beta");
}

#[test]
fn run_executes_cooperative_tasks_until_ready() {
    let arena = arena(1024);
    let mut executor = executor::executor(arena, Vec::<u8>::new());

    executor
        .spawn(|value| async move {
            value.update(|value| value.push(1));
            executor::yield_now().await;
            value.update(|value| value.push(3));
        })
        .unwrap();

    executor
        .spawn(|value| async move {
            value.update(|value| value.push(2));
        })
        .unwrap();

    executor.run().unwrap();

    assert_eq!(executor.pending(), 0);
    assert_eq!(executor.resolve().unwrap(), vec![1, 2, 3]);
}

#[test]
fn step_executes_one_task_at_a_time() {
    let arena = arena(1024);
    let mut executor = executor::executor(arena, Vec::<u8>::new());

    executor
        .spawn(|value| async move {
            value.update(|value| value.push(1));
            executor::yield_now().await;
            value.update(|value| value.push(3));
        })
        .unwrap();

    executor
        .spawn(|value| async move {
            value.update(|value| value.push(2));
        })
        .unwrap();

    assert_eq!(executor.step(), Ok(Step::Progressed));
    assert_eq!(executor.pending(), 2);
    assert_eq!(executor.step(), Ok(Step::Progressed));
    assert_eq!(executor.pending(), 1);
    assert_eq!(executor.step(), Ok(Step::Progressed));
    assert!(executor.is_complete());
    assert_eq!(executor.step(), Ok(Step::Idle));
    assert_eq!(executor.resolve().unwrap(), vec![1, 2, 3]);
}

#[test]
fn multiple_executors_can_be_driven_round_robin() {
    let arena_a = arena(1024);
    let arena_b = arena(1024);
    let mut left = executor::executor(arena_a, Vec::<u8>::new());
    let mut right = executor::executor(arena_b, Vec::<u8>::new());

    left.spawn(|value| async move {
        value.update(|value| value.push(1));
        executor::yield_now().await;
        value.update(|value| value.push(3));
    })
    .unwrap();

    right
        .spawn(|value| async move {
            value.update(|value| value.push(10));
            executor::yield_now().await;
            value.update(|value| value.push(30));
        })
        .unwrap();

    left.spawn(|value| async move {
        value.update(|value| value.push(2));
    })
    .unwrap();

    right
        .spawn(|value| async move {
            value.update(|value| value.push(20));
        })
        .unwrap();

    while !left.is_complete() || !right.is_complete() {
        if !left.is_complete() {
            assert_eq!(left.step(), Ok(Step::Progressed));
        }

        if !right.is_complete() {
            assert_eq!(right.step(), Ok(Step::Progressed));
        }
    }

    assert_eq!(left.resolve().unwrap(), vec![1, 2, 3]);
    assert_eq!(right.resolve().unwrap(), vec![10, 20, 30]);
}

#[test]
fn multiple_executors_with_multiple_yielding_tasks_stay_fair_under_round_robin() {
    let arena_a = arena(8 * 1024);
    let arena_b = arena(8 * 1024);
    let mut left = executor::executor(arena_a, Vec::<&'static str>::new());
    let mut right = executor::executor(arena_b, Vec::<&'static str>::new());

    left.spawn(|value| async move {
        value.update(|value| value.push("left-a-0"));
        executor::yield_now().await;
        value.update(|value| value.push("left-a-1"));
        executor::yield_now().await;
        value.update(|value| value.push("left-a-2"));
        executor::yield_now().await;
        value.update(|value| value.push("left-a-3"));
    })
    .unwrap();

    left.spawn(|value| async move {
        value.update(|value| value.push("left-b-0"));
        executor::yield_now().await;
        value.update(|value| value.push("left-b-1"));
        executor::yield_now().await;
        value.update(|value| value.push("left-b-2"));
        executor::yield_now().await;
        value.update(|value| value.push("left-b-3"));
    })
    .unwrap();

    right
        .spawn(|value| async move {
            value.update(|value| value.push("right-a-0"));
            executor::yield_now().await;
            value.update(|value| value.push("right-a-1"));
            executor::yield_now().await;
            value.update(|value| value.push("right-a-2"));
            executor::yield_now().await;
            value.update(|value| value.push("right-a-3"));
        })
        .unwrap();

    right
        .spawn(|value| async move {
            value.update(|value| value.push("right-b-0"));
            executor::yield_now().await;
            value.update(|value| value.push("right-b-1"));
            executor::yield_now().await;
            value.update(|value| value.push("right-b-2"));
            executor::yield_now().await;
            value.update(|value| value.push("right-b-3"));
        })
        .unwrap();

    while !left.is_complete() || !right.is_complete() {
        if !left.is_complete() {
            assert_eq!(left.step(), Ok(Step::Progressed));
        }

        if !right.is_complete() {
            assert_eq!(right.step(), Ok(Step::Progressed));
        }
    }

    assert_eq!(
        left.resolve().unwrap(),
        vec![
            "left-a-0", "left-b-0", "left-a-1", "left-b-1", "left-a-2", "left-b-2", "left-a-3",
            "left-b-3",
        ]
    );
    assert_eq!(
        right.resolve().unwrap(),
        vec![
            "right-a-0",
            "right-b-0",
            "right-a-1",
            "right-b-1",
            "right-a-2",
            "right-b-2",
            "right-a-3",
            "right-b-3",
        ]
    );
}

#[test]
fn resolve_fails_when_external_shared_references_are_kept() {
    let arena = arena(1024);
    let executor = executor::executor(arena, 11_u32);
    let shared = executor.shared();

    assert_eq!(
        executor.resolve(),
        Err(ExecutorError::OutstandingReferences)
    );

    drop(shared);
}

#[test]
fn run_reports_stalled_tasks() {
    let arena = arena(1024);
    let mut executor = executor::executor(arena, 5_u32);

    executor
        .spawn(|_value| std::future::pending::<()>())
        .unwrap();

    assert_eq!(executor.run(), Err(ExecutorError::Stalled));
}

#[test]
fn spawn_fails_when_the_arena_is_out_of_memory() {
    let arena = arena(0);
    let mut executor = executor::executor(arena, 1_u32);

    assert!(executor.spawn(|_value| async {}).is_none());
    assert_eq!(executor.pending(), 0);
}
