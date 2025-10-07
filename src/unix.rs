use std::fs::{File, Metadata};
use std::hash::Hash;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd};

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

// #[derive(Debug)]
// pub struct Handle {
//     file: Option<File>,
//     // If is_std is true, then we don't drop the corresponding File since it
//     // will close the handle.
//     is_std: bool,
//     id: FileId,
// }

// impl Drop for Handle {
//     fn drop(&mut self) {
//         if self.is_std {
//             // unwrap() will not panic. Since we were able to open an
//             // std stream successfully, then `file` is guaranteed to be Some()
//             #[expect(unused_must_use)]
//             self.file.take().unwrap().into_raw_fd();
//         }
//     }
// }

// impl Eq for Handle {}

// impl PartialEq for Handle {
//     fn eq(&self, other: &Handle) -> bool {
//         self.id == other.id
//     }
// }

// impl Hash for Handle {
//     fn hash<H: Hasher>(&self, state: &mut H) {
//         self.id.hash(state);
//     }
// }

// impl Handle {
//     pub fn from_path<P: AsRef<Path>>(p: P) -> io::Result<Handle> {
//         Handle::from_file(OpenOptions::new().read(true).open(p)?)
//     }

//     pub fn from_file(file: File) -> io::Result<Handle> {
//         let md = file.metadata()?;
//         Ok(Handle {
//             file: Some(file),
//             is_std: false,
//             id: FileId::from_metadata(&md),
//         })
//     }

//     pub fn from_std(file: File) -> io::Result<Handle> {
//         Handle::from_file(file).map(|mut h| {
//             h.is_std = true;
//             h
//         })
//     }

//     pub fn stdin() -> io::Result<Handle> {
//         Handle::from_std(unsafe { File::from_raw_fd(0) })
//     }

//     pub fn stdout() -> io::Result<Handle> {
//         Handle::from_std(unsafe { File::from_raw_fd(1) })
//     }

//     pub fn stderr() -> io::Result<Handle> {
//         Handle::from_std(unsafe { File::from_raw_fd(2) })
//     }

//     pub fn as_file(&self) -> &File {
//         // unwrap() will not panic. Since we were able to open the
//         // file successfully, then `file` is guaranteed to be Some()
//         self.file.as_ref().unwrap()
//     }

//     pub fn as_file_mut(&mut self) -> &mut File {
//         // unwrap() will not panic. Since we were able to open the
//         // file successfully, then `file` is guaranteed to be Some()
//         self.file.as_mut().unwrap()
//     }

//     pub fn id(&self) -> FileId {
//         self.id
//     }

//     pub fn dev(&self) -> u64 {
//         self.id.dev()
//     }

//     pub fn ino(&self) -> u64 {
//         self.id.ino()
//     }
// }

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
        let handle = F::from_raw_fd(fd);
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
