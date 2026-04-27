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
}
