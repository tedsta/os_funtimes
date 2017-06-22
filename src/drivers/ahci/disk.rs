use core::ptr;

use ::syscall::error::Result;
use ::syscall::io::{Dma, DmaAllocator};

use super::hba::{HbaPort, HbaCmdTable, HbaCmdHeader};

pub struct Disk<'a> {
    id: usize,
    port: &'static mut HbaPort,
    size: u64,
    clb: Dma<'a, [HbaCmdHeader; 32]>,
    ctbas: [Dma<'a, HbaCmdTable>; 32],
    _fb: Dma<'a, [u8; 256]>,
    buf: Dma<'a, [u8; 256 * 512]>
}

impl<'a> Disk<'a> {
    pub fn new(dma_alloc: &'a DmaAllocator,
               id: usize, port: &'static mut HbaPort) -> Result<Disk<'a>> {
        let mut clb = dma_alloc.allocate_zeroed()?;
        let mut ctbas = [
            dma_alloc.allocate_zeroed()?, dma_alloc.allocate_zeroed()?, dma_alloc.allocate_zeroed()?, dma_alloc.allocate_zeroed()?,
            dma_alloc.allocate_zeroed()?, dma_alloc.allocate_zeroed()?, dma_alloc.allocate_zeroed()?, dma_alloc.allocate_zeroed()?,
            dma_alloc.allocate_zeroed()?, dma_alloc.allocate_zeroed()?, dma_alloc.allocate_zeroed()?, dma_alloc.allocate_zeroed()?,
            dma_alloc.allocate_zeroed()?, dma_alloc.allocate_zeroed()?, dma_alloc.allocate_zeroed()?, dma_alloc.allocate_zeroed()?,
            dma_alloc.allocate_zeroed()?, dma_alloc.allocate_zeroed()?, dma_alloc.allocate_zeroed()?, dma_alloc.allocate_zeroed()?,
            dma_alloc.allocate_zeroed()?, dma_alloc.allocate_zeroed()?, dma_alloc.allocate_zeroed()?, dma_alloc.allocate_zeroed()?,
            dma_alloc.allocate_zeroed()?, dma_alloc.allocate_zeroed()?, dma_alloc.allocate_zeroed()?, dma_alloc.allocate_zeroed()?,
            dma_alloc.allocate_zeroed()?, dma_alloc.allocate_zeroed()?, dma_alloc.allocate_zeroed()?, dma_alloc.allocate_zeroed()?,
        ];
        let mut fb = dma_alloc.allocate_zeroed()?;
        let buf = dma_alloc.allocate_zeroed()?;

        port.init(&mut clb, &mut ctbas, &mut fb);

        let size = unsafe { port.identify(dma_alloc, &mut clb, &mut ctbas).unwrap_or(0) };

        Ok(Disk {
            id: id,
            port: port,
            size: size,
            clb: clb,
            ctbas: ctbas,
            _fb: fb,
            buf: buf
        })
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn read(&mut self, block: u64, buffer: &mut [u8]) -> Result<usize> {
        let sectors = buffer.len() / 512;

        let mut sector: usize = 0;
        while sectors - sector >= 255 {
            if let Err(err) = self.port.ata_dma(block + sector as u64, 255, false, &mut self.clb, &mut self.ctbas, &mut self.buf) {
                return Err(err);
            }

            unsafe { ptr::copy(self.buf.as_ptr(), buffer.as_mut_ptr().offset(sector as isize * 512), 255 * 512); }

            sector += 255;
        }
        if sector < sectors {
            if let Err(err) = self.port.ata_dma(block + sector as u64, sectors - sector,
                                                false, &mut self.clb, &mut self.ctbas,
                                                &mut self.buf)
            {
                return Err(err);
            }

            unsafe {
                ptr::copy(self.buf.as_ptr(), buffer.as_mut_ptr().offset(sector as isize * 512),
                          (sectors - sector) * 512);
            }

            sector += sectors - sector;
        }

        Ok(sector * 512)
    }

    pub fn write(&mut self, block: u64, buffer: &[u8]) -> Result<usize> {
        let sectors = buffer.len() / 512;

        let mut sector: usize = 0;
        while sectors - sector >= 255 {
            unsafe {
                ptr::copy(buffer.as_ptr().offset(sector as isize * 512),
                          self.buf.as_mut_ptr(), 255 * 512);
            }

            if let Err(err) = self.port.ata_dma(block + sector as u64, 255, true, 
                                                &mut self.clb, &mut self.ctbas, &mut self.buf)
            {
                return Err(err);
            }

            sector += 255;
        }
        if sector < sectors {
            unsafe { ptr::copy(buffer.as_ptr().offset(sector as isize * 512), self.buf.as_mut_ptr(), (sectors - sector) * 512); }

            if let Err(err) = self.port.ata_dma(block + sector as u64, sectors - sector, true,
                                                &mut self.clb, &mut self.ctbas, &mut self.buf)
            {
                return Err(err);
            }

            sector += sectors - sector;
        }

        Ok(sector * 512)
    }
}
