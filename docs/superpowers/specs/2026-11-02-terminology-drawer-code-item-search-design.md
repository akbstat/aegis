# Aegis Desktop — code-item search in the terminology drawer — design

- **Date:** 2026-11-02
- **Branch:** `feat/desktop_sdtm-variable-ct-search`
- **Scope:** `apps/desktop/aegis-desktop` (one component, one feature)
- **Follows:** `docs/superpowers/specs/2026-10-30-desktop-variable-terminology-column-design.md`

---

## Intent

The terminology drawer opened from a variable's **Terminology** chip lists
the permissible values of one code list. A code list can hold hundreds of
values ("Age" has over a hundred), so reading it means scrolling. This
change adds a search box to the drawer that searches **within that code
list**.

## The decision that shapes everything

The behaviour is the code-list detail page's, unchanged:
`features/terminology/pages/CodeListDetailPage.tsx` already searches a code
list's items with a fragment, scoped to that code list. The drawer adopts
the same pattern rather than inventing a second one.

That fixes the two open questions, and it fixes them by *not* changing
the server:

| Question | Answer | Consequence |
|---|---|---|
| Should the search match a code item's NCI code (`C174106`)? | No — the same search the detail page offers | The FTS index covers `submission_value`, `synonym`, `definition` and `nci_preferred_term`, not `code`. An NCI code finds nothing, exactly as on the detail page. |
| What should a multi-word query do (`test code`)? | Nothing special — the same behaviour as the detail page | The server builds a single-token prefix query (`to_tsquery('english', frag \|\| ':*')`), so a fragment with a space is invalid tsquery and the request errors. Pre-existing, app-wide. |

Both limits are inherited deliberately: this change is a desktop feature,
and the detail page is the reference the human partner pointed at. Neither
limit is *introduced* here, and neither is fixed here.

## Goal

A user opens the drawer for a terminology code, types into a search box,
and sees only the code items of that code list matching the text.

## Non-goals

- No server, migration, or terminology-crate change.
- No client-side filtering, and no client-side matching of the NCI code.
- No change to the drawer's resolution, states, or read-only behaviour.
- No change to `CodeItemTable`, `TermFilterBar`, or `useListCodeItems`.

## Design

### The filter bar

The drawer renders a `TermFilterBar` above the code-item table — the same
component, and the same placeholder, the detail page uses:

```tsx
<TermFilterBar
  query={search}
  onQueryChange={setSearch}
  placeholder={t("terminology.codeitem.search.placeholder")}
/>
```

### The query

The fragment is debounced with the shared hook, on the same timings as
the detail page (`delayMs: 300`, `maxWaitMs: 1000`), and handed to the
existing hook:

```tsx
const itemsQuery = useListCodeItems(codelist?.id ?? null, {
  fragment: debouncedFragment,
});
```

`useListCodeItems` already puts the fragment in its query key, so a new
search starts a fresh page series and the previous one is discarded; the
infinite-scroll sentinel keeps working unchanged.

### The empty state

The detail page switches its empty message on the trimmed query; the
drawer does the same:

```tsx
emptyMessage={
  trimmedQuery
    ? t("terminology.codeitem.noMatches")
    : t("terminology.codeitem.empty")
}
```

### The one thing the drawer does that the detail page cannot

The detail page shows one code list for the life of the page. The drawer
is reused: the same mounted component is pointed at a different code
when the user clicks a different chip, and it stays mounted between
opens. A search term left over from the previous code would silently
filter the new list — showing "no matching code items" for a code list
that has plenty.

So the search text resets when the code changes:

```tsx
useEffect(() => {
  setSearch("");
}, [code]);
```

This is the only behavioural difference from the detail page.

## Tests

`src/test/features/terminology/terminology-code-items-drawer.test.tsx`:

- typing into the search box queries the code items with the fragment
  (asserted on the `list_code_items` invoke payload);
- the debounced fragment is what reaches the server, not the raw input
  (assert the request carries the settled value);
- the search is scoped to the resolved code list — the request carries
  that code list's id, not a version-wide one;
- the empty message switches to "no matching code items" once a search
  is active;
- changing the code clears the search box.

## What's left for the builder

- Nothing: the pattern is copied from an existing page, and the copy is
  the deliverable.
