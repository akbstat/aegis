import React from "react";
import { Outlet, useNavigate, useParams } from "@tanstack/react-router";
import { Box, Button, Chip, IconButton } from "@aegis/ui/mui";
import { Sidebar, type MenuItem, type SidebarProps } from "@aegis/ui";
import {
  Assignment as AssignmentIcon,
  Dashboard as DashboardIcon,
  Launch as LaunchIcon,
  Settings as SettingsIcon,
} from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";
import { getAllWebviewWindows } from "@tauri-apps/api/webviewWindow";

import { useActiveBranchId, useProjectBranches } from "../data/project-branches";

const DashboardMenuIcon = () => <DashboardIcon />;
const ConfigMenuIcon = () => <SettingsIcon />;
const CrfMenuIcon = () => <AssignmentIcon />;

/**
 * Workspace window shell. Sidebar header is the project code; menu
 * has Dashboard + Configuration entries only; footer is a "Back to
 * main" button that focuses the main window. Mounted by the
 * `_project/route.tsx` layout.
 */
export function ProjectWorkspaceLayout() {
  const { t } = useI18n();
  const navigate = useNavigate();
  const { projectCode } = useParams({ strict: false }) as {
    projectCode: string;
  };
  const [sidebarOpen, setSidebarOpen] = React.useState(true);

  // The active branch is desktop-local state — the server does not
  // model one — so the chip is resolved from the settings store plus
  // the project's branch list.
  const { data: branches } = useProjectBranches(projectCode);
  const { data: activeBranchId } = useActiveBranchId(projectCode);
  const activeBranch = branches?.find((b) => b.id === activeBranchId);

  const menu: MenuItem[] = [
    {
      link: `/project/${projectCode}/dashboard`,
      title: t("workspace.menu.dashboard"),
      icon: DashboardMenuIcon,
    },
    {
      link: `/project/${projectCode}/crf`,
      title: t("workspace.menu.crf"),
      icon: CrfMenuIcon,
    },
    {
      link: `/project/${projectCode}/configuration`,
      title: t("workspace.menu.configuration"),
      icon: ConfigMenuIcon,
    },
  ];

  async function focusMainWindow() {
    const all = await getAllWebviewWindows();
    const mainWin = all.find((w) => w.label === "main");
    if (mainWin) {
      await mainWin.setFocus();
      await mainWin.show();
    }
  }

  const sidebarProps: SidebarProps = {
    title: projectCode,
    titleAdornment: activeBranch ? (
      <Chip size="small" label={activeBranch.branch} />
    ) : undefined,
    menu,
    open: sidebarOpen,
    onToggle: () => setSidebarOpen((o) => !o),
    onNavigate: (link) => navigate({ to: link }),
    footer: sidebarOpen ? (
      <Button
        size="small"
        variant="outlined"
        fullWidth
        onClick={() => void focusMainWindow()}
      >
        {t("workspace.focusMain")}
      </Button>
    ) : (
      <IconButton
        aria-label={t("workspace.focusMain")}
        onClick={() => void focusMainWindow()}
        size="small"
      >
        <LaunchIcon />
      </IconButton>
    ),
  };

  return (
    <Box sx={{ display: "flex", minHeight: "100vh" }}>
      <Sidebar {...sidebarProps} />
      <Box
        component="main"
        sx={{ flexGrow: 1, transition: "margin 0.3s" }}
      >
        <Outlet />
      </Box>
    </Box>
  );
}