use multiboot2::BootInformation;
use spin;

use self::paging::PhysicalAddress;

pub use self::area_frame_allocator::AreaFrameAllocator;
pub use self::stack_allocator::Stack;

mod area_frame_allocator;
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
    let stack_allocator = {
        let stack_alloc_start = heap_end_page + 1;
        let stack_alloc_end = stack_alloc_start + 100;
        let stack_alloc_range = Page::range_inclusive(stack_alloc_start, stack_alloc_end);
        stack_allocator::StackAllocator::new(stack_alloc_range)
    };

    *MEM_CONTROLLER.lock() = Some(MemoryController {
        active_table: active_table,
        frame_allocator: frame_allocator,
        stack_allocator: stack_allocator,
    });
}

pub struct MemoryController {
    active_table: paging::ActivePageTable,
    frame_allocator: AreaFrameAllocator,
    stack_allocator: stack_allocator::StackAllocator,
}

impl MemoryController {
    /*pub fn alloc_frame(&mut self, size_in_pages: usize) -> Option<FrameRange> {
    }*/

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
    fn allocate_frame(&mut self) -> Option<Frame>;
    fn deallocate_frame(&mut self, frame: Frame);
}
