//! FNV-1a 64-bit (Fowler–Noll–Vo), per the reference specification.
//!
//! FROZEN. The constants and the algorithm are sokgi's stable-hash
//! contract: any change invalidates every content-addressed store
//! keyed on [`crate::FlagSet::stable_hash`]. Do not touch.

const OFFSET_BASIS: u64 = 0xcbf29ce484222325;
const PRIME: u64 = 0x100000001b3;

pub(crate) fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut h = OFFSET_BASIS;
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(PRIME);
    }
    h
}
