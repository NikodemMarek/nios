.section .text.boot
.global boot

# This is the bootloader, it takes care of the necesities needed to run the kernel.
# It's responsibilities currently include:
# - Setting up the root page table for virtual memory
# - Enabling virtual memory
# - Moving the program counter to the higher-half kernel

boot:
    .option push
    .option norelax
    la sp, _stack_top
    .option pop

    jal ra, init_page_table

    j enable_virtual_memory

init_page_table:
    la t0, _root_page_table    # load free memory pointer to t0, this will be the root page table pointer

    # clear the root PTE
    li t1, 0
    li t2, 512
1:  sd t1, 0(t0)
    addi t0, t0, 8
    addi t2, t2, -1
    bnez t2, 1b

    la t0, _root_page_table

    # build the PTE for 0x80000000 (The physical RAM), and load it to t1
    # PPN = 0x80000000 >> 12 = 0x80000
    # flags = 0xEF (V, R, W, X, U, A, D)
    # PTE = (0x80000 << 10) | 0xEF = 0x200000EF
    li t1, 0x200000EF

    # create identity mapping
    sd t1, 16(t0)   # slot 2 (2 * 8)
    # create higher-half mapping
    li t2, 4080     # slot 510 (510 * 8)
    add t2, t2, t0
    sd t1, 0(t2)

    ret

enable_virtual_memory:
    # shift by 12 to get pnn
    la t0, _root_page_table
    srli t0, t0, 12

    li t1, (8 << 60)
    # or pnn with mode 8 to get satp value
    or t0, t0, t1

    sfence.vma
    csrw satp, t0
    sfence.vma

    la t0, _kernel_main_vma
    ld a0, 0(t0)
    jr a0

2:  wfi
    j 2b

_kernel_main_vma:
    .quad kernel_main
