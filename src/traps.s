.section .text
.align 4
.global trap_entry
trap_entry:
    csrrw sp, sscratch, sp

    // if sscratch was 0, it means we were in S-mode.
    // otherwise, we were in U-mode
    bnez sp, from_user_mode

from_supervisor_mode:
    csrr sp, sscratch
    addi sp, sp, -256

    addi t0, sp, 256
    sd t0, 8(sp)

    j save_regs

from_user_mode:
    addi sp, sp, -256

    // store the U-mode stack pointer
    csrr t0, sscratch
    sd t0, 8(sp)

save_regs:
    // store registers in the trap frame
    sd x1, 0(sp)
    // x2 aka sp is already saved (see from_supervisor_mode and from_user_mode)
    sd x3,  16(sp)
    sd x4,  24(sp)
    sd x5,  32(sp)
    sd x6,  40(sp)
    sd x7,  48(sp)
    sd x8,  56(sp)
    sd x9,  64(sp)
    sd x10, 72(sp)
    sd x11, 80(sp)
    sd x12, 88(sp)
    sd x13, 96(sp)
    sd x14, 104(sp)
    sd x15, 112(sp)
    sd x16, 120(sp)
    sd x17, 128(sp)
    sd x18, 136(sp)
    sd x19, 144(sp)
    sd x20, 152(sp)
    sd x21, 160(sp)
    sd x22, 168(sp)
    sd x23, 176(sp)
    sd x24, 184(sp)
    sd x25, 192(sp)
    sd x26, 200(sp)
    sd x27, 208(sp)
    sd x28, 216(sp)
    sd x29, 224(sp)
    sd x30, 232(sp)
    sd x31, 240(sp)

    csrr t0, sepc
    sd t0, 248(sp)

handle_trap:
    // move the machine cause to function argument, and call the trap handler
    mv a0, sp
    csrr a1, scause
    csrr a2, stval
    call trap_handler

restore:
    csrr t0, sstatus

    // if bit 8 was not 0, we were in S-mode
    andi t0, t0, 0x100
    bnez t0, restore_regs

restore_from_u:
    addi t0, sp, 256
    csrw sscratch, t0

restore_regs:
    ld x1,  0(sp)
    ld x3,  16(sp)
    ld x4,  24(sp)
    ld x5,  32(sp)
    ld x6,  40(sp)
    ld x7,  48(sp)
    ld x8,  56(sp)
    ld x9,  64(sp)
    ld x10, 72(sp)
    ld x11, 80(sp)
    ld x12, 88(sp)
    ld x13, 96(sp)
    ld x14, 104(sp)
    ld x15, 112(sp)
    ld x16, 120(sp)
    ld x17, 128(sp)
    ld x18, 136(sp)
    ld x19, 144(sp)
    ld x20, 152(sp)
    ld x21, 160(sp)
    ld x22, 168(sp)
    ld x23, 176(sp)
    ld x24, 184(sp)
    ld x25, 192(sp)
    ld x26, 200(sp)
    ld x27, 208(sp)
    ld x28, 216(sp)
    ld x29, 224(sp)
    ld x30, 232(sp)
    ld x31, 240(sp)

    ld t0, 248(sp)
    csrw sepc, t0

    ld x2,  8(sp)
    sret
