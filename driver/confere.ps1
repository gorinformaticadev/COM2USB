# Confere que os arquivos em driver/ch340 sao byte a byte identicos aos do
# Driver Store de onde foram extraidos (nenhuma modificacao no pacote da WCH).
#   powershell -NoProfile -ExecutionPolicy Bypass -File driver\confere.ps1
param(
    [string]$Origem = 'C:\Windows\System32\DriverStore\FileRepository\ch341ser.inf_amd64_34a3206305b40d57'
)

$copias = Join-Path $PSScriptRoot 'ch340'
if (-not (Test-Path $Origem)) { throw "pasta do Driver Store nao encontrada: $Origem" }

foreach ($f in (Get-ChildItem $copias -File | Where-Object Name -ne 'AVISO.md')) {
    $o = Join-Path $Origem $f.Name
    if (-not (Test-Path $o)) {
        '{0,-22} AUSENTE NA ORIGEM' -f $f.Name
        continue
    }
    $h1 = (Get-FileHash -Algorithm SHA256 $o).Hash
    $h2 = (Get-FileHash -Algorithm SHA256 $f.FullName).Hash
    '{0,-22} {1}' -f $f.Name, $(if ($h1 -eq $h2) { 'IDENTICO' } else { "DIFERE  $h1 / $h2" })
}

$sobram = (Get-ChildItem $Origem -File | Where-Object { -not (Test-Path (Join-Path $copias $_.Name)) }).Name
if ($sobram) { 'nao copiados da origem: ' + ($sobram -join ', ') }
