use collections::{String, Vec};

pub use self::disk::Disk;
use self::hba::{HbaMem, HbaPortType};

use ::memory;
use ::drivers::pci::{PciBar, PciFunc, PciHeader};
use ::syscall::io::{DmaAllocator, Io};

mod disk;
mod fis;
mod hba;

pub fn init(pci_func: PciFunc, pci_header: PciHeader) {
    println!("Starting AHCI driver");

    let dma_alloc = DmaAllocator::new(33691392);
    let bar = match pci_header.bar(5) {
        PciBar::Memory(bar) => bar as usize,
        _ => {
            println!("Failed to initialize AHCI driver, BAR[5] is not a memory BAR");
            return;
        }
    };
    let bar_virt = ::syscall::map_pm(bar, 4096, memory::WRITABLE).unwrap();
    let mut disks = disks(&dma_alloc, bar_virt);

    let mut buf = [0u8; 512];

    disks[0].read(0, &mut buf);
    let mut msg = String::from_utf8(buf.iter().cloned().take_while(|c| *c != 0).collect()).unwrap();
    println!("{:?}", msg);
}

pub fn disks<'a>(dma_alloc: &'a DmaAllocator, base: usize) -> Vec<Disk<'a>> {
    let hba_mem = unsafe { &mut *(base as *mut HbaMem) };
    hba_mem.init();
    let pi = hba_mem.pi.read();
    let ret: Vec<Disk> = (0..32)
          .filter(|&i| pi & 1 << i as i32 == 1 << i as i32)
          .filter_map(|i| {
              let port = &mut unsafe { &mut *(base as *mut HbaMem) }.ports[i];
              let port_type = port.probe();
              println!("disk {}: {:?}", i, port_type);
              match port_type {
                  HbaPortType::SATA => {
                      match Disk::new(dma_alloc, i, port) {
                          Ok(disk) => Some(disk),
                          Err(err) => {
                              println!("{}: {:?}", i, err);
                              None
                          }
                      }
                  }
                  _ => None,
              }
          })
          .collect();

    ret
}
