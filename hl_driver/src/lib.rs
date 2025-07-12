#![cfg_attr(not(feature = "unit-tests"), no_std)]

pub mod debounce;
pub mod encoder;
pub mod switch;

#[cfg(any(test, doc))]
pub mod test_utils;
