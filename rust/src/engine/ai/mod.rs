mod greedy;
mod random;

pub use greedy::Greedy;
pub use random::Random;

#[cfg(feature = "native")]
pub use lookahead::Lookahead;
#[cfg(feature = "native")]
mod lookahead;

#[inline(always)]
fn xorshift(mut value: u32) -> u32 {
    value ^= value << 13;
    value ^= value >> 17;
    value ^= value << 5;
    value
}
