#!/usr/bin/env bash
set -euo pipefail

# ─────────────────────────────────────────────────────────
# release.sh - Sistema de versionado SemVer automatico
# Uso: ./scripts/release.sh [patch|minor|major] [--install] [--dry-run]
# ─────────────────────────────────────────────────────────

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Archivos de version
PKG_JSON="$PROJECT_ROOT/package.json"
CARGO_TOML="$PROJECT_ROOT/src-tauri/Cargo.toml"
TAURI_CONF="$PROJECT_ROOT/src-tauri/tauri.conf.json"
CHANGELOG="$PROJECT_ROOT/CHANGELOG.md"

# Colores
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m'

# ─── Argumentos ──────────────────────────────────────────

BUMP_TYPE="patch"
DO_INSTALL=false
DRY_RUN=false

for arg in "$@"; do
    case "$arg" in
        patch|minor|major) BUMP_TYPE="$arg" ;;
        --install) DO_INSTALL=true ;;
        --dry-run) DRY_RUN=true ;;
        -h|--help)
            echo "Uso: ./scripts/release.sh [patch|minor|major] [--install] [--dry-run]"
            echo ""
            echo "  patch   (default) Incrementa version de parche: 0.4.3 -> 0.4.4"
            echo "  minor   Incrementa version menor: 0.4.3 -> 0.5.0"
            echo "  major   Incrementa version mayor: 0.4.3 -> 1.0.0"
            echo "  --install  Compila e instala binario en ~/.local/bin/"
            echo "  --dry-run  Muestra cambios sin aplicar"
            exit 0
            ;;
        *)
            echo -e "${RED}Error: argumento desconocido '$arg'${NC}"
            echo "Uso: ./scripts/release.sh [patch|minor|major] [--install] [--dry-run]"
            exit 1
            ;;
    esac
done

# ─── Funciones ───────────────────────────────────────────

get_current_version() {
    grep -m1 '"version"' "$PKG_JSON" | sed 's/.*"\([0-9]*\.[0-9]*\.[0-9]*\)".*/\1/'
}

bump_version() {
    local version="$1"
    local type="$2"
    local major minor patch
    IFS='.' read -r major minor patch <<< "$version"

    case "$type" in
        major) echo "$((major + 1)).0.0" ;;
        minor) echo "$major.$((minor + 1)).0" ;;
        patch) echo "$major.$minor.$((patch + 1))" ;;
    esac
}

update_version_files() {
    local old="$1"
    local new="$2"

    # package.json - actualiza solo el campo "version" en la raiz (linea 3)
    sed -i "s/\"version\": \"$old\"/\"version\": \"$new\"/" "$PKG_JSON"

    # Cargo.toml - actualiza solo el campo version del [package]
    sed -i "s/^version = \"$old\"/version = \"$new\"/" "$CARGO_TOML"

    # tauri.conf.json - actualiza el campo "version"
    sed -i "s/\"version\": \"$old\"/\"version\": \"$new\"/" "$TAURI_CONF"
}

generate_changelog_entry() {
    local version="$1"
    local date
    date=$(date +%Y-%m-%d)

    local last_tag
    last_tag=$(git describe --tags --abbrev=0 2>/dev/null || echo "")

    local range
    if [[ -n "$last_tag" ]]; then
        range="${last_tag}..HEAD"
    else
        range="HEAD"
    fi

    # Recoger commits agrupados por tipo
    local feats="" fixes="" tests="" others=""
    local commit_line

    while IFS= read -r commit_line; do
        # Ignorar commits vacios y "checkpoint: auto"
        [[ -z "$commit_line" ]] && continue
        [[ "$commit_line" =~ ^checkpoint:\ auto ]] && continue

        # Limpiar prefijo y clasificar
        if [[ "$commit_line" =~ ^feat:\ ?(.*) ]]; then
            feats+="- ${BASH_REMATCH[1]}"$'\n'
        elif [[ "$commit_line" =~ ^fix:\ ?(.*) ]]; then
            fixes+="- ${BASH_REMATCH[1]}"$'\n'
        elif [[ "$commit_line" =~ ^test:\ ?(.*) ]]; then
            tests+="- ${BASH_REMATCH[1]}"$'\n'
        elif [[ "$commit_line" =~ ^build:\ ?(.*) ]]; then
            others+="- ${BASH_REMATCH[1]}"$'\n'
        elif [[ "$commit_line" =~ ^refactor:\ ?(.*) ]]; then
            others+="- ${BASH_REMATCH[1]}"$'\n'
        elif [[ "$commit_line" =~ ^checkpoint:\ ?(.*) ]]; then
            others+="- ${BASH_REMATCH[1]}"$'\n'
        elif [[ "$commit_line" =~ ^release:\ ? ]]; then
            # Ignorar commits de release
            continue
        else
            others+="- ${commit_line}"$'\n'
        fi
    done < <(git log --format="%s" "$range" 2>/dev/null)

    # Construir entrada
    local entry=""
    entry+="## ${date} - v${version}"$'\n'
    entry+=""$'\n'

    if [[ -n "$feats" ]]; then
        entry+="**Nuevas caracteristicas:**"$'\n'
        entry+=""$'\n'
        entry+="$feats"$'\n'
    fi

    if [[ -n "$fixes" ]]; then
        entry+="**Correcciones:**"$'\n'
        entry+=""$'\n'
        entry+="$fixes"$'\n'
    fi

    if [[ -n "$tests" ]]; then
        entry+="**Tests:**"$'\n'
        entry+=""$'\n'
        entry+="$tests"$'\n'
    fi

    if [[ -n "$others" ]]; then
        entry+="**Otros cambios:**"$'\n'
        entry+=""$'\n'
        entry+="$others"$'\n'
    fi

    # Si no hubo commits, poner un placeholder
    if [[ -z "$feats" && -z "$fixes" && -z "$tests" && -z "$others" ]]; then
        entry+="- Release v${version}"$'\n'
        entry+=""$'\n'
    fi

    echo "$entry"
}

prepend_changelog() {
    local entry="$1"
    local header separator rest

    # Leer las 4 primeras lineas (header del CHANGELOG)
    header=$(head -n 4 "$CHANGELOG")
    rest=$(tail -n +5 "$CHANGELOG")

    {
        echo "$header"
        echo ""
        echo "$entry"
        echo "---"
        echo ""
        echo "$rest"
    } > "$CHANGELOG"
}

# ─── Main ────────────────────────────────────────────────

cd "$PROJECT_ROOT"

# Verificar que estamos en un repo git limpio (o no)
if ! git rev-parse --is-inside-work-tree &>/dev/null; then
    echo -e "${RED}Error: no se encontro repositorio git${NC}"
    exit 1
fi

# Leer version actual
CURRENT_VERSION=$(get_current_version)
NEW_VERSION=$(bump_version "$CURRENT_VERSION" "$BUMP_TYPE")

echo -e "${BOLD}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BOLD}  Release: v${CURRENT_VERSION} → v${NEW_VERSION}${NC}"
echo -e "${BOLD}  Tipo: ${BUMP_TYPE}${NC}"
echo -e "${BOLD}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

# Verificar que el tag no existe ya
if git tag -l "v${NEW_VERSION}" | grep -q .; then
    echo -e "${RED}Error: el tag v${NEW_VERSION} ya existe${NC}"
    exit 1
fi

if $DRY_RUN; then
    echo -e "${YELLOW}[DRY RUN] Se actualizarian los siguientes archivos:${NC}"
    echo "  - package.json"
    echo "  - src-tauri/Cargo.toml"
    echo "  - src-tauri/tauri.conf.json"
    echo "  - CHANGELOG.md"
    echo ""
    echo -e "${YELLOW}[DRY RUN] Entrada de CHANGELOG:${NC}"
    echo ""
    generate_changelog_entry "$NEW_VERSION"
    echo -e "${YELLOW}[DRY RUN] Se crearia commit: release: v${NEW_VERSION}${NC}"
    echo -e "${YELLOW}[DRY RUN] Se crearia tag: v${NEW_VERSION}${NC}"
    if $DO_INSTALL; then
        echo -e "${YELLOW}[DRY RUN] Se compilaria e instalaria en ~/.local/bin/${NC}"
    fi
    exit 0
fi

# Paso 1: Actualizar archivos de version
echo -e "${BLUE}[1/4] Actualizando version en archivos...${NC}"
update_version_files "$CURRENT_VERSION" "$NEW_VERSION"
echo "  ✓ package.json"
echo "  ✓ src-tauri/Cargo.toml"
echo "  ✓ src-tauri/tauri.conf.json"
echo ""

# Paso 2: Generar CHANGELOG
echo -e "${BLUE}[2/4] Generando entrada en CHANGELOG...${NC}"
CHANGELOG_ENTRY=$(generate_changelog_entry "$NEW_VERSION")
prepend_changelog "$CHANGELOG_ENTRY"
echo "  ✓ CHANGELOG.md actualizado"
echo ""

# Paso 3: Commit
echo -e "${BLUE}[3/4] Creando commit...${NC}"
git add "$PKG_JSON" "$CARGO_TOML" "$TAURI_CONF" "$CHANGELOG"
git commit -m "release: v${NEW_VERSION}"
echo "  ✓ Commit creado"
echo ""

# Paso 4: Tag
echo -e "${BLUE}[4/4] Creando tag...${NC}"
git tag -a "v${NEW_VERSION}" -m "Release v${NEW_VERSION}"
echo "  ✓ Tag v${NEW_VERSION} creado"
echo ""

# Opcional: compilar e instalar
if $DO_INSTALL; then
    echo -e "${BLUE}[extra] Compilando e instalando...${NC}"
    cd "$PROJECT_ROOT/src-tauri"
    cargo build --release
    mkdir -p ~/.local/bin
    cp target/release/gestor-proyectos ~/.local/bin/gestor-proyectos
    echo "  ✓ Binario instalado en ~/.local/bin/gestor-proyectos"
    cd "$PROJECT_ROOT"
    echo ""
fi

# Resumen
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}  Release v${NEW_VERSION} completado${NC}"
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo "  Version:   v${CURRENT_VERSION} → v${NEW_VERSION}"
echo "  Commit:    $(git log -1 --format='%h %s')"
echo "  Tag:       v${NEW_VERSION}"
if $DO_INSTALL; then
    echo "  Binario:   ~/.local/bin/gestor-proyectos"
fi
echo ""
echo -e "${YELLOW}Para publicar: git push && git push --tags${NC}"
