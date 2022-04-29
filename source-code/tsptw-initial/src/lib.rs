pub type BitSet = smallbitset::MutSet256;
pub type Random = rand_xoshiro::Xoshiro256Plus;

pub mod error;
pub mod data;
pub mod parse;
