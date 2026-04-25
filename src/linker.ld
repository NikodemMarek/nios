OUTPUT_ARCH(riscv)
ENTRY(boot)

PAGE_SIZE = 4K;

SECTIONS
{
    . = 0x80200000;

    .text : {
        *(.text.boot)
        *(.text .text.*)
    }

    .rodata : {
        *(.rodata .rodata.*)
    }

    .data : {
        *(.data .data.*)
    }

    .bss : {
        *(.bss .bss.*)
    }

    . = ALIGN(16);
    _stack_top = . + 0x4000;

    . = ALIGN(PAGE_SIZE);
    _free_memory_start = .;
}
