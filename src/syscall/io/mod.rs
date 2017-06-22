pub use self::dma::{DmaAllocator, Dma};
pub use self::io::Io;
pub use self::mmio::Mmio;
pub use self::pio::Pio;

mod dma;
mod io;
mod pio;
mod mmio;
