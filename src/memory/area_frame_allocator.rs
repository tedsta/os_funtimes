use multiboot2::{MemoryAreaIter, MemoryArea};

use ::memory::{Frame, FrameAllocator, FrameIter};

pub struct AreaFrameAllocator {
    next_free_frame: Frame,
    current_area: Option<&'static MemoryArea>,
    areas: MemoryAreaIter,
    kernel_start: Frame,
    kernel_end: Frame,
    multiboot_start: Frame,
    multiboot_end: Frame,
}

impl AreaFrameAllocator {
    pub fn new(kernel_start: usize, kernel_end: usize,
               multiboot_start: usize, multiboot_end: usize,
               memory_areas: MemoryAreaIter) -> AreaFrameAllocator
    {
        let mut allocator = AreaFrameAllocator {
            next_free_frame: Frame::containing_address(0),
            current_area: None,
            areas: memory_areas,
            kernel_start: Frame::containing_address(kernel_start),
            kernel_end: Frame::containing_address(kernel_end),
            multiboot_start: Frame::containing_address(multiboot_start),
            multiboot_end: Frame::containing_address(multiboot_end),
        };
        allocator.choose_next_area();
        allocator
    }

    fn choose_next_area(&mut self) {
        self.current_area = self.areas.clone().filter(|area| {
            let address = area.start_address() + area.size() - 1;
            Frame::containing_address(address) >= self.next_free_frame
        }).min_by_key(|area| area.start_address());

        if let Some(area) = self.current_area {
            let start_frame = Frame::containing_address(area.start_address());
            if self.next_free_frame < start_frame {
                self.next_free_frame = start_frame;
            }
        }
    }
}

impl FrameAllocator for AreaFrameAllocator {
    fn allocate_frames(&mut self, count: usize) -> Option<FrameIter> {
        if count > 0 {
            while let Some(area) = self.current_area {
                // "Clone" the frame to return it if it's free. Frame doesn't
                // implement Clone, but we can construct an identical frame.
                let start = self.next_free_frame.clone();
                let end = Frame { number: start.number + count - 1 };

                // the last frame of the current area
                let current_area_last_frame = {
                    let address = area.start_address() + area.size() - 1;
                    Frame::containing_address(address)
                };

                if start > current_area_last_frame {
                    // all frames of current area are used, switch to next area
                    self.choose_next_area();
                } else if start <= self.kernel_end && end >= self.kernel_start {
                    // `frame` is used by the kernel
                    self.next_free_frame = Frame {
                        number: self.kernel_end.number + 1
                    };
                } else if start <= self.multiboot_end && end >= self.multiboot_start {
                    // `frame` is used by the multiboot information structure
                    self.next_free_frame = Frame {
                        number: self.multiboot_end.number + 1
                    };
                } else {
                    // frame is unused, increment `next_free_frame` and return it
                    self.next_free_frame.number += count;
                    return Some(Frame::range_inclusive(start, end));
                }
                // `frame` was not valid, try it again with the updated `next_free_frame`
            }
        }

        None
    }

    fn deallocate_frames(&mut self, frames: FrameIter) {
    }

    /*fn allocate_frame(&mut self) -> Option<Frame> {
        while let Some(area) = self.current_area {
            // "Clone" the frame to return it if it's free. Frame doesn't
            // implement Clone, but we can construct an identical frame.
            let frame = self.next_free_frame.clone();

            // the last frame of the current area
            let current_area_last_frame = {
                let address = area.start_address() + area.size() - 1;
                Frame::containing_address(address)
            };

            if frame > current_area_last_frame {
                // all frames of current area are used, switch to next area
                self.choose_next_area();
            } else if frame >= self.kernel_start && frame <= self.kernel_end {
                // `frame` is used by the kernel
                self.next_free_frame = Frame {
                    number: self.kernel_end.number + 1
                };
            } else if frame >= self.multiboot_start && frame <= self.multiboot_end {
                // `frame` is used by the multiboot information structure
                self.next_free_frame = Frame {
                    number: self.multiboot_end.number + 1
                };
            } else {
                // frame is unused, increment `next_free_frame` and return it
                self.next_free_frame.number += 1;
                return Some(frame);
            }
            // `frame` was not valid, try it again with the updated `next_free_frame`
        }

        None
    }*/
}
