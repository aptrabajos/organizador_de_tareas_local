import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@solidjs/testing-library';
import userEvent from '@testing-library/user-event';
import ProjectList from './ProjectList';
import type { Project } from '../types/project';

describe('ProjectList', () => {
  const mockProjects: Project[] = [
    {
      id: 1,
      name: 'Proyecto 1',
      description: 'Descripción del proyecto 1',
      local_path: '/home/user/proyecto1',
      documentation_url: 'https://docs1.com',
      drive_link: 'https://drive1.com',
      created_at: '2024-01-01T00:00:00Z',
      updated_at: '2024-01-01T00:00:00Z',
    },
    {
      id: 2,
      name: 'Proyecto 2',
      description: 'Descripción del proyecto 2',
      local_path: '/home/user/proyecto2',
      created_at: '2024-01-02T00:00:00Z',
      updated_at: '2024-01-02T00:00:00Z',
    },
  ];

  it('should render empty state when no projects', () => {
    render(() => (
      <ProjectList
        projects={[]}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
        onOpenTerminal={vi.fn()}
      />
    ));

    expect(screen.getByText(/no hay proyectos/i)).toBeTruthy();
  });

  it('should render list of projects', () => {
    render(() => (
      <ProjectList
        projects={mockProjects}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
        onOpenTerminal={vi.fn()}
      />
    ));

    expect(screen.getByText('Proyecto 1')).toBeTruthy();
    expect(screen.getByText('Proyecto 2')).toBeTruthy();
    expect(screen.getByText('Descripción del proyecto 1')).toBeTruthy();
  });

  it('should display project paths', () => {
    render(() => (
      <ProjectList
        projects={mockProjects}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
        onOpenTerminal={vi.fn()}
      />
    ));

    expect(
      screen.getByText((content) => content.includes('/home/user/proyecto1'))
    ).toBeTruthy();
    expect(
      screen.getByText((content) => content.includes('/home/user/proyecto2'))
    ).toBeTruthy();
  });

  it('should call onOpenTerminal when work button is clicked', async () => {
    const user = userEvent.setup();
    const onOpenTerminal = vi.fn();

    render(() => (
      <ProjectList
        projects={mockProjects}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
        onOpenTerminal={onOpenTerminal}
      />
    ));

    const workButtons = screen.getAllByRole('button', { name: /trabajar/i });
    await user.click(workButtons[0]);

    expect(onOpenTerminal).toHaveBeenCalledWith(mockProjects[0]);
  });

  it('should call onEdit when edit button is clicked', async () => {
    const user = userEvent.setup();
    const onEdit = vi.fn();

    render(() => (
      <ProjectList
        projects={mockProjects}
        onEdit={onEdit}
        onDelete={vi.fn()}
        onOpenTerminal={vi.fn()}
      />
    ));

    const editButtons = screen.getAllByRole('button', { name: /editar/i });
    await user.click(editButtons[0]);

    expect(onEdit).toHaveBeenCalledWith(mockProjects[0]);
  });

  it('should call onDelete when delete button is clicked', async () => {
    const user = userEvent.setup();
    const onDelete = vi.fn();

    render(() => (
      <ProjectList
        projects={mockProjects}
        onEdit={vi.fn()}
        onDelete={onDelete}
        onOpenTerminal={vi.fn()}
      />
    ));

    const deleteButtons = screen.getAllByRole('button', { name: /eliminar/i });
    await user.click(deleteButtons[0]);

    expect(onDelete).toHaveBeenCalledWith(mockProjects[0]);
  });

  it('should show documentation link when available', () => {
    render(() => (
      <ProjectList
        projects={mockProjects}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
        onOpenTerminal={vi.fn()}
      />
    ));

    const docLinks = screen.getAllByRole('link', { name: /documentación/i });
    expect(docLinks).toHaveLength(1);
    expect(docLinks[0].getAttribute('href')).toBe('https://docs1.com');
  });

  it('should show drive link when available', () => {
    render(() => (
      <ProjectList
        projects={mockProjects}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
        onOpenTerminal={vi.fn()}
      />
    ));

    const driveLinks = screen.getAllByRole('link', { name: /drive/i });
    expect(driveLinks).toHaveLength(1);
    expect(driveLinks[0].getAttribute('href')).toBe('https://drive1.com');
  });
});
