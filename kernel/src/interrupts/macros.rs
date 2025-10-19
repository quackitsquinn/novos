/// Wrapper for an interrupt handler. Based on [this implementation](https://github.com/bendudson/EuraliOS/blob/main/kernel/src/interrupts.rs#L117)
/// which is further based on another rust os which I'm not bothering to go down the rabbit hole to find.
#[macro_export]
macro_rules! interrupt_wrapper {
    ($handler: path, $raw: ident) => {
        #[unsafe(naked)]
        #[allow(missing_docs)]
        pub extern "x86-interrupt" fn $raw(_: InterruptStackFrame) {
            ::core::arch::naked_asm! {
                // Disable interrupts.
                "cli",
                // Push all registers to the stack. Push the registers in the OPPOSITE order that they are defined in InterruptRegisters.
                "push rax",
                "push rbx",
                "push rcx",
                "push rdx",
                "push rbp",
                "push rdi",
                "push rsi",
                "push r8",
                "push r9",
                "push r10",
                "push r11",
                "push r12",
                "push r13",
                "push r14",
                "push r15",

                // TODO: We don't do any floating point stuff yet, so we don't need to save the floating point registers.

                // C abi requires that the first parameter is in rdi, so we need to move the stack pointer to rdi.
                "mov rdi, rsp",
                "call {handler}",

                // Pop all registers from the stack. Pop the registers in the SAME order that they are defined in InterruptRegisters.
                "pop r15",
                "pop r14",
                "pop r13",
                "pop r12",
                "pop r11",
                "pop r10",
                "pop r9",
                "pop r8",
                "pop rsi",
                "pop rdi",
                "pop rbp",
                "pop rdx",
                "pop rcx",
                "pop rbx",
                "pop rax",

                // Re-enable interrupts.
                "sti",
                // Return from interrupt.
                "iretq",
                handler = sym $handler,
            }
        }
    };
}

/// Defines an interrupt handler that can be used with the IDT.
/// This macro generates unsafe code and must be used within an unsafe block.
#[macro_export]
macro_rules! define_interrupt {
    ($tbl: expr, $inner: path, $name: ident, $code: literal) => {
        ::paste::paste! {
            extern "C" fn [<_$name>](ctx: *mut ()) {
                $inner(unsafe {mem::transmute(ctx)}, $code, BASIC_HANDLERS[$code as usize]);
            }
            interrupt_wrapper!([<_$name>], [<raw_$name>]);
            $tbl.$name.set_handler_fn(mem::transmute([<raw_$name>] as *const ()));
        }
    };
}

/// Initializes the interrupt table with the given handlers.
#[macro_export]
macro_rules! init_idt {
    ($code_handler: path, $page_fault_handler: path, $normal_handler: path, $table: expr) => {
        // First off, assert the types of the handlers.
        let _: CodeHandler = $code_handler;
        let _: PageFaultHandler = $page_fault_handler;
        let _: InterruptHandler = $normal_handler;

        // Now, define the handlers.
        unsafe {
            crate::define_interrupt!($table, $normal_handler, divide_error, 0);
            crate::define_interrupt!($table, $normal_handler, debug, 1);
            crate::define_interrupt!($table, $normal_handler, non_maskable_interrupt, 2);
            crate::define_interrupt!($table, $normal_handler, breakpoint, 3);
            crate::define_interrupt!($table, $normal_handler, overflow, 4);
            crate::define_interrupt!($table, $normal_handler, bound_range_exceeded, 5);
            crate::define_interrupt!($table, $normal_handler, invalid_opcode, 6);
            crate::define_interrupt!($table, $normal_handler, device_not_available, 7);
            crate::define_interrupt!($table, $code_handler, double_fault, 8);
            crate::define_interrupt!($table, $code_handler, invalid_tss, 10);
            crate::define_interrupt!($table, $code_handler, segment_not_present, 11);
            crate::define_interrupt!($table, $code_handler, stack_segment_fault, 12);
            crate::define_interrupt!($table, $code_handler, general_protection_fault, 13);
            crate::interrupt_wrapper!($page_fault_handler, raw_page_fault);
            $table
                .page_fault
                .set_handler_fn(mem::transmute(raw_page_fault as *const ()));
            crate::define_interrupt!($table, $normal_handler, x87_floating_point, 16);
            crate::define_interrupt!($table, $code_handler, alignment_check, 17);
            crate::define_interrupt!($table, $normal_handler, machine_check, 18);
            crate::define_interrupt!($table, $normal_handler, simd_floating_point, 19);
            crate::define_interrupt!($table, $normal_handler, virtualization, 20);
            crate::define_interrupt!($table, $code_handler, cp_protection_exception, 21);
            crate::define_interrupt!($table, $normal_handler, hv_injection_exception, 28);
            crate::define_interrupt!($table, $code_handler, vmm_communication_exception, 29);
            crate::define_interrupt!($table, $code_handler, security_exception, 30);
        }
    };
}
