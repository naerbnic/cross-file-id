use std::fs::File;
use std::io;
use std::path::Path;

use std::convert::Infallible as Never;

static ERROR_MESSAGE: &str = "same-file is not supported on this platform.";

#[derive(Debug, Clone, Copy, Eq, Hash)]
pub struct FileIdentity(Never);

impl FileIdentity {
    pub fn from_os_file(_f: RawOsFile) -> io::Result<FileIdentity> {
        error()
    }
}

impl PartialEq for FileIdentity {
    fn eq(&self, _other: &FileIdentity) -> bool {
        match self.0 {}
    }
}

impl PartialOrd for FileIdentity {
    fn partial_cmp(
        &self,
        _other: &FileIdentity,
    ) -> Option<std::cmp::Ordering> {
        match self.0 {}
    }
}

impl Ord for FileIdentity {
    fn cmp(&self, _other: &FileIdentity) -> std::cmp::Ordering {
        match self.0 {}
    }
}

// This implementation is to allow same-file to be compiled on
// unsupported platforms in case it was incidentally included
// as a transitive, unused dependency
#[derive(Debug, Hash)]
pub struct Handle(Never);

impl Eq for Handle {}

impl PartialEq for Handle {
    fn eq(&self, _other: &Handle) -> bool {
        match self.0 {}
    }
}

impl Handle {
    pub fn from_path<P: AsRef<Path>>(_p: P) -> io::Result<Handle> {
        error()
    }

    pub fn from_file(_file: File) -> io::Result<Handle> {
        error()
    }

    pub fn stdin() -> io::Result<Handle> {
        error()
    }

    pub fn stdout() -> io::Result<Handle> {
        error()
    }

    pub fn stderr() -> io::Result<Handle> {
        error()
    }

    pub fn as_file(&self) -> &File {
        match self.0 {}
    }

    pub fn as_file_mut(&self) -> &mut File {
        match self.0 {}
    }
}

fn error<T>() -> io::Result<T> {
    Err(io::Error::new(io::ErrorKind::Other, ERROR_MESSAGE))
}

pub struct RawOsFile<'a>(Never, std::marker::PhantomData<&'a ()>);
