#!/bin/bash
# Gestor de Proyectos - Shell Hook para Time Tracking
# Este script detecta cuando entras/sales de directorios de proyectos
# y notifica a la app para trackear el tiempo.
#
# Instalación (agregar a ~/.bashrc o ~/.zshrc):
#   source /ruta/a/gestor-track.sh
#
# Requisitos:
#   - netcat (nc) o socat para comunicación con socket
#   - La app Gestor de Proyectos debe estar corriendo

# Configuración
GESTOR_SOCKET="/tmp/gestor-proyectos.sock"
GESTOR_LAST_PROJECT=""
GESTOR_DEBUG="${GESTOR_DEBUG:-0}"

# Función de logging (solo si debug está habilitado)
_gestor_log() {
    if [[ "$GESTOR_DEBUG" == "1" ]]; then
        echo "[gestor-track] $1" >&2
    fi
}

# Buscar .gestor/ hacia arriba en la jerarquía de directorios
_gestor_find_config() {
    local dir="$1"
    while [[ "$dir" != "/" && "$dir" != "" ]]; do
        if [[ -f "$dir/.gestor/config.json" ]]; then
            echo "$dir"
            return 0
        fi
        dir="$(dirname "$dir")"
    done
    return 1
}

# Enviar mensaje al socket de la app
_gestor_send() {
    local message="$1"

    # Verificar si el socket existe
    if [[ ! -S "$GESTOR_SOCKET" ]]; then
        _gestor_log "Socket no disponible: $GESTOR_SOCKET"
        return 1
    fi

    # Intentar enviar con netcat
    if command -v nc &> /dev/null; then
        echo "$message" | nc -U -w 1 "$GESTOR_SOCKET" 2>/dev/null
        return $?
    fi

    # Fallback a socat
    if command -v socat &> /dev/null; then
        echo "$message" | socat - UNIX-CONNECT:"$GESTOR_SOCKET" 2>/dev/null
        return $?
    fi

    _gestor_log "No se encontró nc ni socat para enviar mensajes"
    return 1
}

# Hook principal que se ejecuta al cambiar de directorio
_gestor_chpwd() {
    local current_dir="$PWD"
    local project_dir

    # Buscar .gestor/ en el directorio actual o ancestros
    project_dir=$(_gestor_find_config "$current_dir")

    if [[ -n "$project_dir" ]]; then
        # Estamos en un proyecto trackeable
        if [[ "$project_dir" != "$GESTOR_LAST_PROJECT" ]]; then
            # Cambió el proyecto

            # Notificar salida del proyecto anterior
            if [[ -n "$GESTOR_LAST_PROJECT" ]]; then
                _gestor_log "Saliendo de: $GESTOR_LAST_PROJECT"
                _gestor_send "{\"type\":\"Exit\",\"path\":\"$GESTOR_LAST_PROJECT\"}"
            fi

            # Notificar entrada al nuevo proyecto
            _gestor_log "Entrando a: $project_dir"
            _gestor_send "{\"type\":\"Enter\",\"path\":\"$project_dir\"}"

            GESTOR_LAST_PROJECT="$project_dir"
        fi
    elif [[ -n "$GESTOR_LAST_PROJECT" ]]; then
        # Salimos de un proyecto a un directorio sin tracking
        _gestor_log "Saliendo de: $GESTOR_LAST_PROJECT"
        _gestor_send "{\"type\":\"Exit\",\"path\":\"$GESTOR_LAST_PROJECT\"}"
        GESTOR_LAST_PROJECT=""
    fi
}

# Enviar heartbeat (para detectar inactividad)
_gestor_heartbeat() {
    if [[ -n "$GESTOR_LAST_PROJECT" ]]; then
        _gestor_send "{\"type\":\"Heartbeat\",\"path\":\"$GESTOR_LAST_PROJECT\"}"
    fi
}

# Solicitar estado actual
gestor_status() {
    if [[ -S "$GESTOR_SOCKET" ]]; then
        _gestor_send '{"type":"Status"}'
    else
        echo "Gestor de Proyectos no está corriendo"
    fi
}

# Mostrar qué proyecto está siendo tracked actualmente
gestor_current() {
    if [[ -n "$GESTOR_LAST_PROJECT" ]]; then
        echo "Proyecto actual: $GESTOR_LAST_PROJECT"
    else
        echo "No hay proyecto siendo tracked"
    fi
}

# ==================== INTEGRACIÓN CON SHELLS ====================

# Detectar el shell y configurar los hooks apropiados
if [[ -n "$ZSH_VERSION" ]]; then
    # Zsh - usar chpwd hook
    autoload -Uz add-zsh-hook
    add-zsh-hook chpwd _gestor_chpwd
    _gestor_log "Hook instalado para Zsh"

elif [[ -n "$BASH_VERSION" ]]; then
    # Bash - usar PROMPT_COMMAND
    # Guardamos el comando anterior para no sobrescribirlo
    if [[ -z "$_GESTOR_ORIGINAL_PROMPT_COMMAND" ]]; then
        _GESTOR_ORIGINAL_PROMPT_COMMAND="$PROMPT_COMMAND"
    fi

    _gestor_bash_hook() {
        # Ejecutar hook solo si el directorio cambió
        if [[ "$_GESTOR_LAST_DIR" != "$PWD" ]]; then
            _gestor_chpwd
            _GESTOR_LAST_DIR="$PWD"
        fi

        # Ejecutar el PROMPT_COMMAND original
        if [[ -n "$_GESTOR_ORIGINAL_PROMPT_COMMAND" ]]; then
            eval "$_GESTOR_ORIGINAL_PROMPT_COMMAND"
        fi
    }

    PROMPT_COMMAND="_gestor_bash_hook"
    _GESTOR_LAST_DIR="$PWD"
    _gestor_log "Hook instalado para Bash"
fi

# Ejecutar hook inicial para detectar si ya estamos en un proyecto
_gestor_chpwd

# ==================== HEARTBEAT AUTOMÁTICO ====================

# Configurar heartbeat cada 5 minutos (opcional, usa cron o systemd timer)
# Para activar: export GESTOR_HEARTBEAT=1
if [[ "$GESTOR_HEARTBEAT" == "1" ]]; then
    # Enviar heartbeat cada 5 minutos en background
    (
        while true; do
            sleep 300
            _gestor_heartbeat
        done
    ) &
    GESTOR_HEARTBEAT_PID=$!

    # Cleanup al salir
    trap "kill $GESTOR_HEARTBEAT_PID 2>/dev/null" EXIT
fi

_gestor_log "Gestor Track cargado correctamente"
