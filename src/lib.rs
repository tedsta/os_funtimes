#![feature(abi_x86_interrupt, alloc, asm, collections, core_intrinsics, lang_items, const_fn, unique)]
#![no_std]

#[macro_use]
extern crate bitflags;
extern crate bit_field;
#[macro_use]
extern crate lazy_static;
extern crate multiboot2;
extern crate rlibc;
extern crate spin;
#[macro_use]
extern crate x86_64;

extern crate hole_list_allocator;
extern crate alloc;
#[macro_use]
extern crate collections;

use collections::BTreeMap;

// Declare before other modules so that they can use println! macro
#[macro_use]
pub mod vga_buffer;

mod drivers;
mod memory;
mod interrupts;
mod syscall;

#[no_mangle]
pub extern "C" fn rust_main(multiboot_info_addr: usize) {
    vga_buffer::clear_screen();

    println!("Hello, rust!");

    let boot_info = unsafe { multiboot2::load(multiboot_info_addr) };
    enable_nxe_bit();
    enable_write_protect_bit();
    
    memory::init(boot_info);

    // initialize our IDT
    memory::with_mem_ctrl(|m| { interrupts::init(m); });

    // provoke a divide-by-zero fault
    //divide_by_zero();

    //unsafe { asm!("ud2" :::: "intel"); }

    // invoke a breakpoint exception

    //unsafe { let u: u32 = *(0x20000000 as *const u32); }

    x86_64::instructions::interrupts::int3();
    unsafe { int!(0x80); }

    println!("It did not crash!");

    let mut pci_device_drivers = BTreeMap::new();
    pci_device_drivers.insert((1, 6, 32902, 10530), drivers::ahci::init as drivers::pci::PciDeviceDriver);
    drivers::pci::init_devices(&pci_device_drivers);

    let v = vec![1u32, 2, 3, 4, 5];
    for i in v.iter().rev() {
        println!("{}", i);
    }
    
    loop { }
}

fn divide_by_zero() {
    unsafe {
        asm!("mov dx, 0; div dx" ::: "ax", "dx" : "volatile", "intel")
    }
}

fn enable_nxe_bit() {
    use x86_64::registers::msr::{IA32_EFER, rdmsr, wrmsr};

    let nxe_bit = 1 << 11;
    unsafe {
        let efer = rdmsr(IA32_EFER);
        wrmsr(IA32_EFER, efer | nxe_bit);
    }
}

fn enable_write_protect_bit() {
    use x86_64::registers::control_regs::{cr0, cr0_write, Cr0};

    unsafe { cr0_write(cr0() | Cr0::WRITE_PROTECT) };
}

#[lang = "eh_personality"]
extern "C" fn eh_personality() { }

#[lang = "panic_fmt"]
#[no_mangle]
pub extern "C" fn panic_fmt(fmt: core::fmt::Arguments, file: &str, line: u32) -> ! {
    println!("\n\nPANIC in {} at line {}:", file, line);
    println!("    {}", fmt);
    loop { }
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn _Unwind_Resume() -> ! {
    loop {}
}
