# Script de Compilación Automatizada para Windows
# Gestor de Proyectos v0.3.0

Write-Host "🪟 Build para Windows - Gestor de Proyectos v0.3.0" -ForegroundColor Cyan
Write-Host "=================================================" -ForegroundColor Cyan
Write-Host ""

# Verificar que estamos en el directorio correcto
if (-Not (Test-Path "src-tauri\Cargo.toml")) {
    Write-Host "❌ Error: Este script debe ejecutarse desde la raíz del proyecto" -ForegroundColor Red
    exit 1
}

# Verificar herramientas necesarias
Write-Host "🔍 Verificando herramientas necesarias..." -ForegroundColor Yellow
Write-Host ""

# Node.js
try {
    $nodeVersion = node --version
    Write-Host "✅ Node.js: $nodeVersion" -ForegroundColor Green
} catch {
    Write-Host "❌ Node.js no encontrado. Instalar desde: https://nodejs.org/" -ForegroundColor Red
    exit 1
}

# pnpm
try {
    $pnpmVersion = pnpm --version
    Write-Host "✅ pnpm: v$pnpmVersion" -ForegroundColor Green
} catch {
    Write-Host "❌ pnpm no encontrado. Ejecutar: npm install -g pnpm" -ForegroundColor Red
    exit 1
}

# Rust
try {
    $rustVersion = rustc --version
    Write-Host "✅ Rust: $rustVersion" -ForegroundColor Green
} catch {
    Write-Host "❌ Rust no encontrado. Instalar desde: https://rustup.rs/" -ForegroundColor Red
    exit 1
}

# Cargo
try {
    $cargoVersion = cargo --version
    Write-Host "✅ Cargo: $cargoVersion" -ForegroundColor Green
} catch {
    Write-Host "❌ Cargo no encontrado. Instalar Rust desde: https://rustup.rs/" -ForegroundColor Red
    exit 1
}

Write-Host ""
Write-Host "📦 Todas las herramientas están instaladas!" -ForegroundColor Green
Write-Host ""

# Preguntar tipo de build
Write-Host "🎯 Selecciona el tipo de build:" -ForegroundColor Cyan
Write-Host "1) Build completo (Frontend + Backend + Instaladores MSI/NSIS)" -ForegroundColor White
Write-Host "2) Solo binario EXE (más rápido, sin instaladores)" -ForegroundColor White
Write-Host "3) Solo Frontend (para desarrollo)" -ForegroundColor White
Write-Host ""
$opcion = Read-Host "Opción [1-3]"

if ($opcion -eq "3") {
    Write-Host ""
    Write-Host "🔨 Compilando solo Frontend..." -ForegroundColor Yellow
    pnpm run build

    if ($LASTEXITCODE -eq 0) {
        Write-Host ""
        Write-Host "✅ Frontend compilado exitosamente!" -ForegroundColor Green
        Write-Host "📁 Ubicación: dist/" -ForegroundColor Cyan
    } else {
        Write-Host ""
        Write-Host "❌ Error compilando Frontend" -ForegroundColor Red
        exit 1
    }
    exit 0
}

# Instalar dependencias si no existen
if (-Not (Test-Path "node_modules")) {
    Write-Host ""
    Write-Host "📥 Instalando dependencias de Node..." -ForegroundColor Yellow
    pnpm install

    if ($LASTEXITCODE -ne 0) {
        Write-Host "❌ Error instalando dependencias" -ForegroundColor Red
        exit 1
    }
}

# Limpiar build anterior (opcional)
Write-Host ""
$limpiar = Read-Host "¿Limpiar builds anteriores? (recomendado para build limpio) [s/N]"
if ($limpiar -eq "s" -or $limpiar -eq "S") {
    Write-Host "🧹 Limpiando builds anteriores..." -ForegroundColor Yellow

    if (Test-Path "src-tauri\target") {
        Remove-Item -Recurse -Force "src-tauri\target"
    }

    if (Test-Path "dist") {
        Remove-Item -Recurse -Force "dist"
    }

    Write-Host "✅ Builds anteriores eliminados" -ForegroundColor Green
}

Write-Host ""
Write-Host "🚀 Iniciando compilación..." -ForegroundColor Yellow
Write-Host "⏱️  Esto puede tomar 10-20 minutos en la primera compilación" -ForegroundColor Yellow
Write-Host ""

$startTime = Get-Date

if ($opcion -eq "2") {
    # Solo binario
    Write-Host "🔨 Compilando binario EXE..." -ForegroundColor Cyan

    # Primero compilar frontend
    pnpm run build
    if ($LASTEXITCODE -ne 0) {
        Write-Host "❌ Error compilando Frontend" -ForegroundColor Red
        exit 1
    }

    # Luego compilar backend
    cargo build --release --manifest-path src-tauri\Cargo.toml

    if ($LASTEXITCODE -eq 0) {
        $endTime = Get-Date
        $duration = $endTime - $startTime

        Write-Host ""
        Write-Host "✅ Compilación exitosa! ⏱️  $($duration.Minutes)m $($duration.Seconds)s" -ForegroundColor Green
        Write-Host ""
        Write-Host "📁 Binario generado:" -ForegroundColor Cyan
        Write-Host "   src-tauri\target\release\gestor-proyectos.exe" -ForegroundColor White

        $exeSize = (Get-Item "src-tauri\target\release\gestor-proyectos.exe").Length / 1MB
        Write-Host "   Tamaño: $([math]::Round($exeSize, 2)) MB" -ForegroundColor Gray
    } else {
        Write-Host ""
        Write-Host "❌ Error en la compilación" -ForegroundColor Red
        exit 1
    }
} else {
    # Build completo
    Write-Host "🔨 Compilando proyecto completo (Frontend + Backend + Instaladores)..." -ForegroundColor Cyan

    pnpm run tauri build

    if ($LASTEXITCODE -eq 0) {
        $endTime = Get-Date
        $duration = $endTime - $startTime

        Write-Host ""
        Write-Host "✅ Compilación exitosa! ⏱️  $($duration.Minutes)m $($duration.Seconds)s" -ForegroundColor Green
        Write-Host ""
        Write-Host "📦 Artefactos generados:" -ForegroundColor Cyan
        Write-Host ""

        # Binario EXE
        if (Test-Path "src-tauri\target\release\gestor-proyectos.exe") {
            $exeSize = (Get-Item "src-tauri\target\release\gestor-proyectos.exe").Length / 1MB
            Write-Host "   ✅ Binario EXE:" -ForegroundColor Green
            Write-Host "      src-tauri\target\release\gestor-proyectos.exe" -ForegroundColor White
            Write-Host "      Tamaño: $([math]::Round($exeSize, 2)) MB" -ForegroundColor Gray
            Write-Host ""
        }

        # Instalador MSI
        $msiPath = Get-ChildItem -Path "src-tauri\target\release\bundle\msi" -Filter "*.msi" -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($msiPath) {
            $msiSize = $msiPath.Length / 1MB
            Write-Host "   ✅ Instalador MSI (Windows Installer):" -ForegroundColor Green
            Write-Host "      $($msiPath.FullName)" -ForegroundColor White
            Write-Host "      Tamaño: $([math]::Round($msiSize, 2)) MB" -ForegroundColor Gray
            Write-Host ""
        }

        # Instalador NSIS
        $nsisPath = Get-ChildItem -Path "src-tauri\target\release\bundle\nsis" -Filter "*-setup.exe" -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($nsisPath) {
            $nsisSize = $nsisPath.Length / 1MB
            Write-Host "   ✅ Instalador NSIS:" -ForegroundColor Green
            Write-Host "      $($nsisPath.FullName)" -ForegroundColor White
            Write-Host "      Tamaño: $([math]::Round($nsisSize, 2)) MB" -ForegroundColor Gray
            Write-Host ""
        }

        Write-Host ""
        Write-Host "🎉 Build completo finalizado!" -ForegroundColor Green
        Write-Host ""
        Write-Host "📋 Próximos pasos:" -ForegroundColor Cyan
        Write-Host "   1. Probar el instalador MSI en Windows" -ForegroundColor White
        Write-Host "   2. Verificar que la aplicación funciona correctamente" -ForegroundColor White
        Write-Host "   3. (Opcional) Firmar digitalmente los instaladores" -ForegroundColor White
        Write-Host "   4. Distribuir a los usuarios finales" -ForegroundColor White
        Write-Host ""

        # Preguntar si quiere copiar artefactos a carpeta de distribución
        $copiar = Read-Host "¿Copiar instaladores a carpeta 'distribucion' para fácil acceso? [s/N]"
        if ($copiar -eq "s" -or $copiar -eq "S") {
            Write-Host ""
            Write-Host "📂 Creando carpeta de distribución..." -ForegroundColor Yellow

            if (-Not (Test-Path "distribucion")) {
                New-Item -ItemType Directory -Path "distribucion" | Out-Null
            }

            # Copiar MSI
            if ($msiPath) {
                Copy-Item $msiPath.FullName -Destination "distribucion\" -Force
                Write-Host "   ✅ MSI copiado a: distribucion\$($msiPath.Name)" -ForegroundColor Green
            }

            # Copiar NSIS
            if ($nsisPath) {
                Copy-Item $nsisPath.FullName -Destination "distribucion\" -Force
                Write-Host "   ✅ NSIS copiado a: distribucion\$($nsisPath.Name)" -ForegroundColor Green
            }

            # Copiar EXE
            if (Test-Path "src-tauri\target\release\gestor-proyectos.exe") {
                Copy-Item "src-tauri\target\release\gestor-proyectos.exe" -Destination "distribucion\" -Force
                Write-Host "   ✅ EXE copiado a: distribucion\gestor-proyectos.exe" -ForegroundColor Green
            }

            Write-Host ""
            Write-Host "✅ Artefactos copiados a la carpeta 'distribucion'" -ForegroundColor Green
        }
    } else {
        Write-Host ""
        Write-Host "❌ Error en la compilación" -ForegroundColor Red
        Write-Host ""
        Write-Host "💡 Consejos para solucionar problemas:" -ForegroundColor Yellow
        Write-Host "   1. Verificar que Visual Studio Build Tools está instalado" -ForegroundColor White
        Write-Host "   2. Revisar que WebView2 está instalado" -ForegroundColor White
        Write-Host "   3. Intentar con build limpio (opción de limpiar builds)" -ForegroundColor White
        Write-Host "   4. Ver logs completos arriba para más detalles" -ForegroundColor White
        Write-Host ""
        exit 1
    }
}

Write-Host ""
Write-Host "🎊 ¡Proceso completado!" -ForegroundColor Green
Write-Host ""
