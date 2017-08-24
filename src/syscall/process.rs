use ::memory;

pub fn exec() {
    // TODO create new context
    with_mem_ctrl(|m| {
        let mut new_table = m.new_page_table().expect("Out of frames");

        let kernel_memory_p4_entry = m.active_table.p4()[0].pointed_frame().expect("kernel table not mapped");
        let kernel_memory_flags = m.active_table.p4()[0].flags();

        m.with_inactive_table(&mut new_table, |mapper, allocator| {
            // Copy kernel mapping
            //mapper.p4_mut()[510].set(kernel_memory_p4_entry, flags);
            mapper.p4_mut()[0].set(kernel_memory_p4_entry, flags);
            // TODO map program image
            // TODO create and map stack
            let stack = m.alloc_stack(2048).unwrap(); // 2048 pages = 8MB

            // TODO create and map heap
            // TODO switch to user mode
        });
    });
}
