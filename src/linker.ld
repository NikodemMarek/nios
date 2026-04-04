OUTPUT_ARCH(riscv)
ENTRY(_start)

PHYS_BASE = 0x80000000;
VIRT_BASE = 0xffffffff80000000;

MEMORY {
    RAM (rwx) : ORIGIN = 0x80000000, LENGTH = 128M
}

PHDRS {
    text PT_LOAD FLAGS(5);
    data PT_LOAD FLAGS(6);
}

SECTIONS {
    . = PHYS_BASE;

    .text.boot : {
        *(.text.boot)
    } > RAM

    . = VIRT_BASE + (. - PHYS_BASE);

    .text : AT(ADDR(.text) - VIRT_BASE + PHYS_BASE) {
        _kernel_start = .;
        *(.text .text.*)
    } > RAM

    .rodata : AT(ADDR(.rodata) - VIRT_BASE + PHYS_BASE) {
        *(.rodata .rodata.*)
    } > RAM

    .data : AT(ADDR(.data) - VIRT_BASE + PHYS_BASE) {
        *(.data .data.*)
    } > RAM

    .bss : AT(ADDR(.bss) - VIRT_BASE + PHYS_BASE) {
        sbss = .;
        *(.bss .bss.*)
        ebss = .;
    } > RAM

    . = ALIGN(4096);
    _kernel_end = .;

    _memory_start = PHYS_BASE;
    _memory_end = PHYS_BASE + LENGTH(RAM);

    /DISCARD/ : {
        *(.eh_frame)
    }
}
