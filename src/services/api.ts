import { invoke } from '@tauri-apps/api/core';
import type {
  Project,
  ProjectLink,
  ProjectActivity,
  ProjectStats,
  CreateProjectDTO,
  UpdateProjectDTO,
  ProjectAttachment,
  CreateAttachmentDTO,
  JournalEntry,
  CreateJournalEntryDTO,
  UpdateJournalEntryDTO,
  ProjectTodo,
  CreateTodoDTO,
  UpdateTodoDTO,
  TrashItem,
} from '../types/project';
import type {
  AppConfig,
  DetectedPrograms,
  BackupResult,
  BackupEntry,
  RestoreResult,
} from '../types/config';
// Los tipos de git viven en types/git.ts, que es su dominio. `GitCommit` estaba
// definido TAMBIÉN en types/project.ts y cada consumidor importaba uno distinto:
// eran estructuralmente compatibles, así que TS no se quejaba, pero el día que
// uno cambiara el error iba a aparecer lejos del cambio.
import type { GitCommit, GitFileCount } from '../types/git';
import type { DashboardData } from '../types/dashboard';

export async function createProject(
  project: CreateProjectDTO
): Promise<Project> {
  return await invoke('create_project', { project });
}

export async function getAllProjects(): Promise<Project[]> {
  return await invoke('get_all_projects');
}

export async function getProject(id: number): Promise<Project> {
  return await invoke('get_project', { id });
}

export async function updateProject(
  id: number,
  updates: UpdateProjectDTO
): Promise<Project> {
  return await invoke('update_project', { id, updates });
}

export async function deleteProject(id: number): Promise<void> {
  await invoke('delete_project', { id });
}

export async function restoreProject(id: number): Promise<void> {
  await invoke('restore_project', { id });
}

export async function listTrash(): Promise<TrashItem[]> {
  return await invoke('list_trash');
}

export async function purgeProject(id: number): Promise<void> {
  await invoke('purge_project', { id });
}

export async function emptyTrash(): Promise<void> {
  await invoke('empty_trash');
}

export async function searchProjects(query: string): Promise<Project[]> {
  return await invoke('search_projects', { query });
}

export async function openTerminal(path: string): Promise<void> {
  await invoke('open_terminal', { path });
}

export async function openUrl(url: string): Promise<void> {
  await invoke('open_url', { url });
}

export interface BackupData {
  content: string;
  path: string;
  filename: string;
}

export async function createProjectBackup(
  projectId: number
): Promise<BackupData> {
  return await invoke('create_project_backup', { projectId });
}

export async function syncProjectToBackup(
  sourcePath: string,
  projectName: string
): Promise<string> {
  return await invoke('sync_project_to_backup', { sourcePath, projectName });
}

// Funciones para manejar enlaces de proyectos
export interface CreateLinkDTO {
  project_id: number;
  link_type:
    | 'repository'
    | 'documentation'
    | 'staging'
    | 'production'
    | 'design'
    | 'api'
    | 'other';
  title: string;
  url: string;
}

export interface UpdateLinkDTO {
  link_type?:
    | 'repository'
    | 'documentation'
    | 'staging'
    | 'production'
    | 'design'
    | 'api'
    | 'other';
  title?: string;
  url?: string;
}

export async function createProjectLink(
  link: CreateLinkDTO
): Promise<ProjectLink> {
  return await invoke('create_project_link', { link });
}

export async function getProjectLinks(
  projectId: number
): Promise<ProjectLink[]> {
  return await invoke('get_project_links', { projectId });
}

export async function updateProjectLink(
  id: number,
  link: UpdateLinkDTO
): Promise<ProjectLink> {
  return await invoke('update_project_link', { id, link });
}

export async function deleteProjectLink(id: number): Promise<void> {
  await invoke('delete_project_link', { id });
}

// Funciones para Analytics y Tracking
export async function trackProjectOpen(projectId: number): Promise<void> {
  await invoke('track_project_open', { projectId });
}

export async function addProjectTime(
  projectId: number,
  seconds: number
): Promise<void> {
  await invoke('add_project_time', { projectId, seconds });
}

export async function getProjectStats(): Promise<ProjectStats> {
  return await invoke('get_project_stats');
}

export async function getProjectActivities(
  projectId: number,
  limit: number = 20
): Promise<ProjectActivity[]> {
  return await invoke('get_project_activities', { projectId, limit });
}

// Funciones para manejar archivos adjuntos
export async function addAttachment(
  attachment: CreateAttachmentDTO
): Promise<ProjectAttachment> {
  return await invoke('add_attachment', { attachment });
}

export async function getAttachments(
  projectId: number
): Promise<ProjectAttachment[]> {
  return await invoke('get_attachments', { projectId });
}

export async function deleteAttachment(id: number): Promise<void> {
  await invoke('delete_attachment', { id });
}

// Funciones para obtener información de Git
export async function getGitBranch(path: string): Promise<string> {
  return await invoke('get_git_branch', { path });
}

export async function getGitStatus(path: string): Promise<string> {
  return await invoke('get_git_status', { path });
}

export async function getRecentCommits(
  path: string,
  limit: number = 5
): Promise<GitCommit[]> {
  return await invoke('get_recent_commits', { path, limit });
}

// Funciones Git mejoradas
export async function getGitFileCount(
  path: string
): Promise<GitFileCount> {
  return await invoke('get_git_file_count', { path });
}

export async function getGitModifiedFiles(path: string): Promise<string[]> {
  return await invoke('get_git_modified_files', { path });
}

export async function gitAdd(path: string, files: string[]): Promise<string> {
  return await invoke('git_add', { path, files });
}

export async function gitCommit(
  path: string,
  message: string
): Promise<string> {
  return await invoke('git_commit', { path, message });
}

export async function gitPush(path: string): Promise<string> {
  return await invoke('git_push', { path });
}

export async function gitPull(path: string): Promise<string> {
  return await invoke('git_pull', { path });
}

export async function getGitRemoteUrl(path: string): Promise<string | null> {
  return await invoke('get_git_remote_url', { path });
}

export async function getGitAheadBehind(
  path: string
): Promise<[number, number]> {
  return await invoke('get_git_ahead_behind', { path });
}

// ==================== FUNCIONES PARA PROJECT JOURNAL ====================

export async function createJournalEntry(
  entry: CreateJournalEntryDTO
): Promise<JournalEntry> {
  return await invoke('create_journal_entry', { entry });
}

export async function getJournalEntries(
  projectId: number
): Promise<JournalEntry[]> {
  return await invoke('get_journal_entries', { projectId });
}

export async function updateJournalEntry(
  id: number,
  updates: UpdateJournalEntryDTO
): Promise<JournalEntry> {
  return await invoke('update_journal_entry', { id, updates });
}

export async function deleteJournalEntry(id: number): Promise<void> {
  await invoke('delete_journal_entry', { id });
}

// ==================== FUNCIONES PARA PROJECT TODOS ====================

export async function createTodo(todo: CreateTodoDTO): Promise<ProjectTodo> {
  return await invoke('create_todo', { todo });
}

export async function getProjectTodos(
  projectId: number
): Promise<ProjectTodo[]> {
  return await invoke('get_project_todos', { projectId });
}

export async function updateTodo(
  id: number,
  updates: UpdateTodoDTO
): Promise<ProjectTodo> {
  return await invoke('update_todo', { id, updates });
}

export async function deleteTodo(id: number): Promise<void> {
  await invoke('delete_todo', { id });
}

// ==================== FUNCIONES PARA ESTADOS Y FAVORITOS ====================

export async function updateProjectStatus(
  projectId: number,
  status: string
): Promise<void> {
  await invoke('update_project_status', { projectId, status });
}

export async function togglePinProject(projectId: number): Promise<boolean> {
  return await invoke('toggle_pin_project', { projectId });
}

export async function reorderPinnedProjects(
  projectIds: number[]
): Promise<void> {
  await invoke('reorder_pinned_projects', { projectIds });
}

export async function updateProjectOrder(
  projectId: number,
  newOrder: number
): Promise<void> {
  await invoke('update_project_order', { projectId, newOrder });
}

// ==================== FUNCIONES PARA CONFIGURACIÓN ====================

export async function getConfig(): Promise<AppConfig> {
  return await invoke('get_config');
}

export async function updateConfig(config: AppConfig): Promise<void> {
  await invoke('update_config', { config });
}

export async function resetConfig(): Promise<AppConfig> {
  return await invoke('reset_config');
}

export async function detectPrograms(): Promise<DetectedPrograms> {
  return await invoke('detect_programs');
}

export async function openFileManager(path: string): Promise<void> {
  await invoke('open_file_manager', { path });
}

export async function openTextEditor(path: string): Promise<void> {
  await invoke('open_text_editor', { path });
}

export async function selectBackupFolder(): Promise<string | null> {
  return await invoke('select_backup_folder');
}

// ==================== BACKUP DE LA BASE DE DATOS ====================

export async function backupDatabase(): Promise<BackupResult> {
  return await invoke('backup_database');
}

export async function listBackups(): Promise<BackupEntry[]> {
  return await invoke('list_backups');
}

// Restaura la DB viva a partir de un backup. Operación IRREVERSIBLE: reemplaza
// projects.db completo. La app se cierra sola tras una restauración exitosa
// (la conexión SQLite viva queda apuntando al archivo anterior; hay que
// reabrirla para que lea los datos restaurados).
export async function restoreBackup(
  backupPath: string
): Promise<RestoreResult> {
  return await invoke('restore_backup', { backupPath });
}

// ==================== DIÁLOGOS NATIVOS ====================

export async function selectFolder(title?: string): Promise<string | null> {
  return await invoke('select_folder', { title });
}

export async function selectFile(
  title?: string,
  filters?: Array<[string, string[]]>
): Promise<string | null> {
  return await invoke('select_file', { title, filters });
}

export async function selectFiles(
  title?: string,
  filters?: Array<[string, string[]]>
): Promise<string[]> {
  return await invoke('select_files', { title, filters });
}

export async function saveFileDialog(
  title?: string,
  defaultName?: string,
  filters?: Array<[string, string[]]>
): Promise<string | null> {
  return await invoke('save_file_dialog', { title, defaultName, filters });
}

// ==================== FUNCIONES PARA SHORTCUTS ====================

export async function getShortcutsConfig(): Promise<
  import('../types/config').ShortcutsConfig
> {
  return await invoke('get_shortcuts_config');
}

export async function updateShortcutsConfig(
  shortcutsConfig: import('../types/config').ShortcutsConfig
): Promise<void> {
  await invoke('update_shortcuts_config', { shortcutsConfig });
}

// ==================== FUNCIONES PARA GRUPOS DE PROYECTOS (v0.4.0) ====================

export async function getRootProjects(): Promise<Project[]> {
  return await invoke('get_root_projects');
}

export async function getSubprojects(parentId: number): Promise<Project[]> {
  return await invoke('get_subprojects', { parentId });
}

export async function getProjectWithChildren(
  id: number
): Promise<import('../types/project').ProjectWithChildren> {
  return await invoke('get_project_with_children', { id });
}

export async function countSubprojects(parentId: number): Promise<number> {
  return await invoke('count_subprojects', { parentId });
}

export async function assignProjectToGroup(
  childId: number,
  parentId: number | null
): Promise<void> {
  await invoke('assign_project_to_group', { childId, parentId });
}

// ==================== FUNCIONES PARA EXPORTACIÓN PDF (v0.4.1) ====================

export async function exportProjectToPdf(projectId: number): Promise<string> {
  return await invoke('export_project_to_pdf', { projectId });
}

// ==================== FUNCIONES PARA DASHBOARD (v0.5.0) ====================

export async function getDashboardData(): Promise<DashboardData> {
  return await invoke('get_dashboard_data');
}

// ==================== FUNCIONES PARA TIME TRACKING (v0.5.0) ====================

import type {
  TimeTrackingSession,
  TimeStats,
  TrackingStatusResponse,
  GestorConfig,
} from '../types/project';

export async function initTracking(projectId: number): Promise<string> {
  return await invoke('init_tracking', { projectId });
}

export async function getTrackingSessions(
  projectId: number,
  limit?: number
): Promise<TimeTrackingSession[]> {
  return await invoke('get_tracking_sessions', { projectId, limit });
}

export async function getTrackingStatus(): Promise<TrackingStatusResponse> {
  return await invoke('get_tracking_status');
}

export async function startTracking(projectId: number): Promise<number> {
  return await invoke('start_tracking', { projectId });
}

export async function stopTracking(
  sessionId: number,
  durationSeconds: number
): Promise<void> {
  await invoke('stop_tracking', { sessionId, durationSeconds });
}

export async function checkTrackingConfig(path: string): Promise<boolean> {
  return await invoke('check_tracking_config', { path });
}

export async function findTrackingProject(
  path: string
): Promise<GestorConfig | null> {
  return await invoke('find_tracking_project', { path });
}

export async function getTimeStats(projectId: number): Promise<TimeStats> {
  return await invoke('get_time_stats', { projectId });
}

// ==================== WORK SESSION (v0.5.1) ====================

export interface WorkSessionResponse {
  session_id: number;
  project_id: number;
  project_name: string;
  previous_session_stopped: boolean;
  tracking_initialized: boolean;
}

export async function startWorkSession(
  projectId: number
): Promise<WorkSessionResponse> {
  return await invoke('start_work_session', { projectId });
}

export async function stopWorkSession(): Promise<number | null> {
  return await invoke('stop_work_session');
}

export async function getWorkSessionStatus(): Promise<TrackingStatusResponse> {
  return await invoke('get_work_session_status');
}
