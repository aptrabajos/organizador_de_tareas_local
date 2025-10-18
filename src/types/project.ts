export interface ProjectLink {
  id: number;
  project_id: number;
  type:
    | 'repository'
    | 'documentation'
    | 'staging'
    | 'production'
    | 'design'
    | 'api'
    | 'other';
  title: string;
  url: string;
  created_at: string;
}

export interface Project {
  id: number;
  name: string;
  description: string;
  local_path: string;
  documentation_url?: string;
  ai_documentation_url?: string;
  drive_link?: string;
  notes?: string;
  image_data?: string;
  links?: ProjectLink[];
  created_at: string;
  updated_at: string;
  // Analytics fields
  last_opened_at?: string;
  opened_count?: number;
  total_time_seconds?: number;
  // Quick Start & Context fields
  status?: string; // 'activo' | 'pausado' | 'completado' | 'archivado'
  status_changed_at?: string;
  is_pinned?: boolean;
  pinned_order?: number;
}

export interface ProjectActivity {
  id: number;
  project_id: number;
  activity_type: string;
  description?: string;
  duration_seconds?: number;
  created_at: string;
}

export interface ProjectStats {
  total_projects: number;
  active_today: number;
  total_time_hours: number;
  most_active_project?: string;
  recent_activities: ProjectActivity[];
}

export interface CreateProjectDTO {
  name: string;
  description: string;
  local_path: string;
  documentation_url?: string;
  ai_documentation_url?: string;
  drive_link?: string;
  notes?: string;
  image_data?: string;
}

export interface UpdateProjectDTO {
  name?: string;
  description?: string;
  local_path?: string;
  documentation_url?: string;
  ai_documentation_url?: string;
  drive_link?: string;
  notes?: string;
  image_data?: string;
}

export interface ProjectAttachment {
  id: number;
  project_id: number;
  filename: string;
  file_data: string;
  file_size: number;
  mime_type: string;
  created_at: string;
}

export interface CreateAttachmentDTO {
  project_id: number;
  filename: string;
  file_data: string;
  file_size: number;
  mime_type: string;
}

export interface GitCommit {
  hash: string;
  author: string;
  date: string;
  message: string;
}

// ==================== PROJECT JOURNAL ====================

export interface JournalEntry {
  id: number;
  project_id: number;
  content: string;
  tags?: string; // JSON string array
  created_at: string;
  updated_at: string;
}

export interface CreateJournalEntryDTO {
  project_id: number;
  content: string;
  tags?: string;
}

export interface UpdateJournalEntryDTO {
  content?: string;
  tags?: string;
}

// ==================== PROJECT TODOS ====================

export interface ProjectTodo {
  id: number;
  project_id: number;
  content: string;
  is_completed: boolean;
  created_at: string;
  completed_at?: string;
}

export interface CreateTodoDTO {
  project_id: number;
  content: string;
}

export interface UpdateTodoDTO {
  content?: string;
  is_completed?: boolean;
}
