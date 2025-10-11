#!/bin/bash

# Script de instalación para Gestor de Proyectos
# Instala el ejecutable y crea acceso directo en el menú de aplicaciones

set -e  # Salir si hay errores

echo "╔═══════════════════════════════════════════════════════════╗"
echo "║   🚀 INSTALADOR DE GESTOR DE PROYECTOS                  ║"
echo "╚═══════════════════════════════════════════════════════════╝"
echo ""

# Verificar que el ejecutable existe
if [ ! -f "src-tauri/target/release/gestor-proyectos" ]; then
    echo "❌ Error: El ejecutable no existe. Ejecuta 'pnpm tauri build' primero."
    exit 1
fi

# Crear directorios necesarios
echo "📁 Creando directorios..."
mkdir -p ~/.local/bin
mkdir -p ~/.local/share/applications
mkdir -p ~/.local/share/icons/hicolor/256x256/apps

# Copiar ejecutable
echo "📦 Copiando ejecutable..."
cp src-tauri/target/release/gestor-proyectos ~/.local/bin/
chmod +x ~/.local/bin/gestor-proyectos

# Copiar ícono
echo "🎨 Copiando ícono..."
if [ -f "src-tauri/icons/icon.png" ]; then
    cp src-tauri/icons/icon.png ~/.local/share/icons/hicolor/256x256/apps/gestor-proyectos.png
else
    echo "⚠️  Advertencia: No se encontró el ícono, usando ícono por defecto"
fi

# Crear archivo .desktop
echo "🖥️  Creando entrada en el menú de aplicaciones..."
cat > ~/.local/share/applications/gestor-proyectos.desktop << 'EOF'
[Desktop Entry]
Version=1.0
Type=Application
Name=Gestor de Proyectos
Comment=Gestor local de proyectos con respaldo y sincronización
Exec=/home/manjarodesktop/.local/bin/gestor-proyectos
Icon=gestor-proyectos
Terminal=false
Categories=Development;Utility;
Keywords=proyectos;gestor;backup;rsync;
StartupNotify=true
EOF

# Hacer ejecutable el .desktop file
chmod +x ~/.local/share/applications/gestor-proyectos.desktop

# Actualizar cache de aplicaciones
echo "🔄 Actualizando caché de aplicaciones..."
if command -v update-desktop-database &> /dev/null; then
    update-desktop-database ~/.local/share/applications
fi

# Verificar que ~/.local/bin está en el PATH
if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
    echo ""
    echo "⚠️  ADVERTENCIA: ~/.local/bin no está en tu PATH"
    echo ""
    echo "Agrega esta línea a tu ~/.bashrc o ~/.zshrc:"
    echo "    export PATH=\"\$HOME/.local/bin:\$PATH\""
    echo ""
fi

echo ""
echo "╔═══════════════════════════════════════════════════════════╗"
echo "║   ✅ INSTALACIÓN COMPLETADA EXITOSAMENTE                ║"
echo "╚═══════════════════════════════════════════════════════════╝"
echo ""
echo "📍 Ubicaciones:"
echo "   • Ejecutable: ~/.local/bin/gestor-proyectos"
echo "   • Ícono: ~/.local/share/icons/hicolor/256x256/apps/gestor-proyectos.png"
echo "   • Desktop: ~/.local/share/applications/gestor-proyectos.desktop"
echo ""
echo "🚀 Cómo usar:"
echo "   • Busca 'Gestor de Proyectos' en tu menú de aplicaciones"
echo "   • O ejecuta: gestor-proyectos"
echo ""
echo "🎯 ¡Listo para usar!"
echo ""

