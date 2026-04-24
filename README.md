# sokgi (솎기)

> Note: this is slop coded.

*sokgi* — Korean for "thinning". Canonicalize GCC-style compiler flags
into a stable string for content hashing: weed out redundant and
overridden flags, keep only the effective set.

Inputs differing only in token order, duplicates, or GCC override chains
(last-wins `-O`, `-march` over `-mcpu`, `-U` over `-D`, …) produce the
same canonical string.

```rust
use sokgi::{Dialect, FlagSet};
let (set, _) = FlagSet::parse("-g -O3 -O2 -march=Cortex-A76+crc", Dialect::C).unwrap();
assert_eq!(set.canonical(), "-O2 -g -march=cortex-a76+crc");
```

## Rules

| Flag | Treatment |
|---|---|
| `-O[0-3sgz]` | last-wins; bare `-O` = `-O1` |
| `-g[0-3]`, `-g<format>` | last-wins per axis; `-g0` drops format |
| `-std=` | last-wins |
| `-march=`, `-mcpu=`, `-mtune=` | last-wins per kind; `-march` drops `-mcpu` |
| `-D<N>[=V]` / `-U<N>` | last-wins per `N`; `-U` beats `-D` (POSIX c99) |
| `-f<n>` / `-fno-<n>`, `-W<n>` / `-Wno-<n>` | last-wins per `n` |
| `-pipe` | idempotent |
| `-I`, `-isystem`, `-iquote`, `-idirafter`, `-include` | order preserved |
| `-L`, `-l`, `-Wl,…`, `-Xlinker` | order preserved |
| source/object files | preserved at position |
| unknown | verbatim + `Warning::UnknownFlag` |

CPU name in `-m{arch,cpu,tune}=` is lowercased. Feature suffixes
(`+crc`, `+nocrypto`, …) are verbatim — `+no<feat>` overrides left-to-
right, so reordering is unsound.

Paths are never normalized (`-I/a/../b` stays as-is); symlinks and
bind-mounts make lexical normalization unsafe.

## Use

For build systems that want semantically-equivalent flags to share a
cache entry (content-addressed stores, cross-compile caches, custom
Bazel/Buck2 rules).

Not `ccache`/`sccache`. Those hash raw strings for correctness; this
trades that for cache reuse.

## Scope

CFLAGS, CXXFLAGS, LDFLAGS. RUSTFLAGS planned.

LDFLAGS is shallow: whitespace + `-Wl,` comma split. Linker state
(`--as-needed` pairs, library order) is preserved verbatim.

Out of scope: `-march=native` resolution, ISA feature expansion, CPU
compatibility checks, response-file (`@file`) expansion.

## API

```rust
pub enum Dialect { C, Cxx, Ld, Rust }

impl FlagSet {
    pub fn parse(input: &str, dialect: Dialect)
        -> Result<(Self, Vec<Warning>), Error>;
    pub fn canonical(&self) -> String;
}

pub enum Warning {
    UnknownFlag(String),
    DroppedByOverride { dropped: String, by: String },
    ConflictingDefine(String),
}
```

## Prior art

- [Arash1381-y/cflag-parser](https://github.com/Arash1381-y/cflag-parser)
- [aperezdc/cflag](https://github.com/aperezdc/cflag)
- [Psteven5/CFLAG.h](https://github.com/Psteven5/CFLAG.h)

All three are CLI argument parsers. This crate targets a different
niche — semantic canonicalization for content-addressed build caches
(e.g. [crossdev-stages](https://github.com/lu-zero/crossdev-stages)).

## License

Apache-2.0
