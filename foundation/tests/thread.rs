use foundation::thread;

#[test]
fn spawn_uses_the_default_configuration() {
    let handle = thread::spawn(|| 21_u32 * 2).unwrap();

    assert_eq!(handle.join().unwrap(), 42);
}

#[test]
fn named_threads_expose_the_configured_name() {
    let handle = thread::named("foundation-worker")
        .spawn(|| std::thread::current().name().map(str::to_owned))
        .unwrap();

    assert_eq!(
        handle.join().unwrap().as_deref(),
        Some("foundation-worker")
    );
}

#[test]
fn scoped_threads_can_borrow_stack_data() {
    let values = [1_u32, 2, 3];

    let sum = std::thread::scope(|scope| {
        let handle = thread::config()
            .named("scoped-worker")
            .spawn_scoped(scope, || values.iter().sum::<u32>())
            .unwrap();

        handle.join().unwrap()
    });

    assert_eq!(sum, 6);
}
