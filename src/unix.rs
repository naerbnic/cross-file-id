use std::fs::{File, Metadata};
use std::hash::Hash;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd};
use std::path::Path;

use io_lifetimes::raw::{AsRawFilelike, FromRawFilelike, RawFilelike};

fn get_metadata_from_raw(fd: RawFilelike) -> io::Result<Metadata> {
    // SAFETY: Although we create a File from the file descriptor, we use
    // into_raw_fd() to avoid the drop closing the file descriptor when
    // the File goes out of scope.
    unsafe {
        let temp_file = File::from_raw_filelike(fd);
        // Do not use a '?' here since that would cause the temp_file to be
        // dropped and the file descriptor closed.
        let result = temp_file.metadata();
        // Prevent the File from closing the file descriptor by consuming it.
        let _ = temp_file.into_raw_fd();
        result
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct FileId {
    dev: u64,
    ino: u64,
}

impl FileId {
    pub fn from_filelike(f: RawFilelike) -> io::Result<FileId> {
        Ok(FileId::from_metadata(&get_metadata_from_raw(f)?))
    }

    pub fn from_metadata(md: &Metadata) -> FileId {
        FileId { dev: md.dev(), ino: md.ino() }
    }
}

// Implementations of AsRawFd, FromRawFd, and IntoRawFd for File and RawFd for
// Unix-like systems:

impl<F> AsRawFd for crate::Handle<F>
where
    F: AsRawFd,
{
    fn as_raw_fd(&self) -> RawFd {
        // unwrap() will not panic. Since we were able to open the
        // file successfully, then `file` is guaranteed to be Some()
        self.handle.as_raw_fd()
    }
}

impl<F> FromRawFd for crate::Handle<F>
where
    F: AsRawFilelike + FromRawFd,
{
    unsafe fn from_raw_fd(fd: RawFd) -> crate::Handle<F> {
        let handle = unsafe { F::from_raw_fd(fd) };
        crate::Handle::from_file_like(handle).expect("from_raw_fd failed")
    }
}

impl<F> IntoRawFd for crate::Handle<F>
where
    F: IntoRawFd,
{
    fn into_raw_fd(self) -> RawFd {
        self.handle.into_raw_fd()
    }
}

pub fn open_file(path: &Path) -> io::Result<std::fs::File> {
    std::fs::OpenOptions::new().read(true).open(path)
}
