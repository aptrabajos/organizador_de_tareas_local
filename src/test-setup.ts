import { afterEach, vi } from 'vitest';
import { cleanup } from '@solidjs/testing-library';

// Mock de @tauri-apps/api/core
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn((cmd: string) => {
    // Mocks para time tracking
    if (cmd === 'check_tracking_config') {
      return Promise.resolve(false);
    }
    if (cmd === 'get_time_stats') {
      return Promise.resolve({
        total_seconds: 0,
        session_count: 0,
        avg_session_seconds: 0,
        longest_session_seconds: 0,
        today_seconds: 0,
        week_seconds: 0,
      });
    }
    if (cmd === 'init_tracking') {
      return Promise.resolve('OK');
    }
    if (cmd === 'get_tracking_sessions') {
      return Promise.resolve([]);
    }
    if (cmd === 'get_tracking_status') {
      return Promise.resolve({
        is_tracking: false,
        elapsed_seconds: 0,
      });
    }
    // Mocks para git
    if (cmd === 'get_git_branch') {
      return Promise.resolve('main');
    }
    if (cmd === 'get_git_status') {
      return Promise.resolve('clean');
    }
    if (cmd === 'get_git_file_count') {
      return Promise.resolve({ tracked: 10, modified: 0, staged: 0 });
    }
    if (cmd === 'get_git_ahead_behind') {
      return Promise.resolve([0, 0]);
    }
    // Mocks para proyectos
    if (cmd === 'count_subprojects') {
      return Promise.resolve(0);
    }
    if (cmd === 'track_project_open') {
      return Promise.resolve();
    }
    if (cmd === 'start_work_session') {
      return Promise.resolve({
        session_id: 1,
        project_id: 1,
        project_name: 'Test Project',
        previous_session_stopped: false,
        tracking_initialized: false,
      });
    }
    if (cmd === 'stop_work_session') {
      return Promise.resolve(null);
    }
    if (cmd === 'get_work_session_status') {
      return Promise.resolve({
        is_tracking: false,
        project_id: null,
        project_path: null,
        elapsed_seconds: 0,
      });
    }
    if (cmd === 'get_root_projects') {
      return Promise.resolve([]);
    }
    // Default
    return Promise.resolve(undefined);
  }),
}));

// Mock de @tauri-apps/plugin-dialog
vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(() => Promise.resolve(null)),
  save: vi.fn(() => Promise.resolve(null)),
}));

// Mock de @tauri-apps/plugin-fs
vi.mock('@tauri-apps/plugin-fs', () => ({
  writeTextFile: vi.fn(() => Promise.resolve()),
  readTextFile: vi.fn(() => Promise.resolve('')),
}));

// Cleanup después de cada test
afterEach(() => {
  cleanup();
});
