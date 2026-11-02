import {
  Alert,
  Box,
  Button,
  Chip,
  CircularProgress,
  Drawer,
  Tooltip,
  Typography,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";
import { useState } from "react";

import { errorMessage } from "../../../shared/api/error";
import { InfiniteScrollSentinel } from "../../../shared/components/InfiniteScrollSentinel";
import { useDebouncedValue } from "../../../shared/hooks/useDebouncedValue";
import {
  useFindCodeListBySubmissionValue,
  useListCodeItems,
  useSdtmTerminologyVersionId,
} from "../data";
import { CodeItemTable } from "./CodeItemTable";
import { TermFilterBar } from "./TermFilterBar";

export interface TerminologyCodeItemsDrawerProps {
  open: boolean;
  code: string | null;
  /** Project whose configured SDTM terminology release the code is
   *  resolved against. When absent, the most recently imported SDTM
   *  release is used. */
  projectCode?: string | null;
  onClose: () => void;
}

/**
 * Lists the code items of the code list a terminology code refers to.
 *
 * Resolution: the SDTM terminology version → the code list whose `name`
 * matches the code → that code list's items. The list is read only: no
 * create, edit, or delete affordances, and no "same code in other
 * codelists" dialog.
 *
 * Owns its own queries — the caller controls `open`, `code`,
 * `projectCode`, and `onClose` only.
 */
export function TerminologyCodeItemsDrawer({
  open,
  code,
  projectCode,
  onClose,
}: TerminologyCodeItemsDrawerProps) {
  const { t } = useI18n();
  const activeCode = open ? code : null;
  const versions = useSdtmTerminologyVersionId(
    open ? (projectCode ?? null) : null,
  );
  const codelistsQuery = useFindCodeListBySubmissionValue(
    versions.versionId,
    activeCode,
  );
  const codelist = codelistsQuery.codelist;

  // The search text, together with the code it was typed for.
  //
  // The drawer is one mounted component reused across codes, so a
  // fragment typed for one code must never reach another code's list.
  // The reset happens during render rather than in an effect, and the
  // *settled* value is scoped by its code: the debounced value lags the
  // reset by a debounce window, so an effect-based reset would let the
  // old term reach the new code list in the meantime.
  const [search, setSearch] = useState<{ code: string | null; text: string }>(
    { code, text: "" },
  );
  if (search.code !== code) {
    setSearch({ code, text: "" });
  }
  const searchText = search.code === code ? search.text : "";
  const debouncedSearch = useDebouncedValue(search, {
    delayMs: 300,
    maxWaitMs: 1000,
  });
  const fragment = debouncedSearch.code === code ? debouncedSearch.text : "";
  const trimmedQuery = fragment.trim();

  const itemsQuery = useListCodeItems(codelist?.id ?? null, {
    fragment,
  });
  const rows = itemsQuery.data?.pages.flatMap((p) => p.items) ?? [];
  const hasMore = itemsQuery.hasNextPage ?? false;

  // "No version imported", "still loading" and "the request failed" are
  // three different states — they must not collapse into one message.
  const versionsFailed = versions.versionsError != null;
  const projectFailed = versions.projectError != null;
  const noVersion = versions.versionId == null && !versionsFailed && !projectFailed;
  const resolving = versions.isLoading || codelistsQuery.isLoading;
  // A failed *search* is not a failed code list: it is rendered by the
  // table (which keeps the search box and offers its own retry), so the
  // user can correct the query.
  const lookupError =
    versions.versionsError ?? versions.projectError ?? codelistsQuery.error;

  function handleRetry() {
    // Refetch only the query that failed: the code-item query is
    // disabled until a code list is resolved, and firing it anyway
    // would ask the server for an unfiltered page of code items.
    if (versionsFailed || projectFailed) {
      versions.refetch();
      return;
    }
    void codelistsQuery.refetch();
  }

  return (
    <Drawer
      anchor="right"
      open={open && !!code}
      onClose={onClose}
      slotProps={{ paper: { sx: { width: 720 } } }}
    >
      <Box sx={{ p: 3, display: "flex", flexDirection: "column", gap: 2 }}>
        <Typography variant="h6">
          {t("terminology.codeitem.drawer.title", { code: code ?? "" })}
        </Typography>

        {lookupError ? (
          <Box sx={{ display: "flex", flexDirection: "column", gap: 1 }}>
            <Alert severity="error">
              {versionsFailed
                ? t("terminology.codeitem.drawer.versionsLoadFailed", {
                    message: errorMessage(lookupError),
                  })
                : projectFailed
                  ? t("terminology.codeitem.drawer.projectLoadFailed", {
                      message: errorMessage(lookupError),
                    })
                  : t("terminology.codeitem.drawer.loadFailed", {
                      code: code ?? "",
                      message: errorMessage(lookupError),
                    })}
            </Alert>
            <Box>
              <Button onClick={handleRetry}>{t("common.retry")}</Button>
            </Box>
          </Box>
        ) : resolving ? (
          <Box sx={{ display: "flex", justifyContent: "center", py: 4 }}>
            <CircularProgress />
          </Box>
        ) : noVersion ? (
          <Alert severity="info">
            {t("terminology.codeitem.drawer.noVersion")}
          </Alert>
        ) : !codelist ? (
          <Typography color="text.secondary">
            {t("terminology.codeitem.drawer.notFound", { code: code ?? "" })}
          </Typography>
        ) : (
          <>
            <Box sx={{ display: "flex", alignItems: "center", gap: 1 }}>
              <Typography variant="body2" color="text.secondary">
                {`${codelist.name} · ${codelist.code}`}
              </Typography>
              {codelist.extensible && (
                <Tooltip title={t("terminology.extensible")}>
                  <Chip label="EXT" size="small" />
                </Tooltip>
              )}
            </Box>
            <TermFilterBar
              query={searchText}
              onQueryChange={(value) => setSearch({ code, text: value })}
              placeholder={t("terminology.codeitem.search.placeholder")}
            />
            <CodeItemTable
              rows={rows}
              loading={itemsQuery.isLoading}
              mutationLoading={false}
              error={itemsQuery.error}
              canMutate={false}
              onRetry={() => void itemsQuery.refetch()}
              onCreate={() => {}}
              onEdit={() => {}}
              onDelete={() => {}}
              emptyMessage={
                trimmedQuery
                  ? t("terminology.codeitem.noMatches")
                  : t("terminology.codeitem.empty")
              }
              bottomSlot={(scrollEl) => (
                <InfiniteScrollSentinel
                  root={scrollEl}
                  onIntersect={() => void itemsQuery.fetchNextPage()}
                  hasMore={hasMore}
                  loading={itemsQuery.isFetchingNextPage}
                />
              )}
            />
          </>
        )}
      </Box>
    </Drawer>
  );
}
