/*!
This crate provides a safe and simple **cross platform** way to represent the
identity of a file within a filesystem.

The fundamental type provided by this crate is [`FileId`], which is equatable,
orderable, hashable, and copyable. This is a low-level type that can provide
useful file identity functionality, but must follow additional rules to
ensure that the identity remains valid.

Other types are provided to provide a "safer" interface for using file identity
which ensures that the file remains open for the lifetime of the identity.
*/
#![warn(missing_docs)]

#[cfg(doctest)]
doc_comment::doctest!("../README.md");

use std::io::{self, Stderr, Stdout};
use std::path::Path;
use std::{fs::File, io::Stdin};

use io_lifetimes::raw::{AsRawFilelike, RawFilelike};

// Import the platform-specific implementation.
#[cfg_attr(unix, path = "unix.rs")]
#[cfg_attr(windows, path = "win.rs")]
#[cfg_attr(not(any(unix, windows)), path = "unknown.rs")]
mod imp;

/// A cross-platform representation of a file's identity.
///
/// This represents an OS unique identifier for a file. Two files with the same
/// identity are guaranteed to be the same file, but only while the files they
/// refer to exist. On supported platforms, as long as the file is opened by
/// this process, the identity will remain valid even if the file is deleted or
/// renamed.
///
/// This does not hold onto any system resources, so it is safe to store and
/// copy, but if the safety of the program is dependent on the identity
/// remaining valid, then the file must be kept open by this process.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FileId(imp::FileId);

impl FileId {
    /// Extract a file identity from any type that implements the
    /// [`AsRawOsFile`] trait, and thus the platform-specific traits
    /// that provide access to raw OS representations of files.
    ///
    /// This does not take ownership of the OS file or alter its state.
    pub fn from_file_like<F: AsRawFilelike>(file: &F) -> io::Result<Self> {
        Self::from_raw(file.as_raw_filelike())
    }

    /// Extract a file identity from a raw OS file descriptor or handle.
    ///
    /// This does not take ownership of the OS file or alter its state.
    pub fn from_raw(os_file: RawFilelike) -> io::Result<Self> {
        imp::FileId::from_filelike(os_file).map(FileId)
    }
}

/// A handle to a file that can be tested for equality with other handles.
///
/// If two files are the same, then any two handles of those files will compare
/// equal. If two files are not the same, then any two handles of those files
/// will compare not-equal.
///
/// A handle consumes an open file resource as long as it exists.
///
/// Equality is determined by comparing the platform-specific file identity
/// associated with the handle. If created from a file-like object (via
/// [`from_file_like`]), then the handle will ensure that the file identity
/// remains valid for the lifetime of the handle.
#[derive(Debug)]
pub struct Handle<F> {
    handle: F,
    identity: FileId,
}

impl<F> Handle<F> {
    /// Construct a handle from its parts.
    ///
    /// # Safety
    ///
    /// This may be unsafe as the Handle type guarantees that the identity
    /// gives a true representation of the file referred to by the handle.
    /// This is only the case while the file itself stays open, so the caller
    /// must ensure that the value of type F ensures that the file remains
    /// open for the lifetime of the Handle.
    pub unsafe fn from_parts(handle: F, identity: FileId) -> Self {
        Handle { handle, identity }
    }

    /// Consume the handle and return the underlying file-like object.
    ///
    /// This is provided as an associated function instead of a method
    /// to ensure that operations that rely on the value being accessible via
    /// dereference aren't accidentally masked.
    pub fn into_inner(this: Self) -> F {
        this.handle
    }

    /// Get the file identity for this handle.
    ///
    /// This is provided as an associated function instead of a method
    /// to ensure that operations that rely on the value being accessible via
    /// dereference aren't accidentally masked.
    pub fn id(this: Self) -> FileId {
        this.identity.clone()
    }
}

impl<F> Handle<F>
where
    F: AsRawFilelike,
{
    /// Construct a handle from any type that implements the platform-specific
    /// traits that provide access to raw OS representations of files.
    ///
    /// The resulting handle will act as a wrapper around the given file-like
    /// object, and will ensure that the file remains open for the lifetime of
    /// the handle.
    pub fn from_file_like(file: F) -> io::Result<Self> {
        let file_id = FileId::from_file_like(&file)?;
        Ok(Handle { handle: file, identity: file_id })
    }
}

impl<F> std::ops::Deref for Handle<F> {
    type Target = F;

    fn deref(&self) -> &F {
        &self.handle
    }
}

impl<F> std::ops::DerefMut for Handle<F> {
    fn deref_mut(&mut self) -> &mut F {
        &mut self.handle
    }
}

impl<F1, F2> std::cmp::PartialEq<Handle<F2>> for Handle<F1> {
    fn eq(&self, other: &Handle<F2>) -> bool {
        self.identity == other.identity
    }
}

impl<F> std::cmp::Eq for Handle<F> {}

impl<F1, F2> std::cmp::PartialOrd<Handle<F2>> for Handle<F1> {
    fn partial_cmp(&self, other: &Handle<F2>) -> Option<std::cmp::Ordering> {
        self.identity.partial_cmp(&other.identity)
    }
}

impl<F> std::cmp::Ord for Handle<F> {
    fn cmp(&self, other: &Handle<F>) -> std::cmp::Ordering {
        self.identity.cmp(&other.identity)
    }
}

impl Handle<File> {
    /// Construct a handle from a path.
    ///
    /// Note that the underlying [`File`] is opened in read-only mode on all
    /// platforms.
    ///
    /// [`File`]: https://doc.rust-lang.org/std/fs/struct.File.html
    ///
    /// # Errors
    /// This method will return an [`io::Error`] if the path cannot
    /// be opened, or the file's metadata cannot be obtained.
    /// The most common reasons for this are: the path does not
    /// exist, or there were not enough permissions.
    ///
    /// [`io::Error`]: https://doc.rust-lang.org/std/io/struct.Error.html
    ///
    /// # Examples
    /// Check that two paths are not the same file:
    ///
    /// ```rust,no_run
    /// # use std::error::Error;
    /// use cross_file_id::Handle;
    ///
    /// # fn try_main() -> Result<(), Box<dyn Error>> {
    /// let source = Handle::from_path("./source")?;
    /// let target = Handle::from_path("./target")?;
    /// assert_ne!(source, target, "The files are the same.");
    /// # Ok(())
    /// # }
    /// #
    /// # fn main() {
    /// #     try_main().unwrap();
    /// # }
    /// ```
    pub fn from_path<P: AsRef<Path>>(p: P) -> io::Result<Self> {
        let file = std::fs::File::open(p)?;
        Self::from_file_like(file)
    }

    /// Construct a handle from a file.
    ///
    /// # Errors
    /// This method will return an [`io::Error`] if the metadata for
    /// the given [`File`] cannot be obtained.
    ///
    /// [`io::Error`]: https://doc.rust-lang.org/std/io/struct.Error.html
    /// [`File`]: https://doc.rust-lang.org/std/fs/struct.File.html
    ///
    /// # Examples
    /// Check that two files are not in fact the same file:
    ///
    /// ```rust,no_run
    /// # use std::error::Error;
    /// # use std::fs::File;
    /// use cross_file_id::Handle;
    ///
    /// # fn try_main() -> Result<(), Box<dyn Error>> {
    /// let source = File::open("./source")?;
    /// let target = File::open("./target")?;
    ///
    /// assert_ne!(
    ///     Handle::from_file(source)?,
    ///     Handle::from_file(target)?,
    ///     "The files are the same."
    /// );
    /// #     Ok(())
    /// # }
    /// #
    /// # fn main() {
    /// #     try_main().unwrap();
    /// # }
    /// ```
    pub fn from_file(file: File) -> io::Result<Self> {
        Self::from_file_like(file)
    }
}

impl Handle<Stdin> {
    /// Construct a handle from stdin.
    ///
    /// # Errors
    /// This method will return an [`io::Error`] if stdin cannot
    /// be opened due to any I/O-related reason.
    ///
    /// [`io::Error`]: https://doc.rust-lang.org/std/io/struct.Error.html
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use std::error::Error;
    /// use cross_file_id::Handle;
    ///
    /// # fn try_main() -> Result<(), Box<dyn Error>> {
    /// let stdin = Handle::stdin()?;
    /// let stdout = Handle::stdout()?;
    /// let stderr = Handle::stderr()?;
    ///
    /// if stdin == stdout {
    ///     println!("stdin == stdout");
    /// }
    /// if stdin == stderr {
    ///     println!("stdin == stderr");
    /// }
    /// if stdout == stderr {
    ///     println!("stdout == stderr");
    /// }
    /// #
    /// #     Ok(())
    /// # }
    /// #
    /// # fn main() {
    /// #     try_main().unwrap();
    /// # }
    /// ```
    ///
    /// The output differs depending on the platform.
    ///
    /// On Linux:
    ///
    /// ```text
    /// $ ./example
    /// stdin == stdout
    /// stdin == stderr
    /// stdout == stderr
    /// $ ./example > result
    /// $ cat result
    /// stdin == stderr
    /// $ ./example > result 2>&1
    /// $ cat result
    /// stdout == stderr
    /// ```
    ///
    /// Windows:
    ///
    /// ```text
    /// > example
    /// > example > result 2>&1
    /// > type result
    /// stdout == stderr
    /// ```
    pub fn stdin() -> io::Result<Handle<Stdin>> {
        Self::from_file_like(std::io::stdin())
    }
}

impl Handle<Stdout> {
    /// Construct a handle from stdout.
    ///
    /// # Errors
    /// This method will return an [`io::Error`] if stdout cannot
    /// be opened due to any I/O-related reason.
    ///
    /// [`io::Error`]: https://doc.rust-lang.org/std/io/struct.Error.html
    ///
    /// # Examples
    /// See the example for [`stdin()`].
    ///
    /// [`stdin()`]: #method.stdin
    pub fn stdout() -> io::Result<Self> {
        Self::from_file_like(std::io::stdout())
    }
}

impl Handle<Stderr> {
    /// Construct a handle from stderr.
    ///
    /// # Errors
    /// This method will return an [`io::Error`] if stderr cannot
    /// be opened due to any I/O-related reason.
    ///
    /// [`io::Error`]: https://doc.rust-lang.org/std/io/struct.Error.html
    ///
    /// # Examples
    /// See the example for [`stdin()`].
    ///
    /// [`stdin()`]: #method.stdin
    pub fn stderr() -> io::Result<Self> {
        Self::from_file_like(std::io::stderr())
    }
}

/// Returns true if the two file paths may correspond to the same file.
///
/// Note that it's possible for this to produce a false positive on some
/// platforms. Namely, this can return true even if the two file paths *don't*
/// resolve to the same file.
/// # Errors
/// This function will return an [`io::Error`] if any of the two paths cannot
/// be opened. The most common reasons for this are: the path does not exist,
/// or there were not enough permissions.
///
/// [`io::Error`]: https://doc.rust-lang.org/std/io/struct.Error.html
///
/// # Example
///
/// ```rust,no_run
/// use cross_file_id::is_same_file;
///
/// assert!(is_same_file("./foo", "././foo").unwrap_or(false));
/// ```
pub fn is_same_file<P, Q>(path1: P, path2: Q) -> io::Result<bool>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    Ok(Handle::from_path(path1)? == Handle::from_path(path2)?)
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::error;
    use std::fs::{self, File};
    use std::io;
    use std::path::{Path, PathBuf};
    use std::result;

    use super::is_same_file;

    type Result<T> = result::Result<T, Box<dyn error::Error + Send + Sync>>;

    /// Create an error from a format!-like syntax.
    macro_rules! err {
        ($($tt:tt)*) => {
            Box::<dyn error::Error + Send + Sync>::from(format!($($tt)*))
        }
    }

    /// A simple wrapper for creating a temporary directory that is
    /// automatically deleted when it's dropped.
    ///
    /// We use this in lieu of tempfile because tempfile brings in too many
    /// dependencies.
    #[derive(Debug)]
    struct TempDir(PathBuf);

    impl Drop for TempDir {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    impl TempDir {
        /// Create a new empty temporary directory under the system's
        /// configured temporary directory.
        fn new() -> Result<TempDir> {
            #![allow(deprecated)]

            use std::sync::atomic::{
                AtomicUsize, Ordering, ATOMIC_USIZE_INIT,
            };

            static TRIES: usize = 100;
            static COUNTER: AtomicUsize = ATOMIC_USIZE_INIT;

            let tmpdir = env::temp_dir();
            for _ in 0..TRIES {
                let count = COUNTER.fetch_add(1, Ordering::SeqCst);
                let path = tmpdir.join("rust-walkdir").join(count.to_string());
                if path.is_dir() {
                    continue;
                }
                fs::create_dir_all(&path).map_err(|e| {
                    err!("failed to create {}: {}", path.display(), e)
                })?;
                return Ok(TempDir(path));
            }
            Err(err!("failed to create temp dir after {} tries", TRIES))
        }

        /// Return the underlying path to this temporary directory.
        fn path(&self) -> &Path {
            &self.0
        }
    }

    fn tmpdir() -> TempDir {
        TempDir::new().unwrap()
    }

    #[cfg(unix)]
    pub fn soft_link_dir<P: AsRef<Path>, Q: AsRef<Path>>(
        src: P,
        dst: Q,
    ) -> io::Result<()> {
        use std::os::unix::fs::symlink;
        symlink(src, dst)
    }

    #[cfg(unix)]
    pub fn soft_link_file<P: AsRef<Path>, Q: AsRef<Path>>(
        src: P,
        dst: Q,
    ) -> io::Result<()> {
        soft_link_dir(src, dst)
    }

    #[cfg(windows)]
    pub fn soft_link_dir<P: AsRef<Path>, Q: AsRef<Path>>(
        src: P,
        dst: Q,
    ) -> io::Result<()> {
        use std::os::windows::fs::symlink_dir;
        symlink_dir(src, dst)
    }

    #[cfg(windows)]
    pub fn soft_link_file<P: AsRef<Path>, Q: AsRef<Path>>(
        src: P,
        dst: Q,
    ) -> io::Result<()> {
        use std::os::windows::fs::symlink_file;
        symlink_file(src, dst)
    }

    // These tests are rather uninteresting. The really interesting tests
    // would stress the edge cases. On Unix, this might be comparing two files
    // on different mount points with the same inode number. On Windows, this
    // might be comparing two files whose file indices are the same on file
    // systems where such things aren't guaranteed to be unique.
    //
    // Alas, I don't know how to create those environmental conditions. ---AG

    #[test]
    fn same_file_trivial() {
        let tdir = tmpdir();
        let dir = tdir.path();

        File::create(dir.join("a")).unwrap();
        assert!(is_same_file(dir.join("a"), dir.join("a")).unwrap());
    }

    #[test]
    fn same_dir_trivial() {
        let tdir = tmpdir();
        let dir = tdir.path();

        fs::create_dir(dir.join("a")).unwrap();
        assert!(is_same_file(dir.join("a"), dir.join("a")).unwrap());
    }

    #[test]
    fn not_same_file_trivial() {
        let tdir = tmpdir();
        let dir = tdir.path();

        File::create(dir.join("a")).unwrap();
        File::create(dir.join("b")).unwrap();
        assert!(!is_same_file(dir.join("a"), dir.join("b")).unwrap());
    }

    #[test]
    fn not_same_dir_trivial() {
        let tdir = tmpdir();
        let dir = tdir.path();

        fs::create_dir(dir.join("a")).unwrap();
        fs::create_dir(dir.join("b")).unwrap();
        assert!(!is_same_file(dir.join("a"), dir.join("b")).unwrap());
    }

    #[test]
    fn same_file_hard() {
        let tdir = tmpdir();
        let dir = tdir.path();

        File::create(dir.join("a")).unwrap();
        fs::hard_link(dir.join("a"), dir.join("alink")).unwrap();
        assert!(is_same_file(dir.join("a"), dir.join("alink")).unwrap());
    }

    #[test]
    fn same_file_soft() {
        let tdir = tmpdir();
        let dir = tdir.path();

        File::create(dir.join("a")).unwrap();
        soft_link_file(dir.join("a"), dir.join("alink")).unwrap();
        assert!(is_same_file(dir.join("a"), dir.join("alink")).unwrap());
    }

    #[test]
    fn same_dir_soft() {
        let tdir = tmpdir();
        let dir = tdir.path();

        fs::create_dir(dir.join("a")).unwrap();
        soft_link_dir(dir.join("a"), dir.join("alink")).unwrap();
        assert!(is_same_file(dir.join("a"), dir.join("alink")).unwrap());
    }

    #[test]
    fn test_send() {
        fn assert_send<T: Send>() {}
        assert_send::<super::Handle<File>>();
    }

    #[test]
    fn test_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<super::Handle<File>>();
    }
}
