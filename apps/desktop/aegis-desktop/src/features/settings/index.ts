// Public API of the settings feature.

export {
  useHydrateSettingsFromStore,
  useListenForSettingsChanges,
  persistSettings,
  readActiveBranch,
  writeActiveBranch,
} from "./data/persist";

export { useUpdatePassword } from "./data/update-password";