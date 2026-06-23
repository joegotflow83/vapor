// No-op stubs for UBSan runtime symbols injected into aws-lc-sys
// when it was compiled with zig cc (which enables UBSan in debug mode).
// Compile with:
//   rustc -C panic=abort --edition 2021 --emit=obj -o /tmp/ubsan_stubs.o ubsan_stubs.rs
#![no_std]
#![no_main]
#![allow(non_snake_case)]

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {}
}

#[no_mangle] pub unsafe extern "C" fn __ubsan_handle_alignment_assumption() {}
#[no_mangle] pub unsafe extern "C" fn __ubsan_handle_builtin_unreachable() {}
#[no_mangle] pub unsafe extern "C" fn __ubsan_handle_type_mismatch_v1() {}
#[no_mangle] pub unsafe extern "C" fn __ubsan_handle_shift_out_of_bounds() {}
#[no_mangle] pub unsafe extern "C" fn __ubsan_handle_out_of_bounds() {}
#[no_mangle] pub unsafe extern "C" fn __ubsan_handle_pointer_overflow() {}
#[no_mangle] pub unsafe extern "C" fn __ubsan_handle_add_overflow() {}
#[no_mangle] pub unsafe extern "C" fn __ubsan_handle_sub_overflow() {}
#[no_mangle] pub unsafe extern "C" fn __ubsan_handle_mul_overflow() {}
#[no_mangle] pub unsafe extern "C" fn __ubsan_handle_negate_overflow() {}
#[no_mangle] pub unsafe extern "C" fn __ubsan_handle_divrem_overflow() {}
#[no_mangle] pub unsafe extern "C" fn __ubsan_handle_load_invalid_value() {}
#[no_mangle] pub unsafe extern "C" fn __ubsan_handle_implicit_conversion() {}
#[no_mangle] pub unsafe extern "C" fn __ubsan_handle_invalid_builtin() {}
#[no_mangle] pub unsafe extern "C" fn __ubsan_handle_vla_bound_not_positive() {}
#[no_mangle] pub unsafe extern "C" fn __ubsan_handle_float_cast_overflow() {}
#[no_mangle] pub unsafe extern "C" fn __ubsan_handle_missing_return() {}
#[no_mangle] pub unsafe extern "C" fn __ubsan_handle_nonnull_arg() {}
#[no_mangle] pub unsafe extern "C" fn __ubsan_handle_nonnull_return_v1() {}
#[no_mangle] pub unsafe extern "C" fn __ubsan_handle_nullability_arg() {}
#[no_mangle] pub unsafe extern "C" fn __ubsan_handle_nullability_return_v1() {}
#[no_mangle] pub unsafe extern "C" fn __ubsan_handle_cfi_check_fail() {}
#[no_mangle] pub unsafe extern "C" fn __ubsan_handle_function_type_mismatch() {}
