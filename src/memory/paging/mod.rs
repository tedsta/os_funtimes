use core::ops::{Add, Deref, DerefMut, Sub};

use multiboot2::BootInformation;

use super::{PAGE_SIZE, Frame, FrameAllocator};
use self::temporary_page::TemporaryPage;

pub use self::mapper::Mapper;
pub use self::entry::{EntryFlags, PRESENT, WRITABLE};

mod entry;
mod mapper;
mod table;
mod temporary_page;

const ENTRY_COUNT: usize = 512;

pub type VirtualAddress = usize;
pub type PhysicalAddress = usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Page {
    number: usize,
}

impl Page {
    pub fn containing_address(address: VirtualAddress) -> Page {
        assert!(address < 0x0000_8000_0000_0000 || address >= 0xffff_8000_0000_0000,
                "invalid address: 0x{:x}", address);
        Page { number: address / PAGE_SIZE }
    }

    pub fn start_address(&self) -> usize {
        self.number * PAGE_SIZE
    }

    fn p4_index(&self) -> usize {
        (self.number >> 27) & 0o777
    }
    fn p3_index(&self) -> usize {
        (self.number >> 18) & 0o777
    }
    fn p2_index(&self) -> usize {
        (self.number >> 9) & 0o777
    }
    fn p1_index(&self) -> usize {
        (self.number >> 0) & 0o777
    }

    pub fn range_inclusive(start: Page, end: Page) -> PageIter {
        PageIter {
            start: start,
            end: end,
        }
    }
}

impl Add<usize> for Page {
    type Output = Page;

    fn add(self, rhs: usize) -> Page {
        Page { number: self.number + rhs }
    }
}

impl Sub<usize> for Page {
    type Output = Page;

    fn sub(self, rhs: usize) -> Page {
        Page { number: self.number - rhs }
    }
}

#[derive(Clone)]
pub struct PageIter {
    start: Page,
    end: Page,
}

impl PageIter {
    pub fn size(&self) -> usize {
        self.end.number - self.start.number + 1
    }
}

impl Iterator for PageIter {
    type Item = Page;

    fn next(&mut self) -> Option<Page> {
        if self.start <= self.end {
            let page = self.start;
            self.start.number += 1;
            Some(page)
        } else {
            None
        }
    }

    fn nth(&mut self, n: usize) -> Option<Page> {
        if self.start + n <= self.end {
            let page = self.start + n;
            self.start.number += n + 1;
            Some(page)
        } else {
            None
        }
    }
}

pub struct ActivePageTable {
    mapper: Mapper,
}

impl Deref for ActivePageTable {
    type Target = Mapper;

    fn deref(&self) -> &Mapper {
        &self.mapper
    }
}

impl DerefMut for ActivePageTable {
    fn deref_mut(&mut self) -> &mut Mapper {
        &mut self.mapper
    }
}

impl ActivePageTable {
    unsafe fn new() -> ActivePageTable {
        ActivePageTable {
            mapper: Mapper::new(),
        }
    }

    pub fn with<A, F>(&mut self,
                   table: &mut InactivePageTable,
                   temporary_page: &mut temporary_page::TemporaryPage,
                   allocator: &mut A,
                   f: F)
        where A: FrameAllocator, F: FnOnce(&mut Mapper, &mut A)
    {
        use x86_64::instructions::tlb;
        use x86_64::registers::control_regs;

        let flush_tlb = || tlb::flush_all();

        {
            let backup = Frame::containing_address(control_regs::cr3().0 as usize);

            // map temporary_page to current p4 table
            let p4_table = temporary_page.map_table_frame(backup.clone(), self, allocator);

            // overwrite recursive mapping
            self.p4_mut()[511].set(table.p4_frame.clone(), PRESENT | WRITABLE);
            flush_tlb();

            // execute f in the new context
            f(self, allocator);

            // restore recursive mapping to original p4 table
            p4_table[511].set(backup, PRESENT | WRITABLE);
            flush_tlb();
        }
        
        temporary_page.unmap(self, allocator);
    }

    pub fn switch(&mut self, new_table: InactivePageTable) -> InactivePageTable {
        use x86_64;
        use x86_64::registers::control_regs;

        let old_table = InactivePageTable {
            p4_frame: Frame::containing_address(control_regs::cr3().0 as usize),
        };
        unsafe {
            control_regs::cr3_write(x86_64::PhysicalAddress(new_table.p4_frame.start_address() as u64));
        }
        old_table
    }
}

pub struct InactivePageTable {
    p4_frame: Frame,
}

impl InactivePageTable {
    pub fn new<A: FrameAllocator>(frame: Frame,
                                  active_table: &mut ActivePageTable,
                                  temporary_page: &mut TemporaryPage,
                                  allocator: &mut A)
                                  -> InactivePageTable
    {
        {
            let table =
                temporary_page.map_table_frame(frame.clone(), active_table, allocator);
            // now we are able to zero the table
            table.zero();
            // set up recursive mapping for the table
            table[511].set(frame.clone(), PRESENT | WRITABLE);
        }
        temporary_page.unmap(active_table, allocator);

        InactivePageTable { p4_frame: frame }
    }
}

pub fn remap_kernel<A: FrameAllocator>(allocator: &mut A, boot_info: &BootInformation)
                                       -> ActivePageTable
{
    let mut temporary_page = TemporaryPage::new(Page { number: 0xcafebabe });

    let mut active_table = unsafe { ActivePageTable::new() };
    let mut new_table =
        InactivePageTable::new(allocator.allocate_frame().expect("no more frames"),
                               &mut active_table, &mut temporary_page, allocator);

    active_table.with(&mut new_table, &mut temporary_page, allocator, |mapper, allocator| {
        let elf_sections_tag = boot_info.elf_sections_tag().expect("Memory map tag required");

        for section in elf_sections_tag.sections() {
            if !section.is_allocated() {
                // section is not loaded to memory
                continue;
            }
            assert!(section.start_address() % PAGE_SIZE == 0,
                    "sections need to be page aligned");

            println!("mapping section at addr: {:#x}, size: {:#x}",
                section.start_address(), section.size());

            let flags = EntryFlags::from_elf_section_flags(section);

            let start_frame = Frame::containing_address(section.start_address());
            let end_frame = Frame::containing_address(section.end_address() - 1);
            for frame in Frame::range_inclusive(start_frame, end_frame) {
                mapper.identity_map(frame, flags, allocator);
            }
        }

        // identity map the VGA text buffer
        let vga_buffer_frame = Frame::containing_address(0xb8000);
        mapper.identity_map(vga_buffer_frame, WRITABLE, allocator);

        // identity map the multiboot info structure
        let multiboot_start = Frame::containing_address(boot_info.start_address());
        let multiboot_end = Frame::containing_address(boot_info.end_address() - 1);
        for frame in Frame::range_inclusive(multiboot_start, multiboot_end) {
            mapper.identity_map(frame, PRESENT, allocator);
        }
    });

    let old_table = active_table.switch(new_table);
    println!("NEW TABLE!!!");

    // turn the old p4 page into a guard page
    let old_p4_page = Page::containing_address(old_table.p4_frame.start_address());
    active_table.unmap(old_p4_page, allocator);
    println!("guard page at {:#x}", old_p4_page.start_address());

    active_table
}
