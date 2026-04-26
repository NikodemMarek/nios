OUTPUT_ARCH(riscv)
ENTRY(boot)

PHYS_OFFSET = 0x80000000;
VIRT_OFFSET = 0xffffffff00000000;
KERNEL_OFFSET = 0x200000;

PAGE_SIZE = 4K;

SECTIONS
{
    . = PHYS_OFFSET;
    . = ALIGN(PAGE_SIZE);
    _memory_start = .;

    . += KERNEL_OFFSET;

    .text.boot : {
        *(.text.boot)
    } :text

    . = ALIGN(16);
    . += 0x4000;
    _stack_top = .;

    . = ALIGN(PAGE_SIZE);
    _free_memory_start = .;
    _root_page_table = .;
    /* move by page to free space for bump allocator */
    . += PAGE_SIZE;

    . += VIRT_OFFSET;

    _memory_start_virt = _memory_start + VIRT_OFFSET;

    _root_page_table_virt = _root_page_table + VIRT_OFFSET;

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

    . = ALIGN(PAGE_SIZE);
    _free_memory_start_virt = .;
}
