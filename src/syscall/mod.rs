pub use self::error::{Error, Result};
pub use self::memory::{alloc_grant, free_grant};

pub mod error;
pub mod io;

mod memory;
