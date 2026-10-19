//! Task prefixes ("todo", "remind me to", "✓", …) at the start of the input.

const PREFIXES: &[&str] = &[
    "remind me about",
    "remind me to",
    "remember to",
    "remind me",
    "reminder",
    "to-do",
    "to do",
    "todo",
    "task",
    "remind",
    "✓",
];

/// Returns the byte length of a leading task prefix, including a following
/// `:` if present, or `None`.
pub(crate) fn task_prefix_len(input: &str) -> Option<usize> {
    let lower = input.to_lowercase();
    if lower.len() != input.len() {
        // Lowercasing changed byte lengths; fall back to an ASCII-only check.
        return ascii_prefix_len(input);
    }
    for p in PREFIXES {
        if lower.starts_with(p) {
            let rest = &input[p.len()..];
            let boundary = rest.is_empty()
                || rest.starts_with(char::is_whitespace)
                || rest.starts_with(':')
                || *p == "✓";
            if boundary {
                let colon = usize::from(rest.starts_with(':'));
                return Some(p.len() + colon);
            }
        }
    }
    None
}

fn ascii_prefix_len(input: &str) -> Option<usize> {
    for p in PREFIXES {
        if input.len() >= p.len()
            && input.is_char_boundary(p.len())
            && input[..p.len()].eq_ignore_ascii_case(p)
        {
            let rest = &input[p.len()..];
            if rest.is_empty() || rest.starts_with(char::is_whitespace) || rest.starts_with(':') {
                return Some(p.len() + usize::from(rest.starts_with(':')));
            }
        }
    }
    None
}
