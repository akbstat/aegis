import { useEffect } from "react";
import { load, type Store } from "@tauri-apps/plugin-store";
import { useThemeMode, type ThemeMode } from "@aegis/ui/theme";
import { useI18n, type Locale } from "@aegis/ui/i18n";

/**
 * Lazy singleton over the `settings.bin` store. The store lives on
 * disk at the app-config level, so every Tauri window — main window
 * and every `project:*` workspace window — sees the same file.
 */
let storePromise: Promise<Store> | null = null;
async function getStore(): Promise<Store> {
  if (!storePromise) storePromise = load("settings.bin");
  return storePromise;
}

/**
 * Read theme + locale from the on-disk settings store and apply them
 * to the React providers. Mounted once per window inside the bridge
 * component (see SettingsSyncBridge.tsx) so every window picks up
 * the user's last choice before its first paint. Intentionally
 * single-fire per mount — subsequent changes flow through the
 * `aegis:settings-changed` event listener.
 */
export function useHydrateSettingsFromStore() {
  const { mode, setMode } = useThemeMode();
  const { locale, setLocale } = useI18n();

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      const store = await getStore();
      const theme = await store.get<ThemeMode>("theme");
      const loc = await store.get<Locale>("locale");
      if (cancelled) return;
      if (theme && theme !== mode) setMode(theme);
      if (loc && loc !== locale) setLocale(loc);
    })();
    return () => {
      cancelled = true;
    };
    // Run once per window mount; deps intentionally omitted.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
}

/**
 * Subscribe to `aegis:settings-changed` events from other windows.
 * The event fires from PersistentThemeProvider / PersistentI18nProvider
 * (defined in SettingsSyncBridge.tsx) when the user toggles a setting
 * in the main window. The local main window also receives its own
 * emit — that's a no-op because setMode/setLocale already ran.
 */
export function useListenForSettingsChanges() {
  const { setMode } = useThemeMode();
  const { setLocale } = useI18n();
  useEffect(() => {
    let unlisten: (() => void) | null = null;
    void (async () => {
      const { listen } = await import("@tauri-apps/api/event");
      const u = await listen<{ theme?: ThemeMode; locale?: Locale }>(
        "aegis:settings-changed",
        ({ payload }) => {
          if (payload.theme) void setMode(payload.theme);
          if (payload.locale) void setLocale(payload.locale);
        },
      );
      unlisten = u;
    })();
    return () => {
      if (unlisten) unlisten();
    };
  }, [setMode, setLocale]);
}

/**
 * Imperative write used by PersistentThemeProvider /
 * PersistentI18nProvider to persist a setting change and trigger
 * the cross-window broadcast. Only the keys present in `patch` are
 * written — passing only `theme` does not clobber `locale` and vice
 * versa.
 */
export async function persistSettings(patch: {
  theme?: ThemeMode;
  locale?: Locale;
}) {
  const store = await getStore();
  if (patch.theme !== undefined) await store.set("theme", patch.theme);
  if (patch.locale !== undefined) await store.set("locale", patch.locale);
  await store.save();
}

/**
 * The settings-store key holding a project's active branch. One key
 * per project code.
 */
const activeBranchKey = (projectCode: string) => `activeBranch.${projectCode}`;

/**
 * Read the branch id this installation is working on for a project, or
 * `null` when it has no active branch.
 *
 * The server dropped the `active` flag from the branch row, so this is
 * desktop-local state: it is shared by every window of this app on
 * this machine, and by nothing else. The id is stored rather than the
 * name so that renaming the branch does not lose the selection —
 * branches are never deleted, so an id stays valid.
 */
export async function readActiveBranch(
  projectCode: string,
): Promise<number | null> {
  const store = await getStore();
  const value = await store.get<number | null>(activeBranchKey(projectCode));
  return typeof value === "number" ? value : null;
}

/**
 * Write the branch id this installation is working on for a project.
 * `null` clears it — a project may have no active branch.
 */
export async function writeActiveBranch(
  projectCode: string,
  branchId: number | null,
): Promise<void> {
  const store = await getStore();
  await store.set(activeBranchKey(projectCode), branchId);
  await store.save();
}
