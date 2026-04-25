.section .text.boot
.global boot

# This is the bootloader, it takes care of the necesities needed to run the kernel.
# It's responsibilities currently include:
# - Setting up the bitmap for physical memory manager.
# - Setting up the root page table for virtual memory
# - Enabling virtual memory
# - Moving the program counter to the higher-half kernel

boot:
    .option push
    .option norelax
    la sp, _stack_top
    .option pop

    j init_pmm

init_pmm:
    li a1, 0x80000000 # load first memory address value to a1 (it is the second argument for init_bitmap)
    li t0, 0x88000000 # load last memory address value to t1
    sub a0, t0, a1    # calculate the size of memory (it is the first argument for init_bitmap)

    la a2, _free_memory_start # load the first, page-aligned pointer location (it is the third argument for init_bitmap)

    jal ra, init_bitmap

    mv a1, a0                 # get total_pages from init_bitmap return
    la a0, _free_memory_start # load the first, page-aligned pointer location
    jal ra, print_bitmap

1:  wfi
    j 1b
