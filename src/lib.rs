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

    /// Returns true if this flag set contains any machine-dependent flags
    /// (i.e. flags that resolve to whatever CPU the compiler runs on:
    /// `-march=native`, `-mcpu=native`, `-mtune=native`).
    ///
    /// Such flags cannot produce a stable, portable cache key because their
    /// effective value depends on the build machine, not the flag string itself.
    ///
    /// ```
    /// use sokgi::{Dialect, FlagSet};
    /// let (set, _) = FlagSet::parse("-march=native", Dialect::C).unwrap();
    /// assert!(set.is_machine_dependent());
    ///
    /// let (set, _) = FlagSet::parse("-march=x86-64", Dialect::C).unwrap();
    /// assert!(!set.is_machine_dependent());
    /// ```
    pub fn is_machine_dependent(&self) -> bool {
        self.all_flags().iter().any(|f| {
            match f {
                Flag::March(m) | Flag::Mcpu(m) | Flag::Mtune(m) => m.name == "native",
                _ => false,
            }
        })
    }

    /// Returns a stable hash of only the ABI/ISA-impacting flags.
    ///
    /// Two flag sets with different optimization levels, debug info, or
    /// include paths but the same machine architecture flags will have
    /// the same ABI key, indicating they produce ABI-compatible (but not
    /// necessarily identical) binaries.
    ///
    /// ABI-impacting flags are those that affect code generation in ways
    /// that change binary compatibility:
    /// - `-march=`, `-mcpu=`, `-mtune=` (target selection)
    /// - `-mabi=` (ABI selection)
    ///
    /// This is useful for scenarios where you want to allow reuse across
    /// optimization levels but not across architectures.
    ///
    /// ```
    /// use sokgi::{Dialect, FlagSet};
    /// // Different optimization, same target → same ABI key
    /// let (a, _) = FlagSet::parse("-O2 -march=armv8-a", Dialect::C).unwrap();
    /// let (b, _) = FlagSet::parse("-O3 -march=armv8-a", Dialect::C).unwrap();
    /// assert_eq!(a.abi_key(), b.abi_key());
    ///
    /// // Same optimization, different target → different ABI key
    /// let (c, _) = FlagSet::parse("-O2 -march=armv8-a", Dialect::C).unwrap();
    /// let (d, _) = FlagSet::parse("-O2 -march=armv7-a", Dialect::C).unwrap();
    /// assert_ne!(c.abi_key(), d.abi_key());
    /// ```
    pub fn abi_key(&self) -> String {
        let abi_flags: Vec<Flag> = self
            .all_flags()
            .into_iter()
            .filter(|f| Self::is_abi_impacting(f))
            .collect();

        if abi_flags.is_empty() {
            return String::new();
        }

        // Separate into unordered and ordered (same logic as canonicalize)
        let (unordered, ordered): (Vec<Flag>, Vec<Flag>) = abi_flags
            .into_iter()
            .partition(|f| matches!(
                f,
                Flag::March(_) | Flag::Mcpu(_) | Flag::Mtune(_) | Flag::Mabi(_)
            ));

        let canonical = crate::emit::emit(&unordered, &ordered);
        format!("{:016x}", crate::hash::fnv1a_64(canonical.as_bytes()))
    }

    /// Returns all flags (both unordered and ordered) as a single vector.
    fn all_flags(&self) -> Vec<Flag> {
        let mut all = self.unordered.clone();
        all.extend(self.ordered.clone());
        all
    }

    /// Check if a flag impacts ABI/ISA compatibility.
    fn is_abi_impacting(flag: &Flag) -> bool {
        matches!(flag, Flag::March(_) | Flag::Mcpu(_) | Flag::Mtune(_) | Flag::Mabi(_))
    }
}
