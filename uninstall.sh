#!/bin/bash

# Script de desinstalación para Gestor de Proyectos

echo "╔═══════════════════════════════════════════════════════════╗"
echo "║   🗑️  DESINSTALADOR DE GESTOR DE PROYECTOS              ║"
echo "╚═══════════════════════════════════════════════════════════╝"
echo ""

# Eliminar ejecutable
if [ -f ~/.local/bin/gestor-proyectos ]; then
    echo "🗑️  Eliminando ejecutable..."
    rm ~/.local/bin/gestor-proyectos
fi

# Eliminar ícono
if [ -f ~/.local/share/icons/hicolor/256x256/apps/gestor-proyectos.png ]; then
    echo "🗑️  Eliminando ícono..."
    rm ~/.local/share/icons/hicolor/256x256/apps/gestor-proyectos.png
fi

# Eliminar archivo .desktop
if [ -f ~/.local/share/applications/gestor-proyectos.desktop ]; then
    echo "🗑️  Eliminando entrada del menú..."
    rm ~/.local/share/applications/gestor-proyectos.desktop
fi

# Actualizar cache
if command -v update-desktop-database &> /dev/null; then
    echo "🔄 Actualizando caché..."
    update-desktop-database ~/.local/share/applications
fi

echo ""
echo "✅ Desinstalación completada"
echo ""
echo "⚠️  Nota: La base de datos de proyectos se mantiene en:"
echo "   ~/.local/share/gestor-proyectos/"
echo ""
echo "   Si deseas eliminarla también, ejecuta:"
echo "   rm -rf ~/.local/share/gestor-proyectos/"
echo ""

