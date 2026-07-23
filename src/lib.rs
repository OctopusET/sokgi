//! Canonicalize GCC-style compiler flags for content hashing.
//!
//! ```
//! use sokgi::{Dialect, FlagSet};
//! let (set, _) = Dialect::C.parse("-O3 -O2 -g").unwrap();
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
        self.unordered.iter().chain(&self.ordered).any(|f| match f {
            Flag::March(m) | Flag::Mcpu(m) | Flag::Mtune(m) => m.name == "native",
            _ => false,
        })
    }

    /// Stable hash of only the ABI-selecting flags: `-march=`, `-mcpu=`,
    /// `-mabi=`, `-m16`/`-m32`/`-m64`/`-mx32`, `-mfloat-abi=`, `-mfpu=`,
    /// `-mthumb`/`-mno-thumb`, `-mlittle-endian`/`-mbig-endian`,
    /// `-mhard-float`/`-msoft-float`, `-mcmodel=`, `-malign-double`,
    /// `-mgeneral-regs-only`, `-mms-bitfields`,
    /// and the layout toggles `-f[no-]short-enums`, `-f[no-]short-wchar`,
    /// `-f[no-]pack-struct[=n]`. 16 lowercase hex characters, or `""` if
    /// the set contains none of them.
    ///
    /// `-mtune=` is excluded: it changes instruction scheduling, not the
    /// ABI, so builds differing only in `-mtune=` share a key. Sets
    /// differing only in optimization, debug, or include flags share a
    /// key too, so a cache can allow reuse across those but not across
    /// targets. Flags outside the list above are not modeled.
    ///
    /// Frozen like [`FlagSet::stable_hash`]: FNV-1a 64-bit over the
    /// canonical form of the ABI flags, pinned by golden tests.
    ///
    /// ```
    /// use sokgi::Dialect;
    /// // Different optimization or -mtune, same target → same ABI key
    /// let (a, _) = Dialect::C.parse("-O2 -march=armv8-a").unwrap();
    /// let (b, _) = Dialect::C.parse("-O3 -march=armv8-a -mtune=cortex-a55").unwrap();
    /// assert_eq!(a.abi_key(), b.abi_key());
    ///
    /// // Different float ABI → different ABI key
    /// let (c, _) = Dialect::C.parse("-mfloat-abi=hard").unwrap();
    /// let (d, _) = Dialect::C.parse("-mfloat-abi=softfp").unwrap();
    /// assert_ne!(c.abi_key(), d.abi_key());
    /// ```
    pub fn abi_key(&self) -> String {
        let abi_flags: Vec<Flag> = self
            .unordered
            .iter()
            .chain(&self.ordered)
            .filter(|f| is_abi_flag(f))
            .cloned()
            .collect();

        if abi_flags.is_empty() {
            return String::new();
        }

        let canonical = emit::emit(&abi_flags, &[]);
        format!("{:016x}", hash::fnv1a_64(canonical.as_bytes()))
    }
}

fn is_abi_flag(f: &Flag) -> bool {
    match f {
        Flag::March(_)
        | Flag::Mcpu(_)
        | Flag::Mabi(_)
        | Flag::Mwidth(_)
        | Flag::MfloatAbi(_)
        | Flag::Mfpu(_)
        | Flag::Mthumb(_)
        | Flag::Mendian(_)
        | Flag::MhardFloat(_)
        | Flag::Mcmodel(_)
        | Flag::MalignDouble(_)
        | Flag::MgeneralRegsOnly(_)
        | Flag::MmsBitfields(_) => true,
        Flag::Toggle { name, .. } => {
            name == "short-enums"
                || name == "short-wchar"
                || name == "pack-struct"
                || name.starts_with("pack-struct=")
        }
        _ => false,
    }
}

impl Dialect {
    /// Parse a string of compiler flags, returning a [`FlagSet`] and warnings.
    ///
    /// This is a shorthand for [`FlagSet::parse`]:
    ///
    /// ```
    /// use sokgi::{Dialect, FlagSet};
    /// // Instead of:
    /// let (set, warnings) = FlagSet::parse("-O3 -O2 -g", Dialect::C).unwrap();
    ///
    /// // You can write:
    /// let (set, warnings) = Dialect::C.parse("-O3 -O2 -g").unwrap();
    /// assert_eq!(set.canonical(), "-O2 -g");
    /// ```
    pub fn parse(self, input: &str) -> Result<(FlagSet, Vec<Warning>), Error> {
        FlagSet::parse(input, self)
    }
}
