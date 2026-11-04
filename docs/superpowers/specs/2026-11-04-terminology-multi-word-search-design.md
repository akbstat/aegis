# Terminology Multi-Word Search Design

> **Scope.** Fix the fragment search in `lib/crates/terminology` so a fragment carrying several words searches for all of them instead of failing. Touches the `terminology` crate, the `apis` port documentation, and the server's wire-DTO documentation. **The desktop crate (`apps/desktop/aegis-desktop`) is not touched** — it already passes the fragment through unchanged.

## Goal

A search fragment is **free text**. A user typing `vital sign` expects the code items whose text mentions both *vital* and *sign* — not an error.

Today that fragment answers

```text
repository error: syntax error in tsquery: "vital sign:*"
```

an infrastructure-class error carrying a raw Postgres message.

## The problem

`to_tsquery` does not take free text. It takes a **tsquery expression**, and in that grammar two lexemes must be joined by an operator — whitespace is not an operator:

```sql
to_tsquery('english', 'vital:*')       -- 'vital':*              OK
to_tsquery('english', 'vital sign')    -- ERROR: syntax error in tsquery: "vital sign"
to_tsquery('english', 'vital & sign')  -- 'vital' & 'sign'       OK
```

The adapter built that expression by interpolation:

```rust
qb.push_bind(format!("{frag}:*"));   // "vital sign" → "vital sign:*"
```

Three distinct defects sit behind the one error:

1. **The expression.** Whitespace-separated lexemes with no operator between them are not a valid tsquery expression.
2. **The prefix marker.** `:*` is appended to the *whole* fragment string, so only the **last** word would have been prefix-matched even once (1) is fixed. `"vital sign"` would have become `'vital' & 'sign':*` — an exact match on `vital` and a prefix match on `sign`.
3. **The error class.** The usecase's `validate_tsquery_fragment` rejects `& | ! ( ) :` but not whitespace, so the fragment passed validation and died in the adapter as `DomainError::Repository("syntax error in tsquery: …")`. That is a repository error — the server renders it as a 500 — where the user had typed something perfectly ordinary.

A fourth, quieter defect lives on the same path: an apostrophe opens a quoted lexeme that never closes, so `don't` parses as the lexeme `t` and silently searches for the wrong thing.

## Terminology

- **fragment** — the free-text string the caller supplies as the search term.
- **term** — one maximal run of alphanumeric characters within the fragment. `vital sign` has two terms; `non-small` has two; `C123456` has one.
- **lexeme** — one normalised word in a `tsvector`. The `tsv` column is the concatenation of the searchable text columns, tokenised by Postgres' `to_tsvector`.
- **tsquery expression** — the expression language `to_tsquery` parses: lexemes joined by `&` (and), `|` (or), `!` (not) and `<->` (phrase), with an optional `:*` prefix marker on a lexeme.
- **prefix match** — `vital:*` matches every lexeme that *starts with* `vital`.

## Decisions

| Question | Decision | Why |
| --- | --- | --- |
| What does a multi-word fragment mean? | **AND** — every term must match. | The terms are joined with `&`, which is what the tsquery expression is for. It also narrows rather than widens: a user who types two words usually means one specific concept. OR would return everything matching either word. |
| Where is the fragment split? | One shared splitter in the domain (`domain::search_terms`), used by the Postgres adapter **and** by both in-memory repositories. | The rule "what does this fragment mean" must not be restated per adapter. The two in-memory fakes are the crates' own test doubles; when they disagreed with Postgres, the unit tests stopped describing the product. |
| What separates two terms? | Every character that is not alphanumeric. | It mirrors Postgres' own `to_tsvector` tokeniser, which breaks on the same boundary — so a term always lines up with one lexeme of `tsv`. `_` and `-` are separators, not part of a term: `BMI_CATEGORY` becomes two terms, exactly as the `tsv` column holds `bmi` and `categori`. |
| Is prefix matching kept? | Yes — `:*` is applied to **every** term. | It is the behaviour the single-word path has today (`ag` finds `AGE`), and it is what makes the search useful on a terminology database where the user types the beginning of a term. |
| Why not `websearch_to_tsquery`? | Not used. | It never errors on raw user input, which is attractive — but on PostgreSQL 17.10 it **drops the `*`**: `websearch_to_tsquery('english', 'zeb*')` yields `'zeb'` and does not match `zebra`, where `to_tsquery('english', 'zeb:*')` does. Adopting it would silently drop the prefix behaviour. |
| Which characters are an error? | `& \| ! ( ) : ' "` are rejected with `DomainError::InvalidFragment` (a 400). Everything else is a term separator. | The operators carry query intent that cannot be guessed from a search box. The quotes are the ones that today fail *silently* — a 400 says "this character is not supported", where the current behaviour searches for the wrong word. |
| Does the fragment get trimmed? | Yes. | The usecase's doc comment always said it passed the trimmed fragment; the code did not. |

## Design

### The term splitter

`domain::search_terms::split` is the single place a fragment becomes terms:

```rust
fragment
    .split(|c: char| !c.is_alphanumeric())
    .filter(|term| !term.is_empty())
    .map(str::to_owned)
    .collect()
```

It is `pub(crate)` — the splitting rule is crate-internal machinery, not part of the published surface.

### The tsquery builder

`adapter::persistence::postgres::tsquery::build` turns the terms into the expression `to_tsquery` accepts:

```text
"vital sign"  →  "vital:* & sign:*"  →  'vital':* & 'sign':*
```

It returns `None` when the fragment carries no usable term, and the caller then takes the plain list path — the same path an empty fragment takes today.

### The repositories

Both `code_list_repo.rs` and `code_item_repo.rs` bind the built expression twice, once for the filter and once for the rank:

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

The GIN index on `tsv` still backs the filter, and the ranking is unchanged.

### The usecase validation

`validate_tsquery_fragment` now rejects the quote characters as well, and returns the trimmed fragment. Everything else — whitespace, hyphens, dots, commas — is not an error; it is a term separator.

### The in-memory repositories

Both fakes filter on the same terms: a row is kept when **every** term occurs in the searchable text. The fakes match on substrings rather than on stemmed lexemes, which is a difference from Postgres that predates this change; what this change removes is the difference in *how many terms* have to match.

## Consequences

- A multi-word fragment now searches. `vital sign` finds the rows containing both terms.
- The error class for a bad fragment is now a 400 (`validation_failed`) instead of a 500 with a Postgres message — for the characters that are still rejected.
- `'` and `"` are now rejected. They were silently corrupting the query before, so this trades a wrong answer for an honest error.
- **Unchanged, and still true:** the `tsv` column does not index the `code` columns, so searching for a code item by its *value code* (`VSTESTCD`) matches in the in-memory fake but not in Postgres. Fixing that needs a migration on the generated column and is deliberately out of scope here.
- The `tsv` column is unweighted across the five (code lists) and four (code items) searchable columns, so the rank cannot distinguish a hit in `name` from a hit in `definition`. Unchanged.
