use crate::flag::Flag;

pub(crate) fn emit(unordered: &[Flag], ordered: &[Flag]) -> String {
    unordered
        .iter()
        .chain(ordered)
        .flat_map(Flag::to_tokens)
        .map(|t| shell_quote(&t))
        .collect::<Vec<_>>()
        .join(" ")
}

// shlex::try_quote quotes on `=` and `,`; too noisy for flags.
fn shell_quote(tok: &str) -> String {
    if tok.is_empty() {
        return "''".into();
    }
    if tok.chars().all(is_shell_safe) {
        return tok.into();
    }
    let mut out = String::with_capacity(tok.len() + 2);
    out.push('\'');
    for c in tok.chars() {
        if c == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(c);
        }
    }
    out.push('\'');
    out
}

fn is_shell_safe(c: char) -> bool {
    matches!(
        c,
        'a'..='z' | 'A'..='Z' | '0'..='9'
        | '-' | '_' | '.' | '/' | '=' | ',' | '+' | ':' | '@' | '%'
    )
}
