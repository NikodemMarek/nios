OUTPUT_ARCH(riscv)
ENTRY(boot)

PHYS_KERNEL = 0x80200000;
VIRT_OFFSET = 0xffffffff00000000;

PAGE_SIZE = 4K;

SECTIONS
{
    . = PHYS_KERNEL;

    .text.boot : {
        *(.text.boot)
    } :text

    . = ALIGN(16);
    . += 0x4000;
    _stack_top = .;

    . = ALIGN(PAGE_SIZE);
    _free_memory_start = .;

    /* move by page to free space for bump allocator */
    . += PAGE_SIZE;

    . += VIRT_OFFSET;

    .text ALIGN(PAGE_SIZE) : AT(ADDR(.text) - VIRT_OFFSET) {
        *(.text .text.*)
    }

    .rodata ALIGN(PAGE_SIZE) : AT(ADDR(.rodata) - VIRT_OFFSET) {
        *(.rodata .rodata.*)
    }

    .eh_frame ALIGN(PAGE_SIZE) : AT(ADDR(.eh_frame) - VIRT_OFFSET) {
        *(.eh_frame)
    }

    .data ALIGN(PAGE_SIZE) : AT(ADDR(.data) - VIRT_OFFSET) {
        *(.data .data.*)
    }

    .bss ALIGN(PAGE_SIZE) : AT(ADDR(.bss) - VIRT_OFFSET) {
        *(.bss .bss.*)
    }
}
