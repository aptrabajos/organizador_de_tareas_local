import type { Project, ProjectTodo, JournalEntry } from './project';

export interface DashboardTodo extends ProjectTodo {
  project_name: string;
}

export interface DashboardJournalEntry extends JournalEntry {
  project_name: string;
}

export interface DashboardData {
  recent_projects: Project[];
  pending_todos: DashboardTodo[];
  recent_journal_entries: DashboardJournalEntry[];
}
