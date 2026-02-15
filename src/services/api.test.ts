import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import * as api from './api';
import type { CreateProjectDTO, UpdateProjectDTO } from '../types/project';

// Mock Tauri's invoke
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

describe('API Service', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('createProject', () => {
    it('should call create_project command with correct data', async () => {
      const mockProject: CreateProjectDTO = {
        name: 'Test Project',
        description: 'Test Description',
        local_path: '/home/user/test',
        documentation_url: 'https://docs.test.com',
        drive_link: 'https://drive.test.com',
      };

      const mockResponse = {
        id: 1,
        ...mockProject,
        created_at: '2024-01-01T00:00:00Z',
        updated_at: '2024-01-01T00:00:00Z',
      };

      vi.mocked(invoke).mockResolvedValue(mockResponse);

      const result = await api.createProject(mockProject);

      expect(invoke).toHaveBeenCalledWith('create_project', {
        project: mockProject,
      });
      expect(result).toEqual(mockResponse);
    });
  });

  describe('getAllProjects', () => {
    it('should call get_all_projects command', async () => {
      const mockProjects = [
        {
          id: 1,
          name: 'Project 1',
          description: 'Desc 1',
          local_path: '/path1',
          created_at: '2024-01-01T00:00:00Z',
          updated_at: '2024-01-01T00:00:00Z',
        },
      ];

      vi.mocked(invoke).mockResolvedValue(mockProjects);

      const result = await api.getAllProjects();

      expect(invoke).toHaveBeenCalledWith('get_all_projects');
      expect(result).toEqual(mockProjects);
    });
  });

  describe('getProject', () => {
    it('should call get_project command with id', async () => {
      const mockProject = {
        id: 1,
        name: 'Project 1',
        description: 'Desc 1',
        local_path: '/path1',
        created_at: '2024-01-01T00:00:00Z',
        updated_at: '2024-01-01T00:00:00Z',
      };

      vi.mocked(invoke).mockResolvedValue(mockProject);

      const result = await api.getProject(1);

      expect(invoke).toHaveBeenCalledWith('get_project', { id: 1 });
      expect(result).toEqual(mockProject);
    });
  });

  describe('updateProject', () => {
    it('should call update_project command with id and updates', async () => {
      const updates: UpdateProjectDTO = {
        name: 'Updated Name',
        description: 'Updated Desc',
      };

      const mockResponse = {
        id: 1,
        name: 'Updated Name',
        description: 'Updated Desc',
        local_path: '/path1',
        created_at: '2024-01-01T00:00:00Z',
        updated_at: '2024-01-02T00:00:00Z',
      };

      vi.mocked(invoke).mockResolvedValue(mockResponse);

      const result = await api.updateProject(1, updates);

      expect(invoke).toHaveBeenCalledWith('update_project', {
        id: 1,
        updates,
      });
      expect(result).toEqual(mockResponse);
    });
  });

  describe('deleteProject', () => {
    it('should call delete_project command with id', async () => {
      vi.mocked(invoke).mockResolvedValue(undefined);

      await api.deleteProject(1);

      expect(invoke).toHaveBeenCalledWith('delete_project', { id: 1 });
    });
  });

  describe('searchProjects', () => {
    it('should call search_projects command with query', async () => {
      const mockProjects = [
        {
          id: 1,
          name: 'Test Project',
          description: 'Test Desc',
          local_path: '/test',
          created_at: '2024-01-01T00:00:00Z',
          updated_at: '2024-01-01T00:00:00Z',
        },
      ];

      vi.mocked(invoke).mockResolvedValue(mockProjects);

      const result = await api.searchProjects('test');

      expect(invoke).toHaveBeenCalledWith('search_projects', { query: 'test' });
      expect(result).toEqual(mockProjects);
    });
  });

  describe('openTerminal', () => {
    it('should call open_terminal command with path', async () => {
      vi.mocked(invoke).mockResolvedValue(undefined);

      await api.openTerminal('/home/user/project');

      expect(invoke).toHaveBeenCalledWith('open_terminal', {
        path: '/home/user/project',
      });
    });
  });

  // ==================== TODOS API TESTS ====================

  describe('TODOs API', () => {
    describe('createTodo', () => {
      it('should create a TODO', async () => {
        const mockTodo = {
          project_id: 1,
          content: 'Implementar feature X',
        };

        const mockResponse = {
          id: 1,
          ...mockTodo,
          is_completed: false,
          created_at: '2024-01-01T00:00:00Z',
          completed_at: null,
        };

        vi.mocked(invoke).mockResolvedValue(mockResponse);

        const result = await api.createTodo(mockTodo);

        expect(invoke).toHaveBeenCalledWith('create_todo', { todo: mockTodo });
        expect(result).toEqual(mockResponse);
        expect(result.is_completed).toBe(false);
      });
    });

    describe('getProjectTodos', () => {
      it('should get all TODOs for a project', async () => {
        const mockTodos = [
          {
            id: 1,
            project_id: 1,
            content: 'TODO 1',
            is_completed: false,
            created_at: '2024-01-01T00:00:00Z',
            completed_at: null,
          },
          {
            id: 2,
            project_id: 1,
            content: 'TODO 2',
            is_completed: true,
            created_at: '2024-01-02T00:00:00Z',
            completed_at: '2024-01-03T00:00:00Z',
          },
        ];

        vi.mocked(invoke).mockResolvedValue(mockTodos);

        const result = await api.getProjectTodos(1);

        expect(invoke).toHaveBeenCalledWith('get_project_todos', {
          projectId: 1,
        });
        expect(result).toHaveLength(2);
        expect(result[0].is_completed).toBe(false);
        expect(result[1].is_completed).toBe(true);
      });
    });

    describe('updateTodo', () => {
      it('should update TODO content', async () => {
        const updates = { content: 'Updated content' };
        const mockResponse = {
          id: 1,
          project_id: 1,
          content: 'Updated content',
          is_completed: false,
          created_at: '2024-01-01T00:00:00Z',
          completed_at: null,
        };

        vi.mocked(invoke).mockResolvedValue(mockResponse);

        const result = await api.updateTodo(1, updates);

        expect(invoke).toHaveBeenCalledWith('update_todo', {
          id: 1,
          updates,
        });
        expect(result.content).toBe('Updated content');
      });

      it('should toggle TODO completion', async () => {
        const updates = { is_completed: true };
        const mockResponse = {
          id: 1,
          project_id: 1,
          content: 'Some TODO',
          is_completed: true,
          created_at: '2024-01-01T00:00:00Z',
          completed_at: '2024-01-05T00:00:00Z',
        };

        vi.mocked(invoke).mockResolvedValue(mockResponse);

        const result = await api.updateTodo(1, updates);

        expect(result.is_completed).toBe(true);
        expect(result.completed_at).toBeTruthy();
      });
    });

    describe('deleteTodo', () => {
      it('should delete a TODO', async () => {
        vi.mocked(invoke).mockResolvedValue(undefined);

        await api.deleteTodo(1);

        expect(invoke).toHaveBeenCalledWith('delete_todo', { id: 1 });
      });
    });
  });

  // ==================== STATUS & PIN API TESTS ====================

  describe('Status & Pin API', () => {
    describe('updateProjectStatus', () => {
      it('should update project status to "pausado"', async () => {
        vi.mocked(invoke).mockResolvedValue(undefined);

        await api.updateProjectStatus(1, 'pausado');

        expect(invoke).toHaveBeenCalledWith('update_project_status', {
          projectId: 1,
          status: 'pausado',
        });
      });

      it('should update project status to "completado"', async () => {
        vi.mocked(invoke).mockResolvedValue(undefined);

        await api.updateProjectStatus(1, 'completado');

        expect(invoke).toHaveBeenCalledWith('update_project_status', {
          projectId: 1,
          status: 'completado',
        });
      });
    });

    describe('togglePinProject', () => {
      it('should pin a project and return true', async () => {
        vi.mocked(invoke).mockResolvedValue(true);

        const result = await api.togglePinProject(1);

        expect(invoke).toHaveBeenCalledWith('toggle_pin_project', {
          projectId: 1,
        });
        expect(result).toBe(true);
      });

      it('should unpin a project and return false', async () => {
        vi.mocked(invoke).mockResolvedValue(false);

        const result = await api.togglePinProject(1);

        expect(result).toBe(false);
      });
    });

    describe('reorderPinnedProjects', () => {
      it('should reorder pinned projects', async () => {
        const newOrder = [3, 1, 5, 2];

        vi.mocked(invoke).mockResolvedValue(undefined);

        await api.reorderPinnedProjects(newOrder);

        expect(invoke).toHaveBeenCalledWith('reorder_pinned_projects', {
          projectIds: newOrder,
        });
      });
    });
  });

  // ==================== LINKS API TESTS ====================

  describe('Links API', () => {
    describe('createProjectLink', () => {
      it('should call create_project_link with link data', async () => {
        const link = {
          project_id: 1,
          link_type: 'repository' as const,
          title: 'GitHub Repo',
          url: 'https://github.com/user/repo',
        };
        const mockResponse = {
          id: 1,
          ...link,
          created_at: '2024-01-01T00:00:00Z',
        };

        vi.mocked(invoke).mockResolvedValue(mockResponse);

        const result = await api.createProjectLink(link);

        expect(invoke).toHaveBeenCalledWith('create_project_link', { link });
        expect(result).toEqual(mockResponse);
      });
    });

    describe('getProjectLinks', () => {
      it('should call get_project_links with projectId', async () => {
        const mockLinks = [
          {
            id: 1,
            project_id: 1,
            link_type: 'repository',
            title: 'Repo',
            url: 'https://github.com',
            created_at: '2024-01-01T00:00:00Z',
          },
        ];

        vi.mocked(invoke).mockResolvedValue(mockLinks);

        const result = await api.getProjectLinks(1);

        expect(invoke).toHaveBeenCalledWith('get_project_links', {
          projectId: 1,
        });
        expect(result).toEqual(mockLinks);
      });
    });

    describe('updateProjectLink', () => {
      it('should call update_project_link with id and link data', async () => {
        const linkUpdate = { title: 'Updated Title' };
        const mockResponse = {
          id: 1,
          project_id: 1,
          link_type: 'repository',
          title: 'Updated Title',
          url: 'https://github.com',
          created_at: '2024-01-01T00:00:00Z',
        };

        vi.mocked(invoke).mockResolvedValue(mockResponse);

        const result = await api.updateProjectLink(1, linkUpdate);

        expect(invoke).toHaveBeenCalledWith('update_project_link', {
          id: 1,
          link: linkUpdate,
        });
        expect(result.title).toBe('Updated Title');
      });
    });

    describe('deleteProjectLink', () => {
      it('should call delete_project_link with id', async () => {
        vi.mocked(invoke).mockResolvedValue(undefined);

        await api.deleteProjectLink(1);

        expect(invoke).toHaveBeenCalledWith('delete_project_link', { id: 1 });
      });
    });
  });

  // ==================== JOURNAL API TESTS ====================

  describe('Journal API', () => {
    describe('createJournalEntry', () => {
      it('should call create_journal_entry with entry data', async () => {
        const entry = {
          project_id: 1,
          content: 'Today I fixed a bug',
          tags: 'bugfix,backend',
        };
        const mockResponse = {
          id: 1,
          ...entry,
          created_at: '2024-01-01T00:00:00Z',
          updated_at: '2024-01-01T00:00:00Z',
        };

        vi.mocked(invoke).mockResolvedValue(mockResponse);

        const result = await api.createJournalEntry(entry);

        expect(invoke).toHaveBeenCalledWith('create_journal_entry', { entry });
        expect(result).toEqual(mockResponse);
      });
    });

    describe('getJournalEntries', () => {
      it('should call get_journal_entries with projectId', async () => {
        const mockEntries = [
          {
            id: 1,
            project_id: 1,
            content: 'Entry 1',
            created_at: '2024-01-01T00:00:00Z',
            updated_at: '2024-01-01T00:00:00Z',
          },
          {
            id: 2,
            project_id: 1,
            content: 'Entry 2',
            created_at: '2024-01-02T00:00:00Z',
            updated_at: '2024-01-02T00:00:00Z',
          },
        ];

        vi.mocked(invoke).mockResolvedValue(mockEntries);

        const result = await api.getJournalEntries(1);

        expect(invoke).toHaveBeenCalledWith('get_journal_entries', {
          projectId: 1,
        });
        expect(result).toHaveLength(2);
      });
    });

    describe('updateJournalEntry', () => {
      it('should call update_journal_entry with id and updates', async () => {
        const updates = { content: 'Updated content' };
        const mockResponse = {
          id: 1,
          project_id: 1,
          content: 'Updated content',
          created_at: '2024-01-01T00:00:00Z',
          updated_at: '2024-01-02T00:00:00Z',
        };

        vi.mocked(invoke).mockResolvedValue(mockResponse);

        const result = await api.updateJournalEntry(1, updates);

        expect(invoke).toHaveBeenCalledWith('update_journal_entry', {
          id: 1,
          updates,
        });
        expect(result.content).toBe('Updated content');
      });
    });

    describe('deleteJournalEntry', () => {
      it('should call delete_journal_entry with id', async () => {
        vi.mocked(invoke).mockResolvedValue(undefined);

        await api.deleteJournalEntry(1);

        expect(invoke).toHaveBeenCalledWith('delete_journal_entry', { id: 1 });
      });
    });
  });

  // ==================== GIT API TESTS ====================

  describe('Git API', () => {
    describe('getGitBranch', () => {
      it('should call get_git_branch with path', async () => {
        vi.mocked(invoke).mockResolvedValue('feature/new-feature');

        const result = await api.getGitBranch('/home/user/project');

        expect(invoke).toHaveBeenCalledWith('get_git_branch', {
          path: '/home/user/project',
        });
        expect(result).toBe('feature/new-feature');
      });
    });

    describe('getGitFileCount', () => {
      it('should return file count structure', async () => {
        const mockCount = { modified: 3, staged: 1, untracked: 2 };

        vi.mocked(invoke).mockResolvedValue(mockCount);

        const result = await api.getGitFileCount('/home/user/project');

        expect(invoke).toHaveBeenCalledWith('get_git_file_count', {
          path: '/home/user/project',
        });
        expect(result).toEqual(mockCount);
      });
    });

    describe('getRecentCommits', () => {
      it('should call get_recent_commits with path and limit', async () => {
        const mockCommits = [
          {
            hash: 'abc123',
            author: 'User',
            date: '2024-01-01',
            message: 'Initial commit',
          },
          {
            hash: 'def456',
            author: 'User',
            date: '2024-01-02',
            message: 'Add feature',
          },
        ];

        vi.mocked(invoke).mockResolvedValue(mockCommits);

        const result = await api.getRecentCommits('/home/user/project', 5);

        expect(invoke).toHaveBeenCalledWith('get_recent_commits', {
          path: '/home/user/project',
          limit: 5,
        });
        expect(result).toHaveLength(2);
        expect(result[0].hash).toBe('abc123');
      });
    });

    describe('gitCommit', () => {
      it('should call git_commit with path and message', async () => {
        vi.mocked(invoke).mockResolvedValue('Committed successfully');

        const result = await api.gitCommit(
          '/home/user/project',
          'fix: resolve bug'
        );

        expect(invoke).toHaveBeenCalledWith('git_commit', {
          path: '/home/user/project',
          message: 'fix: resolve bug',
        });
        expect(result).toBe('Committed successfully');
      });
    });

    describe('getGitAheadBehind', () => {
      it('should return ahead/behind tuple', async () => {
        vi.mocked(invoke).mockResolvedValue([3, 1]);

        const result = await api.getGitAheadBehind('/home/user/project');

        expect(invoke).toHaveBeenCalledWith('get_git_ahead_behind', {
          path: '/home/user/project',
        });
        expect(result).toEqual([3, 1]);
      });
    });

    describe('getGitRemoteUrl', () => {
      it('should return null when no remote', async () => {
        vi.mocked(invoke).mockResolvedValue(null);

        const result = await api.getGitRemoteUrl('/home/user/project');

        expect(invoke).toHaveBeenCalledWith('get_git_remote_url', {
          path: '/home/user/project',
        });
        expect(result).toBeNull();
      });
    });
  });

  // ==================== CONFIG API TESTS ====================

  describe('Config API', () => {
    describe('getConfig', () => {
      it('should call get_config and return AppConfig', async () => {
        const mockConfig = {
          version: '0.4.3',
          platform: {
            os_override: 'auto',
            terminal: { mode: 'auto', custom_args: [] },
            browser: { mode: 'auto', custom_args: [] },
            file_manager: { mode: 'auto', custom_args: [] },
            text_editor: { mode: 'auto', custom_args: [] },
            environment: {},
          },
          backup: {
            auto_backup_enabled: false,
            auto_backup_interval: 24,
            cleanup_old_backups: false,
            retention_days: 30,
          },
          ui: {
            theme: 'dark',
            language: 'es',
            confirm_delete: true,
            show_welcome: true,
          },
          advanced: {
            log_level: 'info',
            enable_analytics: true,
            enable_auto_update: true,
          },
          shortcuts: { enabled: true, shortcuts: {} },
        };

        vi.mocked(invoke).mockResolvedValue(mockConfig);

        const result = await api.getConfig();

        expect(invoke).toHaveBeenCalledWith('get_config');
        expect(result.version).toBe('0.4.3');
        expect(result.ui.theme).toBe('dark');
      });
    });

    describe('updateConfig', () => {
      it('should call update_config with config object', async () => {
        const config = {
          version: '0.4.3',
          platform: {
            os_override: 'auto' as const,
            terminal: { mode: 'auto' as const, custom_args: [] as string[] },
            browser: { mode: 'auto' as const, custom_args: [] as string[] },
            file_manager: {
              mode: 'auto' as const,
              custom_args: [] as string[],
            },
            text_editor: { mode: 'auto' as const, custom_args: [] as string[] },
            environment: {},
          },
          backup: {
            auto_backup_enabled: false,
            auto_backup_interval: 24,
            cleanup_old_backups: false,
            retention_days: 30,
          },
          ui: {
            theme: 'light' as const,
            language: 'es',
            confirm_delete: true,
            show_welcome: true,
          },
          advanced: {
            log_level: 'info' as const,
            enable_analytics: true,
            enable_auto_update: true,
          },
          shortcuts: { enabled: true, shortcuts: {} },
        };

        vi.mocked(invoke).mockResolvedValue(undefined);

        await api.updateConfig(config);

        expect(invoke).toHaveBeenCalledWith('update_config', { config });
      });
    });

    describe('resetConfig', () => {
      it('should call reset_config and return default config', async () => {
        const mockDefault = { version: '0.4.3', ui: { theme: 'auto' } };

        vi.mocked(invoke).mockResolvedValue(mockDefault);

        const result = await api.resetConfig();

        expect(invoke).toHaveBeenCalledWith('reset_config');
        expect(result).toEqual(mockDefault);
      });
    });

    describe('detectPrograms', () => {
      it('should call detect_programs and return detected programs', async () => {
        const mockPrograms = {
          terminals: [
            { name: 'kitty', path: '/usr/bin/kitty', is_default: true },
          ],
          browsers: [
            { name: 'firefox', path: '/usr/bin/firefox', is_default: true },
          ],
          file_managers: [
            { name: 'dolphin', path: '/usr/bin/dolphin', is_default: true },
          ],
          text_editors: [
            { name: 'code', path: '/usr/bin/code', is_default: false },
          ],
        };

        vi.mocked(invoke).mockResolvedValue(mockPrograms);

        const result = await api.detectPrograms();

        expect(invoke).toHaveBeenCalledWith('detect_programs');
        expect(result.terminals).toHaveLength(1);
        expect(result.terminals[0].name).toBe('kitty');
      });
    });
  });

  // ==================== GROUPS API TESTS ====================

  describe('Groups API', () => {
    describe('getRootProjects', () => {
      it('should call get_root_projects without parameters', async () => {
        const mockProjects = [
          {
            id: 1,
            name: 'Root Project',
            description: 'Root',
            local_path: '/root',
            created_at: '2024-01-01T00:00:00Z',
            updated_at: '2024-01-01T00:00:00Z',
          },
        ];

        vi.mocked(invoke).mockResolvedValue(mockProjects);

        const result = await api.getRootProjects();

        expect(invoke).toHaveBeenCalledWith('get_root_projects');
        expect(result).toEqual(mockProjects);
      });
    });

    describe('getSubprojects', () => {
      it('should call get_subprojects with parentId', async () => {
        const mockSubprojects = [
          {
            id: 2,
            name: 'Sub Project',
            description: 'Sub',
            local_path: '/sub',
            parent_id: 1,
            created_at: '2024-01-01T00:00:00Z',
            updated_at: '2024-01-01T00:00:00Z',
          },
        ];

        vi.mocked(invoke).mockResolvedValue(mockSubprojects);

        const result = await api.getSubprojects(1);

        expect(invoke).toHaveBeenCalledWith('get_subprojects', { parentId: 1 });
        expect(result).toEqual(mockSubprojects);
      });
    });

    describe('assignProjectToGroup', () => {
      it('should call assign_project_to_group with childId and parentId', async () => {
        vi.mocked(invoke).mockResolvedValue(undefined);

        await api.assignProjectToGroup(2, 1);

        expect(invoke).toHaveBeenCalledWith('assign_project_to_group', {
          childId: 2,
          parentId: 1,
        });
      });
    });

    describe('countSubprojects', () => {
      it('should call count_subprojects and return number', async () => {
        vi.mocked(invoke).mockResolvedValue(5);

        const result = await api.countSubprojects(1);

        expect(invoke).toHaveBeenCalledWith('count_subprojects', {
          parentId: 1,
        });
        expect(result).toBe(5);
      });
    });
  });

  // ==================== TIME TRACKING & WORK SESSION API TESTS ====================

  describe('Time Tracking & Work Session API', () => {
    describe('initTracking', () => {
      it('should call init_tracking with projectId', async () => {
        vi.mocked(invoke).mockResolvedValue('OK');

        const result = await api.initTracking(1);

        expect(invoke).toHaveBeenCalledWith('init_tracking', { projectId: 1 });
        expect(result).toBe('OK');
      });
    });

    describe('getTimeStats', () => {
      it('should return complete TimeStats structure', async () => {
        const mockStats = {
          total_seconds: 3600,
          session_count: 5,
          avg_session_seconds: 720,
          longest_session_seconds: 1200,
          today_seconds: 900,
          week_seconds: 2400,
        };

        vi.mocked(invoke).mockResolvedValue(mockStats);

        const result = await api.getTimeStats(1);

        expect(invoke).toHaveBeenCalledWith('get_time_stats', { projectId: 1 });
        expect(result).toEqual(mockStats);
        expect(result.total_seconds).toBe(3600);
        expect(result.session_count).toBe(5);
      });
    });

    describe('startWorkSession', () => {
      it('should return WorkSessionResponse', async () => {
        const mockResponse = {
          session_id: 42,
          project_id: 1,
          project_name: 'Test Project',
          previous_session_stopped: true,
          tracking_initialized: false,
        };

        vi.mocked(invoke).mockResolvedValue(mockResponse);

        const result = await api.startWorkSession(1);

        expect(invoke).toHaveBeenCalledWith('start_work_session', {
          projectId: 1,
        });
        expect(result.session_id).toBe(42);
        expect(result.previous_session_stopped).toBe(true);
      });
    });

    describe('stopWorkSession', () => {
      it('should return elapsed seconds or null', async () => {
        vi.mocked(invoke).mockResolvedValue(1800);

        const result = await api.stopWorkSession();

        expect(invoke).toHaveBeenCalledWith('stop_work_session');
        expect(result).toBe(1800);
      });
    });

    describe('getWorkSessionStatus', () => {
      it('should return TrackingStatusResponse', async () => {
        const mockStatus = {
          is_tracking: true,
          project_id: 1,
          project_path: '/home/user/project',
          elapsed_seconds: 600,
        };

        vi.mocked(invoke).mockResolvedValue(mockStatus);

        const result = await api.getWorkSessionStatus();

        expect(invoke).toHaveBeenCalledWith('get_work_session_status');
        expect(result.is_tracking).toBe(true);
        expect(result.elapsed_seconds).toBe(600);
      });
    });
  });

  // ==================== ERROR HANDLING TESTS ====================

  describe('Error Handling', () => {
    it('should propagate error when createProject fails', async () => {
      vi.mocked(invoke).mockRejectedValue(new Error('Database error'));

      await expect(
        api.createProject({
          name: 'Test',
          description: 'Test',
          local_path: '/test',
        })
      ).rejects.toThrow('Database error');
    });

    it('should propagate error when getProject fails', async () => {
      vi.mocked(invoke).mockRejectedValue(new Error('Not found'));

      await expect(api.getProject(999)).rejects.toThrow('Not found');
    });

    it('should propagate error when deleteProject fails', async () => {
      vi.mocked(invoke).mockRejectedValue(new Error('Permission denied'));

      await expect(api.deleteProject(1)).rejects.toThrow('Permission denied');
    });
  });
});
