.section .text.boot
.global boot

boot:
    .option push
    .option norelax
    la sp, _stack_top
    .option pop

    j kernel_main
