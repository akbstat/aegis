import React from "react";
import { Outlet, useNavigate } from "@tanstack/react-router";
import { Box } from "@aegis/ui/mui";
import { Sidebar, type MenuItem, type SidebarProps } from "@aegis/ui";
import {
  AccountTree as AccountTreeIcon,
  AdminPanelSettings as AdminPanelSettingsIcon,
  Description as DescriptionIcon,
  Home as HomeIcon,
  LibraryBooks as LibraryBooksIcon,
  People as PeopleIcon,
  Settings as SettingsIcon,
  Workspaces as WorkspacesIcon,
} from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";
import { useCurrentUser } from "../../auth";
import { UserFooter } from "../../auth/components/UserFooter";

const HomeMenuIcon = () => <HomeIcon />;
const ProjectsMenuIcon = () => <WorkspacesIcon />;
const SettingsMenuIcon = () => <SettingsIcon />;
const ManagementMenuIcon = () => <AdminPanelSettingsIcon />;
const UsersMenuIcon = () => <PeopleIcon />;
const KnowledgeBaseMenuIcon = () => <LibraryBooksIcon />;
const MetadataMenuIcon = () => <DescriptionIcon />;
const TemplatesMenuIcon = () => <AccountTreeIcon />;

/**
 * Authenticated app shell: the `Sidebar` plus the active child route.
 * Lives in `src/pages/` (not a route file) so TanStack Router can code-
 * split the route file cleanly. The route file imports this as the
 * component for `/_layout`.
 */
export function AppLayout() {
  const { t } = useI18n();
  const navigate = useNavigate();
  const [sidebarOpen, setSidebarOpen] = React.useState(true);
  const currentUser = useCurrentUser();

  const role = currentUser.data?.role;
  const canManage = role === "root" || role === "admin";

  const managementEntry: MenuItem = {
    link: "#management",
    title: t("nav.management"),
    icon: ManagementMenuIcon,
    subMenu: [
      {
        link: "/management/users",
        title: t("nav.management.users"),
        icon: UsersMenuIcon,
      },
    ],
  };

  const metadataEntry: MenuItem = {
    link: "#metadata",
    title: t("nav.knowledgeBase"),
    icon: KnowledgeBaseMenuIcon,
    subMenu: [
      {
        link: "/metadata",
        title: t("nav.metadata"),
        icon: MetadataMenuIcon,
      },
      {
        link: "/templates",
        title: t("nav.templates"),
        icon: TemplatesMenuIcon,
      },
    ],
  };

  const baseMenu: MenuItem[] = [
    { link: "/", title: t("nav.home"), icon: HomeMenuIcon },
    { link: "/projects", title: t("nav.projects"), icon: ProjectsMenuIcon },
    metadataEntry, // Knowledge Base (submenu: Metadata)
    { link: "/settings", title: t("nav.settings"), icon: SettingsMenuIcon },
  ];

  const menu: MenuItem[] = canManage
    ? [
        ...baseMenu.slice(0, 3), // Home, Projects, Knowledge Base
        managementEntry, // Management (submenu: Users)
        ...baseMenu.slice(3), // Settings
      ]
    : baseMenu;

  const sidebarProps: SidebarProps = {
    title: t("app.title"),
    menu,
    open: sidebarOpen,
    onToggle: () => setSidebarOpen((o) => !o),
    onNavigate: (link) => navigate({ to: link }),
    footer: <UserFooter sidebarOpen={sidebarOpen} />,
  };

  return (
    <Box sx={{ display: "flex", minHeight: "100vh" }}>
      <Sidebar {...sidebarProps} />
      <Box
        component="main"
        sx={{
          flexGrow: 1,
          transition: "margin 0.3s",
        }}
      >
        <Outlet />
      </Box>
    </Box>
  );
}
