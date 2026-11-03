# Terminology drawer code-item search — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a search box to the terminology drawer that searches the resolved code list's code items.

**Architecture:** The drawer copies the code-list detail page's pattern exactly: a `TermFilterBar`, the fragment debounced with the shared `useDebouncedValue` hook, handed to the existing `useListCodeItems(codelistId, { fragment })` hook, which already scopes the server-side full-text search to that code list and puts the fragment in its query key. No server change, no client-side filtering.

**Tech Stack:** React + TypeScript, TanStack Query, MUI (`@aegis/ui`), Vitest + Testing Library.

**Spec:** `docs/superpowers/specs/2026-11-02-terminology-drawer-code-item-search-design.md` — read it before starting.

## Global Constraints

- The behaviour is the code-list detail page's, unchanged (`features/terminology/pages/CodeListDetailPage.tsx`): server-side fragment search scoped by `codelistId`, `TermFilterBar` with the code-item placeholder, `useDebouncedValue` with `delayMs: 300` / `maxWaitMs: 1000`, and the empty message switching on the trimmed query.
- No server, migration, or terminology-crate change. No client-side filtering, and no client-side matching of the item's NCI code.
- The search text resets when `code` changes — the drawer is one mounted component reused across codes, unlike the detail page.
- All user-visible strings come through `useI18n()` → `t("path.to.key")`. All MUI primitives and icons come from `@aegis/ui/mui` and `@aegis/ui/icons`.
- Cross-feature imports go through a feature's `index.ts` barrel — the drawer already imports `TermFilterBar` and `CodeItemTable` from within the terminology feature.
- Verification gate:
  ```bash
  cd apps/desktop/aegis-desktop && pnpm exec vitest run src/test/features/terminology
  pnpm --filter aegis-desktop typecheck
  pnpm --filter @aegis/ui test
  ```
  (The desktop frontend suite has 13 pre-existing failures in 6 files — CRF, project-list, metadata, workspace routing — verified pre-existing on `main`. The Rust side is untouched by this change.)

## Review Focus

1. **A search left over from a previous code.** The drawer is reused across codes; a stale term would filter the new code list into a false "no matching code items". → pinned by "clears the search when the code changes".
2. **The search escaping its code list.** The user must not see code items from another code list or version. → pinned by "scopes the search to the resolved code list".
3. **Every keystroke hitting the server.** The search must be debounced, so a burst of typing is one request. → pinned by "waits for the debounce before querying".
4. **An empty search.** Clearing the box must restore the full list, not an empty one. → pinned by "clearing the search restores every code item".
5. **A search that matches nothing.** The user must be told the list has no matches, rather than seeing an empty table with no explanation. → pinned by "shows the no-matches message for an active search".

---

### Task 1: Search the code items in the terminology drawer

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/terminology/components/TerminologyCodeItemsDrawer.tsx`
- Test: `apps/desktop/aegis-desktop/src/test/features/terminology/terminology-code-items-drawer.test.tsx`

**Interfaces:**
- Consumes: `TermFilterBar` from `../components/TermFilterBar` (props `{ query: string; onQueryChange: (value: string) => void; placeholder?: string }`); `useDebouncedValue` from `../../../shared/hooks/useDebouncedValue` (props `{ delayMs: number; maxWaitMs: number }`); `useListCodeItems(codelistId: number | null, options?: { fragment?: string })` from `../data`.
- Produces: nothing other than the drawer's behaviour — no new exports, no new props, no new query keys.

- [ ] **Step 1: Write the failing tests**

In `terminology-code-items-drawer.test.tsx`, add to the `describe("TerminologyCodeItemsDrawer")` block. The existing `setupMocks` helper already mocks `list_code_items`, and `renderDrawer` accepts `{ open?, code?, projectCode? }`.

```tsx
  it("searches the resolved code list's items", async () => {
    setupMocks();
    renderDrawer();
    await screen.findByText("PASS");
    await userEvent.type(
      screen.getByPlaceholderText(/search by code, submission value/i),
      "pas",
    );
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith(
        "list_code_items",
        expect.objectContaining({ codelistId: 11, fragment: "pas" }),
      );
    });
  });

  it("waits for the debounce before querying", async () => {
    setupMocks();
    renderDrawer();
    await screen.findByText("PASS");
    const before = mockInvoke.mock.calls.filter(
      (c) => c[0] === "list_code_items",
    ).length;
    await userEvent.type(
      screen.getByPlaceholderText(/search by code, submission value/i),
      "p",
    );
    // The raw keystroke must not have reached the server yet.
    const during = mockInvoke.mock.calls.filter(
      (c) => c[0] === "list_code_items",
    ).length;
    expect(during).toBe(before);
    await waitFor(
      () => {
        expect(mockInvoke).toHaveBeenCalledWith(
          "list_code_items",
          expect.objectContaining({ fragment: "p" }),
        );
      },
      { timeout: 2000 },
    );
  });

  it("scopes the search to the resolved code list", async () => {
    setupMocks();
    renderDrawer();
    await screen.findByText("PASS");
    await userEvent.type(
      screen.getByPlaceholderText(/search by code, submission value/i),
      "pas",
    );
    await waitFor(() => {
      const calls = mockInvoke.mock.calls.filter(
        (c) => c[0] === "list_code_items",
      );
      expect(calls.length).toBeGreaterThan(0);
      expect(
        calls.every(
          (c) => (c[1] as { codelistId: number }).codelistId === 11,
        ),
      ).toBe(true);
    });
  });

  it("clearing the search restores every code item", async () => {
    setupMocks();
    renderDrawer();
    await screen.findByText("PASS");
    const input = screen.getByPlaceholderText(
      /search by code, submission value/i,
    );
    await userEvent.type(input, "zzz");
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith(
        "list_code_items",
        expect.objectContaining({ fragment: "zzz" }),
      );
    });
    await userEvent.clear(input);
    await waitFor(
      () => {
        const calls = mockInvoke.mock.calls.filter(
          (c) => c[0] === "list_code_items",
        );
        expect(
          calls.some(
            (c) => (c[1] as { fragment?: string }).fragment === undefined,
          ),
        ).toBe(true);
      },
      { timeout: 2000 },
    );
  });

  it("shows the no-matches message for an active search", async () => {
    setupMocks({ codeItems: { items: [] } });
    renderDrawer();
    const input = await screen.findByPlaceholderText(
      /search by code, submission value/i,
    );
    await userEvent.type(input, "zzz");
    expect(
      await screen.findByText(/no matching code items/i),
    ).toBeInTheDocument();
  });

  it("clears the search when the code changes", async () => {
    setupMocks();
    const { rerender } = renderDrawer();
    await screen.findByText("PASS");
    const input = screen.getByPlaceholderText(
      /search by code, submission value/i,
    ) as HTMLInputElement;
    await userEvent.type(input, "pas");
    expect(input.value).toBe("pas");
    rerender(
      <AegisThemeProvider>
        <TestQueryProvider>
          <AegisI18nProvider>
            <TerminologyCodeItemsDrawer
              open={true}
              code="AESEV"
              projectCode={null}
              onClose={() => {}}
            />
          </AegisI18nProvider>
        </TestQueryProvider>
      </AegisThemeProvider>,
    );
    await waitFor(() => {
      expect(
        (
          screen.getByPlaceholderText(
            /search by code, submission value/i,
          ) as HTMLInputElement
        ).value,
      ).toBe("");
    });
  });
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cd apps/desktop/aegis-desktop && pnpm exec vitest run src/test/features/terminology/terminology-code-items-drawer.test.tsx`
Expected: FAIL — no search field exists ("Unable to find an element with the placeholder"), and the `list_code_items` calls carry no `fragment`.

- [ ] **Step 3: Implement the search**

In `TerminologyCodeItemsDrawer.tsx`:

1. Add the imports:
   ```tsx
   import { useEffect, useState } from "react";
   import { useDebouncedValue } from "../../../shared/hooks/useDebouncedValue";
   import { TermFilterBar } from "./TermFilterBar";
   ```

2. Add the state and the debounced fragment next to the other hooks:
   ```tsx
   const [search, setSearch] = useState("");
   const debouncedFragment = useDebouncedValue(search, {
     delayMs: 300,
     maxWaitMs: 1000,
   });
   const trimmedQuery = debouncedFragment.trim();
   ```

3. Reset the search when the code changes — the drawer is one mounted
   component reused across codes, so a term left over from the previous
   code would filter the new list:
   ```tsx
   useEffect(() => {
     setSearch("");
   }, [code]);
   ```

4. Hand the fragment to the existing hook:
   ```tsx
   const itemsQuery = useListCodeItems(codelist?.id ?? null, {
     fragment: debouncedFragment,
   });
   ```

5. Render the filter bar directly above the code-item table (inside the
   `codelist` branch, before `<CodeItemTable>`):
   ```tsx
   <TermFilterBar
     query={search}
     onQueryChange={setSearch}
     placeholder={t("terminology.codeitem.search.placeholder")}
   />
   ```

6. Switch the table's empty message on the trimmed query:
   ```tsx
   emptyMessage={
     trimmedQuery
       ? t("terminology.codeitem.noMatches")
       : t("terminology.codeitem.empty")
   }
   ```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cd apps/desktop/aegis-desktop && pnpm exec vitest run src/test/features/terminology/terminology-code-items-drawer.test.tsx`
Expected: PASS — the new tests plus the existing ones in the file.

- [ ] **Step 5: Run the verification gate**

Run:
```bash
cd apps/desktop/aegis-desktop && pnpm exec vitest run src/test/features/terminology
pnpm --filter aegis-desktop typecheck
pnpm --filter @aegis/ui test
```
Expected: the terminology suite green, typecheck clean, the `@aegis/ui` suite green.

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/terminology/components/TerminologyCodeItemsDrawer.tsx \
        apps/desktop/aegis-desktop/src/test/features/terminology/terminology-code-items-drawer.test.tsx \
        docs/superpowers/specs/2026-11-02-terminology-drawer-code-item-search-design.md \
        docs/superpowers/plans/2026-11-02-terminology-drawer-code-item-search.md
git commit -m "feat(aegis-desktop): search the code items in the terminology drawer

Adds a search box to the drawer, scoped to the resolved code list, using
the same server-side fragment search the code-list detail page uses. The
search text resets when the code changes, since the drawer is one
mounted component reused across codes.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Implementation notes (deviations from the plan above)

Recorded after the branch review, so a reader comparing this plan to the
code is not left guessing.

1. **The reset moved from an effect into render, and the settled value
   is scoped by code.** The plan prescribed
   `useEffect(() => setSearch(""), [code])`. The review showed that is
   one step too late: the debounced value lags the reset by a debounce
   window, so the previous code's term reached the new code list's
   request in between — the new list flashed "no matching code items".
   The search state now carries the code it was typed for, the reset
   happens during render, and the fragment passed to the query is the
   settled value *scoped by that code*.

2. **A failed search no longer replaces the drawer body.** The
   code-item query's error is rendered inside the table (the
   `CodeItemTable` `error` prop), the way the code-list detail page
   does it, so the search box survives a rejected fragment and the
   user can correct it.

3. **Two tests were tightened.** "Scopes the search to the resolved code
   list" and "clearing the search restores every code item" asserted
   conditions that were already true before the action they claimed to
   check — the review showed both survive a mutation that breaks the
   behaviour. Both now look at the request that carries the fragment
   / the request issued after the clear.

4. **The debounce test is deterministic.** It used real timers and
   asserted that no request had gone out yet — a wall-clock margin
   against a 300 ms debounce. It now runs under fake timers and
   advances the clock explicitly. The test file restores real timers
   after every test, so a failure under fake timers cannot leak into
   the tests that follow.
