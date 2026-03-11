use foundation::thread;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

#[test]
fn work_queue_pushes_pops_and_closes() {
    let queue = thread::work_queue();

    assert!(queue.is_empty());
    queue.push(1_u32).unwrap();
    queue.push(2_u32).unwrap();

    assert_eq!(queue.len(), 2);
    assert_eq!(queue.try_pop(), Some(1));
    assert_eq!(queue.pop(), Some(2));

    queue.close();

    assert!(queue.is_closed());
    assert_eq!(queue.pop(), None);
    assert!(queue.push(3).is_err());
}

#[test]
fn work_queue_wakes_blocked_consumers_on_close() {
    let queue = thread::work_queue::<u32>();
    let worker_queue = queue.clone();
    let handle = std::thread::spawn(move || worker_queue.pop());

    queue.close();

    assert_eq!(handle.join().unwrap(), None);
}

#[test]
fn pool_executes_submitted_work() {
    let pool = thread::pool(2).named("pool-worker").build().unwrap();
    let values = Arc::new(Mutex::new(Vec::new()));

    for value in [1_u32, 2, 3, 4] {
        let values = values.clone();
        pool.execute(move || values.lock().unwrap().push(value)).unwrap();
    }

    pool.finish().unwrap();

    let mut values = values.lock().unwrap().clone();
    values.sort_unstable();
    assert_eq!(values, vec![1, 2, 3, 4]);
}

#[test]
fn pool_uses_indexed_worker_names() {
    let pool = thread::pool(2).named("queue-worker").build().unwrap();
    let (tx, rx) = mpsc::channel();

    for _ in 0..4 {
        let tx = tx.clone();
        pool.execute(move || {
            let name = std::thread::current().name().map(str::to_owned).unwrap();
            tx.send(name).unwrap();
        })
        .unwrap();
    }

    drop(tx);
    pool.finish().unwrap();

    let names: Vec<_> = rx.iter().collect();
    assert!(names.iter().all(|name| name.starts_with("queue-worker-")));
}

#[test]
fn pool_rejects_new_work_after_close() {
    let pool = thread::pool(1).build().unwrap();

    pool.close();

    assert_eq!(pool.execute(|| {}), Err(thread::PoolError::Closed));
    pool.finish().unwrap();
}
