import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@solidjs/testing-library';
import ProjectLinks from './ProjectLinks';
import type { ProjectLink } from '../types/project';

vi.mock('../services/api', () => ({
  getProjectLinks: vi.fn(),
  createProjectLink: vi.fn(),
  deleteProjectLink: vi.fn(),
  openUrl: vi.fn(),
}));

import {
  getProjectLinks,
  createProjectLink,
  deleteProjectLink,
  openUrl,
} from '../services/api';

describe('ProjectLinks', () => {
  const mockLinks: ProjectLink[] = [
    {
      id: 1,
      project_id: 1,
      link_type: 'repository',
      title: 'GitHub Repo',
      url: 'https://github.com/user/repo',
      created_at: '2025-01-01T00:00:00Z',
    },
    {
      id: 2,
      project_id: 1,
      link_type: 'documentation',
      title: 'API Docs',
      url: 'https://docs.example.com',
      created_at: '2025-01-02T00:00:00Z',
    },
  ];

  beforeEach(() => {
    vi.clearAllMocks();
    (getProjectLinks as ReturnType<typeof vi.fn>).mockResolvedValue(mockLinks);
  });

  describe('Rendering', () => {
    it('should render list of links with titles and URLs', async () => {
      render(() => <ProjectLinks projectId={1} projectName="Test Project" />);

      await waitFor(() => {
        expect(screen.getByText('GitHub Repo')).toBeTruthy();
      });
      expect(screen.getByText('API Docs')).toBeTruthy();
      expect(screen.getByText('https://github.com/user/repo')).toBeTruthy();
      expect(screen.getByText('https://docs.example.com')).toBeTruthy();
    });

    it('should show badge with link type label', async () => {
      render(() => <ProjectLinks projectId={1} projectName="Test Project" />);

      await waitFor(() => {
        expect(screen.getByText('Repositorio')).toBeTruthy();
      });
      expect(screen.getByText('Documentación')).toBeTruthy();
    });

    it('should show "No hay enlaces agregados aún" when empty', async () => {
      (getProjectLinks as ReturnType<typeof vi.fn>).mockResolvedValue([]);

      render(() => <ProjectLinks projectId={1} projectName="Test Project" />);

      await waitFor(() => {
        expect(screen.getByText('No hay enlaces agregados aún')).toBeTruthy();
      });
    });

    it('should show "Cargando enlaces..." while loading with no links', () => {
      (getProjectLinks as ReturnType<typeof vi.fn>).mockReturnValue(
        new Promise(() => {})
      );

      render(() => <ProjectLinks projectId={1} projectName="Test Project" />);

      expect(screen.getByText('Cargando enlaces...')).toBeTruthy();
    });

    it('should render "+ Agregar Enlace" button', () => {
      render(() => <ProjectLinks projectId={1} projectName="Test Project" />);

      expect(screen.getByText('+ Agregar Enlace')).toBeTruthy();
    });
  });

  describe('Creating links', () => {
    it('should show form when clicking "+ Agregar Enlace"', async () => {
      render(() => <ProjectLinks projectId={1} projectName="Test Project" />);

      await waitFor(() => {
        expect(screen.getByText('+ Agregar Enlace')).toBeTruthy();
      });

      const addButton = screen.getByText('+ Agregar Enlace');
      addButton.click();

      await waitFor(() => {
        expect(screen.getByText('Nuevo Enlace')).toBeTruthy();
      });
    });

    it('should create link with valid data', async () => {
      const newLink = {
        id: 3,
        project_id: 1,
        link_type: 'repository',
        title: 'New Repo',
        url: 'https://gitlab.com/user/repo',
        created_at: '2025-01-03T00:00:00Z',
      };
      (createProjectLink as ReturnType<typeof vi.fn>).mockResolvedValue(
        newLink
      );

      render(() => <ProjectLinks projectId={1} projectName="Test Project" />);

      await waitFor(() => {
        expect(screen.getByText('+ Agregar Enlace')).toBeTruthy();
      });

      // Open form
      screen.getByText('+ Agregar Enlace').click();

      await waitFor(() => {
        expect(screen.getByText('Nuevo Enlace')).toBeTruthy();
      });

      // Fill title
      const titleInput = screen.getByPlaceholderText(
        /Ej: Repositorio principal/
      );
      fireEvent.input(titleInput, { target: { value: 'New Repo' } });

      // Fill URL
      const urlInput = screen.getByPlaceholderText(/github\.com/);
      fireEvent.input(urlInput, {
        target: { value: 'https://gitlab.com/user/repo' },
      });

      // Click save
      const saveButton = screen.getByText('Guardar');
      await fireEvent.click(saveButton);

      expect(createProjectLink).toHaveBeenCalledWith(
        expect.objectContaining({
          project_id: 1,
          title: 'New Repo',
          url: 'https://gitlab.com/user/repo',
        })
      );
    });

    it('should hide form after successful creation', async () => {
      (createProjectLink as ReturnType<typeof vi.fn>).mockResolvedValue({
        id: 3,
        project_id: 1,
        link_type: 'repository',
        title: 'New Repo',
        url: 'https://gitlab.com/user/repo',
        created_at: '2025-01-03T00:00:00Z',
      });

      render(() => <ProjectLinks projectId={1} projectName="Test Project" />);

      await waitFor(() => {
        expect(screen.getByText('+ Agregar Enlace')).toBeTruthy();
      });

      // Open form
      screen.getByText('+ Agregar Enlace').click();

      await waitFor(() => {
        expect(screen.getByText('Nuevo Enlace')).toBeTruthy();
      });

      // Fill required fields
      const titleInput = screen.getByPlaceholderText(
        /Ej: Repositorio principal/
      );
      fireEvent.input(titleInput, { target: { value: 'New Repo' } });
      const urlInput = screen.getByPlaceholderText(/github\.com/);
      fireEvent.input(urlInput, { target: { value: 'https://gitlab.com' } });

      // Submit
      const saveButton = screen.getByText('Guardar');
      await fireEvent.click(saveButton);

      await waitFor(() => {
        expect(screen.queryByText('Nuevo Enlace')).toBeFalsy();
      });
    });

    it('should reload links after creating', async () => {
      (createProjectLink as ReturnType<typeof vi.fn>).mockResolvedValue({
        id: 3,
        project_id: 1,
        link_type: 'repository',
        title: 'X',
        url: 'https://x.com',
        created_at: '2025-01-03T00:00:00Z',
      });

      render(() => <ProjectLinks projectId={1} projectName="Test Project" />);

      await waitFor(() => {
        expect(screen.getByText('GitHub Repo')).toBeTruthy();
      });

      const initialCallCount = (getProjectLinks as ReturnType<typeof vi.fn>)
        .mock.calls.length;

      // Open form, fill, submit
      screen.getByText('+ Agregar Enlace').click();
      await waitFor(() => {
        expect(screen.getByText('Nuevo Enlace')).toBeTruthy();
      });

      const titleInput = screen.getByPlaceholderText(
        /Ej: Repositorio principal/
      );
      fireEvent.input(titleInput, { target: { value: 'X' } });
      const urlInput = screen.getByPlaceholderText(/github\.com/);
      fireEvent.input(urlInput, { target: { value: 'https://x.com' } });

      const saveButton = screen.getByText('Guardar');
      await fireEvent.click(saveButton);

      await waitFor(() => {
        expect(
          (getProjectLinks as ReturnType<typeof vi.fn>).mock.calls.length
        ).toBeGreaterThan(initialCallCount);
      });
    });
  });

  describe('Interactions', () => {
    it('should call openUrl when open link button is clicked', async () => {
      (openUrl as ReturnType<typeof vi.fn>).mockResolvedValue(undefined);

      render(() => <ProjectLinks projectId={1} projectName="Test Project" />);

      await waitFor(() => {
        expect(screen.getByText('GitHub Repo')).toBeTruthy();
      });

      const openButtons = screen.getAllByTitle('Abrir enlace');
      await fireEvent.click(openButtons[0]);

      expect(openUrl).toHaveBeenCalledWith('https://github.com/user/repo');
    });

    it('should call deleteProjectLink when delete button is clicked', async () => {
      (deleteProjectLink as ReturnType<typeof vi.fn>).mockResolvedValue(
        undefined
      );
      // Mock window.confirm
      vi.spyOn(window, 'confirm').mockReturnValue(true);

      render(() => <ProjectLinks projectId={1} projectName="Test Project" />);

      await waitFor(() => {
        expect(screen.getByText('GitHub Repo')).toBeTruthy();
      });

      const deleteButtons = screen.getAllByTitle('Eliminar enlace');
      await fireEvent.click(deleteButtons[0]);

      expect(deleteProjectLink).toHaveBeenCalledWith(1);

      vi.restoreAllMocks();
    });
  });
});
