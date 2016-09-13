#![feature(alloc, collections)]
#![feature(lang_items)]
#![feature(const_fn)]
#![feature(unique)]
#![no_std]

#[macro_use]
extern crate bitflags;
extern crate rlibc;
extern crate spin;
extern crate multiboot2;
extern crate x86;

extern crate hole_list_allocator;
extern crate alloc;
#[macro_use]
extern crate collections;

#[macro_use]
mod vga_buffer;
mod memory;

#[no_mangle]
pub extern fn rust_main(multiboot_info_addr: usize) {
    enable_nxe_bit();
    enable_write_protect_bit();

    vga_buffer::clear_screen();

    println!("Hello, rust!");

    let boot_info = unsafe { multiboot2::load(multiboot_info_addr) };
    
    memory::init(boot_info);

    println!("It did not crash!");

    let v = vec![1u32, 2, 3, 4, 5];
    for i in v.iter().rev() {
        println!("{}", i);
    }
    
    loop { }
}

fn enable_nxe_bit() {
    use x86::msr::{IA32_EFER, rdmsr, wrmsr};

    let nxe_bit = 1 << 11;
    unsafe {
        let efer = rdmsr(IA32_EFER);
        wrmsr(IA32_EFER, efer | nxe_bit);
    }
}

fn enable_write_protect_bit() {
    use x86::controlregs::{cr0, cr0_write};

    let wp_bit = 1 << 16;
    unsafe { cr0_write(cr0() | wp_bit) };
}

#[lang = "eh_personality"] extern fn eh_personality() { }
#[lang = "panic_fmt"]
extern fn panic_fmt(fmt: core::fmt::Arguments, file: &str, line: u32) -> ! {
    println!("\n\nPANIC in {} at line {}:", file, line);
    println!("    {}", fmt);
    loop { }
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn _Unwind_Resume() -> ! {
    loop { }
}
