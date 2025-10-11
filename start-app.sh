#!/bin/bash

# Script para iniciar Gestor de Proyectos de forma limpia
# Uso: ./start-app.sh

set -e

echo "🚀 Iniciando Gestor de Proyectos..."
echo ""

# Colores
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 1. Verificar y detener instancias existentes
echo "🔍 Verificando instancias existentes..."
if pgrep -f "gestor-proyectos" > /dev/null; then
    echo -e "${YELLOW}⚠️  Hay instancias corriendo, deteniendo...${NC}"
    pkill -f "gestor-proyectos" 2>/dev/null || true
    pkill -f "tauri dev" 2>/dev/null || true
    sleep 2
    echo -e "${GREEN}✅ Instancias detenidas${NC}"
else
    echo -e "${GREEN}✅ No hay instancias previas${NC}"
fi

# 2. Verificar y liberar puerto 1420
echo "🔓 Verificando puerto 1420..."
if lsof -ti:1420 > /dev/null 2>&1; then
    echo -e "${YELLOW}⚠️  Puerto 1420 ocupado, liberando...${NC}"
    lsof -ti:1420 | xargs kill -9 2>/dev/null || true
    sleep 1
    echo -e "${GREEN}✅ Puerto 1420 liberado${NC}"
else
    echo -e "${GREEN}✅ Puerto 1420 disponible${NC}"
fi

# 3. Verificar que node_modules existe
echo "📦 Verificando dependencias..."
if [ ! -d "node_modules" ]; then
    echo -e "${YELLOW}⚠️  Dependencias no instaladas, instalando...${NC}"
    pnpm install
    echo -e "${GREEN}✅ Dependencias instaladas${NC}"
else
    echo -e "${GREEN}✅ Dependencias instaladas${NC}"
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo -e "${GREEN}🎯 Iniciando aplicación...${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "⏳ Compilando... (esto toma ~5-10 segundos)"
echo ""
echo "📝 Los logs aparecerán aquí:"
echo ""

# 4. Iniciar aplicación
pnpm run tauri:dev

