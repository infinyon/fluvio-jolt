#![no_main]

use libfuzzer_sys::fuzz_target;
use fluvio_jolt::dsl::{Lhs, Rhs};

fuzz_target!(|data: &str| {
    Lhs::parse(data).ok();
    Rhs::parse(data).ok();
});
