use io_lifetimes::raw::{FromRawFilelike, RawFilelike};
use std::hash::{Hash, Hasher};
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::io::{AsRawHandle, IntoRawHandle, RawHandle};
use std::path::Path;
use windows::Win32::Foundation::GENERIC_READ;
use windows::core::PCWSTR;

use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAG_BACKUP_SEMANTICS, FILE_ID_128, FILE_ID_INFO,
    FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, FILE_TYPE_DISK,
    FileIdInfo, GetFileInformationByHandleEx, GetFileType, OPEN_EXISTING,
};

// For correctness, it is critical that both file handles remain open while
// their attributes are checked for equality. In particular, the file index
// numbers on a Windows stat object are not guaranteed to remain stable over
// time.
//
// See the docs and remarks on MSDN:
// https://msdn.microsoft.com/en-us/library/windows/desktop/aa363788(v=vs.85).aspx
//
// It gets worse. It appears that the index numbers are not always
// guaranteed to be unique. Namely, ReFS uses 128 bit numbers for unique
// identifiers. This requires a distinct syscall to get `FILE_ID_INFO`
// documented here:
// https://msdn.microsoft.com/en-us/library/windows/desktop/hh802691(v=vs.85).aspx
//
// It seems straight-forward enough to modify this code to use
// `FILE_ID_INFO` when available (minimum Windows Server 2012), but I don't
// have access to such Windows machines.
//
// Two notes.
//
// 1. Java's NIO uses the approach implemented here and appears to ignore
//    `FILE_ID_INFO` altogether. So Java's NIO and this code are
//    susceptible to bugs when running on a file system where
//    `nFileIndex{Low,High}` are not unique.
//
// 2. LLVM has a bug where they fetch the id of a file and continue to use
//    it even after the handle has been closed, so that uniqueness is no
//    longer guaranteed (when `nFileIndex{Low,High}` are unique).
//    bug report: http://lists.llvm.org/pipermail/llvm-bugs/2014-December/037218.html
//
// All said and done, checking whether two files are the same on Windows
// seems quite tricky. Moreover, even if the code is technically incorrect,
// it seems like the chances of actually observing incorrect behavior are
// extremely small. Nevertheless, we mitigate this by checking size too.
//
// In the case where this code is erroneous, two files will be reported
// as equivalent when they are in fact distinct. This will cause the loop
// detection code to report a false positive, which will prevent descending
// into the offending directory. As far as failure modes goes, this isn't
// that bad.

fn compare_file_id_128(a: FILE_ID_128, b: FILE_ID_128) -> std::cmp::Ordering {
    a.Identifier.cmp(&b.Identifier)
}

#[derive(Debug, Clone, PartialEq)]
pub struct FileId {
    file_id_info: FILE_ID_INFO,
}

impl Eq for FileId {}

impl PartialOrd for FileId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FileId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.file_id_info
            .VolumeSerialNumber
            .cmp(&other.file_id_info.VolumeSerialNumber)
            .then_with(|| {
                compare_file_id_128(
                    self.file_id_info.FileId,
                    other.file_id_info.FileId,
                )
            })
    }
}

impl Hash for FileId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.file_id_info.VolumeSerialNumber);
        state.write(&self.file_id_info.FileId.Identifier);
    }
}

impl FileId {
    pub fn from_filelike(f: RawFilelike) -> io::Result<FileId> {
        let file_id_info = unsafe {
            let handle = windows::Win32::Foundation::HANDLE(f);
            let file_type = GetFileType(handle);
            if file_type != FILE_TYPE_DISK {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "Unable to get information about handle of type {:?}",
                        file_type
                    ),
                ));
            }
            let mut info = FILE_ID_INFO::default();
            GetFileInformationByHandleEx(
                handle,
                FileIdInfo,
                &mut info as *mut FILE_ID_INFO as *mut _,
                std::mem::size_of::<FILE_ID_INFO>() as u32,
            )?;
            info
        };

        Ok(FileId { file_id_info })
    }
}

impl<F> AsRawHandle for crate::Handle<F>
where
    F: AsRawHandle,
{
    fn as_raw_handle(&self) -> RawHandle {
        self.handle.as_raw_handle()
    }
}

impl<F> IntoRawHandle for crate::Handle<F>
where
    F: IntoRawHandle,
{
    fn into_raw_handle(self) -> RawHandle {
        self.handle.into_raw_handle()
    }
}

pub fn open_file(path: &Path) -> io::Result<std::fs::File> {
    let wide_path: Vec<_> =
        path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    let file = unsafe {
        let handle = CreateFileW(
            PCWSTR::from_raw(wide_path.as_ptr()),
            GENERIC_READ.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            None,
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS,
            None,
        )?;
        std::fs::File::from_raw_filelike(handle.0)
    };
    Ok(file)
}
