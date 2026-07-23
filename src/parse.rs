use crate::error::{Error, Warning};
use crate::flag::{Flag, MachineSpec, OptLevel};

pub(crate) fn parse(input: &str) -> Result<(Vec<Flag>, Vec<Warning>), Error> {
    if input.contains('\0') {
        return Err(Error::NulByte);
    }
    let tokens = shlex::split(input).ok_or(Error::UnterminatedQuote)?;
    let mut flags = Vec::with_capacity(tokens.len());
    let mut warnings = Vec::new();

    let mut i = 0;
    while i < tokens.len() {
        let tok = &tokens[i];
        let next = tokens.get(i + 1);
        let (flag, consumed) = classify(tok, next, &mut warnings)?;
        flags.push(flag);
        i += consumed;
    }
    Ok((flags, warnings))
}

fn classify(
    tok: &str,
    next: Option<&String>,
    warnings: &mut Vec<Warning>,
) -> Result<(Flag, usize), Error> {
    let need_arg = |t: &str| -> Result<&String, Error> {
        next.ok_or_else(|| Error::MissingArgument(t.to_string()))
    };
    match tok {
        "-Xlinker" => return Ok((Flag::Xlinker(need_arg(tok)?.clone()), 2)),
        "-include" => return Ok((Flag::PreInclude(need_arg(tok)?.clone()), 2)),
        "-isystem" | "-iquote" | "-idirafter" | "-I" | "-L" | "-D" | "-U" | "-l" => {
            let glued = format!("{}{}", tok, need_arg(tok)?);
            return classify(&glued, None, warnings).map(|(f, _)| (f, 2));
        }
        _ => {}
    }
    if let Some(f) = classify_single(tok) {
        return Ok((f, 1));
    }
    warnings.push(Warning::UnknownFlag(tok.to_string()));
    Ok((Flag::Raw(tok.to_string()), 1))
}

fn classify_single(tok: &str) -> Option<Flag> {
    // bare -O = -O1 (clang Options.td).
    if let Some(rest) = tok.strip_prefix("-O") {
        if rest.is_empty() {
            return Some(Flag::OptLevel(OptLevel('1')));
        }
        return parse_opt_level(rest).map(Flag::OptLevel);
    }
    // -g level and format are independent axes.
    if tok == "-g" {
        return Some(Flag::DebugLevel(None));
    }
    if let Some(rest) = tok.strip_prefix("-g") {
        if rest.len() == 1 {
            let b = rest.as_bytes()[0];
            if (b'0'..=b'3').contains(&b) {
                return Some(Flag::DebugLevel(Some(b - b'0')));
            }
        }
        return Some(Flag::DebugFormat(rest.to_string()));
    }
    if let Some(rest) = tok.strip_prefix("-std=") {
        return Some(Flag::Std(rest.to_string()));
    }
    if tok == "-pipe" {
        return Some(Flag::Pipe);
    }
    if let Some(rest) = tok.strip_prefix("-march=") {
        return parse_machine_spec(rest).map(Flag::March);
    }
    if let Some(rest) = tok.strip_prefix("-mcpu=") {
        return parse_machine_spec(rest).map(Flag::Mcpu);
    }
    if let Some(rest) = tok.strip_prefix("-mtune=") {
        return parse_machine_spec(rest).map(Flag::Mtune);
    }
    if let Some(rest) = tok.strip_prefix("-mabi=").filter(|s| !s.is_empty()) {
        return Some(Flag::Mabi(rest.to_ascii_lowercase()));
    }
    if let Some(rest) = tok.strip_prefix("-mfloat-abi=").filter(|s| !s.is_empty()) {
        return Some(Flag::MfloatAbi(rest.to_ascii_lowercase()));
    }
    if let Some(rest) = tok.strip_prefix("-mfpu=").filter(|s| !s.is_empty()) {
        return Some(Flag::Mfpu(rest.to_ascii_lowercase()));
    }
    if let Some(rest) = tok.strip_prefix("-mcmodel=").filter(|s| !s.is_empty()) {
        return Some(Flag::Mcmodel(rest.to_ascii_lowercase()));
    }
    match tok {
        "-m16" | "-m32" | "-m64" | "-mx32" => return Some(Flag::Mwidth(tok[2..].into())),
        "-mthumb" => return Some(Flag::Mthumb(true)),
        // GCC: -marm is the RejectNegative inverse of -mthumb.
        "-mno-thumb" | "-marm" => return Some(Flag::Mthumb(false)),
        "-mlittle-endian" => return Some(Flag::Mendian("little".into())),
        "-mbig-endian" => return Some(Flag::Mendian("big".into())),
        "-mhard-float" => return Some(Flag::MhardFloat(true)),
        "-msoft-float" => return Some(Flag::MhardFloat(false)),
        "-malign-double" => return Some(Flag::MalignDouble(true)),
        "-mno-align-double" => return Some(Flag::MalignDouble(false)),
        "-mgeneral-regs-only" => return Some(Flag::MgeneralRegsOnly(true)),
        "-mno-general-regs-only" => return Some(Flag::MgeneralRegsOnly(false)),
        "-mms-bitfields" => return Some(Flag::MmsBitfields(true)),
        "-mno-ms-bitfields" => return Some(Flag::MmsBitfields(false)),
        _ => {}
    }
    if let Some(rest) = tok.strip_prefix("-D") {
        if rest.is_empty() {
            return None;
        }
        let (name, value) = match rest.split_once('=') {
            Some((n, v)) => (n.into(), Some(v.into())),
            None => (rest.into(), None),
        };
        return Some(Flag::Define { name, value });
    }
    if let Some(rest) = tok.strip_prefix("-U").filter(|s| !s.is_empty()) {
        return Some(Flag::Undef(rest.into()));
    }
    // -Wl, must precede -W (linker, not warning).
    if let Some(rest) = tok.strip_prefix("-Wl,") {
        return Some(Flag::LinkerArg(rest.split(',').map(str::to_string).collect()));
    }
    if let Some(rest) = tok.strip_prefix("-Wno-").filter(|s| !s.is_empty()) {
        return Some(Flag::Warn { name: rest.into(), on: false });
    }
    if let Some(rest) = tok.strip_prefix("-W").filter(|s| !s.is_empty()) {
        return Some(Flag::Warn { name: rest.into(), on: true });
    }
    if let Some(rest) = tok.strip_prefix("-fno-").filter(|s| !s.is_empty()) {
        return Some(Flag::Toggle { name: rest.into(), on: false });
    }
    if let Some(rest) = tok.strip_prefix("-f").filter(|s| !s.is_empty()) {
        return Some(Flag::Toggle { name: rest.into(), on: true });
    }
    if let Some(rest) = tok.strip_prefix("-isystem") {
        return Some(Flag::SystemInclude(rest.into()));
    }
    if let Some(rest) = tok.strip_prefix("-iquote") {
        return Some(Flag::QuoteInclude(rest.into()));
    }
    if let Some(rest) = tok.strip_prefix("-idirafter") {
        return Some(Flag::DirAfterInclude(rest.into()));
    }
    if let Some(rest) = tok.strip_prefix("-I").filter(|s| !s.is_empty()) {
        return Some(Flag::Include(rest.into()));
    }
    if let Some(rest) = tok.strip_prefix("-L").filter(|s| !s.is_empty()) {
        return Some(Flag::LibPath(rest.into()));
    }
    if let Some(rest) = tok.strip_prefix("-l").filter(|s| !s.is_empty()) {
        return Some(Flag::Lib(rest.into()));
    }
    None
}

fn parse_opt_level(s: &str) -> Option<OptLevel> {
    let mut chars = s.chars();
    let c = chars.next()?;
    if chars.next().is_some() {
        return None;
    }
    matches!(c, '0'..='3' | 's' | 'g' | 'z').then_some(OptLevel(c))
}

fn parse_machine_spec(s: &str) -> Option<MachineSpec> {
    let mut parts = s.split('+');
    let name = parts.next()?.to_ascii_lowercase();
    if name.is_empty() {
        return None;
    }
    let suffixes = parts.map(str::to_string).collect();
    Some(MachineSpec { name, suffixes })
}
