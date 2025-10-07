# `cross-file-id`

This crate provides primitives for working with cross-platform file identity,
with a focus on the identities of currently open files.

Most operating systems have some way of distinguishing different files
on the filesystem from each other. It can be important to know if two file
objects are the same in order to prevent Time-of-Check/Time-of-Use errors
or vulnerabilities.

This library provides a high-level and low-level API. The low-level API provides
the `FileId` type which represents a cross-platform file identity value that is
cheap to clone, and can be used as a key in ordered or hashed contexts. While it
can provide reliable file identity, it only works as long as the file exists,
which generally means you can only be certain of it while the file is kept
open by the program.

The high-level API is a `Handle` which ensures that the file whose identity
it contains will be kept open. As such, it can be guaranteed that its value is
stable and represents true file identity as long as the underlying file does
not change while the value is kept.

Example:

```rust,no_run
use cross_file_id::FileId;

let path = "/tmp/test/path.txt";

let file1 = std::fs::File::create(path)?;
let file2 = std::fs::File::create(path)?;

let file_id1 = FileId::from_file_like(&file1)?;
let file_id2 = FileId::from_file_like(&file2)?;
assert!(file_id1 == file_id2);
# Ok::<_, std::io::Error>(())
```

## Related Crates

### `same-file`

Much of this code is derived from the `same-file` crate by BurntSushi. This
crate has a slightly different focus from that one, although some of the API is
similar. In particular, the `FileId` type is a separate cheap to clone type that
provides a comparable file ID, but requires some care to use correctly.

### `file-id`

This crate does provide a `FileId` type, but the type makes all of the platform
specific fields part of its public interface, making it hard to add new API
representations or other necessary changes. In addition it only works on paths,
not file descriptors/handles.
