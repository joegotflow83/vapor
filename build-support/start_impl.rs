// Minimal _start for x86_64 Linux with glibc.
// Required because the environment has no system crt1.o (no C dev packages).
// This replaces the CRT startup file by calling glibc's __libc_start_main directly.
//
// Compile with:
//   rustc --edition 2021 --emit=obj -o /tmp/start_impl.o start_impl.rs
#![no_main]
use std::arch::naked_asm;

#[no_mangle]
#[unsafe(naked)]
unsafe extern "C" fn _start() -> ! {
    // Standard x86_64 Linux entry point (equivalent to glibc's crt1/Scrt1.o):
    //   - clear frame pointer
    //   - save rtld_fini (passed in rdx by ld.so)
    //   - pop argc from stack
    //   - set argv = rsp
    //   - 16-byte align stack
    //   - call __libc_start_main(main, argc, argv, NULL, NULL, rtld_fini, stack_end)
    naked_asm!(
        "xor ebp, ebp",
        "mov r9, rdx",
        "pop rsi",
        "mov rdx, rsp",
        "and rsp, -16",
        "push rax",
        "push rsp",
        "xor r8d, r8d",
        "xor ecx, ecx",
        "mov rdi, [rip + main@GOTPCREL]",
        "call __libc_start_main@PLT",
        "hlt",
    );
}
