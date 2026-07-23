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
| `-mabi=` | last-wins; lowercased |
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

`-march=native` (and `-mcpu`/`-mtune`) is kept verbatim but raises
`Warning::MachineDependent`: it resolves to the build machine's CPU,
so it can never be a machine-independent cache key.

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
    pub fn stable_hash(&self) -> u64;
    pub fn stable_hash_hex(&self) -> String; // 16 lowercase hex chars
    pub fn abi_key(&self) -> String;         // target flags only; "" if none
    pub fn is_machine_dependent(&self) -> bool; // any -m*=native
}

impl Dialect {
    pub fn parse(self, input: &str) // shorthand for FlagSet::parse
        -> Result<(FlagSet, Vec<Warning>), Error>;
}

pub enum Warning {
    UnknownFlag(String),
    DroppedByOverride { dropped: String, by: String },
    ConflictingDefine(String),
    MachineDependent(String),
}
```

## Stable hash

`stable_hash()` is FNV-1a 64-bit over the UTF-8 bytes of
`canonical()` — nothing else. Offset basis `0xcbf29ce484222325`,
prime `0x100000001b3`, implemented inline; no dependencies.

```rust
use sokgi::{Dialect, FlagSet};
let (set, _) = FlagSet::parse("-pipe -O3 -O3 -march=rv64gc", Dialect::C).unwrap();
assert_eq!(set.stable_hash_hex(), "d70421711002d7dc"); // hash of "-pipe -O3 -march=rv64gc"
```

**Stability guarantee:** the algorithm and the canonical form it
feeds on are frozen. Hash values are independent of rustc version,
platform, endianness, and future sokgi releases, so they are safe as
persistent content-addressed store keys. Golden tests in
`tests/stable_hash.rs` pin literal values; a change there means every
existing store key is invalid, and is treated as a breaking release.

## ABI key

`abi_key()` hashes only the target-selection flags (`-march=`,
`-mcpu=`, `-mtune=`, `-mabi=`) after canonicalization: FNV-1a 64-bit,
16 hex chars, `""` if none present. Sets differing only in `-O`, `-g`,
or include flags share a key, so a cache can reuse across optimization
levels but not across targets.

Equal keys mean equal target flags, not proven ABI compatibility:
other ABI-affecting flags (`-mfloat-abi=`, `-fshort-enums`, …) are not
modeled, and `-mtune=` is included although it affects only
scheduling. Frozen like `stable_hash`, pinned in
`tests/stable_hash.rs`.

## Prior art

- [Arash1381-y/cflag-parser](https://github.com/Arash1381-y/cflag-parser)
- [aperezdc/cflag](https://github.com/aperezdc/cflag)
- [Psteven5/CFLAG.h](https://github.com/Psteven5/CFLAG.h)

All three are CLI argument parsers. This crate targets a different
niche — semantic canonicalization for content-addressed build caches
(e.g. [crossdev-stages](https://github.com/lu-zero/crossdev-stages)).

## License

Apache-2.0
