use ::memory::{with_mem_ctrl, EntryFlags, PhysicalAddress, VirtualAddress};

pub fn alloc_grant(size: usize, flags: EntryFlags) -> Option<VirtualAddress> {
    with_mem_ctrl(|m| {
        m.alloc_grant(size, flags).map(|g| g.start_address())
    })
}

pub fn free_grant(start_address: VirtualAddress) {
    with_mem_ctrl(|m| {
        // TODO
    });
}

pub fn translate_addr(addr: VirtualAddress) -> Option<PhysicalAddress> {
    with_mem_ctrl(|m| { m.translate_address(addr) })
}
