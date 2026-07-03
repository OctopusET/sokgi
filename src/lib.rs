//! Canonicalize GCC-style compiler flags for content hashing.
//!
//! ```
//! use sokgi::{Dialect, FlagSet};
//! let (set, _) = FlagSet::parse("-O3 -O2 -g", Dialect::C).unwrap();
//! assert_eq!(set.canonical(), "-O2 -g");
//! ```

mod canon;
mod emit;
mod error;
mod flag;
mod hash;
mod parse;

pub use error::{Error, Warning};
pub use flag::{Dialect, Flag, MachineSpec, OptLevel};

#[derive(Debug, Clone)]
pub struct FlagSet {
    unordered: Vec<Flag>,
    ordered: Vec<Flag>,
}

impl FlagSet {
    pub fn parse(input: &str, dialect: Dialect) -> Result<(Self, Vec<Warning>), Error> {
        let (flags, mut warnings) = parse::parse(input)?;
        let (unordered, ordered) = canon::canonicalize(flags, dialect, &mut warnings);
        Ok((FlagSet { unordered, ordered }, warnings))
    }

    pub fn canonical(&self) -> String {
        emit::emit(&self.unordered, &self.ordered)
    }

    /// Stable 64-bit content hash for use as a persistent store key.
    ///
    /// Hashes exactly the UTF-8 bytes of [`FlagSet::canonical`] — nothing
    /// else — with FNV-1a 64-bit (offset basis `0xcbf29ce484222325`,
    /// prime `0x100000001b3`).
    ///
    /// The algorithm is frozen forever: the value is independent of rustc
    /// version, platform, endianness, and future sokgi releases, so store
    /// keys built on it never rot. Changing the hash — or the canonical
    /// form it feeds on — invalidates every store keyed on it, which is
    /// why both are pinned by literal-value tests.
    ///
    /// ```
    /// use sokgi::{Dialect, FlagSet};
    /// let (a, _) = FlagSet::parse("-O2 -g", Dialect::C).unwrap();
    /// let (b, _) = FlagSet::parse("-g -O2 -O2", Dialect::C).unwrap();
    /// assert_eq!(a.stable_hash(), b.stable_hash());
    /// ```
    pub fn stable_hash(&self) -> u64 {
        hash::fnv1a_64(self.canonical().as_bytes())
    }

    /// [`FlagSet::stable_hash`] as 16 lowercase hex characters.
    pub fn stable_hash_hex(&self) -> String {
        format!("{:016x}", self.stable_hash())
    }
}
