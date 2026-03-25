OUTPUT_ARCH(riscv)
ENTRY(_start)

MEMORY {
    RAM (rwx) : ORIGIN = 0x80000000, LENGTH = 128M
}

SECTIONS {
    . = 0x80000000;

    _kernel_start = .;

    .text : {
        *(.text.boot)
        *(.text .text.*)
    } > RAM

    .rodata : {
        *(.rodata .rodata.*)
    } > RAM

    .data : {
        *(.data .data.*)
    } > RAM

    .bss : {
        sbss = .;
        *(.bss .bss.*)
        ebss = .;
    } > RAM

    _kernel_end = .;

    /DISCARD/ : {
        *(.eh_frame)
    }
}
