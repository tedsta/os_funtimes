use core::result;

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub struct Error {
    errno: u32,
}

impl Error {
    pub fn new(errno: u32) -> Error {
        Error { errno: errno }
    }
}

pub const EPERM: u32 = 1; // Insufficient permissions
pub const EIO: u32 = 2;
pub const ENOMEM: u32 = 3; // Out of memory
