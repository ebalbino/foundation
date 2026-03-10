use foundation::alloc::{arena, string_builder};
use foundation::process;

#[test]
fn stdin_writes_child_input_and_stdout_bytes_collect_output() {
    let mut child = process::execute("sh", ["-c", "cat"]);

    assert!(child.stdin([0_u8, 1, 2, 3]));
    assert_eq!(child.stdout_bytes().unwrap(), vec![0_u8, 1, 2, 3]);
}

#[test]
fn stdin_can_only_be_taken_once() {
    let mut child = process::execute("sh", ["-c", "cat"]);

    assert!(child.stdin("once".as_bytes()));
    assert!(!child.stdin("twice".as_bytes()));
}

#[test]
fn stdout_collects_child_output() {
    let arena = arena(1024);
    let mut builder = string_builder(arena, 4);
    let mut child = process::execute("sh", ["-c", "printf 'hello world'"]);

    let output = child.stdout(&mut builder).unwrap();
    assert_eq!(&output, "hello world");
}

#[test]
fn stderr_collects_child_output() {
    let arena = arena(1024);
    let mut builder = string_builder(arena, 8);
    let mut child = process::execute("sh", ["-c", "printf 'warning' >&2"]);

    let output = child.stderr(&mut builder).unwrap();
    assert_eq!(&output, "warning");
}

#[test]
fn stdout_can_only_be_taken_once() {
    let arena = arena(1024);
    let mut builder = string_builder(arena.clone(), 8);
    let mut child = process::execute("sh", ["-c", "printf 'once'"]);

    let output = child.stdout(&mut builder).unwrap();

    assert_eq!(&output, "once");

    let mut second_builder = string_builder(arena, 8);
    assert!(child.stdout(&mut second_builder).is_none());
}

#[test]
fn stderr_can_only_be_taken_once() {
    let arena = arena(1024);
    let mut builder = string_builder(arena.clone(), 8);
    let mut child = process::execute("sh", ["-c", "printf 'once' >&2"]);

    let output = child.stderr(&mut builder).unwrap();

    assert_eq!(&output, "once");

    let mut second_builder = string_builder(arena, 8);
    assert!(child.stderr(&mut second_builder).is_none());
}
