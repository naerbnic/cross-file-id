use cross_file_id::is_same_file_path;
use std::io;

fn try_main() -> Result<(), io::Error> {
    assert!(is_same_file_path("/bin/sh", "/usr/bin/sh")?);
    Ok(())
}

fn main() {
    try_main().unwrap();
}
