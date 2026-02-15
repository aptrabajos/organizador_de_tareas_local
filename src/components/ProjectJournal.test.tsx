import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@solidjs/testing-library';
import ProjectJournal from './ProjectJournal';
import type { JournalEntry } from '../types/project';

vi.mock('../services/api', () => ({
  getJournalEntries: vi.fn(),
  createJournalEntry: vi.fn(),
  updateJournalEntry: vi.fn(),
  deleteJournalEntry: vi.fn(),
}));

import {
  getJournalEntries,
  createJournalEntry,
  updateJournalEntry,
} from '../services/api';

describe('ProjectJournal', () => {
  const mockEntries: JournalEntry[] = [
    {
      id: 1,
      project_id: 1,
      content: 'Primera entrada del diario',
      tags: '["bug","fix"]',
      created_at: '2025-01-15T14:00:00Z',
      updated_at: '2025-01-15T14:00:00Z',
    },
    {
      id: 2,
      project_id: 1,
      content: 'Segunda entrada del diario',
      created_at: '2025-01-14T10:00:00Z',
      updated_at: '2025-01-14T10:00:00Z',
    },
  ];

  const mockOnClose = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    (getJournalEntries as ReturnType<typeof vi.fn>).mockResolvedValue(
      mockEntries
    );
  });

  describe('Rendering', () => {
    it('should render list of journal entries', async () => {
      render(() => <ProjectJournal projectId={1} onClose={mockOnClose} />);

      await waitFor(() => {
        expect(screen.getByText('Primera entrada del diario')).toBeTruthy();
      });
      expect(screen.getByText('Segunda entrada del diario')).toBeTruthy();
    });

    it('should show "No hay entradas aún" when list is empty', async () => {
      (getJournalEntries as ReturnType<typeof vi.fn>).mockResolvedValue([]);

      render(() => <ProjectJournal projectId={1} onClose={mockOnClose} />);

      await waitFor(() => {
        expect(screen.getByText(/No hay entradas aún/)).toBeTruthy();
      });
    });

    it('should show "Cargando entradas..." while loading', () => {
      (getJournalEntries as ReturnType<typeof vi.fn>).mockReturnValue(
        new Promise(() => {})
      );

      render(() => <ProjectJournal projectId={1} onClose={mockOnClose} />);

      expect(screen.getByText('Cargando entradas...')).toBeTruthy();
    });

    it('should render tags as badges', async () => {
      render(() => <ProjectJournal projectId={1} onClose={mockOnClose} />);

      await waitFor(() => {
        expect(screen.getByText('#bug')).toBeTruthy();
      });
      expect(screen.getByText('#fix')).toBeTruthy();
    });

    it('should show error message when API fails', async () => {
      (getJournalEntries as ReturnType<typeof vi.fn>).mockRejectedValue(
        new Error('DB error')
      );

      render(() => <ProjectJournal projectId={1} onClose={mockOnClose} />);

      await waitFor(() => {
        expect(
          screen.getByText('Error al cargar las entradas del diario')
        ).toBeTruthy();
      });
    });

    it('should call onClose when close button is clicked', async () => {
      render(() => <ProjectJournal projectId={1} onClose={mockOnClose} />);

      const closeButton = screen.getByText('×');
      closeButton.click();

      expect(mockOnClose).toHaveBeenCalledTimes(1);
    });
  });

  describe('Creating entries', () => {
    it('should create new entry with content and tags', async () => {
      const newEntry: JournalEntry = {
        id: 3,
        project_id: 1,
        content: 'Nueva entrada',
        tags: 'feature',
        created_at: '2025-01-16T10:00:00Z',
        updated_at: '2025-01-16T10:00:00Z',
      };
      (createJournalEntry as ReturnType<typeof vi.fn>).mockResolvedValue(
        newEntry
      );

      render(() => <ProjectJournal projectId={1} onClose={mockOnClose} />);

      await waitFor(() => {
        expect(screen.getByText('Primera entrada del diario')).toBeTruthy();
      });

      // Type content in textarea
      const textarea = screen.getByPlaceholderText(/Escribe una nota rápida/);
      fireEvent.input(textarea, { target: { value: 'Nueva entrada' } });

      // Type tags
      const tagsInput = screen.getByPlaceholderText(/Tags: bug, tip, idea/);
      fireEvent.input(tagsInput, { target: { value: 'feature' } });

      // Click save button
      const saveButton = screen.getByText(/Guardar/);
      await fireEvent.click(saveButton);

      expect(createJournalEntry).toHaveBeenCalledWith({
        project_id: 1,
        content: 'Nueva entrada',
        tags: 'feature',
      });
    });

    it('should disable save button when content is empty', async () => {
      render(() => <ProjectJournal projectId={1} onClose={mockOnClose} />);

      await waitFor(() => {
        expect(screen.getByText('Primera entrada del diario')).toBeTruthy();
      });

      const saveButton = screen.getByText(/Guardar/);
      expect(saveButton.hasAttribute('disabled')).toBe(true);
    });

    it('should clear textarea after successful creation', async () => {
      const newEntry: JournalEntry = {
        id: 3,
        project_id: 1,
        content: 'Nueva entrada',
        created_at: '2025-01-16T10:00:00Z',
        updated_at: '2025-01-16T10:00:00Z',
      };
      (createJournalEntry as ReturnType<typeof vi.fn>).mockResolvedValue(
        newEntry
      );

      render(() => <ProjectJournal projectId={1} onClose={mockOnClose} />);

      await waitFor(() => {
        expect(screen.getByText('Primera entrada del diario')).toBeTruthy();
      });

      const textarea = screen.getByPlaceholderText(
        /Escribe una nota rápida/
      ) as HTMLTextAreaElement;
      fireEvent.input(textarea, { target: { value: 'Nueva entrada' } });

      const saveButton = screen.getByText(/Guardar/);
      await fireEvent.click(saveButton);

      await waitFor(() => {
        expect(textarea.value).toBe('');
      });
    });

    it('should reload entries after creating', async () => {
      const newEntry: JournalEntry = {
        id: 3,
        project_id: 1,
        content: 'Nueva entrada',
        created_at: '2025-01-16T10:00:00Z',
        updated_at: '2025-01-16T10:00:00Z',
      };
      (createJournalEntry as ReturnType<typeof vi.fn>).mockResolvedValue(
        newEntry
      );

      render(() => <ProjectJournal projectId={1} onClose={mockOnClose} />);

      await waitFor(() => {
        expect(screen.getByText('Primera entrada del diario')).toBeTruthy();
      });

      // Initial load + after create = at least 2 calls
      const initialCallCount = (getJournalEntries as ReturnType<typeof vi.fn>)
        .mock.calls.length;

      const textarea = screen.getByPlaceholderText(/Escribe una nota rápida/);
      fireEvent.input(textarea, { target: { value: 'Nueva entrada' } });

      const saveButton = screen.getByText(/Guardar/);
      await fireEvent.click(saveButton);

      await waitFor(() => {
        expect(
          (getJournalEntries as ReturnType<typeof vi.fn>).mock.calls.length
        ).toBeGreaterThan(initialCallCount);
      });
    });
  });

  describe('Editing entries', () => {
    it('should enter edit mode when edit button is clicked', async () => {
      render(() => <ProjectJournal projectId={1} onClose={mockOnClose} />);

      await waitFor(() => {
        expect(screen.getByText('Primera entrada del diario')).toBeTruthy();
      });

      // Click the edit button (emoji)
      const editButtons = screen.getAllByText('✏️');
      editButtons[0].click();

      // Should show edit form with save and cancel buttons
      await waitFor(() => {
        expect(screen.getByText(/✓ Guardar/)).toBeTruthy();
      });
      expect(screen.getByText(/✗ Cancelar/)).toBeTruthy();
    });

    it('should save edited entry', async () => {
      (updateJournalEntry as ReturnType<typeof vi.fn>).mockResolvedValue({
        ...mockEntries[0],
        content: 'Contenido editado',
      });

      render(() => <ProjectJournal projectId={1} onClose={mockOnClose} />);

      await waitFor(() => {
        expect(screen.getByText('Primera entrada del diario')).toBeTruthy();
      });

      // Enter edit mode
      const editButtons = screen.getAllByText('✏️');
      editButtons[0].click();

      await waitFor(() => {
        expect(screen.getByText(/✓ Guardar/)).toBeTruthy();
      });

      // Click save
      const saveEditButton = screen.getByText(/✓ Guardar/);
      await fireEvent.click(saveEditButton);

      expect(updateJournalEntry).toHaveBeenCalledWith(
        1,
        expect.objectContaining({
          content: 'Primera entrada del diario',
        })
      );
    });

    it('should cancel edit without saving', async () => {
      render(() => <ProjectJournal projectId={1} onClose={mockOnClose} />);

      await waitFor(() => {
        expect(screen.getByText('Primera entrada del diario')).toBeTruthy();
      });

      // Enter edit mode
      const editButtons = screen.getAllByText('✏️');
      editButtons[0].click();

      await waitFor(() => {
        expect(screen.getByText(/✗ Cancelar/)).toBeTruthy();
      });

      // Click cancel
      const cancelButton = screen.getByText(/✗ Cancelar/);
      cancelButton.click();

      // updateJournalEntry should NOT have been called
      expect(updateJournalEntry).not.toHaveBeenCalled();

      // Should exit edit mode - edit buttons visible again
      await waitFor(() => {
        expect(screen.getAllByText('✏️').length).toBeGreaterThan(0);
      });
    });
  });
});
