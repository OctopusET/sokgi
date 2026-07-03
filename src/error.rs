use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum Error {
    #[error("unterminated quote in input")]
    UnterminatedQuote,
    #[error("missing argument for flag {0}")]
    MissingArgument(String),
    #[error("NUL byte in input is not representable as a shell token")]
    NulByte,
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum Warning {
    #[error("unknown flag kept verbatim: {0}")]
    UnknownFlag(String),
    #[error("dropped {dropped} (overridden by {by})")]
    DroppedByOverride { dropped: String, by: String },
    #[error("conflicting -D{0} definitions; last wins")]
    ConflictingDefine(String),
    #[error("{0} depends on the build machine; unfit as a stable cache key")]
    MachineDependent(String),
}
