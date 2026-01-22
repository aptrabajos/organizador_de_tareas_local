# Gestor de Proyectos - Shell Hook para Time Tracking (Fish)
# Este script detecta cuando entras/sales de directorios de proyectos
# y notifica a la app para trackear el tiempo.
#
# Instalación (agregar a ~/.config/fish/config.fish):
#   source /ruta/a/gestor-track.fish

# Configuración
set -g GESTOR_SOCKET "/tmp/gestor-proyectos.sock"
set -g GESTOR_LAST_PROJECT ""
set -q GESTOR_DEBUG; or set -g GESTOR_DEBUG 0

# Función de logging
function _gestor_log
    if test "$GESTOR_DEBUG" = "1"
        echo "[gestor-track] $argv" >&2
    end
end

# Buscar .gestor/ hacia arriba en la jerarquía de directorios
function _gestor_find_config
    set -l dir $argv[1]
    while test "$dir" != "/"
        if test -f "$dir/.gestor/config.json"
            echo $dir
            return 0
        end
        set dir (dirname $dir)
    end
    return 1
end

# Enviar mensaje al socket de la app
function _gestor_send
    set -l message $argv[1]

    # Verificar si el socket existe
    if not test -S "$GESTOR_SOCKET"
        _gestor_log "Socket no disponible: $GESTOR_SOCKET"
        return 1
    end

    # Intentar enviar con netcat
    if type -q nc
        echo $message | nc -U -w 1 "$GESTOR_SOCKET" 2>/dev/null
        return $status
    end

    # Fallback a socat
    if type -q socat
        echo $message | socat - UNIX-CONNECT:"$GESTOR_SOCKET" 2>/dev/null
        return $status
    end

    _gestor_log "No se encontró nc ni socat para enviar mensajes"
    return 1
end

# Hook principal que se ejecuta al cambiar de directorio
function _gestor_chpwd --on-variable PWD
    set -l current_dir $PWD
    set -l project_dir (_gestor_find_config $current_dir)

    if test -n "$project_dir"
        # Estamos en un proyecto trackeable
        if test "$project_dir" != "$GESTOR_LAST_PROJECT"
            # Cambió el proyecto

            # Notificar salida del proyecto anterior
            if test -n "$GESTOR_LAST_PROJECT"
                _gestor_log "Saliendo de: $GESTOR_LAST_PROJECT"
                _gestor_send '{"type":"Exit","path":"'$GESTOR_LAST_PROJECT'"}'
            end

            # Notificar entrada al nuevo proyecto
            _gestor_log "Entrando a: $project_dir"
            _gestor_send '{"type":"Enter","path":"'$project_dir'"}'

            set -g GESTOR_LAST_PROJECT $project_dir
        end
    else if test -n "$GESTOR_LAST_PROJECT"
        # Salimos de un proyecto a un directorio sin tracking
        _gestor_log "Saliendo de: $GESTOR_LAST_PROJECT"
        _gestor_send '{"type":"Exit","path":"'$GESTOR_LAST_PROJECT'"}'
        set -g GESTOR_LAST_PROJECT ""
    end
end

# Enviar heartbeat
function _gestor_heartbeat
    if test -n "$GESTOR_LAST_PROJECT"
        _gestor_send '{"type":"Heartbeat","path":"'$GESTOR_LAST_PROJECT'"}'
    end
end

# Comandos de usuario
function gestor_status
    if test -S "$GESTOR_SOCKET"
        _gestor_send '{"type":"Status"}'
    else
        echo "Gestor de Proyectos no está corriendo"
    end
end

function gestor_current
    if test -n "$GESTOR_LAST_PROJECT"
        echo "Proyecto actual: $GESTOR_LAST_PROJECT"
    else
        echo "No hay proyecto siendo tracked"
    end
end

# Ejecutar hook inicial para detectar si ya estamos en un proyecto
_gestor_chpwd

_gestor_log "Gestor Track (Fish) cargado correctamente"
