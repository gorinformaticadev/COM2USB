# Gerar instalador do COM2USB.
#   powershell -NoProfile -ExecutionPolicy Bypass -File installer\gerar-instalador.ps1
#   -SemBuild  : nao recompila o Rust, so empacota o executavel que ja existe
param(
    [switch]$SemBuild,
    [string]$Exe
)

$ErrorActionPreference = 'Stop'

$raiz = Split-Path -Parent $PSScriptRoot
$installer = Join-Path $raiz 'installer'
if (-not $Exe) { $Exe = Join-Path $raiz 'src-tauri\target\release\com2usb.exe' }
$Exe = (Resolve-Path $Exe -ErrorAction SilentlyContinue).Path
if (-not $Exe) { throw "executavel do app nao encontrado: reforce o build ou passe -Exe" }
$versao = (Get-Content (Join-Path $raiz 'src-tauri\tauri.conf.json') -Raw | ConvertFrom-Json).version

# 0. O app em execucao trava o executavel e o cargo falha com "os error 5".
$rodando = Get-Process -Name 'com2usb', 'COM2USB' -ErrorAction SilentlyContinue
if ($rodando -and -not $SemBuild) {
    Write-Host ("O COM2USB esta em execucao (pid {0}). Feche pelo menu Sair da bandeja e rode de novo, " -f ($rodando.Id -join ',')) -ForegroundColor Yellow
    Write-Host 'ou empacote um executavel ja compilado com -SemBuild -Exe <caminho>.' -ForegroundColor Yellow
    exit 1
}

# 1. Compilar o aplicativo sem gerar o bundle do proprio Tauri.
if (-not $SemBuild) {
    Write-Host "==> tauri build --no-bundle" -ForegroundColor Cyan
    Push-Location $raiz
    try { & (Join-Path $raiz 'node_modules\.bin\tauri.cmd') build --no-bundle
          if ($LASTEXITCODE -ne 0) { throw "tauri build falhou (codigo $LASTEXITCODE)" } }
    finally { Pop-Location }
}
if (-not (Test-Path $Exe)) { throw "falta o executavel: $Exe" }

# 2. Precisa do pacote CH340 versionado no repositorio.
$pacote = Join-Path $raiz 'driver\ch340\CH341SER.INF'
if (-not (Test-Path $pacote)) { throw "falta o pacote do driver: $pacote" }

# 3. Bootstrapper silencioso do WebView2: reaproveita o cache do Tauri.
$cache = Join-Path $installer 'cache'
New-Item -ItemType Directory -Force -Path $cache | Out-Null
$webview2 = Join-Path $cache 'MicrosoftEdgeWebview2Setup.exe'
if (-not (Test-Path $webview2)) {
    $origem = Join-Path $env:LOCALAPPDATA 'tauri\MicrosoftEdgeWebview2Setup.exe'
    if (-not (Test-Path $origem)) {
        throw "falta o bootstrapper do WebView2. Baixe https://go.microsoft.com/fwlink/p/?LinkId=2124703 para $origem"
    }
    Copy-Item $origem $webview2
}

# 4. makensis: no PATH ou no cache do Tauri.
$makensis = (Get-Command makensis.exe -ErrorAction SilentlyContinue).Source
if (-not $makensis) { $makensis = Join-Path $env:LOCALAPPDATA 'tauri\NSIS\Bin\makensis.exe' }
if (-not (Test-Path $makensis)) { throw "makensis nao encontrado" }

# 5. Compilar o instalador.
Write-Host "==> makensis" -ForegroundColor Cyan
Push-Location $installer
try {
    & $makensis /V2 "/DEXE_FONTE=$Exe" com2usb.nsi
    if ($LASTEXITCODE -ne 0) { throw "makensis falhou (codigo $LASTEXITCODE)" }
} finally { Pop-Location }

# 6. Resumo do artefato.
$setup = Join-Path $installer "dist\COM2USB_$($versao)_setup.exe"
if (-not (Test-Path $setup)) { throw "instalador nao foi gerado: $setup" }
$info = Get-Item $setup
$sha = (Get-FileHash -Algorithm SHA256 $setup).Hash
Write-Host ""
Write-Host "OK: $setup" -ForegroundColor Green
Write-Host ("   {0:N0} bytes  compilado {1:yyyy-MM-dd HH:mm:ss}" -f $info.Length, $info.LastWriteTime)
Write-Host "   SHA-256 $sha"
