//! Splitting a search fragment into the terms a search matches on.

/// Split a free-text search fragment into the individual terms to match.
///
/// A term is a maximal run of alphanumeric characters; every other
/// character — whitespace, `_`, `-`, `.`, `/`, `,`, … — separates
/// terms. The rule mirrors Postgres' `to_tsvector` tokeniser, which
/// breaks on the same boundary, so a term always lines up with one
/// lexeme of the `tsv` column.
///
/// Every consumer splits the same way: the Postgres adapter embeds
/// the terms in a `tsquery` expression, and the in-memory
/// repositories match each term separately. That keeps the question
/// "what does the fragment `vital sign` mean?" — two terms, both of
/// which must match — answered in exactly one place.
pub(crate) fn split(fragment: &str) -> Vec<String> {
    fragment
        .split(|c: char| !c.is_alphanumeric())
        .filter(|term| !term.is_empty())
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::split;

    #[test]
    fn splits_on_whitespace() {
        assert_eq!(split("vital sign"), vec!["vital", "sign"]);
    }

    #[test]
    fn splits_on_punctuation_and_drops_empty_terms() {
        assert_eq!(split("vital, sign -- age"), vec!["vital", "sign", "age"]);
        assert_eq!(split("   "), Vec::<String>::new());
        assert_eq!(split("---"), Vec::<String>::new());
        assert_eq!(split("&|!():"), Vec::<String>::new());
    }

    #[test]
    fn splits_inside_hyphenated_and_underscored_identifiers() {
        assert_eq!(split("non-small"), vec!["non", "small"]);
        assert_eq!(split("BMI_CATEGORY"), vec!["BMI", "CATEGORY"]);
    }

    #[test]
    fn keeps_unicode_letters_together() {
        assert_eq!(split("体重 身高"), vec!["体重", "身高"]);
    }

    #[test]
    fn keeps_digits_with_their_letters() {
        assert_eq!(split("C123456"), vec!["C123456"]);
        assert_eq!(split("COVID-19"), vec!["COVID", "19"]);
    }
}
