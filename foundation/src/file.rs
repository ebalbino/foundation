//! Small filesystem helpers used throughout the crate.
//!
//! The API is intentionally lightweight:
//!
//! - [`load`] reads file contents into arena-backed memory
//! - [`save`] and [`append`] write raw bytes
//! - path predicates such as [`exists`], [`is_file`], and [`is_dir`] query the
//!   filesystem through shell commands
//! - directory and path mutation helpers such as [`create_dir`], [`copy`],
//!   [`rename`], and [`remove`] also shell out to standard Unix utilities
//!
//! Because several helpers invoke external commands like `test`, `mkdir`, `cp`,
//! `mv`, `rm`, and `ls`, this module currently assumes a Unix-like environment.

use crate::alloc::{Allocated, Arena};
use crate::rust_alloc::format;
use crate::rust_alloc::rc::Rc;
use crate::rust_alloc::string::String;
use crate::rust_alloc::vec::Vec;
use std::ffi::OsStr;
use std::io::Write;
use std::{
    fs, io,
    path::{Path, PathBuf},
    process::Command,
};

/// Reads an entire file into arena-backed memory.
///
/// Returns `None` if the file cannot be read or the arena does not have enough
/// remaining capacity to hold the bytes.
pub fn load<P: AsRef<Path>>(arena: Rc<Arena>, path: P) -> Option<Allocated<u8>> {
    let data = fs::read(path).ok()?;
    arena.allocate::<u8>(data.len()).map(|mut buffer: Allocated<u8>| {
        buffer.copy_from_slice(&data);
        buffer
    })
}

/// Writes `data` to `path`, replacing any existing file contents.
pub fn save<P: AsRef<Path>, D: AsRef<[u8]>>(path: P, data: D) -> io::Result<()> {
    fs::write(path, data)
}

/// Appends `data` to an existing file and returns the number of bytes written.
///
/// This fails if `path` does not already exist or is not writable.
pub fn append<P: AsRef<Path>, D: AsRef<[u8]>>(path: P, data: D) -> io::Result<usize> {
    let mut file = fs::File::options().append(true).open(path)?;
    file.write(data.as_ref())
}

/// Returns the current working directory.
///
/// This is implemented by invoking `pwd`.
pub fn cwd() -> io::Result<PathBuf> {
    shell_output("pwd", std::iter::empty::<&OsStr>())
        .map(|output: String| PathBuf::from(output.trim()))
}

/// Returns `true` when `path` exists.
///
/// This is implemented by invoking `test -e`.
pub fn exists(path: impl AsRef<Path>) -> bool {
    shell_status("test", [OsStr::new("-e"), path.as_ref().as_os_str()]).is_ok()
}

/// Returns `true` when `path` exists and is a regular file.
///
/// This is implemented by invoking `test -f`.
pub fn is_file(path: impl AsRef<Path>) -> bool {
    shell_status("test", [OsStr::new("-f"), path.as_ref().as_os_str()]).is_ok()
}

/// Returns `true` when `path` exists and is a directory.
///
/// This is implemented by invoking `test -d`.
pub fn is_dir(path: impl AsRef<Path>) -> bool {
    shell_status("test", [OsStr::new("-d"), path.as_ref().as_os_str()]).is_ok()
}

/// Creates `path` and any missing parent directories.
///
/// This is implemented by invoking `mkdir -p`.
pub fn create_dir(path: impl AsRef<Path>) -> io::Result<()> {
    shell_status("mkdir", [OsStr::new("-p"), path.as_ref().as_os_str()])
}

/// Copies a file or directory tree from `from` to `to`.
///
/// This is implemented by invoking `cp -r`.
pub fn copy(from: impl AsRef<Path>, to: impl AsRef<Path>) -> io::Result<()> {
    shell_status(
        "cp",
        [
            OsStr::new("-r"),
            from.as_ref().as_os_str(),
            to.as_ref().as_os_str(),
        ],
    )
}

/// Renames or moves a file or directory.
///
/// This is implemented by invoking `mv`.
pub fn rename(from: impl AsRef<Path>, to: impl AsRef<Path>) -> io::Result<()> {
    shell_status("mv", [from.as_ref().as_os_str(), to.as_ref().as_os_str()])
}

/// Removes a file or directory tree recursively.
///
/// This is implemented by invoking `rm -rf`.
pub fn remove(path: impl AsRef<Path>) -> io::Result<()> {
    shell_status("rm", [OsStr::new("-rf"), path.as_ref().as_os_str()])
}

/// Lists direct entries inside `path`.
///
/// Returned paths are joined against `path`, so each entry is absolute or
/// relative according to the input root. Hidden entries are included, except for
/// `.` and `..`.
///
/// This is implemented by invoking `ls -1A`.
pub fn list(path: impl AsRef<Path>) -> io::Result<Vec<PathBuf>> {
    let root = path.as_ref();
    let output = shell_output("ls", [OsStr::new("-1A"), root.as_os_str()])?;

    if output.trim().is_empty() {
        return Ok(Vec::new());
    }

    Ok(output.lines().map(|entry| root.join(entry)).collect())
}

fn shell_status<S, I>(command: &str, args: I) -> io::Result<()>
where
    S: AsRef<OsStr>,
    I: IntoIterator<Item = S>,
{
    let status = Command::new(command).args(args).status()?;

    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "{command} failed with status {status}"
        )))
    }
}

fn shell_output<S, I>(command: &str, args: I) -> io::Result<String>
where
    S: AsRef<OsStr>,
    I: IntoIterator<Item = S>,
{
    let output = Command::new(command).args(args).output()?;

    if output.status.success() {
        String::from_utf8(output.stdout)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
    } else {
        Err(io::Error::other(format!(
            "{command} failed with status {}",
            output.status
        )))
    }
}
