import { describe, it, expect, vi } from 'vitest';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Sidebar } from './Sidebar';
import type { MenuItem } from './types';
import { renderWithTheme } from './test-utils';

const Icon = () => <svg data-testid="mock-icon" />;

const baseMenu: MenuItem[] = [
  { link: '/home', title: 'Home', icon: Icon },
  {
    link: '/settings',
    title: 'Settings',
    icon: Icon,
    subMenu: [{ link: '/settings/profile', title: 'Profile', icon: Icon }],
  },
];

const defaultProps = {
  title: 'My App',
  menu: baseMenu,
  open: true,
  onToggle: () => {},
};

describe('Sidebar', () => {
  it('renders title when open=true', () => {
    renderWithTheme(<Sidebar {...defaultProps} />);
    expect(screen.getByText('My App')).toBeInTheDocument();
  });

  it('hides title when open=false', () => {
    renderWithTheme(<Sidebar {...defaultProps} open={false} />);
    expect(screen.queryByText('My App')).not.toBeInTheDocument();
  });

  it('renders the title adornment next to the title', () => {
    renderWithTheme(
      <Sidebar {...defaultProps} titleAdornment={<span>2026Q1</span>} />,
    );
    expect(screen.getByText('2026Q1')).toBeInTheDocument();
  });

  it('hides the title adornment when open=false', () => {
    renderWithTheme(
      <Sidebar {...defaultProps} open={false} titleAdornment={<span>2026Q1</span>} />,
    );
    expect(screen.queryByText('2026Q1')).not.toBeInTheDocument();
  });

  it('toggle button calls onToggle when clicked', async () => {
    const onToggle = vi.fn();
    renderWithTheme(<Sidebar {...defaultProps} onToggle={onToggle} />);
    await userEvent.click(screen.getByLabelText('toggle sidebar'));
    expect(onToggle).toHaveBeenCalledTimes(1);
  });

  it('toggle icon is FormatIndentDecrease when open=true', () => {
    renderWithTheme(<Sidebar {...defaultProps} />);
    expect(screen.getByTestId('FormatIndentDecreaseIcon')).toBeInTheDocument();
  });

  it('toggle icon is FormatIndentIncrease when open=false', () => {
    renderWithTheme(<Sidebar {...defaultProps} open={false} />);
    expect(screen.getByTestId('FormatIndentIncreaseIcon')).toBeInTheDocument();
  });

  it('clicking a leaf menu item calls onNavigate with its link', async () => {
    const onNavigate = vi.fn();
    renderWithTheme(<Sidebar {...defaultProps} onNavigate={onNavigate} />);
    await userEvent.click(screen.getByText('Home'));
    expect(onNavigate).toHaveBeenCalledWith('/home');
  });

  it('clicking a parent menu toggles its submenu open', async () => {
    renderWithTheme(<Sidebar {...defaultProps} />);
    expect(screen.queryByText('Profile')).not.toBeInTheDocument();
    await userEvent.click(screen.getByText('Settings'));
    expect(screen.getByText('Profile')).toBeInTheDocument();
    await userEvent.click(screen.getByText('Settings'));
    await waitFor(() =>
      expect(screen.queryByText('Profile')).not.toBeInTheDocument(),
    );
  });

  it('clicking a parent menu does NOT call onNavigate', async () => {
    const onNavigate = vi.fn();
    renderWithTheme(<Sidebar {...defaultProps} onNavigate={onNavigate} />);
    await userEvent.click(screen.getByText('Settings'));
    expect(onNavigate).not.toHaveBeenCalled();
  });

  it('collapsed mode renders only icons, hides menu text', () => {
    renderWithTheme(<Sidebar {...defaultProps} open={false} />);
    expect(screen.queryByText('Home')).not.toBeInTheDocument();
    expect(screen.queryByText('Settings')).not.toBeInTheDocument();
    expect(screen.queryByText('Profile')).not.toBeInTheDocument();
    expect(screen.getAllByTestId('mock-icon').length).toBeGreaterThan(0);
  });

  it('applies custom width and collapsedWidth without crashing', () => {
    expect(() =>
      renderWithTheme(
        <Sidebar {...defaultProps} width={300} collapsedWidth={64} />,
      ),
    ).not.toThrow();
  });

  it('renders footer content when provided', () => {
    renderWithTheme(
      <Sidebar
        {...defaultProps}
        footer={<div data-testid="custom-footer">Signed in as Alice</div>}
      />,
    );
    expect(screen.getByTestId('custom-footer')).toBeInTheDocument();
    expect(screen.getByText('Signed in as Alice')).toBeInTheDocument();
  });

  it('omits footer area when footer prop is not provided', () => {
    renderWithTheme(<Sidebar {...defaultProps} />);
    expect(screen.queryByTestId('custom-footer')).not.toBeInTheDocument();
  });
});