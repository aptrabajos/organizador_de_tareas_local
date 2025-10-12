import { invoke } from '@tauri-apps/api/core';
import type {
  Project,
  ProjectLink,
  ProjectActivity,
  ProjectStats,
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
