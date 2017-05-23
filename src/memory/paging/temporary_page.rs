use super::super::{Frame, FrameAllocator};
use super::{ActivePageTable, Page, VirtualAddress};
use super::table::{Table, Level1};

pub struct TemporaryPage {
    page: Page,
}

impl TemporaryPage {
    pub fn new(page: Page) -> TemporaryPage {
        TemporaryPage {
            page: page,
        }
    }

    /// Maps the temporary page to the given frame in the active table.
    /// Returns the start address of the temporary page.
    pub fn map<A: FrameAllocator>(&mut self, frame: Frame, active_table: &mut ActivePageTable,
                                  allocator: &mut A) -> VirtualAddress
    {
        use super::entry::WRITABLE;

        assert!(active_table.translate_page(self.page).is_none(),
                "temporary page is already mapped");
        active_table.map_to(self.page, frame, WRITABLE, allocator);
        self.page.start_address()
    }

    /// Unmaps the temporary page in the active table.
    pub fn unmap<A: FrameAllocator>(&mut self, active_table: &mut ActivePageTable,
                                    allocator: &mut A) {
        active_table.unmap(self.page, allocator)
    }

    /// Maps the temporary page to the given page table frame in the active
    /// table. Returns a reference to the now mapped table.
    pub fn map_table_frame<A: FrameAllocator>(&mut self,
                                              frame: Frame,
                                              active_table: &mut ActivePageTable,
                                              allocator: &mut A)
                                              -> &mut Table<Level1> {
        unsafe { &mut *(self.map(frame, active_table, allocator) as *mut Table<Level1>) }
    }
}
