arch ?= x86_64
kernel := build/kernel-$(arch).bin
target ?= $(arch)-blog_os
rust_os := target/$(target)/debug/libos_funtimes.a
iso := build/os-$(arch).iso

linker_script := src/arch/$(arch)/linker.ld
grub_cfg := src/arch/$(arch)/grub.cfg
assembly_source_files := $(wildcard src/arch/$(arch)/*.asm)
assembly_object_files := $(patsubst src/arch/$(arch)/%.asm, \
	build/arch/$(arch)/%.o, $(assembly_source_files))

.PHONY: all clean run iso

all: $(kernel)

clean:
	@rm -r build

run: $(iso)
	@qemu-system-x86_64 -boot d -cdrom $(iso) -drive id=disk0,format=raw,media=disk,file=data/disk_test.img,if=none -device ahci,id=ahci -device ide-drive,drive=disk0,bus=ahci.0

debug: $(iso)
	@qemu-system-x86_64 -s -S -cdrom $(iso)

gdb:
	@rust-os-gdb/bin/rust-gdb "build/kernel-x86_64.bin" -ex "target remote :1234"

iso: $(iso)

$(iso): $(kernel) $(grub_cfg)
	@mkdir -p build/isofiles/boot/grub
	@cp $(kernel) build/isofiles/boot/kernel.bin
	@cp $(grub_cfg) build/isofiles/boot/grub
	@grub-mkrescue -o $(iso) build/isofiles 2> /dev/null
	@rm -r build/isofiles

$(kernel): cargo $(rust_os) $(assembly_object_files) $(linker_script)
	@ld --gc-sections -n -T $(linker_script) -o $(kernel) $(assembly_object_files) $(rust_os)

cargo:
	@xargo build --target $(target)

# compile assembly files
build/arch/$(arch)/%.o: src/arch/$(arch)/%.asm
	@mkdir -p $(shell dirname $@)
	@nasm -felf64 $< -o $@
