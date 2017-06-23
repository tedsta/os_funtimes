use core::cell::Cell;
use core::{mem, ptr};
use core::ops::{Deref, DerefMut};

use ::memory::{self, VirtualAddress, PhysicalAddress, with_mem_ctrl, WRITABLE};
use ::syscall::{self, Result};

pub struct DmaAllocator {
    start: VirtualAddress,
    start_phys: PhysicalAddress,
    size: usize,
    next_free_byte: Cell<usize>, // Offset of next free byte
}

impl DmaAllocator {
    /// Create a DmaAllocator contiguous chunk of unmapped virtual memory
    pub fn new(size: usize) -> DmaAllocator {
        let start = syscall::alloc_vm(size, WRITABLE).unwrap();
        let start_phys =
            with_mem_ctrl(|m| {
                m.translate_address(start).unwrap()
            });
        DmaAllocator {
            start: start,
            start_phys: start_phys,
            size: size,
            next_free_byte: Cell::new(0),
        }
    }

    pub fn allocate<'a, T>(&'a self, value: T) -> Result<Dma<'a, T>> {
        let addr_virt = self.start + self.next_free_byte.get();
        let addr_phys = self.start_phys + self.next_free_byte.get();
        self.next_free_byte.set(self.next_free_byte.get() + mem::size_of::<T>());
        println!("next free byte {} size {}", self.next_free_byte.get(), self.size);
        if self.next_free_byte.get() < self.size {
            Ok(Dma::new(addr_virt, addr_phys, value))
        } else {
            Err(syscall::Error::new(syscall::error::ENOMEM))
        }
    }

    pub fn allocate_zeroed<'a, T>(&'a self) -> Result<Dma<'a, T>> {
        let addr_virt = self.start + self.next_free_byte.get();
        let addr_phys = self.start_phys + self.next_free_byte.get();
        self.next_free_byte.set(self.next_free_byte.get() + mem::size_of::<T>());
        println!("next free byte {} size {}", self.next_free_byte.get(), self.size);
        if self.next_free_byte.get() < self.size {
            Ok(Dma::zeroed(addr_virt, addr_phys))
        } else {
            Err(syscall::Error::new(syscall::error::ENOMEM))
        }
    }
}

impl Drop for DmaAllocator {
    fn drop(&mut self) {
        let _ = unsafe { syscall::free_vm(self.start) };
    }
}

pub struct Dma<'a, T: 'a> {
    virt: &'a mut T,
    phys: PhysicalAddress,
}

impl<'a, T: 'a> Dma<'a, T> {
    fn new(virt: VirtualAddress, phys: PhysicalAddress, value: T) -> Dma<'a, T> {
        let virt = unsafe { virt as *mut T };
        unsafe { ptr::write(virt, value); }
        Dma {
            virt: unsafe { mem::transmute(virt) },
            phys: phys,
        }
    }

    pub fn zeroed(virt: VirtualAddress, phys: PhysicalAddress) -> Dma<'a, T> {
        let virt = unsafe { virt as *mut T };
        unsafe { ptr::write_bytes(virt as *mut u8, 0, mem::size_of::<T>()); }
        Dma {
            virt: unsafe { mem::transmute(virt) },
            phys: phys,
        }
    }

    pub fn physical(&self) -> PhysicalAddress {
        self.phys
    }
}

impl<'a, T> Deref for Dma<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.virt }
    }
}

impl<'a, T> DerefMut for Dma<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.virt }
    }
}

impl<'a, T> Drop for Dma<'a, T> {
    fn drop(&mut self) {
        //unsafe { drop(ptr::read(self.virt)); }
    }
}
