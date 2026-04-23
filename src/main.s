.section .text.boot
.global _start
_start:
    la sp, 0x88000000

    // M-mode setup
    // delegate everything to S-mode
    li t0, 0xffff
    csrw mideleg, t0
    li t0, 0xffff
    csrw medeleg, t0

    // grant S-mode access to all memory
    li t0, 0x1f
    csrw pmpcfg0, t0
    li t0, 0xffffffffffffffff
    csrw pmpaddr0, t0

    // Switch to S-mode
    li t0, 0x1800         // clear MPP (M-mode bits)
    csrc mstatus, t0
    li t0, 0x0800         // set MPP=01 (S-mode)
    csrs mstatus, t0
    la t0, s_mode_entry
    csrw mepc, t0
    mret

s_mode_entry:
    la t0, trap_entry
    csrw stvec, t0
    // reset stack to a known virtual address
    la sp, 0x88000000    // still identity mapped, fine for now
    call kernel_main

// loop forever in case something goes wrong
1:  wfi
    j 1b
