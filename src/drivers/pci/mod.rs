use collections::BTreeMap;

pub use self::bar::PciBar;
pub use self::class::PciClass;
pub use self::header::PciHeader;

mod bar;
mod class;
mod header;

pub type PciDeviceDriver = fn(PciFunc, PciHeader);

pub struct PciFunc {
    pub bus: u8,
    pub dev: u8,
    pub func: u8,
}

impl PciFunc {
    pub fn new(bus: u8, dev: u8, func: u8) -> PciFunc {
        PciFunc {
            bus: bus,
            dev: dev,
            func: func,
        }
    }

    pub fn header(&self) -> Option<PciHeader> {
        // The header's binary representation will be all 1's if there's no PCI device
        if unsafe { self.read(0) } != 0xFFFFFFFF {
            let mut header = PciHeader::default();
            {
                let dwords = header.as_dwords_mut();
                /*for (header_dword, offset) in dwords.iter_mut().zip((0..).step_by(4)) {
                    *header_dword = unsafe { read(offset) };
                }*/
                dwords.iter_mut().fold(0usize, |offset, dword| {
                    *dword = unsafe { self.read(offset as u8) };
                    offset + 4
                });
            }
            Some(header)
        } else {
            None
        }
    }

    pub unsafe fn read(&self, offset: u8) -> u32 {
        read(self.bus, self.dev, self.func, offset)
    }

    pub unsafe fn write(&self, offset: u8, value: u32) {
        write(self.bus, self.dev, self.func, offset, value);
    }
}

pub fn for_each_pci_func<F>(f: F) where F: Fn(PciFunc) {
    for bus in 0..255 {
        // TODO: don't ignore 0xFF bus
        for dev in 0..32 {
            for func in 0..8 {
                f(PciFunc::new(bus, dev, func))
            }
        }
    }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
unsafe fn read(bus: u8, dev: u8, func: u8, offset: u8) -> u32 {
    let address = 0x80000000 | ((bus as u32) << 16) | ((dev as u32) << 11) | ((func as u32) << 8) | ((offset as u32) & 0xFC);
    let value: u32;
    asm!("mov dx, 0xCF8
        out dx, eax
        mov dx, 0xCFC
        in eax, dx"
        : "={eax}"(value) : "{eax}"(address) : "dx" : "intel", "volatile");
    value
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
unsafe fn write(bus: u8, dev: u8, func: u8, offset: u8, value: u32) {
    let address = 0x80000000 | ((bus as u32) << 16) | ((dev as u32) << 11) | ((func as u32) << 8) | ((offset as u32) & 0xFC);
    asm!("mov dx, 0xCF8
        out dx, eax"
        : : "{eax}"(address) : "dx" : "intel", "volatile");
    asm!("mov dx, 0xCFC
        out dx, eax"
        : : "{eax}"(value) : "dx" : "intel", "volatile");
}

// `drivers` is a map of (class, subclass, vendor, device) to device drivers
pub fn init_devices(drivers: &BTreeMap<(u8, u8, u16, u16), PciDeviceDriver>) {
    for_each_pci_func(|func| {
        let header = if let Some(header) = func.header() {
            header
        } else { return; };

        let pci_class = PciClass::from(header.class);

        let mut string = format!("PCI {:>02X}/{:>02X}/{:>02X} {:>04X}:{:>04X} {:>02X}.{:>02X}.{:>02X}.{:>02X} {:?}",
                func.bus, func.dev, func.func,
                header.vendor_id, header.device_id,
                header.class, header.subclass, header.interface, header.revision,
                pci_class);

        match pci_class {
            PciClass::Storage => match header.subclass {
                0x01 => {
                    string.push_str(" IDE");
                },
                0x06 => {
                    string.push_str(" SATA");
                },
                _ => ()
            },
            PciClass::SerialBus => match header.subclass {
                0x03 => match header.interface {
                    0x00 => {
                        string.push_str(" UHCI");
                    },
                    0x10 => {
                        string.push_str(" OHCI");
                    },
                    0x20 => {
                        string.push_str(" EHCI");
                    },
                    0x30 => {
                        string.push_str(" XHCI");
                    },
                    _ => ()
                },
                _ => ()
            },
            _ => ()
        }

        for i in 0..header.bars.len() {
            match PciBar::from(header.bars[i]) {
                PciBar::None => (),
                PciBar::Memory(address) => string.push_str(&format!(" {}={:>08X}", i, address)),
                PciBar::Port(address) => string.push_str(&format!(" {}={:>04X}", i, address))
            }
        }

        println!("{}", string);

        if let Some(driver) = drivers.get(&(header.class, header.subclass, header.vendor_id, header.device_id)) {
            // Enable bus mastering, memory space, and I/O space
            unsafe {
                let cmd = func.read(0x04);
                println!("PCI CMD: {:>02X}", cmd);
                func.write(0x04, cmd | 7);
            }

            // Launch device driver
            driver(func, header);
        }
    });
}
