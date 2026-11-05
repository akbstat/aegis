import { Box, Tab, Tabs } from "@aegis/ui/mui";
import { AccountTree, Folder, People, Settings } from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";
import type { ComponentType } from "react";

export type ConfigurationSection =
  | "general"
  | "members"
  | "branches"
  | "filepath";

export interface ConfigurationSidebarProps {
  value: ConfigurationSection;
  onChange: (next: ConfigurationSection) => void;
  width?: number;
}

interface SectionEntry {
  key: ConfigurationSection;
  icon: ComponentType;
}

const ORDER: SectionEntry[] = [
  { key: "general", icon: Settings },
  { key: "members", icon: People },
  { key: "branches", icon: AccountTree },
  { key: "filepath", icon: Folder },
];

/**
 * Right-side nav for the configuration page. Vertical orientation
 * stacks the three sections so the icon sits above its label. The
 * active tab gets MUI's default indicator treatment (vertical bar
 * on the leading edge). Items get generous padding so the icon
 * and label don't hug the container edges.
 *
 * No routing here — selection is in-page state, driven by `value` /
 * `onChange`. The parent owns the state so it can also rehydrate on
 * back-navigation or external deep-links later without restructuring
 * this component.
 */
export function ConfigurationSidebar({
  value,
  onChange,
  width = 150,
}: ConfigurationSidebarProps) {
  const { t } = useI18n();

  return (
    <Box
      component="nav"
      aria-label="configuration-section"
      sx={{
        width,
        py: 13,
        px: 1,
      }}
    >
      <Tabs
        orientation="vertical"
        value={value}
        onChange={(_e, next: ConfigurationSection) => onChange(next)}
        sx={{ minHeight: "auto" }}
      >
        {ORDER.map(({ key: section, icon: Icon }) => (
          <Tab
            key={section}
            value={section}
            icon={<Icon />}
            iconPosition="end"
            label={t(`project.configuration.section.${section}` as const)}
            data-testid={`config-section-${section}`}
            sx={{ justifyContent: "flex-end", minHeight: "auto", py: 1.5 }}
          />
        ))}
      </Tabs>
    </Box>
  );
}