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
  // Group/Hierarchy fields (v0.4.0)
  parent_id?: number | null;
  group_color?: string;
  group_icon?: string;
  is_group_expanded?: boolean;
  // Computed fields (frontend only)
  subproject_count?: number;
  children?: Project[];
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

// Proyecto con sus hijos (v0.4.0)
export interface ProjectWithChildren {
  project: Project;
  children: Project[];
  subproject_count: number;
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
  // Group fields (v0.4.0)
  parent_id?: number | null;
  group_color?: string;
  group_icon?: string;
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
  // Group fields (v0.4.0)
  parent_id?: number | null;
  group_color?: string;
  group_icon?: string;
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

// ==================== TIME TRACKING ====================

export interface TimeTrackingSession {
  id: number;
  project_id: number;
  started_at: string;
  ended_at?: string;
  duration_seconds?: number;
  source: string;
}

export interface TimeStats {
  total_seconds: number;
  session_count: number;
  avg_session_seconds: number;
  longest_session_seconds: number;
  today_seconds: number;
  week_seconds: number;
}

export interface TrackingStatusResponse {
  is_tracking: boolean;
  project_id?: number;
  project_path?: string;
  elapsed_seconds: number;
}

export interface GestorConfig {
  project_id: number;
  project_name: string;
  created_at: string;
  tracking_enabled: boolean;
}
