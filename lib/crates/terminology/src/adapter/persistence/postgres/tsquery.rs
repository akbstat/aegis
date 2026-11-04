//! Building the `tsquery` expression the fragment search runs against.

use crate::domain::search_terms;

/// Build the argument for `to_tsquery('english', …)` from a search fragment.
///
/// Every term becomes a prefix-matched lexeme and the terms are
/// combined with `&`, so `"vital sign"` becomes `vital:* & sign:*` and
/// matches the rows whose text contains both terms.
///
/// `to_tsquery` parses its argument as a tsquery *expression*, not as
/// free text: two words separated by a space are a syntax error
/// (`syntax error in tsquery: "vital sign"`), because the grammar
/// wants an operator between the two lexemes. Building the expression
/// here keeps that `&` explicit.
///
/// Returns `None` when the fragment holds no usable term — the caller
/// then takes the plain list path instead of the search path.
pub(crate) fn build(fragment: &str) -> Option<String> {
    let terms = search_terms::split(fragment);
    if terms.is_empty() {
        return None;
    }
    Some(
        terms
            .iter()
            .map(|term| format!("{term}:*"))
            .collect::<Vec<_>>()
            .join(" & "),
    )
}

#[cfg(test)]
mod tests {
    use super::build;

    #[test]
    fn single_term_becomes_a_prefix_match() {
        assert_eq!(build("vital").as_deref(), Some("vital:*"));
    }

    #[test]
    fn multiple_terms_are_anded_with_explicit_operators() {
        assert_eq!(build("vital sign").as_deref(), Some("vital:* & sign:*"));
    }

    #[test]
    fn punctuation_only_fragments_have_no_terms() {
        assert_eq!(build(""), None);
        assert_eq!(build("   "), None);
        assert_eq!(build("&|!():"), None);
    }
}
