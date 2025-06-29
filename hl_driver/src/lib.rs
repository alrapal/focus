#![cfg_attr(not(feature = "unit-tests"), no_std)]

pub mod debounce;
pub mod switch;

#[cfg(test)]
pub mod test_utils;
