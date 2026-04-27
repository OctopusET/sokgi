use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
    C,
    Cxx,
    Ld,
    Rust,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Flag {
    OptLevel(OptLevel),
    DebugLevel(Option<u8>),
    DebugFormat(String),
    Std(String),
    Pipe,
    March(MachineSpec),
    Mcpu(MachineSpec),
    Mtune(MachineSpec),
    Define { name: String, value: Option<String> },
    Undef(String),
    Toggle { name: String, on: bool },
    Warn { name: String, on: bool },

    Include(String),
    SystemInclude(String),
    QuoteInclude(String),
    DirAfterInclude(String),
    PreInclude(String),
    LibPath(String),
    Lib(String),
    LinkerArg(Vec<String>),
    Xlinker(String),

    Raw(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineSpec {
    pub name: String,
    pub suffixes: Vec<String>,
}

impl fmt::Display for MachineSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)?;
        for s in &self.suffixes {
            write!(f, "+{}", s)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OptLevel(pub char);

impl fmt::Display for OptLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Flag {
    pub fn to_tokens(&self) -> Vec<String> {
        match self {
            Flag::OptLevel(o) => vec![format!("-O{}", o)],
            Flag::DebugLevel(None) => vec!["-g".into()],
            Flag::DebugLevel(Some(n)) => vec![format!("-g{}", n)],
            Flag::DebugFormat(s) => vec![format!("-g{}", s)],
            Flag::Std(s) => vec![format!("-std={}", s)],
            Flag::Pipe => vec!["-pipe".into()],
            Flag::March(m) => vec![format!("-march={}", m)],
            Flag::Mcpu(m) => vec![format!("-mcpu={}", m)],
            Flag::Mtune(m) => vec![format!("-mtune={}", m)],
            Flag::Define { name, value: Some(v) } => vec![format!("-D{}={}", name, v)],
            Flag::Define { name, value: None } => vec![format!("-D{}", name)],
            Flag::Undef(n) => vec![format!("-U{}", n)],
            Flag::Toggle { name, on: true } => vec![format!("-f{}", name)],
            Flag::Toggle { name, on: false } => vec![format!("-fno-{}", name)],
            Flag::Warn { name, on: true } => vec![format!("-W{}", name)],
            Flag::Warn { name, on: false } => vec![format!("-Wno-{}", name)],
            Flag::Include(p) => vec![format!("-I{}", p)],
            Flag::SystemInclude(p) => vec![format!("-isystem{}", p)],
            Flag::QuoteInclude(p) => vec![format!("-iquote{}", p)],
            Flag::DirAfterInclude(p) => vec![format!("-idirafter{}", p)],
            Flag::PreInclude(p) => vec!["-include".into(), p.clone()],
            Flag::LibPath(p) => vec![format!("-L{}", p)],
            Flag::Lib(n) => vec![format!("-l{}", n)],
            Flag::LinkerArg(parts) => {
                let mut s = String::from("-Wl");
                for p in parts {
                    s.push(',');
                    s.push_str(p);
                }
                vec![s]
            }
            Flag::Xlinker(a) => vec!["-Xlinker".into(), a.clone()],
            Flag::Raw(s) => vec![s.clone()],
        }
    }
}

