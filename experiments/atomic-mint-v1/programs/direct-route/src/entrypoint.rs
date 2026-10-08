use pinocchio::{no_allocator, nostd_panic_handler, program_entrypoint};

program_entrypoint!(crate::process_instruction);
no_allocator!();
nostd_panic_handler!();
