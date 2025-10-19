# Gestor de Proyectos - Windows Installation Script
# Version: 0.2.1
# Requiere PowerShell 5.1 o superior

param(
    [switch]$SkipDependencies,
    [switch]$Dev,
    [string]$InstallPath = "$env:LOCALAPPDATA\GestorProyectos"
)

$ErrorActionPreference = "Stop"

# Colores para output
function Write-Success { param($Message) Write-Host "✓ $Message" -ForegroundColor Green }
function Write-Info { param($Message) Write-Host "ℹ $Message" -ForegroundColor Cyan }
function Write-Warning { param($Message) Write-Host "⚠ $Message" -ForegroundColor Yellow }
function Write-Error { param($Message) Write-Host "✗ $Message" -ForegroundColor Red }

Write-Host "`n╔════════════════════════════════════════════════╗" -ForegroundColor Magenta
Write-Host "║   Gestor de Proyectos v0.2.1                   ║" -ForegroundColor Magenta
Write-Host "║   Instalador para Windows 10/11                ║" -ForegroundColor Magenta
Write-Host "╚════════════════════════════════════════════════╝`n" -ForegroundColor Magenta

# Verificar PowerShell version
if ($PSVersionTable.PSVersion.Major -lt 5) {
    Write-Error "Se requiere PowerShell 5.1 o superior"
    Write-Error "Tu versión: $($PSVersionTable.PSVersion)"
    exit 1
}
Write-Success "PowerShell $($PSVersionTable.PSVersion) detectado"

# Verificar permisos de administrador (opcional, solo advertencia)
$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) {
    Write-Warning "No se está ejecutando como Administrador"
    Write-Warning "Algunas funciones pueden requerir permisos elevados"
}

# Crear directorio de instalación
Write-Info "Directorio de instalación: $InstallPath"
if (-not (Test-Path $InstallPath)) {
    New-Item -ItemType Directory -Path $InstallPath -Force | Out-Null
    Write-Success "Directorio de instalación creado"
} else {
    Write-Info "El directorio de instalación ya existe"
}

# Función para verificar si un comando existe
function Test-Command {
    param($Command)
    try {
        if (Get-Command $Command -ErrorAction Stop) { return $true }
    } catch {
        return $false
    }
}

# Verificar e instalar dependencias
if (-not $SkipDependencies) {
    Write-Info "`nVerificando dependencias...`n"

    # Verificar WebView2
    Write-Info "Verificando WebView2..."
    $webview2Path = "$env:ProgramFiles (x86)\Microsoft\EdgeWebView\Application"
    if (Test-Path $webview2Path) {
        Write-Success "WebView2 encontrado"
    } else {
        Write-Warning "WebView2 no encontrado"
        Write-Info "Descargando WebView2 Runtime..."
        $webview2Url = "https://go.microsoft.com/fwlink/p/?LinkId=2124703"
        $webview2Installer = "$env:TEMP\MicrosoftEdgeWebview2Setup.exe"

        try {
            Invoke-WebRequest -Uri $webview2Url -OutFile $webview2Installer
            Write-Info "Instalando WebView2..."
            Start-Process -FilePath $webview2Installer -ArgumentList "/silent /install" -Wait
            Remove-Item $webview2Installer
            Write-Success "WebView2 instalado correctamente"
        } catch {
            Write-Error "Error al instalar WebView2: $_"
            Write-Info "Descarga manual desde: https://developer.microsoft.com/en-us/microsoft-edge/webview2/"
        }
    }

    # Verificar Node.js (solo para desarrollo)
    if ($Dev) {
        Write-Info "`nVerificando Node.js..."
        if (Test-Command "node") {
            $nodeVersion = node --version
            Write-Success "Node.js $nodeVersion encontrado"
        } else {
            Write-Warning "Node.js no encontrado"
            Write-Info "Instalando Node.js vía winget..."
            if (Test-Command "winget") {
                winget install OpenJS.NodeJS.LTS
                Write-Success "Node.js instalado"
            } else {
                Write-Warning "winget no encontrado. Instala Node.js manualmente:"
                Write-Info "https://nodejs.org/"
            }
        }

        # Verificar pnpm
        Write-Info "Verificando pnpm..."
        if (Test-Command "pnpm") {
            $pnpmVersion = pnpm --version
            Write-Success "pnpm $pnpmVersion encontrado"
        } else {
            Write-Warning "pnpm no encontrado"
            Write-Info "Instalando pnpm..."
            npm install -g pnpm
            Write-Success "pnpm instalado"
        }

        # Verificar Rust
        Write-Info "Verificando Rust..."
        if (Test-Command "cargo") {
            $rustVersion = cargo --version
            Write-Success "$rustVersion encontrado"
        } else {
            Write-Warning "Rust no encontrado"
            Write-Info "Instalando Rust vía winget..."
            if (Test-Command "winget") {
                winget install Rustlang.Rustup
                Write-Success "Rust instalado"
                Write-Warning "IMPORTANTE: Reinicia PowerShell para que Rust esté disponible"
            } else {
                Write-Warning "Instala Rust manualmente:"
                Write-Info "https://www.rust-lang.org/tools/install"
            }
        }
    }
}

# Si es modo desarrollo, compilar desde fuente
if ($Dev) {
    Write-Info "`n═══════════════════════════════════════"
    Write-Info "   MODO DESARROLLO - Compilando desde fuente"
    Write-Info "═══════════════════════════════════════`n"

    $sourceDir = Join-Path $PSScriptRoot "..\..\source"
    if (-not (Test-Path $sourceDir)) {
        Write-Error "No se encontró el directorio 'source'"
        Write-Error "Asegúrate de que el código fuente esté en: $sourceDir"
        exit 1
    }

    Push-Location $sourceDir

    Write-Info "Instalando dependencias de Node.js..."
    pnpm install
    Write-Success "Dependencias instaladas"

    Write-Info "Compilando aplicación (esto puede tomar varios minutos)..."
    pnpm run tauri:build

    $msiPath = "src-tauri\target\release\bundle\msi\*.msi"
    if (Test-Path $msiPath) {
        $msi = Get-Item $msiPath | Select-Object -First 1
        Write-Success "Build completado: $($msi.Name)"
        Write-Info "`nInstalador generado en:"
        Write-Host "  $($msi.FullName)" -ForegroundColor Yellow

        # Preguntar si instalar
        $install = Read-Host "`n¿Deseas instalar ahora? (S/N)"
        if ($install -eq "S" -or $install -eq "s") {
            Write-Info "Instalando..."
            Start-Process -FilePath msiexec.exe -ArgumentList "/i `"$($msi.FullName)`" /quiet" -Wait
            Write-Success "Instalación completada"
        }
    } else {
        Write-Error "No se encontró el instalador MSI"
        Write-Error "Verifica los logs de compilación"
    }

    Pop-Location
} else {
    # Instalación desde binarios pre-compilados
    Write-Info "`n═══════════════════════════════════════"
    Write-Info "   Buscando instalador MSI..."
    Write-Info "═══════════════════════════════════════`n"

    $msiPath = Join-Path $PSScriptRoot "..\*.msi"
    $msi = Get-Item $msiPath -ErrorAction SilentlyContinue | Select-Object -First 1

    if ($msi) {
        Write-Success "Instalador encontrado: $($msi.Name)"
        Write-Info "Iniciando instalación..."

        Start-Process -FilePath msiexec.exe -ArgumentList "/i `"$($msi.FullName)`" /qn" -Wait

        Write-Success "`nInstalación completada exitosamente!"
        Write-Info "`nLa aplicación está disponible en:"
        Write-Host "  - Menú de Inicio: Gestor de Proyectos" -ForegroundColor Yellow
        Write-Host "  - Ruta: $env:LOCALAPPDATA\Programs\gestor-proyectos" -ForegroundColor Yellow
    } else {
        Write-Warning "No se encontró un instalador MSI pre-compilado"
        Write-Info "`nOpciones:"
        Write-Host "  1. Ejecuta este script con -Dev para compilar desde fuente" -ForegroundColor Cyan
        Write-Host "  2. Descarga el instalador MSI desde el repositorio" -ForegroundColor Cyan
        Write-Host "  3. Copia el archivo .msi a este directorio" -ForegroundColor Cyan
        exit 1
    }
}

# Crear acceso directo en Desktop (opcional)
$createShortcut = Read-Host "`n¿Crear acceso directo en el Escritorio? (S/N)"
if ($createShortcut -eq "S" -or $createShortcut -eq "s") {
    $exePath = "$env:LOCALAPPDATA\Programs\gestor-proyectos\Gestor de Proyectos.exe"
    if (Test-Path $exePath) {
        $shell = New-Object -ComObject WScript.Shell
        $shortcut = $shell.CreateShortcut("$env:USERPROFILE\Desktop\Gestor de Proyectos.lnk")
        $shortcut.TargetPath = $exePath
        $shortcut.WorkingDirectory = "$env:LOCALAPPDATA\Programs\gestor-proyectos"
        $shortcut.Description = "Gestor de Proyectos v0.2.1"
        $shortcut.Save()
        Write-Success "Acceso directo creado en el Escritorio"
    } else {
        Write-Warning "No se pudo crear el acceso directo (ruta no encontrada)"
    }
}

Write-Host "`n╔════════════════════════════════════════════════╗" -ForegroundColor Green
Write-Host "║   ✓ Instalación completada exitosamente       ║" -ForegroundColor Green
Write-Host "╚════════════════════════════════════════════════╝`n" -ForegroundColor Green

Write-Info "Próximos pasos:"
Write-Host "  1. Abre 'Gestor de Proyectos' desde el Menú de Inicio" -ForegroundColor Cyan
Write-Host "  2. Completa el wizard de bienvenida" -ForegroundColor Cyan
Write-Host "  3. Configura tus programas en Settings → Programas" -ForegroundColor Cyan
Write-Host "  4. ¡Empieza a gestionar tus proyectos!" -ForegroundColor Cyan

Write-Host "`n¡Gracias por usar Gestor de Proyectos! 🚀`n" -ForegroundColor Magenta
