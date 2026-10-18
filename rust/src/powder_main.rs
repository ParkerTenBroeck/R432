#![no_std]
#![no_main]

mod engine;

mod powder;

#[unsafe(no_mangle)]
pub fn main() {
    powder::run();
}
