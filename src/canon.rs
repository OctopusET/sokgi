use crate::error::Warning;
use crate::flag::{Dialect, Flag, MachineSpec};

// `native` resolves to whatever CPU runs the compiler; the canonical
// string is kept verbatim but can never be a machine-independent key.
fn warn_native(kind: &str, m: &MachineSpec, warnings: &mut Vec<Warning>) {
    if m.name == "native" {
        warnings.push(Warning::MachineDependent(format!("{}={}", kind, m)));
    }
}

pub(crate) fn canonicalize(
    flags: Vec<Flag>,
    dialect: Dialect,
    warnings: &mut Vec<Warning>,
) -> (Vec<Flag>, Vec<Flag>) {
    let mut ordered: Vec<Flag> = Vec::new();
    let mut unordered: Vec<Flag> = Vec::new();
    let ld = matches!(dialect, Dialect::Ld);

    let mut opt: Option<Flag> = None;
    let mut dbg_level: Option<Flag> = None;
    let mut dbg_format: Option<Flag> = None;
    let mut std_flag: Option<Flag> = None;
    let mut pipe = false;
    let mut march: Option<Flag> = None;
    let mut mcpu: Option<Flag> = None;
    let mut mtune: Option<Flag> = None;
    let mut mabi: Option<Flag> = None;
    let mut mwidth: Option<Flag> = None;
    let mut mfloat_abi: Option<Flag> = None;
    let mut mfpu: Option<Flag> = None;
    let mut defines: Vec<(String, Option<String>)> = Vec::new();
    let mut undefs: Vec<String> = Vec::new();
    let mut toggles: Vec<(String, bool)> = Vec::new();
    let mut warns: Vec<(String, bool)> = Vec::new();
    let mut last_mcpu_raw: Option<String> = None;
    let mut last_march_raw: Option<String> = None;

    for f in flags {
        if ld && !matches!(f, Flag::Raw(_)) {
            ordered.push(f);
            continue;
        }
        match f {
            Flag::OptLevel(_) => opt = Some(f),
            Flag::DebugLevel(_) => dbg_level = Some(f),
            Flag::DebugFormat(_) => dbg_format = Some(f),
            Flag::Std(_) => std_flag = Some(f),
            Flag::Pipe => pipe = true,
            Flag::March(ref m) => {
                warn_native("-march", m, warnings);
                last_march_raw = Some(format!("-march={}", m));
                march = Some(f);
            }
            Flag::Mcpu(ref m) => {
                warn_native("-mcpu", m, warnings);
                last_mcpu_raw = Some(format!("-mcpu={}", m));
                mcpu = Some(f);
            }
            Flag::Mtune(ref m) => {
                warn_native("-mtune", m, warnings);
                mtune = Some(f);
            }
            Flag::Mabi(_) => mabi = Some(f),
            Flag::Mwidth(_) => mwidth = Some(f),
            Flag::MfloatAbi(_) => mfloat_abi = Some(f),
            Flag::Mfpu(_) => mfpu = Some(f),
            Flag::Define { name, value } => {
                if let Some(i) = defines.iter().position(|(n, _)| n == &name) {
                    if defines[i].1 != value {
                        warnings.push(Warning::ConflictingDefine(name.clone()));
                    }
                    defines.remove(i);
                }
                defines.push((name, value));
            }
            Flag::Undef(n) => {
                if !undefs.contains(&n) {
                    undefs.push(n);
                }
            }
            Flag::Toggle { name, on } => {
                if let Some(i) = toggles.iter().position(|(n, _)| n == &name) {
                    toggles.remove(i);
                }
                toggles.push((name, on));
            }
            Flag::Warn { name, on } => {
                if let Some(i) = warns.iter().position(|(n, _)| n == &name) {
                    warns.remove(i);
                }
                warns.push((name, on));
            }
            _ => ordered.push(f),
        }
    }

    if march.is_some() && mcpu.is_some() {
        warnings.push(Warning::DroppedByOverride {
            dropped: last_mcpu_raw.unwrap_or_else(|| "-mcpu".into()),
            by: last_march_raw.unwrap_or_else(|| "-march".into()),
        });
        mcpu = None;
    }

    // POSIX c99: -U beats -D.
    defines.retain(|(n, _)| !undefs.contains(n));

    // -g0 nullifies format.
    if matches!(dbg_level, Some(Flag::DebugLevel(Some(0)))) {
        dbg_format = None;
    }

    if pipe {
        unordered.push(Flag::Pipe);
    }
    unordered.extend(std_flag);
    unordered.extend(opt);
    unordered.extend(dbg_level);
    unordered.extend(dbg_format);
    unordered.extend(march);
    unordered.extend(mcpu);
    unordered.extend(mtune);
    unordered.extend(mabi);
    unordered.extend(mwidth);
    unordered.extend(mfloat_abi);
    unordered.extend(mfpu);

    defines.sort_by(|a, b| a.0.cmp(&b.0));
    for (name, value) in defines {
        unordered.push(Flag::Define { name, value });
    }
    undefs.sort();
    unordered.extend(undefs.into_iter().map(Flag::Undef));

    warns.sort_by(|a, b| a.0.cmp(&b.0));
    for (name, on) in warns {
        unordered.push(Flag::Warn { name, on });
    }
    toggles.sort_by(|a, b| a.0.cmp(&b.0));
    for (name, on) in toggles {
        unordered.push(Flag::Toggle { name, on });
    }

    (unordered, ordered)
}
