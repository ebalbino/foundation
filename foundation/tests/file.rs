use foundation::alloc::arena;
use foundation::alloc::string;
use foundation::file;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_file_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    std::env::temp_dir().join(format!("foundation-{name}-{nanos}.tmp"))
}

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

#[test]
fn load_reads_fixture_bytes_into_arena_memory() {
    let arena = arena(16 * 1024);
    let bytes = file::load(arena.clone(), fixture_path("test.toml")).unwrap();
    let contents = std::str::from_utf8(&bytes).unwrap();

    assert!(contents.contains("[workspace]"));
    assert!(contents.contains("members = [\"foundation\"]"));
}

#[test]
fn load_returns_none_for_missing_files() {
    let arena = arena(1024);

    assert!(file::load(arena, "data/does-not-exist.txt").is_none());
}

#[test]
fn save_and_append_round_trip_file_contents() {
    let arena = arena(1024);
    let path = temp_file_path("save-append");

    file::save(&path, b"hello").unwrap();
    let appended = file::append(&path, b" world").unwrap();
    let written = file::load(arena.clone(), &path).map(string::wrap).unwrap();

    assert_eq!(appended, 6);
    assert_eq!(&written, "hello world");

    file::remove(&path).unwrap();
}

#[test]
fn append_returns_error_for_missing_files() {
    let path = temp_file_path("missing-append");

    assert!(file::append(&path, b"data").is_err());
}

#[test]
fn shell_helpers_manage_files_and_directories() {
    let arena = arena(1024);
    let root = temp_file_path("fs-helpers");
    let nested = root.join("nested");
    let source = nested.join("source.txt");
    let copy = nested.join("copy.txt");
    let moved = root.join("moved.txt");

    file::create_dir(&nested).unwrap();
    file::save(&source, b"hello").unwrap();

    assert!(file::exists(&nested));
    assert!(file::is_dir(&nested));
    assert!(file::exists(&source));
    assert!(file::is_file(&source));

    let entries = file::list(&nested).unwrap();
    assert_eq!(entries, vec![source.clone()]);

    file::copy(&source, &copy).unwrap();
    assert_eq!(
        &file::load(arena.clone(), &copy).map(string::wrap).unwrap(),
        "hello"
    );

    file::rename(&copy, &moved).unwrap();
    assert!(!file::exists(&copy));
    assert!(file::exists(&moved));

    file::remove(&root).unwrap();
    assert!(!file::exists(&root));
}

#[test]
fn cwd_matches_the_process_working_directory() {
    assert_eq!(file::cwd().unwrap(), std::env::current_dir().unwrap());
}
