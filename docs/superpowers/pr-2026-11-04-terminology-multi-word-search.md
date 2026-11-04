fix(terminology): match every term of a search fragment

Searching the terminology for `vital sign` answers

    repository error: syntax error in tsquery: "vital sign:*"

a raw Postgres message wrapped in a repository error — a 500 — for a
fragment nobody would call invalid.

**The cause.** `to_tsquery` does not take free text; it takes a tsquery
*expression*, and in that grammar two lexemes must be joined by an
operator. Whitespace is not an operator:

    to_tsquery('english', 'vital:*')       -- 'vital':*              OK
    to_tsquery('english', 'vital sign')    -- ERROR: syntax error in tsquery: "vital sign"
    to_tsquery('english', 'vital & sign')  -- 'vital' & 'sign'       OK

The adapter built that expression by interpolation —
`qb.push_bind(format!("{frag}:*"))` — so a multi-word fragment reached
the database as one malformed expression. Two further defects sat
behind the same line: the `:*` was appended to the *whole* string, so
only the last word would have been prefix-matched, and the usecase's
`validate_tsquery_fragment` rejected `& | ! ( ) :` but not whitespace,
so the fragment passed validation and died in the adapter — a
repository error, where the user had typed something ordinary.

**The fix.** The fragment is split into terms in one shared place, and
the adapter turns those terms into an expression `to_tsquery` accepts:

    vital sign  ->  vital:* & sign:*  ->  'vital':* & 'sign':*

The split rule lives in `domain::search_terms::split` — a term is a
maximal run of alphanumeric characters, and every other character
separates terms. That mirrors Postgres' own `to_tsvector` tokeniser,
which breaks on the same boundary, so a term always lines up with one
lexeme of the `tsv` column. Both Postgres repositories and both
in-memory repositories use it, so "what does this fragment mean" is
answered once: **every term has to match**. `vital sign` finds the
rows whose text mentions both; `vital` alone still finds the 82 rows
it found before.

Prefix matching is preserved — `:*` now goes on *every* term rather
than on the fragment as a whole. `websearch_to_tsquery` was
considered and rejected: it never errors on raw user input, which is
attractive, but on PostgreSQL 17.10 it drops the `*`
(`websearch_to_tsquery('english', 'zeb*')` yields `'zeb'` and does not
match `zebra`, where `to_tsquery('english', 'zeb:*')` does), so it
would have silently removed the prefix behaviour.

**What changes for a caller.** The wire shape does not change —
`fragment` is still a string on the `apis` port, on the server's query
DTO and on the desktop's TypeScript types, and the desktop already
passed it through untouched. The characters `& | ! ( ) :` were already
rejected with a 400; `'` and `"` join them, because an apostrophe
opens a quoted lexeme that never closes and today the search quietly
looks for the wrong word (`don't` searched for `t`). Hyphens, dots,
commas and whitespace are not errors — they are term separators.

One gap this PR does **not** close: the `tsv` column does not index
the `code` columns, so searching a code item by its value code
(`VSTESTCD`) matches in the in-memory fake but not in Postgres.
Closing that needs a migration on the generated column and is
deliberately out of scope.

Spec: docs/superpowers/specs/2026-11-04-terminology-multi-word-search-design.md
Plan: docs/superpowers/plans/2026-11-04-terminology-multi-word-search.md

Verification:

    cargo fmt --all -- --check
    cargo clippy -p terminology --all-targets --all-features -- -D warnings
    cargo test -p terminology
    cargo test -p terminology -- --ignored --test-threads=1
    cargo test -p apis
    cargo check --workspace

(`terminology` 79 passed · `apis` 11 passed · `public_api` 3 passed ·
live PostgreSQL 9 passed · workspace check clean.)

The live run was made against a scratch database created for it, not
against the development database: the live tests **drop**
`code_items`, `code_lists`, `terminology_versions` and
`_sqlx_migrations` before they run.

Two pre-existing conditions in the repo, unchanged by this PR and
verified as such at the base commit:

- `cargo test -p aegis-server` fails 28 tests on `main` — the same 28
  (257 passed / 28 failed) before and after this change, all in the
  auth and DTO round-trip suites. This PR touches one server file
  (`transport/http/dto.rs`) and only its doc comment.
- `cargo fmt --all -- --check` reports 2 violations, both in
  `apps/desktop/aegis-desktop/src-tauri/src/{commands,http}/structures.rs`
  — files this PR does not touch. `cargo clippy -p terminology`
  reports 3 `map_or` → `is_none_or` lints in the crates' own test
  fakes, on the same lines before and after this change.

Co-Authored-By: Claude Code <noreply@anthropic.com>
