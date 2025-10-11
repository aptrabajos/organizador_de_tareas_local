#!/bin/bash

# Script para monitorear logs del Gestor de Proyectos en tiempo real

echo "╔═══════════════════════════════════════════════════════════╗"
echo "║         📊 MONITOR DE LOGS - GESTOR DE PROYECTOS         ║"
echo "╚═══════════════════════════════════════════════════════════╝"
echo ""
echo "🔍 Monitoreando logs del backend Rust..."
echo "   Estos logs aparecen cuando:"
echo "   • Creas/editas/eliminas proyectos"
echo "   • Haces backup"
echo "   • Sincronizas"
echo "   • Abres terminales o URLs"
echo ""
echo "💡 Para salir: Ctrl+C"
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Buscar el proceso de la aplicación y seguir sus logs
PID=$(pgrep -f "target/debug/gestor-proyectos" | head -1)

if [ -z "$PID" ]; then
    echo "❌ La aplicación no está corriendo"
    echo "   Inicia la aplicación con: ./start-app.sh"
    exit 1
fi

echo "✅ Aplicación encontrada (PID: $PID)"
echo ""
echo "═══════════════ LOGS EN TIEMPO REAL ═══════════════════"
echo ""

# Intentar seguir logs del journal del usuario
journalctl --user -f -n 0 _PID=$PID 2>/dev/null || {
    echo "⚠️  No se pueden capturar logs automáticamente"
    echo ""
    echo "📝 Los logs aparecen en la terminal donde ejecutaste:"
    echo "   ./start-app.sh"
    echo "   o"
    echo "   pnpm run tauri:dev"
    echo ""
    echo "💡 Mantén esta ventana abierta y observa la otra terminal"
    echo ""
    echo "Presiona Ctrl+C para salir..."
    sleep infinity
}

