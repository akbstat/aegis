//! Test-only helpers shared by the command modules' wire-shape tests.
//!
//! Tauri v2 derives a command's invoke payload keys from its Rust
//! parameter names: `tauri-macros` lower-camel-cases each parameter
//! and `tauri`'s resolver looks that key up in the payload with no
//! whole-payload fallback. A command whose parameter is renamed
//! therefore changes the payload the frontend must send, and the
//! mismatch only shows up at runtime — as "missing required key body".
//!
//! These helpers let a command module read its own source and check
//! the invoke keys it promises, so a rename breaks a test instead of
//! the app.

/// The index of the `)` that closes the parameter list.
pub fn matching_paren(s: &str) -> usize {
    let mut depth = 0usize;
    for (i, ch) in s.char_indices() {
        match ch {
            '(' | '<' | '[' => depth += 1,
            ')' | '>' | ']' => {
                if depth == 0 {
                    return i;
                }
                depth -= 1;
            }
            _ => {}
        }
    }
    panic!("no closing parenthesis");
}

/// Split a parameter list on the commas that separate parameters.
pub fn split_params(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut current = String::new();
    for ch in s.chars() {
        match ch {
            '(' | '<' | '[' => {
                depth += 1;
                current.push(ch);
            }
            ')' | '>' | ']' => {
                depth -= 1;
                current.push(ch);
            }
            ',' if depth == 0 => {
                out.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    if !current.trim().is_empty() {
        out.push(current.trim().to_string());
    }
    out
}

/// `structure_id` → `structureId`, the shape Tauri v2 looks up.
pub fn to_lower_camel(snake: &str) -> String {
    let mut out = String::new();
    for (i, part) in snake.split('_').enumerate() {
        if i == 0 {
            out.push_str(part);
        } else if let Some(first) = part.chars().next() {
            out.extend(first.to_uppercase());
            out.push_str(&part[1..]);
        }
    }
    out
}

/// Assert that every command in `expected` takes exactly the listed
/// invoke keys.
///
/// `source` is the command module's own source text — the caller
/// passes `include_str!("<module>.rs")`.
pub fn assert_invoke_keys(source: &str, expected: &[(&str, &[&str])]) {
    for (command, expected) in expected {
        let marker = format!("pub async fn {command}(");
        let start = source
            .find(&marker)
            .unwrap_or_else(|| panic!("command {command} not found in this file"));
        let params = &source[start + marker.len()..];
        let params = &params[..matching_paren(params)];

        let keys: Vec<String> = split_params(params)
            .iter()
            .filter_map(|p| p.split_once(':').map(|(name, _)| name.trim()))
            // `app_state` is the injected Tauri state, not an argument.
            .filter(|name| *name != "app_state")
            .map(to_lower_camel)
            .collect();

        let expected_keys: Vec<String> = expected.iter().map(|k| k.to_string()).collect();

        assert_eq!(keys, expected_keys, "invoke keys for {command}");
    }
}
