use ::memory::{with_mem_ctrl, EntryFlags, PhysicalAddress, VirtualAddress};

pub fn alloc_vm(size: usize, flags: EntryFlags) -> Option<VirtualAddress> {
    with_mem_ctrl(|m| {
        m.alloc_vm(size, flags)
    })
}

pub fn map_pm(address: PhysicalAddress, size: usize, flags: EntryFlags) -> Option<VirtualAddress> {
    with_mem_ctrl(|m| {
        m.map_pm(address, size, flags)
    })
}

pub fn free_vm(start_address: VirtualAddress) {
    with_mem_ctrl(|m| {
        // TODO
    });
}

pub fn translate_addr(addr: VirtualAddress) -> Option<PhysicalAddress> {
    with_mem_ctrl(|m| { m.translate_address(addr) })
}
