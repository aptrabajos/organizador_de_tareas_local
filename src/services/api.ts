import { invoke } from '@tauri-apps/api/core';
import type {
  Project,
  CreateProjectDTO,
  UpdateProjectDTO,
} from '../types/project';

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

export async function syncProject(
  sourcePath: string,
  destinationPath: string
): Promise<string> {
  return await invoke('sync_project', { sourcePath, destinationPath });
}
