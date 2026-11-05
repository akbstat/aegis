import type { ComponentType, ReactNode } from 'react';

export interface SubMenuItem {
  link: string;
  title: string;
  icon: ComponentType;
}

export interface MenuItem extends SubMenuItem {
  subMenu?: SubMenuItem[];
}

export interface SidebarProps {
  title: string;
  /** Rendered next to the title in the header row — a chip, a badge,
   *  anything the caller wants beside the title. Only shown while the
   *  sidebar is expanded, alongside the title itself. */
  titleAdornment?: ReactNode;
  menu: MenuItem[];
  open: boolean;
  onToggle: () => void;
  onNavigate?: (link: string) => void;
  footer?: ReactNode;
  width?: number;
  collapsedWidth?: number;
}