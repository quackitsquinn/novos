# TODO: in the future #[panic_handler]s should just be no_mangle so this can be reduced to just `panic_handler
break kernel::panic::panic
break exception_brk