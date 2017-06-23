use memory::paging::{Page, PageIter, VirtualAddress};
use memory::PAGE_SIZE;

pub struct PageAllocator {
    range: PageIter,
}

impl PageAllocator {
    pub fn new(page_range: PageIter) -> PageAllocator {
        PageAllocator { range: page_range }
    }

    pub fn allocate(&mut self, size_in_pages: usize) -> Option<PageIter> {
        if size_in_pages == 0 {
            return None; // a zero sized VM area makes no sense
        }

        // clone the range, since we only want to change it on success
        let mut range = self.range.clone();

        // try to allocate the VM area pages
        let start_page = range.next();
        let end_page = if size_in_pages == 1 {
            start_page
        } else {
            // choose the (size_in_pages-2)th element, since index
            // starts at 0 and we already allocated the start page
            range.nth(size_in_pages - 2)
        };

        match (start_page, end_page) {
            (Some(start), Some(end)) => {
                // success! write back updated range
                self.range = range;

                // create a new VM area
                Some(Page::range_inclusive(start, end))
            }
            _ => None, // not enough pages
        }
    }
}
