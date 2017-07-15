use multiboot2::BootInformation;
use spin;

use self::area_frame_allocator::AreaFrameAllocator;
use self::page_allocator::PageAllocator;
use self::paging::{InactivePageTable, Mapper, Page, TemporaryPage};
use self::stack_allocator::StackAllocator;

pub use self::layout::*;
pub use self::paging::{PhysicalAddress, VirtualAddress, EntryFlags, WRITABLE};
pub use self::stack_allocator::Stack;

mod area_frame_allocator;
mod layout;
mod page_allocator;
mod paging;
mod stack_allocator;

pub const PAGE_SIZE: usize = 4096;

static MEM_CONTROLLER: spin::Mutex<Option<MemoryController>> = spin::Mutex::new(None);

#[inline(always)]
pub fn with_mem_ctrl<F, R>(f: F) -> R
    where F: FnOnce(&mut MemoryController) -> R
{
    let mut memory_controller = MEM_CONTROLLER.lock();
    f(memory_controller.as_mut().expect("Tried to access MemoryController before initialized."))
}

pub fn init(boot_info: &BootInformation) {
    let memory_map_tag =
        boot_info.memory_map_tag().expect("Memory map tag required");
    let elf_sections_tag =
        boot_info.elf_sections_tag().expect("Elf sections tag required");

    let kernel_start = elf_sections_tag.sections()
                                       .filter(|s| s.is_allocated())
                                       .map(|s| s.start_address()).min().unwrap();
    let kernel_end = elf_sections_tag.sections()
                                     .filter(|s| s.is_allocated())
                                     .map(|s| s.end_address()).max().unwrap();

    println!("kernel start: {:#x}, kernel end: {:#x}",
             kernel_start,
             kernel_end);
    println!("multiboot start: {:#x}, multiboot end: {:#x}",
             boot_info.start_address(),
             boot_info.end_address());

    let mut frame_allocator = AreaFrameAllocator::new(kernel_start, kernel_end,
                                                      boot_info.start_address(),
                                                      boot_info.end_address(),
                                                      memory_map_tag.memory_areas());

    let mut active_table = paging::remap_kernel(&mut frame_allocator, boot_info);

    // Memory map the kernel heap
    use self::paging::Page;
    use hole_list_allocator::{HEAP_START, HEAP_SIZE};

    let heap_start_page = Page::containing_address(HEAP_START);
    let heap_end_page = Page::containing_address(HEAP_START + HEAP_SIZE-1);

    for page in Page::range_inclusive(heap_start_page, heap_end_page) {
        active_table.map(page, paging::WRITABLE, &mut frame_allocator);
    }

    // Create a stack allocator
    let stack_alloc_start = heap_end_page + 1;
    let stack_alloc_end = stack_alloc_start + 100;
    let stack_allocator =
        StackAllocator::new(Page::range_inclusive(stack_alloc_start, stack_alloc_end));

    let page_alloc_start = stack_alloc_end + 1;
    let page_alloc_end = page_alloc_start + 65536; // 256 MB of pages
    let page_allocator =
        PageAllocator::new(Page::range_inclusive(page_alloc_start, page_alloc_end));

    *MEM_CONTROLLER.lock() = Some(MemoryController {
        active_table: active_table,
        frame_allocator: frame_allocator,
        stack_allocator: stack_allocator,
        page_allocator: page_allocator,
    });
}

pub struct MemoryController {
    active_table: paging::ActivePageTable,
    frame_allocator: AreaFrameAllocator,
    stack_allocator: StackAllocator,
    page_allocator: PageAllocator,
}

impl MemoryController {
    pub fn new_page_table(&mut self) -> Option<InactivePageTable> {
        let mut tmp_page = TemporaryPage::new(Page::containing_address(KERNEL_TMP_PAGE_OFFSET));

        self.frame_allocator.allocate_frame().map(|new_table_frame| {
            InactivePageTable::new(new_table_frame, &mut self.active_table,
                                   &mut tmp_page, &mut self.frame_allocator)
        })
    }

    pub fn with_inactive_table<F>(&mut self, table: &mut InactivePageTable, f: F)
        where F: FnOnce(&mut Mapper, &mut AreaFrameAllocator)
    {
        let mut tmp_page = TemporaryPage::new(Page::containing_address(KERNEL_TMP_PAGE_OFFSET));
        self.active_table.with(table, &mut tmp_page, &mut self.frame_allocator, f);
    }

    pub fn translate_address(&mut self, address: VirtualAddress) -> Option<PhysicalAddress> {
        self.active_table.translate(address)
    }

    pub fn alloc_vm(&mut self, size: usize, flags: paging::EntryFlags) -> Option<VirtualAddress> {
        self.page_allocator.allocate((size + 4095) / PAGE_SIZE).map(|pages| {
            let start_address = pages.start_address();
            // TODO verify pages were mapped properly and return None otherwise
            self.active_table.map_range(pages, flags, &mut self.frame_allocator);
            start_address
        })
    }

    pub fn map_pm(&mut self, address: PhysicalAddress, size: usize, flags: paging::EntryFlags) -> Option<VirtualAddress> {
        self.page_allocator.allocate((size + 4095) / PAGE_SIZE).map(|pages| {
            let start_address = pages.start_address();

            let start_frame = Frame::containing_address(address);
            let end_frame = Frame::containing_address(address + size - 1);

            // map grant pages to physical frames
            self.active_table.map_range_to(pages, Frame::range_inclusive(start_frame, end_frame),
                                           flags, &mut self.frame_allocator);
            
            start_address
        })
    }

    pub fn alloc_stack(&mut self, size_in_pages: usize) -> Option<Stack> {
        self.stack_allocator.alloc_stack(&mut self.active_table, &mut self.frame_allocator,
                                         size_in_pages)
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Frame {
    number: usize,
}

impl Frame {
    fn containing_address(address: usize) -> Frame {
        Frame{ number: address / PAGE_SIZE }
    }

    fn start_address(&self) -> PhysicalAddress {
        self.number * PAGE_SIZE
    }

    fn clone(&self) -> Frame {
        Frame { number: self.number }
    }

    fn range_inclusive(start: Frame, end: Frame) -> FrameIter {
        FrameIter {
            start: start,
            end: end,
        }
    }
}

#[derive(PartialEq)]
struct FrameIter {
    start: Frame,
    end: Frame,
}

impl Iterator for FrameIter {
    type Item = Frame;

    fn next(&mut self) -> Option<Frame> {
        if self.start <= self.end {
            let frame = self.start.clone();
            self.start.number += 1;
            Some(frame)
        } else {
            None
        }
    }
}

pub trait FrameAllocator {
    fn allocate_frames(&mut self, count: usize) -> Option<FrameIter>;
    fn deallocate_frames(&mut self, frames: FrameIter);

    fn allocate_frame(&mut self) -> Option<Frame> {
        self.allocate_frames(1).map(|frames| {
            assert!(frames.start == frames.end);
            frames.start
        })
    }

    fn deallocate_frame(&mut self, frame: Frame) {
        self.deallocate_frames(Frame::range_inclusive(frame.clone(), frame));
    }
}
