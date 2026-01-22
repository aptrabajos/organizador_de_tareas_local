#!/bin/zsh
# Gestor de Proyectos - Shell Hook para Time Tracking (Zsh)
# Este script detecta cuando entras/sales de directorios de proyectos
# y notifica a la app para trackear el tiempo.
#
# Instalación (agregar a ~/.zshrc):
#   source /ruta/a/gestor-track.zsh

# Configuración
typeset -g GESTOR_SOCKET="/tmp/gestor-proyectos.sock"
typeset -g GESTOR_LAST_PROJECT=""
typeset -g GESTOR_DEBUG="${GESTOR_DEBUG:-0}"

# Función de logging (solo si debug está habilitado)
_gestor_log() {
    if [[ "$GESTOR_DEBUG" == "1" ]]; then
        print -P "[gestor-track] $1" >&2
    fi
}

# Buscar .gestor/ hacia arriba en la jerarquía de directorios
_gestor_find_config() {
    local dir="$1"
    while [[ "$dir" != "/" && -n "$dir" ]]; do
        if [[ -f "$dir/.gestor/config.json" ]]; then
            print -r -- "$dir"
            return 0
        fi
        dir="${dir:h}"
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

    # Usar zsh/net/socket si está disponible
    zmodload -e zsh/net/socket 2>/dev/null
    if (( $? == 0 )); then
        integer fd
        if zsocket "$GESTOR_SOCKET" 2>/dev/null; then
            fd=$REPLY
            print -u $fd "$message"
            exec {fd}>&-
            return 0
        fi
    fi

    # Fallback a netcat
    if (( $+commands[nc] )); then
        print -r -- "$message" | nc -U -w 1 "$GESTOR_SOCKET" 2>/dev/null
        return $?
    fi

    # Fallback a socat
    if (( $+commands[socat] )); then
        print -r -- "$message" | socat - UNIX-CONNECT:"$GESTOR_SOCKET" 2>/dev/null
        return $?
    fi

    _gestor_log "No se encontró método para enviar mensajes"
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

# Enviar heartbeat
_gestor_heartbeat() {
    if [[ -n "$GESTOR_LAST_PROJECT" ]]; then
        _gestor_send "{\"type\":\"Heartbeat\",\"path\":\"$GESTOR_LAST_PROJECT\"}"
    fi
}

# Comandos de usuario
gestor_status() {
    if [[ -S "$GESTOR_SOCKET" ]]; then
        _gestor_send '{"type":"Status"}'
    else
        print "Gestor de Proyectos no está corriendo"
    fi
}

gestor_current() {
    if [[ -n "$GESTOR_LAST_PROJECT" ]]; then
        print "Proyecto actual: $GESTOR_LAST_PROJECT"
    else
        print "No hay proyecto siendo tracked"
    fi
}

# ==================== INSTALACIÓN DEL HOOK ====================

# Cargar add-zsh-hook si no está cargado
autoload -Uz add-zsh-hook

# Registrar el hook chpwd
add-zsh-hook chpwd _gestor_chpwd

# Ejecutar hook inicial
_gestor_chpwd

_gestor_log "Gestor Track (Zsh) cargado correctamente"
