param([Parameter(Mandatory = $true)][string]$Caminho)
$bytes = [IO.File]::ReadAllBytes($Caminho)
$texto = [Text.Encoding]::ASCII.GetString($bytes)
foreach ($alvo in @(
    '--registrar-autostart', '--remover-autostart', 'sem reprogramar',
    'wedge_test', 'COM2USB.log', 'iniciando', 'autostart registrado', 'autostart recusado'
)) {
    $achou = $texto.Contains($alvo)
    '{0,-24} {1}' -f $alvo, $(if ($achou) { 'presente' } else { 'AUSENTE' })
}
'{0,-24} {1:N0} bytes, mtime {2:HH:mm:ss}' -f 'arquivo', (Get-Item $Caminho).Length, (Get-Item $Caminho).LastWriteTime
