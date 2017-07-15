use ::memory;

pub fn exec() {
    // TODO create new context
    with_mem_ctrl(|m| {
        let mut new_table = m.new_page_table().expect("Out of frames");

        m.with_inactive_table(&mut new_table, |mapper, allocator| {
            // TODO map kernel memory
            // TODO map program image
            // TODO create and map stack
            // TODO create and map heap
            // TODO switch to user mode
        });
    });
}
