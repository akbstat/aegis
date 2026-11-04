# Terminology Multi-Word Search Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make a multi-word search fragment work — `vital sign` must search for both terms instead of failing with `syntax error in tsquery: "vital sign:*"`.

**Architecture:** The fragment is split into terms in one shared place (the domain), and the Postgres adapter turns those terms into a tsquery *expression* — `vital:* & sign:*` — that `to_tsquery` accepts. The two in-memory repositories learn the same "every term must match" rule, so the crates' own tests describe what Postgres does.

**Tech Stack:** Rust 2024, SQLx (runtime API), PostgreSQL full-text search (`to_tsquery`, `tsvector`, GIN).

**Spec:** [`docs/superpowers/specs/2026-11-04-terminology-multi-word-search-design.md`](../specs/2026-11-04-terminology-multi-word-search-design.md)

## Global Constraints

- **`to_tsquery` parses an expression, not free text.** Two lexemes must be joined by `&`; whitespace is not an operator. Every expression this change builds states the `&` explicitly.
- **Prefix matching is preserved.** `:*` goes on **every** term, not on the fragment as a whole. The single-word path must keep matching the way it does today (`ag` finds `AGE`).
- **`websearch_to_tsquery` is not used.** On PostgreSQL 17.10 it drops the `*`, which would silently remove the prefix behaviour.
- **The split rule lives in exactly one place.** `domain::search_terms::split` is the only thing that decides where a term ends. Both Postgres repositories and both in-memory repositories use it.
- **A term is a maximal run of alphanumeric characters.** Every other character separates terms — including `_` and `-`, which mirrors Postgres' own `to_tsvector` tokeniser.
- **The wire shape does not change.** `fragment` stays a `String` on the `apis` port, on the server's query DTO, and on the desktop's TypeScript types. The desktop already passes the fragment through unchanged and is not touched.
- **`& | ! ( ) : ' "` are rejected** with `DomainError::InvalidFragment`, which the server renders as `400 validation_failed`. Every other character is a term separator, not an error.
- Verification gate before each commit:
  ```bash
  cargo fmt --all -- --check
  cargo clippy -p terminology --all-targets --all-features -- -D warnings
  cargo test -p terminology
  ```

## Review Focus

Inputs and failure modes the spec implies that a task's own happy-path test does not exercise. Each line has its test in the task that owns the code.

1. **A fragment with two words** → both terms must match; the rows carrying only one of them are filtered out. → Task 3
2. **A single-word fragment** → the rows must be exactly the ones the old prefix-match path returned. No narrowing, no widening. → Task 3
3. **A fragment whose words are separated by a hyphen** (`non-small`) → two terms, not one phrase; the row whose text reads `non small` matches. → Task 1
4. **A fragment with an underscore** (`BMI_CATEGORY`) → two terms, matching the `tsv` lexemes `bmi` and `categori`, not the single lexeme `bmi_categori`. → Task 1
5. **A fragment carrying an apostrophe** (`don't`) → `400 validation_failed`, not a silently wrong result set. → Task 2
6. **A fragment that is empty, whitespace-only, or punctuation-only** → the plain list path, every row returned, no error. → Task 1
7. **A fragment reaching the repository directly**, bypassing the usecase's validation → the adapter still builds a valid expression. The split rule is the adapter's own guarantee, not a courtesy of the usecase. → Task 3

### Task 1: One rule for what a term is

The splitter and the expression builder. Everything else in the plan uses them.

**Files:**
- Create: `lib/crates/terminology/src/domain/search_terms.rs`
- Modify: `lib/crates/terminology/src/domain.rs`
- Create: `lib/crates/terminology/src/adapter/persistence/postgres/tsquery.rs`
- Modify: `lib/crates/terminology/src/adapter/persistence/postgres.rs`

**Interfaces:**
- Consumes: nothing from earlier tasks — this is the first task.
- Produces:
  - `terminology::domain::search_terms::split(fragment: &str) -> Vec<String>` — `pub(crate)`, crate-internal.
  - `terminology::adapter::persistence::postgres::tsquery::build(fragment: &str) -> Option<String>` — `pub(crate)`; `None` when the fragment carries no term.

- [ ] **Step 1: Write the failing splitter tests**

In `lib/crates/terminology/src/domain/search_terms.rs`, add:

```rust
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
```

- [ ] **Step 2: Write the failing builder tests**

In `lib/crates/terminology/src/adapter/persistence/postgres/tsquery.rs`, add:

```rust
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
```

- [ ] **Step 3: Run the tests to verify they fail**

```bash
cargo test -p terminology search_terms
cargo test -p terminology tsquery
```

Both fail — the modules do not exist yet.

- [ ] **Step 4: Implement the splitter**

Create `lib/crates/terminology/src/domain/search_terms.rs`:

```rust
//! Splitting a search fragment into the terms a search matches on.

/// Split a free-text search fragment into the individual terms to match.
///
/// A term is a maximal run of alphanumeric characters; every other
/// character — whitespace, `_`, `-`, `.`, `/`, `,`, … — separates
/// terms. The rule mirrors Postgres' `to_tsvector` tokeniser, which
/// breaks on the same boundary, so a term always lines up with one
/// lexeme of the `tsv` column.
pub(crate) fn split(fragment: &str) -> Vec<String> {
    fragment
        .split(|c: char| !c.is_alphanumeric())
        .filter(|term| !term.is_empty())
        .map(str::to_owned)
        .collect()
}
```

Register it in `lib/crates/terminology/src/domain.rs`:

```rust
mod repository;
pub(crate) mod search_terms;
mod terminology_kind;
```

- [ ] **Step 5: Implement the builder**

Create `lib/crates/terminology/src/adapter/persistence/postgres/tsquery.rs`:

```rust
//! Building the `tsquery` expression the fragment search runs against.

use crate::domain::search_terms;

/// Build the argument for `to_tsquery('english', …)` from a search fragment.
///
/// Every term becomes a prefix-matched lexeme and the terms are
/// combined with `&`, so `"vital sign"` becomes `vital:* & sign:*` and
/// matches the rows whose text contains both terms.
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
```

Register it in `lib/crates/terminology/src/adapter/persistence/postgres.rs`:

```rust
pub mod code_item_repo;
pub mod code_list_repo;
pub mod terminology_version_repo;
mod tsquery;
```

- [ ] **Step 6: Run the tests to verify they pass**

```bash
cargo test -p terminology search_terms
cargo test -p terminology tsquery
```

Both pass.

### Task 2: The repositories, the validation and the fakes

**Files:**
- Modify: `lib/crates/terminology/src/adapter/persistence/postgres/code_list_repo.rs`
- Modify: `lib/crates/terminology/src/adapter/persistence/postgres/code_item_repo.rs`
- Modify: `lib/crates/terminology/src/usecase/terminology_usecase.rs`
- Modify: `lib/crates/terminology/src/domain/error.rs`
- Modify: `lib/crates/terminology/src/domain/repository.rs`
- Modify: `lib/crates/terminology/src/usecase/tests.rs`
- Modify: `lib/crates/terminology/src/adapter/facade/internal/tests.rs`

**Interfaces:**
- Consumes: `domain::search_terms::split` and `postgres::tsquery::build` from Task 1.
- Produces:
  - `code_list_repo.rs` / `code_item_repo.rs` bind the built expression instead of `format!("{frag}:*")`.
  - `terminology::domain::DomainError::InvalidFragment` — message `"search fragment contains reserved tsquery characters: & | ! ( ) : ' \""`.

- [ ] **Step 1: Bind the built expression in both repositories**

In `code_list_repo.rs`, replace the fragment branch with:

```rust
if let Some(expr) = tsquery::build(q.fragment.as_deref().unwrap_or_default()) {
    qb.push(" AND tsv @@ to_tsquery('english', ");
    qb.push_bind(expr.clone());
    qb.push(") ORDER BY ts_rank(tsv, to_tsquery('english', ");
    qb.push_bind(expr);
    qb.push(")) DESC, id ASC LIMIT ");
} else {
    qb.push(" ORDER BY id ASC LIMIT ");
}
```

In `code_item_repo.rs`, the same, behind the existing `has_where` bookkeeping:

```rust
if let Some(expr) = frag_filter.and_then(tsquery::build) {
    qb.push(if has_where { " AND " } else { " WHERE " });
    qb.push("tsv @@ to_tsquery('english', ");
    qb.push_bind(expr.clone());
    qb.push(") ORDER BY ts_rank(tsv, to_tsquery('english', ");
    qb.push_bind(expr);
    qb.push(")) DESC, id ASC LIMIT ");
} else {
    qb.push(" ORDER BY id ASC LIMIT ");
}
```

Both files need `use super::tsquery;`.

- [ ] **Step 2: Reject the quote characters in the usecase**

In `terminology_usecase.rs`:

```rust
fn validate_tsquery_fragment(s: &str) -> Result<&str, UsecaseError> {
    if s.chars()
        .any(|c| matches!(c, '&' | '|' | '!' | '(' | ')' | ':' | '\'' | '"'))
    {
        return Err(UsecaseError::Validation(DomainError::InvalidFragment));
    }
    Ok(s.trim())
}
```

Update `DomainError::InvalidFragment`'s message to
`"search fragment contains reserved tsquery characters: & | ! ( ) : ' \""`.

- [ ] **Step 3: Teach both in-memory repositories the same rule**

In `src/usecase/tests.rs` and `src/adapter/facade/internal/tests.rs`, the fake repositories' fragment filter becomes:

```rust
if let Some(frag) = q.fragment.as_deref().filter(|s| !s.trim().is_empty()) {
    let terms = search_terms::split(frag)
        .iter()
        .map(|t| t.to_lowercase())
        .collect::<Vec<_>>();
    all.retain(|cl| {
        let haystack = [
            cl.name.as_str(),
            cl.submission_value.as_str(),
            cl.synonym.as_str(),
            cl.definition.as_str(),
            cl.nci_preferred_term.as_str(),
        ]
        .join(" ")
        .to_lowercase();
        terms.iter().all(|term| haystack.contains(term.as_str()))
    });
}
```

The code-item fake adds `item.code.as_str()` to the haystack, exactly as before.

- [ ] **Step 4: Extend the existing validation test**

In `src/usecase/tests.rs`, add `a'b` and `a"b` to the rejected fragments:

```rust
for bad in ["a&b", "a|b", "a!b", "a(b", "a)b", "a:b", "a'b", "a\"b"] {
```

- [ ] **Step 5: Add the multi-word tests**

In `src/usecase/tests.rs`, add a test that creates `Vital Signs` and `Vital Status` under one version and asserts that the fragment `vital sign` returns only the first. In `src/adapter/facade/internal/tests.rs`, add the same through `TerminologyServiceImpl`.

- [ ] **Step 6: Run the crate's tests**

```bash
cargo test -p terminology
```

All pass.

### Task 3: The live database round trip, and the documentation

**Files:**
- Modify: `lib/crates/terminology/tests/integration_persistence.rs`
- Modify: `lib/crates/apis/src/terminology.rs`
- Modify: `apps/server/aegis-server/src/transport/http/dto.rs`

**Interfaces:**
- Consumes: everything from Tasks 1 and 2.
- Produces: no new interfaces — documentation, and a live test that pins the behaviour against a real PostgreSQL.

- [ ] **Step 1: Write the live test**

In `tests/integration_persistence.rs`, add `multi_word_fragment_matches_every_term_in_postgres`: it creates two code lists (`Vital Signs`, `Vital Status`) and two code items under them, then asserts that

- the fragment `vital sign` returns only the `Vital Signs` row — for both code lists and code items;
- the fragment `vital` returns both rows — the single-word path is unchanged.

The test is `#[ignore]`-gated like every other live test.

- [ ] **Step 2: Run the live test**

```bash
cargo test -p terminology -- --ignored --test-threads=1
```

> The live tests **drop** `code_items`, `code_lists`, `terminology_versions` and
> `_sqlx_migrations` before they run. Point `AEGIS_TERMINOLOGY_DATABASE_URL` at a
> scratch database, never at a database holding data.

- [ ] **Step 3: Update the documentation**

- `lib/crates/apis/src/terminology.rs` — the `CodeListListQuery` doc and the `list_code_lists` doc state that the fragment is split into terms and every term has to match.
- `apps/server/aegis-server/src/transport/http/dto.rs` — the `CodeListListQuery` doc says the same.
- `lib/crates/terminology/src/domain/repository.rs` — the `search_or_list` doc describes the built expression instead of `$2 || ':*'`.

- [ ] **Step 4: Verify the whole workspace**

```bash
cargo fmt --all -- --check
cargo clippy -p terminology --all-targets --all-features -- -D warnings
cargo test -p terminology
cargo test -p apis
cargo check --workspace
```
