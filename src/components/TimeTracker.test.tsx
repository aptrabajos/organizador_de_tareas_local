import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@solidjs/testing-library';
import TimeTracker from './TimeTracker';

// Mock de las funciones de API
vi.mock('../services/api', () => ({
  getTimeStats: vi.fn(),
  checkTrackingConfig: vi.fn(),
  initTracking: vi.fn(),
}));

import { getTimeStats, checkTrackingConfig, initTracking } from '../services/api';

describe('TimeTracker', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('shows loading state initially', () => {
    vi.mocked(checkTrackingConfig).mockImplementation(() => new Promise(() => {}));

    render(() => (
      <TimeTracker projectId={1} projectPath="/test/path" />
    ));

    expect(screen.getByText('Cargando...')).toBeTruthy();
  });

  it('shows setup button for untracked projects', async () => {
    vi.mocked(checkTrackingConfig).mockResolvedValue(false);

    render(() => (
      <TimeTracker projectId={1} projectPath="/test/path" />
    ));

    // Esperar a que cargue
    await vi.waitFor(() => {
      expect(screen.getByText('Activar Time Tracking')).toBeTruthy();
    });
  });

  it('shows time stats when tracking is enabled', async () => {
    vi.mocked(checkTrackingConfig).mockResolvedValue(true);
    vi.mocked(getTimeStats).mockResolvedValue({
      total_seconds: 7200, // 2 horas
      session_count: 5,
      avg_session_seconds: 1440, // 24 minutos
      longest_session_seconds: 3600,
      today_seconds: 1800, // 30 minutos
      week_seconds: 5400, // 1.5 horas
    });

    render(() => (
      <TimeTracker projectId={1} projectPath="/test/path" />
    ));

    // Esperar a que cargue
    await vi.waitFor(() => {
      expect(screen.getByText('2h')).toBeTruthy(); // Total
    });

    expect(screen.getByText('30m')).toBeTruthy(); // Hoy
    expect(screen.getByText('5')).toBeTruthy(); // Sesiones
  });

  it('displays time in human-readable format', async () => {
    vi.mocked(checkTrackingConfig).mockResolvedValue(true);
    vi.mocked(getTimeStats).mockResolvedValue({
      total_seconds: 3661, // 1h 1m 1s
      session_count: 1,
      avg_session_seconds: 3661,
      longest_session_seconds: 3661,
      today_seconds: 65, // 1m 5s
      week_seconds: 3661,
    });

    render(() => (
      <TimeTracker projectId={1} projectPath="/test/path" />
    ));

    await vi.waitFor(() => {
      // Puede haber múltiples elementos con el mismo texto
      const elements = screen.getAllByText('1h 1m');
      expect(elements.length).toBeGreaterThan(0);
    });
  });

  it('shows compact view when compact prop is true', async () => {
    vi.mocked(checkTrackingConfig).mockResolvedValue(true);
    vi.mocked(getTimeStats).mockResolvedValue({
      total_seconds: 3600,
      session_count: 2,
      avg_session_seconds: 1800,
      longest_session_seconds: 2400,
      today_seconds: 600,
      week_seconds: 3600,
    });

    render(() => (
      <TimeTracker projectId={1} projectPath="/test/path" compact={true} />
    ));

    await vi.waitFor(() => {
      // En modo compacto solo debería mostrar el total
      expect(screen.getByText('1h')).toBeTruthy();
    });

    // No debería mostrar los detalles expandidos
    expect(screen.queryByText('Time Tracking')).toBeNull();
  });

  it('shows track button in compact mode for untracked projects', async () => {
    vi.mocked(checkTrackingConfig).mockResolvedValue(false);

    render(() => (
      <TimeTracker projectId={1} projectPath="/test/path" compact={true} />
    ));

    await vi.waitFor(() => {
      expect(screen.getByText('Track')).toBeTruthy();
    });
  });

  it('calls initTracking on setup button click', async () => {
    vi.mocked(checkTrackingConfig).mockResolvedValue(false);
    vi.mocked(initTracking).mockResolvedValue('OK');

    render(() => (
      <TimeTracker projectId={1} projectPath="/test/path" />
    ));

    await vi.waitFor(() => {
      expect(screen.getByText('Activar Time Tracking')).toBeTruthy();
    });

    const button = screen.getByText('Activar Time Tracking');
    button.click();

    await vi.waitFor(() => {
      expect(initTracking).toHaveBeenCalledWith(1);
    });
  });
});
