import { Component, For, Show, createSignal, createEffect } from 'solid-js';
import toast from 'solid-toast';
import { getErrorMessage } from '../utils/errors';
import { marked } from 'marked';
import DOMPurify from 'dompurify';
import {
  DragDropProvider,
  DragDropSensors,
  SortableProvider,
  createSortable,
  closestCenter,
  type DragEvent,
} from '@thisbeyond/solid-dnd';
import type { Project } from '../types/project';
import EnhancedGitInfo from './EnhancedGitInfo';
import GitCommitModal from './GitCommitModal';
import ProjectJournal from './ProjectJournal';
import TodoList from './TodoList';
import ProjectContext from './ProjectContext';
import GroupCard from './GroupCard';
import TimeTracker from './TimeTracker';
import {
  openUrl,
  createProjectBackup,
  syncProjectToBackup,
  trackProjectOpen,
  togglePinProject,
  updateProjectStatus,
  updateProjectOrder,
  countSubprojects,
  exportProjectToPdf,
  startWorkSession,
} from '../services/api';
import { open } from '@tauri-apps/plugin-dialog';
import { writeTextFile } from '@tauri-apps/plugin-fs';
import { join, normalize, sep } from '@tauri-apps/api/path';

// Configurar marked para soportar GFM y checkboxes
marked.use({
  breaks: true,
  gfm: true,
});

interface ProjectListProps {
  projects: Project[];
  onEdit: (project: Project) => void;
  onDelete: (project: Project) => void;
  onOpenTerminal: (project: Project) => void;
  onProjectsChanged?: () => void; // Callback para refrescar proyectos
  // Props para controlar filtros desde el padre (App.tsx)
  renderFilters?: (filterProps: {
    statusFilter: () => string;
    setStatusFilter: (value: string) => void;
    showPinnedOnly: () => boolean;
    setShowPinnedOnly: (value: boolean) => void;
    filteredCount: number;
    totalCount: number;
  }) => void;
  // Props para sistema de grupos (v0.4.0)
  viewMode?: 'groups' | 'subprojects'; // Modo de vista actual
  onViewGroup?: (project: Project) => void; // Callback para navegar a subproyectos
  searchActive?: boolean; // Indica si hay una búsqueda activa (mostrar todos los resultados)
  currentGroup?: Project | null; // Grupo actual al que se ingresó (para mostrar su tarjeta como cabecera)
}

const ProjectList: Component<ProjectListProps> = (props) => {
  // Estado para controlar el journal modal
  const [journalProjectId, setJournalProjectId] = createSignal<number | null>(
    null
  );

  // Estado para controlar el modal de TODOs
  const [todosProjectId, setTodosProjectId] = createSignal<number | null>(null);

  // Estado para controlar el modal de contexto del proyecto
  const [contextProjectId, setContextProjectId] = createSignal<number | null>(
    null
  );

  // Estado para controlar el modal de commit Git
  const [commitProjectPath, setCommitProjectPath] = createSignal<string | null>(
    null
  );

  // Estado para filtros
  const [statusFilter, setStatusFilter] = createSignal<string>('all');
  const [showPinnedOnly, setShowPinnedOnly] = createSignal(false);

  // Estado para rastrear qué proyectos son grupos (v0.4.0)
  const [projectGroups, setProjectGroups] = createSignal<Set<number>>(
    new Set()
  );

  // Detectar qué proyectos tienen hijos (son grupos)
  createEffect(() => {
    // En búsqueda o vista de grupos, detectar qué proyectos son grupos
    if (props.viewMode === 'groups' || props.searchActive) {
      (async () => {
        const groupIds = new Set<number>();
        for (const project of props.projects) {
          try {
            const count = await countSubprojects(project.id);
            if (count > 0) {
              groupIds.add(project.id);
            }
          } catch (err) {
            console.error('Error contando subproyectos:', err);
          }
        }
        setProjectGroups(groupIds);
      })();
    } else {
      // En vista de subproyectos (sin búsqueda), ninguno es grupo (nivel único)
      setProjectGroups(new Set<number>());
    }
  });

  // Función para filtrar proyectos
  const filteredProjects = () => {
    let filtered = [...props.projects];

    // Filtrar por estado
    if (statusFilter() !== 'all') {
      filtered = filtered.filter((p) => p.status === statusFilter());
    }

    // Filtrar por pinned
    if (showPinnedOnly()) {
      filtered = filtered.filter((p) => p.is_pinned === true);
    }

    return filtered;
  };

  // Exponer filtros al padre (App.tsx) para renderizar en el header
  createEffect(() => {
    if (props.renderFilters) {
      props.renderFilters({
        statusFilter,
        setStatusFilter,
        showPinnedOnly,
        setShowPinnedOnly,
        filteredCount: filteredProjects().length,
        totalCount: props.projects.length,
      });
    }
  });

  // Función helper para renderizar markdown de forma segura
  const renderMarkdown = (markdown: string): string => {
    let html = marked.parse(markdown) as string;

    // Estilizar checkboxes
    html = html.replace(
      /<input type="checkbox"(.*?)>/g,
      '<input type="checkbox" class="mr-1 h-3 w-3 cursor-pointer" onclick="return false;"$1>'
    );

    // Sanitizar HTML
    return DOMPurify.sanitize(html);
  };

  const handleOpenTerminal = async (project: Project) => {
    try {
      // Registrar apertura del proyecto para analytics
      await trackProjectOpen(project.id);

      // Iniciar sesión de trabajo (tracking automático de tiempo)
      const session = await startWorkSession(project.id);
      console.log(`🕒 Sesión de trabajo iniciada: ${session.project_name}`);

      if (session.tracking_initialized) {
        toast.success(`📁 Tracking activado para ${project.name}`, {
          duration: 2000,
        });
      }
      if (session.previous_session_stopped) {
        toast('⏱️ Sesión anterior guardada', { duration: 1500, icon: '✓' });
      }

      // Abrir terminal
      await props.onOpenTerminal(project);
    } catch (error) {
      console.error('Error al abrir proyecto:', error);
      // Aunque falle el tracking, abrimos el terminal igual
      await props.onOpenTerminal(project);
    }
  };

  const handleTogglePin = async (project: Project) => {
    try {
      const newPinned = await togglePinProject(project.id);
      const message = newPinned
        ? `📌 ${project.name} marcado como favorito`
        : `📌 ${project.name} desmarcado como favorito`;
      toast.success(message, { duration: 2000 });

      // Refrescar lista de proyectos de forma suave
      if (props.onProjectsChanged) {
        props.onProjectsChanged();
      }
    } catch (error) {
      console.error('Error al cambiar pin:', error);
      toast.error('Error al cambiar favorito');
    }
  };

  const handleChangeStatus = async (project: Project, newStatus: string) => {
    try {
      await updateProjectStatus(project.id, newStatus);
      toast.success(`Estado cambiado a: ${newStatus}`, { duration: 2000 });

      // Refrescar lista de proyectos de forma suave
      if (props.onProjectsChanged) {
        props.onProjectsChanged();
      }
    } catch (error) {
      console.error('Error al cambiar estado:', error);
      toast.error('Error al cambiar estado');
    }
  };

  // v0.4.0 - Handler para cuando se suelta un proyecto sobre un GroupCard
  const handleDropOnGroup = async (projectId: number, groupId: number) => {
    const project = filteredProjects().find((p) => p.id === projectId);
    const group = filteredProjects().find((p) => p.id === groupId);

    if (!project || !group) return;

    try {
      // Usar el comando assign_project_to_group de la API
      const { assignProjectToGroup } = await import('../services/api');
      await assignProjectToGroup(projectId, groupId);

      toast.success(`✅ "${project.name}" agregado al grupo "${group.name}"`, {
        duration: 3000,
      });

      // Actualizar el set de grupos inmediatamente para feedback visual
      const currentGroups = new Set(projectGroups());
      currentGroups.add(groupId);
      setProjectGroups(currentGroups);

      // Refrescar lista de proyectos (esto recargará desde la BD)
      if (props.onProjectsChanged) {
        props.onProjectsChanged();
      }
    } catch (error) {
      console.error('Error al asignar proyecto a grupo:', error);
      // Surfacea el mensaje del backend (ej. validación de ciclo en español)
      toast.error(getErrorMessage(error));
      throw error;
    }
  };

  // v0.4.0 - Handler para sacar un proyecto de un grupo
  const handleRemoveFromGroup = async (project: Project) => {
    try {
      // Usar el comando assign_project_to_group con parent_id = null
      const { assignProjectToGroup } = await import('../services/api');
      await assignProjectToGroup(project.id, null);

      toast.success(`🔓 "${project.name}" removido del grupo`, {
        duration: 3000,
      });

      // Refrescar lista de proyectos
      if (props.onProjectsChanged) {
        props.onProjectsChanged();
      }
    } catch (error) {
      console.error('Error al remover proyecto del grupo:', error);
      toast.error('Error al remover proyecto del grupo');
    }
  };

  const handleDragEnd = async (event: DragEvent) => {
    const { draggable, droppable } = event;
    if (!droppable) return;

    const activeId = Number(draggable.id);
    const overId = Number(droppable.id);

    if (activeId === overId) return;

    const projects = filteredProjects();
    const draggedProject = projects.find((p) => p.id === activeId);
    const targetProject = projects.find((p) => p.id === overId);

    if (!draggedProject || !targetProject) return;

    // v0.4.0 - Detectar si el target es un grupo (tiene hijos)
    const isTargetGroup = projectGroups().has(overId);

    if (isTargetGroup && props.viewMode === 'groups') {
      // Caso: Arrastrar proyecto sobre un GroupCard para convertirlo en subproyecto
      try {
        await handleDropOnGroup(activeId, overId);
        return;
      } catch (error) {
        console.error('Error al asignar proyecto a grupo:', error);
        toast.error('Error al asignar proyecto al grupo');
        return;
      }
    }

    // Caso normal: Reordenamiento de proyectos en la lista
    const oldIndex = projects.findIndex((p) => p.id === activeId);
    const newIndex = projects.findIndex((p) => p.id === overId);

    if (oldIndex === -1 || newIndex === -1) return;

    try {
      // Reordenar array localmente
      const reordered = [...projects];
      const [removed] = reordered.splice(oldIndex, 1);
      reordered.splice(newIndex, 0, removed);

      // Actualizar display_order en BD para todos los proyectos afectados
      const updates = reordered.map((p, index) =>
        updateProjectOrder(p.id, index)
      );

      await Promise.all(updates);

      toast.success('Orden actualizado', { duration: 2000 });

      // Refrescar lista de proyectos
      if (props.onProjectsChanged) {
        props.onProjectsChanged();
      }
    } catch (error) {
      console.error('Error al actualizar orden:', error);
      toast.error('Error al actualizar orden');
    }
  };

  const handleBackup = async (project: Project) => {
    try {
      // Pedir al usuario que seleccione la carpeta destino
      const destinationFolder = await open({
        directory: true,
        multiple: false,
        title: 'Selecciona carpeta donde guardar el backup',
        defaultPath: project.local_path, // Sugerencia: carpeta del proyecto
      });

      if (!destinationFolder) {
        return; // Usuario canceló
      }

      const toastId = toast.loading('Creando backup...');
      try {
        const backupData = await createProjectBackup(project.id);
        // Construir ruta completa con la carpeta elegida y validar, como defensa
        // adicional a la sanitización del backend, que el resultado no escape la
        // carpeta destino elegida por el usuario (path traversal).
        const normalizedDestination = await normalize(destinationFolder);
        const fullPath = await normalize(
          await join(destinationFolder, backupData.filename)
        );
        const separator = sep();
        const destinationWithSep = normalizedDestination.endsWith(separator)
          ? normalizedDestination
          : `${normalizedDestination}${separator}`;
        if (
          fullPath !== normalizedDestination &&
          !fullPath.startsWith(destinationWithSep)
        ) {
          throw new Error(
            'El nombre de archivo del backup intenta escribir fuera de la carpeta seleccionada'
          );
        }
        await writeTextFile(fullPath, backupData.content);
        toast.success(`✅ Backup creado en:\n${fullPath}`, {
          id: toastId,
          duration: 5000,
        });
      } catch (error) {
        console.error('Error al crear backup:', error);
        toast.error(`Error al crear backup: ${error}`, { id: toastId });
      }
    } catch (error) {
      console.error('Error abriendo selector:', error);
      toast.error(`Error al abrir selector de carpetas: ${error}`);
    }
  };

  const handleExportPdf = async (project: Project) => {
    const toastId = toast.loading(`📄 Generando PDF de "${project.name}"...`);
    try {
      const pdfPath = await exportProjectToPdf(project.id);
      toast.success(`✅ PDF exportado exitosamente:\n${pdfPath}`, {
        id: toastId,
        duration: 6000,
      });
    } catch (error) {
      console.error('Error al exportar PDF:', error);
      toast.error(`❌ Error al exportar PDF: ${error}`, { id: toastId });
    }
  };

  const handleSync = async (project: Project) => {
    try {
      // Sincroniza los archivos del proyecto a la carpeta de backup configurada (rsync)
      const toastId = toast.loading(
        `Sincronizando ${project.name} con rsync...`
      );

      try {
        const result = await syncProjectToBackup(
          project.local_path,
          project.name
        );
        toast.success(`✅ ${result}`, {
          id: toastId,
          duration: 5000,
        });
      } catch (error) {
        console.error('Error al sincronizar con rsync:', error);
        toast.error(`Error al sincronizar: ${error}`, { id: toastId });
      }
    } catch (error) {
      console.error('Error:', error);
      toast.error(`Error: ${error}`);
    }
  };

  // Render de una ProjectCard reutilizable. Comparte la clausura del componente,
  // así reutiliza todos los handlers/señales sin pasar props.
  //  - variant 'grid' (default): tarjeta de un hijo dentro de la grilla ordenable
  //    (drag & drop + botón "Sacar del grupo" según el contexto).
  //  - variant 'self-group': cabecera del propio grupo, sin drag & drop y sin
  //    el botón "Sacar del grupo".
  const renderProjectCard = (
    project: Project,
    variant: 'grid' | 'self-group' = 'grid'
  ) => {
    const isGrid = variant === 'grid';
    const isGroup = () => projectGroups().has(project.id);

    // El sortable solo existe en la grilla; la cabecera del propio grupo no se arrastra
    const sortable = isGrid ? createSortable(project.id) : null;
    const transform = () => {
      const t = sortable?.transform;
      if (t) {
        return `translate3d(${t.x}px, ${t.y}px, 0)`;
      }
      return undefined;
    };

    return (
      <div
        ref={isGrid ? sortable!.ref : undefined}
        class="project-card group flex h-full min-h-[300px] flex-col"
        classList={{
          'opacity-25 scale-105': isGrid && sortable!.isActiveDraggable,
          // Acento sutil para distinguir la tarjeta del propio grupo
          'ring-2 ring-accent-500/50 ring-offset-2 ring-offset-surface-50 dark:ring-offset-surface-900':
            !isGrid,
        }}
        style={{
          transform: transform(),
          transition:
            isGrid && sortable!.isActiveDraggable
              ? undefined
              : 'transform 200ms ease',
        }}
      >
        {/* Header con botón de pin y drag handle */}
        <div class="mb-2 flex items-center justify-between gap-2">
          <div class="flex flex-1 items-center gap-2">
            {/* Drag handle - solo en la grilla ordenable */}
            <Show when={isGrid}>
              <button
                {...sortable!.dragActivators}
                class="cursor-grab rounded-md p-1.5 text-surface-400 transition-colors hover:bg-surface-100 hover:text-surface-600 active:cursor-grabbing dark:hover:bg-surface-700 dark:hover:text-surface-300"
                title="Arrastrar para reordenar"
              >
                <svg class="h-4 w-4" fill="currentColor" viewBox="0 0 20 20">
                  <path d="M7 2a2 2 0 1 0 .001 4.001A2 2 0 0 0 7 2zm0 6a2 2 0 1 0 .001 4.001A2 2 0 0 0 7 8zm0 6a2 2 0 1 0 .001 4.001A2 2 0 0 0 7 14zm6-8a2 2 0 1 0-.001-4.001A2 2 0 0 0 13 6zm0 2a2 2 0 1 0 .001 4.001A2 2 0 0 0 13 8zm0 6a2 2 0 1 0 .001 4.001A2 2 0 0 0 13 14z" />
                </svg>
              </button>
            </Show>
            <Show when={project.status}>
              <div class="relative">
                <select
                  value={project.status || 'activo'}
                  onChange={(e) =>
                    handleChangeStatus(project, e.currentTarget.value)
                  }
                  class="appearance-none rounded-lg border border-surface-200 bg-surface-50 py-1 pl-2 pr-7 text-xs font-medium text-surface-700 transition-colors focus:border-accent-500 focus:outline-none focus:ring-1 focus:ring-accent-500 dark:border-surface-600 dark:bg-surface-700 dark:text-surface-200"
                  onClick={(e) => e.stopPropagation()}
                >
                  <option value="activo">🟢 Activo</option>
                  <option value="pausado">🟡 Pausado</option>
                  <option value="completado">✅ Completado</option>
                  <option value="archivado">📦 Archivado</option>
                </select>
                <svg
                  class="pointer-events-none absolute right-1.5 top-1/2 h-3 w-3 -translate-y-1/2 text-surface-400"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M19 9l-7 7-7-7"
                  />
                </svg>
              </div>
            </Show>
          </div>
          <div class="flex items-center gap-1">
            <TimeTracker
              projectId={project.id}
              projectPath={project.local_path}
              compact={true}
            />
            <button
              onClick={() => handleTogglePin(project)}
              class="rounded-lg p-1.5 transition-all hover:scale-110 hover:bg-amber-50 dark:hover:bg-amber-900/30"
              title={
                project.is_pinned
                  ? 'Desmarcar como favorito'
                  : 'Marcar como favorito'
              }
            >
              <span
                class={`text-lg transition-transform ${project.is_pinned ? 'drop-shadow-sm' : 'opacity-50 grayscale'}`}
              >
                {project.is_pinned ? '📌' : '📍'}
              </span>
            </button>
          </div>
        </div>

        {/* Project Info */}
        <div class="flex gap-3">
          <Show when={project.image_data}>
            <div class="relative flex-shrink-0">
              <img
                src={project.image_data}
                alt={project.name}
                class="h-14 w-14 rounded-xl border-2 border-surface-200 object-cover shadow-sm transition-transform group-hover:scale-105 dark:border-surface-600"
              />
              <div class="absolute -bottom-1 -right-1 h-3 w-3 rounded-full border-2 border-white bg-emerald-500 dark:border-surface-800" />
            </div>
          </Show>
          <div class="min-w-0 flex-1">
            <h3 class="truncate font-display text-base font-semibold text-surface-900 dark:text-white">
              {project.name}
            </h3>
            <p class="mt-0.5 line-clamp-2 text-xs leading-relaxed text-surface-600 dark:text-surface-400">
              {project.description}
            </p>
            {/* Badge indicando jerarquía en búsqueda */}
            <Show when={props.searchActive}>
              <div class="mt-1.5">
                <Show
                  when={project.parent_id}
                  fallback={
                    <Show when={isGroup()}>
                      <span class="badge badge-accent">
                        <svg
                          class="mr-1 h-3 w-3"
                          fill="none"
                          viewBox="0 0 24 24"
                          stroke="currentColor"
                        >
                          <path
                            stroke-linecap="round"
                            stroke-linejoin="round"
                            stroke-width="2"
                            d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"
                          />
                        </svg>
                        Grupo
                      </span>
                    </Show>
                  }
                >
                  <span
                    class="badge"
                    style={{
                      background: 'rgba(168, 85, 247, 0.15)',
                      color: 'rgb(168, 85, 247)',
                    }}
                  >
                    <svg
                      class="mr-1 h-3 w-3"
                      fill="none"
                      viewBox="0 0 24 24"
                      stroke="currentColor"
                    >
                      <path
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        stroke-width="2"
                        d="M13 5l7 7-7 7M5 5l7 7-7 7"
                      />
                    </svg>
                    Subproyecto
                  </span>
                </Show>
              </div>
            </Show>
          </div>
        </div>

        {/* Path & Git Info */}
        <div class="mt-3 space-y-2 border-t border-surface-100 pt-3 dark:border-surface-700">
          <div class="flex items-center gap-2 rounded-lg bg-surface-50 px-2 py-1.5 font-mono text-xs text-surface-500 dark:bg-surface-800/50 dark:text-surface-400">
            <svg
              class="h-3.5 w-3.5 flex-shrink-0 text-surface-400"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"
              />
            </svg>
            <span class="truncate" title={project.local_path}>
              {project.local_path}
            </span>
          </div>
          <EnhancedGitInfo
            projectPath={project.local_path}
            onCommitClick={() => setCommitProjectPath(project.local_path)}
          />
        </div>

        {/* Notes */}
        <Show when={project.notes}>
          <div class="mt-3 rounded-lg border border-surface-100 bg-gradient-to-br from-surface-50 to-transparent p-2.5 dark:border-surface-700 dark:from-surface-800/50">
            <div class="mb-1.5 flex items-center gap-1.5 text-xs font-semibold text-surface-600 dark:text-surface-300">
              <svg
                class="h-3.5 w-3.5 text-amber-500"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"
                />
              </svg>
              Notas
            </div>
            <div
              class="prose-xs scrollbar-thin prose max-h-20 max-w-none overflow-y-auto break-words text-surface-600 dark:prose-invert dark:text-surface-300"
              // eslint-disable-next-line solid/no-innerhtml
              innerHTML={renderMarkdown(project.notes!)}
            />
          </div>
        </Show>

        {/* Quick Links */}
        <Show
          when={
            project.documentation_url ||
            project.ai_documentation_url ||
            project.drive_link
          }
        >
          <div class="mt-3 flex flex-wrap gap-1.5">
            <Show when={project.documentation_url}>
              <button
                onClick={() => openUrl(project.documentation_url!)}
                class="action-pill action-pill-accent"
                type="button"
              >
                <svg
                  class="h-3 w-3"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5c1.747 0 3.332.477 4.5 1.253v13C19.832 18.477 18.247 18 16.5 18c-1.746 0-3.332.477-4.5 1.253"
                  />
                </svg>
                Docs
              </button>
            </Show>
            <Show when={project.ai_documentation_url}>
              <button
                onClick={() => openUrl(project.ai_documentation_url!)}
                class="action-pill"
                style={{
                  background: 'rgba(168, 85, 247, 0.1)',
                  color: 'rgb(168, 85, 247)',
                }}
                type="button"
              >
                <svg
                  class="h-3 w-3"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z"
                  />
                </svg>
                AI Docs
              </button>
            </Show>
            <Show when={project.drive_link}>
              <button
                onClick={() => openUrl(project.drive_link!)}
                class="action-pill action-pill-emerald"
                type="button"
              >
                <svg
                  class="h-3 w-3"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"
                  />
                </svg>
                Drive
              </button>
            </Show>
          </div>
        </Show>

        {/* Action Buttons - Auto-grow to bottom */}
        <div class="mt-auto pt-3">
          {/* Primary Action */}
          <button
            onClick={() => handleOpenTerminal(project)}
            class="btn-primary mb-2 w-full justify-center gap-2"
          >
            <svg
              class="h-4 w-4"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M13 10V3L4 14h7v7l9-11h-7z"
              />
            </svg>
            Trabajar
          </button>

          {/* Secondary Actions Grid */}
          <div class="grid grid-cols-5 gap-1">
            {/* v0.4.0 - Botón para sacar proyecto del grupo.
                Oculto durante búsqueda: los resultados son globales
                y "sacar del grupo" mutaría datos de otro contexto.
                Oculto también en la cabecera del propio grupo (variant self-group). */}
            <Show
              when={
                isGrid &&
                props.viewMode === 'subprojects' &&
                !props.searchActive
              }
            >
              <button
                onClick={() => handleRemoveFromGroup(project)}
                class="btn-icon text-amber-600 hover:bg-amber-50 dark:text-amber-400 dark:hover:bg-amber-900/30"
                aria-label="Sacar del grupo"
                title="Sacar este proyecto del grupo"
              >
                <svg
                  class="h-4 w-4"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M8 11V7a4 4 0 118 0m-4 8v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2z"
                  />
                </svg>
              </button>
            </Show>
            <button
              onClick={() => setContextProjectId(project.id)}
              class="btn-icon text-accent-600 hover:bg-accent-50 dark:text-accent-400 dark:hover:bg-accent-900/30"
              aria-label="Contexto"
              title="Ver contexto del proyecto"
            >
              <svg
                class="h-4 w-4"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2"
                />
              </svg>
            </button>
            <button
              onClick={() => setJournalProjectId(project.id)}
              class="btn-icon text-amber-600 hover:bg-amber-50 dark:text-amber-400 dark:hover:bg-amber-900/30"
              aria-label="Diario"
              title="Diario del proyecto"
            >
              <svg
                class="h-4 w-4"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5c1.747 0 3.332.477 4.5 1.253v13C19.832 18.477 18.247 18 16.5 18c-1.746 0-3.332.477-4.5 1.253"
                />
              </svg>
            </button>
            <button
              onClick={() => setTodosProjectId(project.id)}
              class="btn-icon text-emerald-600 hover:bg-emerald-50 dark:text-emerald-400 dark:hover:bg-emerald-900/30"
              aria-label="TODOs"
              title="Lista de tareas"
            >
              <svg
                class="h-4 w-4"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"
                />
              </svg>
            </button>
            <button
              onClick={() => handleExportPdf(project)}
              class="btn-icon text-rose-600 hover:bg-rose-50 dark:text-rose-400 dark:hover:bg-rose-900/30"
              aria-label="Exportar PDF"
              title="Exportar proyecto a PDF"
            >
              <svg
                class="h-4 w-4"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M7 21h10a2 2 0 002-2V9.414a1 1 0 00-.293-.707l-5.414-5.414A1 1 0 0012.586 3H7a2 2 0 00-2 2v14a2 2 0 002 2z"
                />
              </svg>
            </button>
          </div>

          {/* Backup & Utility Actions */}
          <div class="mt-1.5 grid grid-cols-5 gap-1">
            <button
              onClick={() => handleBackup(project)}
              class="btn-icon text-blue-600 hover:bg-blue-50 dark:text-blue-400 dark:hover:bg-blue-900/30"
              aria-label="Crear backup"
              title="Crear backup - Elegir carpeta"
            >
              <svg
                class="h-4 w-4"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M8 7H5a2 2 0 00-2 2v9a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-3m-1 4l-3 3m0 0l-3-3m3 3V4"
                />
              </svg>
            </button>
            <button
              onClick={() => handleSync(project)}
              class="btn-icon text-emerald-600 hover:bg-emerald-50 dark:text-emerald-400 dark:hover:bg-emerald-900/30"
              aria-label="Sincronizar"
              title="Sincronizar con rsync"
            >
              <svg
                class="h-4 w-4"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
                />
              </svg>
            </button>
            <button
              onClick={() => props.onEdit(project)}
              class="btn-icon text-surface-600 hover:bg-surface-100 dark:text-surface-400 dark:hover:bg-surface-700"
              aria-label="Editar"
              title="Editar proyecto"
            >
              <svg
                class="h-4 w-4"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"
                />
              </svg>
            </button>
            <button
              onClick={() => props.onDelete(project)}
              class="btn-icon text-rose-500 hover:bg-rose-50 dark:text-rose-400 dark:hover:bg-rose-900/30"
              aria-label="Eliminar"
              title="Eliminar proyecto"
            >
              <svg
                class="h-4 w-4"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"
                />
              </svg>
            </button>
          </div>
        </div>
      </div>
    );
  };

  return (
    <>
      {/* Cabecera "Este grupo": la tarjeta del propio grupo, SIEMPRE visible al entrar
          a un grupo (aunque no tenga hijos o los filtros los oculten), para poder
          Trabajar/editar/borrar el grupo. Fuera del Sortable, sin DnD ni "Sacar del grupo". */}
      <Show
        when={
          props.viewMode === 'subprojects' &&
          !props.searchActive &&
          props.currentGroup
        }
      >
        <div class="mb-4">
          <div class="mb-2 text-xs font-semibold uppercase tracking-wide text-surface-500 dark:text-surface-400">
            Este grupo
          </div>
          {renderProjectCard(props.currentGroup!, 'self-group')}
          <div class="mb-1 mt-4 border-t border-surface-200 pt-3 text-xs font-semibold uppercase tracking-wide text-surface-500 dark:border-surface-700 dark:text-surface-400">
            Proyectos dentro del grupo ({filteredProjects().length})
          </div>
        </div>
      </Show>

      <Show
        when={props.projects.length > 0}
        fallback={
          <div class="flex flex-col items-center justify-center py-16 text-center">
            <div class="mb-4 rounded-2xl bg-surface-100 p-6 dark:bg-surface-800">
              <svg
                class="h-16 w-16 text-surface-400"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="1.5"
                  d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"
                />
              </svg>
            </div>
            <p class="text-lg font-medium text-surface-600 dark:text-surface-300">
              {props.viewMode === 'subprojects' && props.currentGroup
                ? 'Este grupo todavía no tiene subproyectos'
                : 'No hay proyectos disponibles'}
            </p>
            <p class="mt-2 text-sm text-surface-500 dark:text-surface-400">
              {props.viewMode === 'subprojects' && props.currentGroup
                ? 'Arrastrá proyectos acá o creá uno nuevo dentro del grupo'
                : 'Crea tu primer proyecto para comenzar'}
            </p>
          </div>
        }
      >
        <Show
          when={filteredProjects().length > 0}
          fallback={
            <div class="flex flex-col items-center justify-center py-16 text-center">
              <div class="mb-4 rounded-2xl bg-surface-100 p-6 dark:bg-surface-800">
                <svg
                  class="h-16 w-16 text-surface-400"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="1.5"
                    d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
                  />
                </svg>
              </div>
              <p class="text-lg font-medium text-surface-600 dark:text-surface-300">
                No hay proyectos que coincidan
              </p>
              <p class="mt-2 text-sm text-surface-500 dark:text-surface-400">
                Intenta cambiar los filtros aplicados
              </p>
            </div>
          }
        >
          <DragDropProvider
            onDragEnd={handleDragEnd}
            collisionDetector={closestCenter}
          >
            <DragDropSensors />
            <SortableProvider ids={filteredProjects().map((p) => p.id)}>
              <div class="grid auto-rows-fr gap-4 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 2xl:grid-cols-5">
                <For each={filteredProjects()}>
                  {(project) => {
                    // Determinar si este proyecto es un grupo (tiene hijos)
                    const isGroup = () => projectGroups().has(project.id);

                    // Si es un grupo y estamos en vista de grupos (sin búsqueda), usar GroupCard
                    return (
                      <Show
                        when={
                          isGroup() &&
                          (props.viewMode === 'groups' || props.searchActive)
                        }
                        fallback={renderProjectCard(project, 'grid')}
                      >
                        {/* Render GroupCard para proyectos que son grupos */}
                        <GroupCard
                          project={project}
                          onViewProjects={(group) => {
                            if (props.onViewGroup) {
                              props.onViewGroup(group);
                            }
                          }}
                          onEdit={props.onEdit}
                          onDelete={props.onDelete}
                        />
                      </Show>
                    );
                  }}
                </For>
              </div>
            </SortableProvider>
          </DragDropProvider>
        </Show>
      </Show>

      {/* Modales a nivel raíz: montados según su propia señal, INDEPENDIENTE de si hay
          hijos, para que funcionen también desde la cabecera de un grupo vacío. */}
      {/* Journal Modal */}
      <Show when={journalProjectId() !== null}>
        <ProjectJournal
          projectId={journalProjectId()!}
          onClose={() => setJournalProjectId(null)}
        />
      </Show>

      {/* TODOs Modal */}
      <Show when={todosProjectId() !== null}>
        <div class="modal-overlay" onClick={() => setTodosProjectId(null)}>
          <div
            class="modal-content max-w-2xl"
            onClick={(e) => e.stopPropagation()}
          >
            {/* Header */}
            <div class="flex items-center justify-between border-b border-surface-200 p-5 dark:border-surface-700">
              <div class="flex items-center gap-3">
                <div class="flex h-10 w-10 items-center justify-center rounded-xl bg-emerald-100 text-emerald-600 dark:bg-emerald-900/30 dark:text-emerald-400">
                  <svg
                    class="h-5 w-5"
                    fill="none"
                    viewBox="0 0 24 24"
                    stroke="currentColor"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"
                    />
                  </svg>
                </div>
                <h2 class="font-display text-xl font-semibold text-surface-900 dark:text-white">
                  Lista de Tareas
                </h2>
              </div>
              <button
                onClick={() => setTodosProjectId(null)}
                class="btn-icon text-surface-500 hover:bg-surface-100 dark:hover:bg-surface-700"
              >
                <svg
                  class="h-5 w-5"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M6 18L18 6M6 6l12 12"
                  />
                </svg>
              </button>
            </div>

            {/* Content */}
            <div class="scrollbar-thin max-h-[70vh] overflow-y-auto p-5">
              <TodoList projectId={todosProjectId()!} />
            </div>
          </div>
        </div>
      </Show>

      {/* Project Context Modal */}
      <Show when={contextProjectId() !== null}>
        <ProjectContext
          projectId={contextProjectId()!}
          onClose={() => setContextProjectId(null)}
        />
      </Show>

      {/* Git Commit Modal */}
      <Show when={commitProjectPath() !== null}>
        <GitCommitModal
          projectPath={commitProjectPath()!}
          onClose={() => setCommitProjectPath(null)}
          onSuccess={() => {
            // Recargar proyectos después de commit exitoso
            props.onProjectsChanged?.();
          }}
        />
      </Show>
    </>
  );
};

export default ProjectList;
