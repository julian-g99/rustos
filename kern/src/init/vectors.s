.global context_save
context_save:
    // FIXME: Save the remaining context to the stack.
    stp x8, x9, [SP, #-16]!
    stp x10, x11, [SP, #-16]!
    stp x12, x13, [SP, #-16]!
    stp x14, x15, [SP, #-16]!
    stp x16, x17, [SP, #-16]!
    stp x18, xzr, [SP, #-16]!
    stp x29, x30, [SP, #-16]!
    mov x0, x29
    mrs x1, ESR_EL1
    mov x2, #0
    bl handle_exception

.global context_restore
context_restore:
    // FIXME: Restore the context from the stack.
    ldp x29, x30, [SP], #16
    ldp x18, xzr, [SP], #16
    ldp x16, x17, [SP], #16
    ldp x14, x15, [SP], #16
    ldp x12, x13, [SP], #16
    ldp x10, x11, [SP], #16
    ldp x8, x9, [SP], #16
    ret

.macro HANDLER source, kind
    .align 7
    stp     lr, xzr, [SP, #-16]!
    stp     x28, x29, [SP, #-16]!
    
    mov     x29, \source
    movk    x29, \kind, LSL #16
    bl      context_save
    
    ldp     x28, x29, [SP], #16
    ldp     lr, xzr, [SP], #16
    eret
.endm
    
.align 11
.global vectors
vectors:
    // FIXME: Setup the 16 exception vectors.
    HANDLER 0, 0
    HANDLER 0, 1
    HANDLER 0, 2
    HANDLER 0, 3
    HANDLER 1, 0
    HANDLER 1, 1
    HANDLER 1, 2
    HANDLER 1, 3
    HANDLER 2, 0
    HANDLER 2, 1
    HANDLER 2, 2
    HANDLER 2, 3
    HANDLER 3, 0
    HANDLER 3, 1
    HANDLER 3, 2
    HANDLER 3, 3
