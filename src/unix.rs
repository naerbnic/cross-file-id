use std::fs::{File, OpenOptions};
use std::hash::{Hash, Hasher};
use std::io;
use std::os::unix::fs::MetadataExt;
use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd};
use std::path::Path;

use crate::AsRawOsFile;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct FileIdentity {
    dev: u64,
    ino: u64,
}

impl FileIdentity {
    pub fn from_os_file(f: RawOsFile) -> io::Result<FileIdentity> {
        // SAFETY: Although we create a File from the file descriptor, we use
        // into_raw_fd() to avoid the drop closing the file descriptor when
        // the File goes out of scope.
        let metadata = unsafe {
            let temp_file = File::from_raw_fd(f.0.as_raw_fd());
            // Do not use a '?' here since that would cause the temp_file to be
            // dropped and the file descriptor closed.
            let result = temp_file.metadata();
            // Prevent the File from closing the file descriptor by consuming it.
            let _ = temp_file.into_raw_fd();
            result
        }?;
        Ok(FileIdentity::from_metadata(&metadata))
    }

    pub fn from_metadata(md: &std::fs::Metadata) -> FileIdentity {
        FileIdentity { dev: md.dev(), ino: md.ino() }
    }

    pub fn dev(&self) -> u64 {
        self.dev
    }

    pub fn ino(&self) -> u64 {
        self.ino
    }
}

#[derive(Debug)]
pub struct Handle {
    file: Option<File>,
    // If is_std is true, then we don't drop the corresponding File since it
    // will close the handle.
    is_std: bool,
    id: FileIdentity,
}

impl Drop for Handle {
    fn drop(&mut self) {
        if self.is_std {
            // unwrap() will not panic. Since we were able to open an
            // std stream successfully, then `file` is guaranteed to be Some()
            #[expect(unused_must_use)]
            self.file.take().unwrap().into_raw_fd();
        }
    }
}

impl Eq for Handle {}

impl PartialEq for Handle {
    fn eq(&self, other: &Handle) -> bool {
        self.id == other.id
    }
}

impl AsRawFd for crate::Handle {
    fn as_raw_fd(&self) -> RawFd {
        // unwrap() will not panic. Since we were able to open the
        // file successfully, then `file` is guaranteed to be Some()
        self.0.file.as_ref().unwrap().as_raw_fd()
    }
}

impl IntoRawFd for crate::Handle {
    fn into_raw_fd(mut self) -> RawFd {
        // unwrap() will not panic. Since we were able to open the
        // file successfully, then `file` is guaranteed to be Some()
        self.0.file.take().unwrap().into_raw_fd()
    }
}

impl Hash for Handle {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Handle {
    pub fn from_path<P: AsRef<Path>>(p: P) -> io::Result<Handle> {
        Handle::from_file(OpenOptions::new().read(true).open(p)?)
    }

    pub fn from_file(file: File) -> io::Result<Handle> {
        let md = file.metadata()?;
        Ok(Handle {
            file: Some(file),
            is_std: false,
            id: FileIdentity::from_metadata(&md),
        })
    }

    pub fn from_std(file: File) -> io::Result<Handle> {
        Handle::from_file(file).map(|mut h| {
            h.is_std = true;
            h
        })
    }

    pub fn stdin() -> io::Result<Handle> {
        Handle::from_std(unsafe { File::from_raw_fd(0) })
    }

    pub fn stdout() -> io::Result<Handle> {
        Handle::from_std(unsafe { File::from_raw_fd(1) })
    }

    pub fn stderr() -> io::Result<Handle> {
        Handle::from_std(unsafe { File::from_raw_fd(2) })
    }

    pub fn as_file(&self) -> &File {
        // unwrap() will not panic. Since we were able to open the
        // file successfully, then `file` is guaranteed to be Some()
        self.file.as_ref().unwrap()
    }

    pub fn as_file_mut(&mut self) -> &mut File {
        // unwrap() will not panic. Since we were able to open the
        // file successfully, then `file` is guaranteed to be Some()
        self.file.as_mut().unwrap()
    }

    pub fn id(&self) -> FileIdentity {
        self.id
    }

    pub fn dev(&self) -> u64 {
        self.id.dev()
    }

    pub fn ino(&self) -> u64 {
        self.id.ino()
    }
}

/// A blanket implementation of AsRawOsFile for any types that implement
/// AsRawFd.
impl<T> AsRawOsFile for T
where
    T: AsRawFd,
{
    fn as_raw_os_file(&self) -> crate::RawOsFile<'_> {
        crate::RawOsFile(RawOsFile(self.as_raw_fd(), std::marker::PhantomData))
    }
}

pub struct RawOsFile<'a>(
    std::os::unix::io::RawFd,
    std::marker::PhantomData<&'a ()>,
);
