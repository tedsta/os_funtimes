#![feature(lang_items)]
#![feature(const_fn)]
#![feature(unique)]
#![no_std]

extern crate rlibc;
extern crate spin;
extern crate multiboot2;

#[macro_use]
mod vga_buffer;

#[no_mangle]
pub extern fn rust_main(multiboot_info_addr: usize) {
    vga_buffer::clear_screen();

    println!("Hello, rust!");

    let boot_info = unsafe { multiboot2::load(multiboot_info_addr) };
    let memory_map_tag = boot_info.memory_map_tag()
                                  .expect("Memory map tag required");

    println!("memory areas:");
    for area in memory_map_tag.memory_areas() {
        println!("    start: 0x{:x}, length: 0x{:x}",
                 area.base_addr, area.length);
    }
    
    loop { }
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
