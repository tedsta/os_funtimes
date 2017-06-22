use memory::paging::{ActivePageTable, EntryFlags, Page, PageIter, VirtualAddress};
use memory::{PAGE_SIZE, FrameAllocator};

pub struct Grant {
    start: VirtualAddress,
    size: usize,
}

impl Grant {
    pub fn start_address(&self) -> VirtualAddress {
        self.start
    }

    pub fn size(&self) -> usize {
        self.size
    }
}

pub struct GrantAllocator {
    range: PageIter,
}

impl GrantAllocator {
    pub fn new(page_range: PageIter) -> GrantAllocator {
        GrantAllocator { range: page_range }
    }

    pub fn allocate<FA: FrameAllocator>(&mut self,
                                        active_table: &mut ActivePageTable,
                                        frame_allocator: &mut FA,
                                        size_in_pages: usize,
                                        flags: EntryFlags)
                                        -> Option<Grant> {
        if size_in_pages == 0 {
            return None; // a zero sized grant makes no sense
        }

        // clone the range, since we only want to change it on success
        let mut range = self.range.clone();

        // try to allocate the grant pages
        let grant_start = range.next();
        let grant_end = if size_in_pages == 1 {
            grant_start
        } else {
            // choose the (size_in_pages-2)th element, since index
            // starts at 0 and we already allocated the start page
            range.nth(size_in_pages - 2)
        };

        match (grant_start, grant_end) {
            (Some(start), Some(end)) => {
                // success! write back updated range
                self.range = range;

                // map grant pages to physical frames
                active_table.map_range(Page::range_inclusive(start, end), flags, frame_allocator);

                // create a new grant
                Some(Grant{ start: start.start_address(), size: size_in_pages * PAGE_SIZE })
            }
            _ => None, // not enough pages
        }
    }
}
