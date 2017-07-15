// Switch to usermode, start executing at ip with stack at sp
pub unsafe fn usermode(ip: usize, sp: usize) -> ! {
    // Go to usermode
    asm!("mov ds, ax
        mov es, ax
        mov fs, bx
        mov gs, ax
        push rax
        push rcx
        push rdx
        push rsi
        push rdi
        iretq"
        : // No output because it never returns
        :   "{rax}"(gdt::GDT_USER_DATA << 3 | 3), // Data segment
            "{rbx}"(gdt::GDT_USER_TLS << 3 | 3), // TLS segment
            "{rcx}"(sp), // Stack pointer
            "{rdx}"(3 << 12 | 1 << 9), // Flags - Set IOPL and interrupt enable flag
            "{rsi}"(gdt::GDT_USER_CODE << 3 | 3), // Code segment
            "{rdi}"(ip) // IP
        : // No clobers because it never returns
        : "intel", "volatile");
    unreachable!();
}
