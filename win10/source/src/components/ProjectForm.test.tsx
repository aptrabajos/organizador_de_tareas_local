import { describe, it, expect, vi } from 'vitest';
import { render, screen, waitFor } from '@solidjs/testing-library';
import userEvent from '@testing-library/user-event';
import ProjectForm from './ProjectForm';
import type { Project } from '../types/project';

describe('ProjectForm', () => {
  const mockProject: Project = {
    id: 1,
    name: 'Test Project',
    description: 'Test Description',
    local_path: '/home/user/test',
    documentation_url: 'https://docs.test.com',
    drive_link: 'https://drive.test.com',
    created_at: '2024-01-01T00:00:00Z',
    updated_at: '2024-01-01T00:00:00Z',
  };

  it('should render form with empty fields for new project', () => {
    render(() => <ProjectForm onSubmit={vi.fn()} onCancel={vi.fn()} />);

    expect(screen.getByLabelText(/nombre/i)).toBeTruthy();
    expect(screen.getByLabelText(/descripción/i)).toBeTruthy();
    expect(screen.getByLabelText(/ruta local/i)).toBeTruthy();
  });

  it('should render form with project data when editing', () => {
    render(() => (
      <ProjectForm
        project={mockProject}
        onSubmit={vi.fn()}
        onCancel={vi.fn()}
      />
    ));

    const nameInput = screen.getByLabelText(/nombre/i) as HTMLInputElement;
    const descInput = screen.getByLabelText(
      /descripción/i
    ) as HTMLTextAreaElement;
    const pathInput = screen.getByLabelText(/ruta local/i) as HTMLInputElement;

    expect(nameInput.value).toBe('Test Project');
    expect(descInput.value).toBe('Test Description');
    expect(pathInput.value).toBe('/home/user/test');
  });

  it('should call onSubmit with form data when submitting new project', async () => {
    const user = userEvent.setup();
    const onSubmit = vi.fn();

    render(() => <ProjectForm onSubmit={onSubmit} onCancel={vi.fn()} />);

    await user.type(screen.getByLabelText(/nombre/i), 'New Project');
    await user.type(screen.getByLabelText(/descripción/i), 'New Description');
    await user.type(screen.getByLabelText(/ruta local/i), '/home/user/new');

    await user.click(screen.getByRole('button', { name: /guardar/i }));

    await waitFor(() => {
      expect(onSubmit).toHaveBeenCalledWith({
        name: 'New Project',
        description: 'New Description',
        local_path: '/home/user/new',
        documentation_url: undefined,
        ai_documentation_url: undefined,
        drive_link: undefined,
        notes: undefined,
        image_data: undefined,
      });
    });
  });

  it('should call onSubmit with updated data when editing', async () => {
    const user = userEvent.setup();
    const onSubmit = vi.fn();

    render(() => (
      <ProjectForm
        project={mockProject}
        onSubmit={onSubmit}
        onCancel={vi.fn()}
      />
    ));

    const nameInput = screen.getByLabelText(/nombre/i);
    await user.clear(nameInput);
    await user.type(nameInput, 'Updated Name');

    await user.click(screen.getByRole('button', { name: /guardar/i }));

    await waitFor(() => {
      expect(onSubmit).toHaveBeenCalledWith(
        expect.objectContaining({
          name: 'Updated Name',
        })
      );
    });
  });

  it('should call onCancel when cancel button is clicked', async () => {
    const user = userEvent.setup();
    const onCancel = vi.fn();

    render(() => <ProjectForm onSubmit={vi.fn()} onCancel={onCancel} />);

    await user.click(screen.getByRole('button', { name: /cancelar/i }));

    expect(onCancel).toHaveBeenCalled();
  });

  it('should show validation error for empty name', async () => {
    const user = userEvent.setup();
    const onSubmit = vi.fn();

    render(() => <ProjectForm onSubmit={onSubmit} onCancel={vi.fn()} />);

    await user.type(screen.getByLabelText(/descripción/i), 'Description');
    await user.type(screen.getByLabelText(/ruta local/i), '/path');

    await user.click(screen.getByRole('button', { name: /guardar/i }));

    expect(onSubmit).not.toHaveBeenCalled();
  });

  it('should show validation error for empty description', async () => {
    const user = userEvent.setup();
    const onSubmit = vi.fn();

    render(() => <ProjectForm onSubmit={onSubmit} onCancel={vi.fn()} />);

    await user.type(screen.getByLabelText(/nombre/i), 'Name');
    await user.type(screen.getByLabelText(/ruta local/i), '/path');

    await user.click(screen.getByRole('button', { name: /guardar/i }));

    expect(onSubmit).not.toHaveBeenCalled();
  });

  it('should show validation error for empty path', async () => {
    const user = userEvent.setup();
    const onSubmit = vi.fn();

    render(() => <ProjectForm onSubmit={onSubmit} onCancel={vi.fn()} />);

    await user.type(screen.getByLabelText(/nombre/i), 'Name');
    await user.type(screen.getByLabelText(/descripción/i), 'Description');

    await user.click(screen.getByRole('button', { name: /guardar/i }));

    expect(onSubmit).not.toHaveBeenCalled();
  });

  it('should handle optional fields', async () => {
    const user = userEvent.setup();
    const onSubmit = vi.fn();

    render(() => <ProjectForm onSubmit={onSubmit} onCancel={vi.fn()} />);

    await user.type(screen.getByLabelText(/nombre/i), 'Name');
    await user.type(screen.getByLabelText(/descripción/i), 'Description');
    await user.type(screen.getByLabelText(/ruta local/i), '/path');
    await user.type(
      screen.getByLabelText(/^URL Documentación$/i),
      'https://docs.com'
    );
    await user.type(
      screen.getByLabelText(/google drive/i),
      'https://drive.com'
    );

    await user.click(screen.getByRole('button', { name: /guardar/i }));

    await waitFor(() => {
      expect(onSubmit).toHaveBeenCalledWith({
        name: 'Name',
        description: 'Description',
        local_path: '/path',
        documentation_url: 'https://docs.com',
        ai_documentation_url: undefined,
        drive_link: 'https://drive.com',
        notes: undefined,
        image_data: undefined,
      });
    });
  });
});
