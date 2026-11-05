import { useState } from 'react';
import {
  Drawer,
  Box,
  Typography,
  IconButton,
  Divider,
  List,
  ListItem,
  ListItemButton,
  ListItemIcon,
  ListItemText,
  Collapse,
  Tooltip,
} from '@mui/material';
import { FormatIndentDecrease, FormatIndentIncrease } from '@mui/icons-material';
import type { SidebarProps, MenuItem } from './types';

export function Sidebar({
  title,
  titleAdornment,
  menu,
  open,
  onToggle,
  onNavigate,
  footer,
  width = 240,
  collapsedWidth = 56,
}: SidebarProps) {
  const [expandedKeys, setExpandedKeys] = useState<Set<string>>(new Set());
  const drawerWidth = open ? width : collapsedWidth;

  const toggleExpanded = (link: string) => {
    setExpandedKeys((prev) => {
      const next = new Set(prev);
      if (next.has(link)) next.delete(link);
      else next.add(link);
      return next;
    });
  };

  return (
    <Drawer
      variant="permanent"
      data-testid="sidebar"
      sx={{
        width: drawerWidth,
        flexShrink: 0,
        '& .MuiDrawer-paper': {
          width: drawerWidth,
          boxSizing: 'border-box',
          transition: 'width 0.3s',
          overflowX: 'hidden',
        },
      }}
    >
      <Box sx={{ display: 'flex', alignItems: 'center', px: 2.5, py: 1, minHeight: 56 }}>
        <IconButton onClick={onToggle} aria-label="toggle sidebar" edge="start">
          {open ? <FormatIndentDecrease /> : <FormatIndentIncrease />}
        </IconButton>
        {open && (
          <Typography variant="h6" sx={{ ml: 0.5, mr: 0.5 }} noWrap>
            {title}
          </Typography>
        )}
        {open && titleAdornment}
      </Box>
      <Divider />
      <List>
        {menu.map((item) => (
          <SidebarMenuItem
            key={item.link}
            item={item}
            open={open}
            expanded={expandedKeys.has(item.link)}
            onToggle={() => toggleExpanded(item.link)}
            onNavigate={onNavigate}
          />
        ))}
      </List>
      {footer && (
        <Box sx={{ mt: 'auto' }}>
          <Divider />
          <Box sx={{ p: 1.5 }}>{footer}</Box>
        </Box>
      )}
    </Drawer>
  );
}

interface SidebarMenuItemProps {
  item: MenuItem;
  open: boolean;
  expanded: boolean;
  onToggle: () => void;
  onNavigate?: (link: string) => void;
}

function SidebarMenuItem({
  item,
  open,
  expanded,
  onToggle,
  onNavigate,
}: SidebarMenuItemProps) {
  const hasSubmenu = !!item.subMenu?.length;
  const Icon = item.icon;

  const handleClick = () => {
    if (hasSubmenu) onToggle();
    else onNavigate?.(item.link);
  };

  const button = (
    <ListItemButton onClick={handleClick}>
      <ListItemIcon>
        <Icon />
      </ListItemIcon>
      {open && <ListItemText primary={item.title} />}
    </ListItemButton>
  );

  return (
    <>
      <ListItem disablePadding>
        {open ? (
          button
        ) : (
          <Tooltip title={item.title} placement="right">
            <span>{button}</span>
          </Tooltip>
        )}
      </ListItem>
      {hasSubmenu && open && (
        <Collapse in={expanded} unmountOnExit>
          <List disablePadding>
            {item.subMenu!.map((sub) => {
              const SubIcon = sub.icon;
              return (
                <ListItem key={sub.link} disablePadding sx={{ pl: 2 }}>
                  <ListItemButton onClick={() => onNavigate?.(sub.link)}>
                    <ListItemIcon>
                      <SubIcon />
                    </ListItemIcon>
                    <ListItemText primary={sub.title} />
                  </ListItemButton>
                </ListItem>
              );
            })}
          </List>
        </Collapse>
      )}
    </>
  );
}