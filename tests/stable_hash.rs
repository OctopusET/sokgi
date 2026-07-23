//! Golden tests pinning `stable_hash` to literal values.
//!
//! These literals are the stability contract for content-addressed
//! stores keyed on sokgi. If any assertion here starts failing, the
//! canonical form or the hash drifted and every existing store key
//! is invalid. Never "fix" the expected values without accepting
//! that consequence.

use sokgi::{Dialect, FlagSet};

fn set(input: &str) -> FlagSet {
    FlagSet::parse(input, Dialect::C).unwrap().0
}

fn hash(input: &str) -> u64 {
    set(input).stable_hash()
}

#[test]
fn golden_empty_is_offset_basis() {
    // canonical("") == "" hashes to the FNV-1a offset basis.
    assert_eq!(hash(""), 0xcbf29ce484222325);
    assert_eq!(hash("   "), 0xcbf29ce484222325);
    assert_eq!(set("").stable_hash_hex(), "cbf29ce484222325");
}

#[test]
fn golden_simple() {
    assert_eq!(hash("-O2 -g"), 0x4ef5b040b8c34ca3);
    assert_eq!(hash("-O2 -pipe"), 0xf60f7cfabf169e32);
    assert_eq!(hash("-O2"), 0xdef04f17de7d5c05);
    assert_eq!(hash("-O3"), 0xdef04e17de7d5a52);
}

#[test]
fn golden_reorder_and_dedup_equal() {
    assert_eq!(hash("-O3 -march=rv64gc -pipe"), 0xd70421711002d7dc);
    assert_eq!(hash("-pipe -O3 -O3 -march=rv64gc"), 0xd70421711002d7dc);
}

#[test]
fn golden_different_flags_differ() {
    assert_eq!(hash("-march=rv64gc"), 0xc201ea439831812a);
    assert_eq!(hash("-march=rv64gc_zba_zbb"), 0xbb54edd9d191c7a7);
    assert_ne!(hash("-march=rv64gc"), hash("-march=rv64gc_zba_zbb"));
    assert_ne!(hash("-O2"), hash("-O3"));
}

#[test]
fn golden_march_last_wins() {
    // Duplicate -march: last occurrence wins, hashing as if it were alone.
    assert_eq!(
        hash("-march=rv64gc -march=rv64gc_zba_zbb"),
        0xbb54edd9d191c7a7
    );
}

#[test]
fn golden_riscv_board_flags() {
    assert_eq!(hash("-march=rv64gc -mabi=lp64d -O2 -pipe"), 0x9da444df1bff96e8);
    assert_eq!(hash("-pipe -mabi=lp64d -O2 -march=rv64gc"), 0x9da444df1bff96e8);
}

#[test]
fn hex_is_16_lowercase_chars() {
    let hex = set("-O2 -pipe").stable_hash_hex();
    assert_eq!(hex, "f60f7cfabf169e32");
    assert_eq!(hex.len(), 16);
    assert!(hex.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
}

#[test]
fn golden_abi_key() {
    // abi_key == FNV-1a-64 hex of the canonical target flags only.
    assert_eq!(set("-O2 -g -march=armv8-a").abi_key(), "bd45f0705391150c");
    assert_eq!(
        set("-pipe -mabi=lp64d -O2 -march=rv64gc").abi_key(),
        "bd73b76f1d69b62d"
    );
    assert_eq!(set("-mtune=generic").abi_key(), "625cec8df7399ff1");
    // -march drops -mcpu before keying.
    assert_eq!(set("-march=x86-64 -mcpu=nehalem").abi_key(), "bb36f8e7f8447eb1");
    assert_eq!(set("-mcpu=cortex-a55").abi_key(), "088bb00be97cac57");
    assert_eq!(set("-O2 -g -pipe").abi_key(), "");
}

#[test]
fn hash_is_over_canonical_bytes_only() {
    // Contract: stable_hash == FNV-1a-64 of canonical(); re-parsing the
    // canonical string yields the same hash.
    let a = set("-DFOO=1 -g -O2 -I/a -I/b");
    let b = set(&a.canonical());
    assert_eq!(a.canonical(), b.canonical());
    assert_eq!(a.stable_hash(), b.stable_hash());
}
