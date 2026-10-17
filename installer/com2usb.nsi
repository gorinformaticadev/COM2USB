; ============================================================================
; COM2USB — instalador para Windows x64
;
; A página "Escolha de componentes" oferece:
;   * Programa COM2USB ................ sempre marcado (SectionIn RO)
;   * Instalar o driver USB-SERIAL CH340 ... desmarcado por padrão (Section /o)
;   * Iniciar com o Windows (tarefa de logon elevada) .. marcada por padrão
;
; Build: installer\gerar-instalador.ps1
; ============================================================================

Unicode true

!include "MUI2.nsh"
!include "LogicLib.nsh"
!include "Sections.nsh"
!include "x64.nsh"

!define APPNAME "COM2USB"
!define VERSION "1.0.0"
!define PUBLISHER "Pluggor Tecnologia LTDA"
!define EXE "COM2USB.exe"
!define WEBSITE "https://github.com/gorinformaticadev/com2usb"
!define UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPNAME}"
!define WEBVIEW2APPGUID "{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}"
; Caminho do executável compilado; o script de build pode sobrepô-lo com /DEXE_FONTE.
!ifndef EXE_FONTE
  !define EXE_FONTE "..\src-tauri\target\release\com2usb.exe"
!endif
; Identificador da pasta deste pacote no Driver Store (o nome é derivado do
; conteúdo do INF, então ele só existe se este exato driver já estiver lá).
!define CH340_PASTA_STORE "ch341ser.inf_amd64_34a3206305b40d57"

Name "${APPNAME} ${VERSION}"
OutFile "dist\${APPNAME}_${VERSION}_setup.exe"
InstallDir "$PROGRAMFILES64\${APPNAME}"
InstallDirRegKey HKLM "${UNINST_KEY}" "InstallLocation"
RequestExecutionLevel admin
SetCompressor /SOLID lzma
ShowInstDetails show
ShowUninstDetails show

VIProductVersion "${VERSION}.0"
VIAddVersionKey /LANG=1046 "ProductName" "${APPNAME}"
VIAddVersionKey /LANG=1046 "ProductVersion" "${VERSION}"
VIAddVersionKey /LANG=1046 "FileVersion" "${VERSION}"
VIAddVersionKey /LANG=1046 "CompanyName" "${PUBLISHER}"
VIAddVersionKey /LANG=1046 "LegalCopyright" "© 2026 ${PUBLISHER}"
VIAddVersionKey /LANG=1046 "FileDescription" "Instalador do ${APPNAME}"
VIAddVersionKey /LANG=1046 "OriginalFilename" "${APPNAME}_${VERSION}_setup.exe"

!define MUI_ICON "..\src-tauri\icons\icon.ico"
!define MUI_UNICON "..\src-tauri\icons\icon.ico"
!define MUI_ABORTWARNING
!define MUI_WELCOMEPAGE_TITLE "Bem-vindo ao instalador do ${APPNAME} ${VERSION}"
!define MUI_WELCOMEPAGE_TEXT "Este instalador copia o ${APPNAME} para o computador, cria o atalho do menu Iniciar e registra a tarefa de logon que o inicia oculto, já elevado, sem pedir confirmação do Windows.$\r$\n$\r$\nNa bandeja, o ${APPNAME} lê o código de barras na porta serial e o digita no documento que estiver em foco. Ele fecha apenas pelo menu Sair.$\r$\n$\r$\nRecomendamos fechar a janela do ${APPNAME} antes de continuar."
!define MUI_COMPONENTSPAGE_SMALLDESC
!define MUI_FINISHPAGE_RUN "$INSTDIR\${EXE}"
!define MUI_FINISHPAGE_RUN_TEXT "Abrir o ${APPNAME} agora"
!define MUI_FINISHPAGE_TITLE "Instalação concluída"
!define MUI_FINISHPAGE_TEXT "O ${APPNAME} está na bandeja. A porta padrão é COM4: ajuste em Propriedades, assim como a velocidade, a paridade e as demais opções.$\r$\n$\r$\nAs leituras ficam em $INSTDIR\COM2USB.rec e a inicialização é registrada em $INSTDIR\COM2USB.log."

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "..\LICENSE"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "PortugueseBR"

Var CodSaida
Var SaidaPnputil

; ---------------------------------------------------------------------------
; Programa + atalhos + chaves do Painel de Controle.
; ---------------------------------------------------------------------------
Section "Programa ${APPNAME}" SecApp
  SectionIn RO

  SetOutPath "$INSTDIR"
  File /oname=${EXE} "${EXE_FONTE}"
  File "..\LICENSE"

  ; O programa grava COM2USB.INI, COM2USB.log e COM2USB.rec ao lado do
  ; executável. Só criamos os três na primeira instalação: reinstalar ou
  ; atualizar preserva a porta escolhida pelo usuário e o histórico de
  ; leituras, que são dados dele, não peças do pacote.
  ${IfNot} ${FileExists} "$INSTDIR\COM2USB.INI"
    WriteINIStr "$INSTDIR\COM2USB.INI" "Settings" "Porta" "COM4"
    WriteINIStr "$INSTDIR\COM2USB.INI" "Settings" "Velocidade" "9600"
    WriteINIStr "$INSTDIR\COM2USB.INI" "Settings" "Paridade" "sem"
    WriteINIStr "$INSTDIR\COM2USB.INI" "Settings" "BitsDados" "8"
    WriteINIStr "$INSTDIR\COM2USB.INI" "Settings" "StopsBits" "2"
    WriteINIStr "$INSTDIR\COM2USB.INI" "Settings" "Ativo" "0"
    WriteINIStr "$INSTDIR\COM2USB.INI" "Settings" "Gravar" "1"
    WriteINIStr "$INSTDIR\COM2USB.INI" "Settings" "Deduplicar" "1"
    WriteINIStr "$INSTDIR\COM2USB.INI" "Settings" "Digitar" "1"
    WriteINIStr "$INSTDIR\COM2USB.INI" "Settings" "Enter" "1"
  ${EndIf}
  ${IfNot} ${FileExists} "$INSTDIR\COM2USB.log"
    FileOpen $0 "$INSTDIR\COM2USB.log" w
    FileClose $0
  ${EndIf}
  ${IfNot} ${FileExists} "$INSTDIR\COM2USB.rec"
    FileOpen $0 "$INSTDIR\COM2USB.rec" w
    FileClose $0
  ${EndIf}
  ; Escrita concedida aos Usuários (por SID, para não depender do idioma) para
  ; que uma instância aberta com dois cliques, sem elevação, consiga salvar a
  ; configuração. O icacls aceita UM arquivo por chamada, daí as três linhas.
  nsExec::ExecToLog 'icacls.exe "$INSTDIR\COM2USB.INI" /grant *S-1-5-32-545:M'
  nsExec::ExecToLog 'icacls.exe "$INSTDIR\COM2USB.log" /grant *S-1-5-32-545:M'
  nsExec::ExecToLog 'icacls.exe "$INSTDIR\COM2USB.rec" /grant *S-1-5-32-545:M'

  WriteUninstaller "$INSTDIR\uninstall.exe"
  CreateDirectory "$SMPROGRAMS\${APPNAME}"
  CreateShortCut "$SMPROGRAMS\${APPNAME}\${APPNAME}.lnk" "$INSTDIR\${EXE}"
  CreateShortCut "$SMPROGRAMS\${APPNAME}\Desinstalar ${APPNAME}.lnk" "$INSTDIR\uninstall.exe"

  WriteRegStr HKLM "${UNINST_KEY}" "DisplayName" "${APPNAME}"
  WriteRegStr HKLM "${UNINST_KEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKLM "${UNINST_KEY}" "Publisher" "${PUBLISHER}"
  WriteRegStr HKLM "${UNINST_KEY}" "URLInfoAbout" "${WEBSITE}"
  WriteRegStr HKLM "${UNINST_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKLM "${UNINST_KEY}" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegDWORD HKLM "${UNINST_KEY}" "NoModify" 1
  WriteRegDWORD HKLM "${UNINST_KEY}" "NoRepair" 1

  ; O aplicativo depende do WebView2. Se ainda não houver runtime, usamos o
  ; instalador silencioso da Microsoft embutido neste pacote.
  ReadRegStr $1 HKLM "SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\${WEBVIEW2APPGUID}" "pv"
  ${If} $1 == ""
    ReadRegStr $1 HKLM "SOFTWARE\Microsoft\EdgeUpdate\Clients\${WEBVIEW2APPGUID}" "pv"
  ${EndIf}
  ${If} $1 == ""
    ReadRegStr $1 HKCU "SOFTWARE\Microsoft\EdgeUpdate\Clients\${WEBVIEW2APPGUID}" "pv"
  ${EndIf}
  ${If} $1 == ""
    DetailPrint "Instalando o tempo de execução do WebView2 (silencioso)..."
    File /oname=$PLUGINSDIR\MicrosoftEdgeWebview2Setup.exe "cache\MicrosoftEdgeWebview2Setup.exe"
    ExecWait '"$PLUGINSDIR\MicrosoftEdgeWebview2Setup.exe" /silent /install' $CodSaida
    ${If} $CodSaida != 0
      DetailPrint "O WebView2 não pôde ser instalado (código $CodSaida). O Windows 10 já costuma trazê-lo."
    ${Else}
      DetailPrint "WebView2 instalado."
    ${EndIf}
  ${Else}
    DetailPrint "WebView2 já presente (versão $1)."
  ${EndIf}
SectionEnd

; ---------------------------------------------------------------------------
; Driver CH340: opcional e desmarcado por padrão.
; ---------------------------------------------------------------------------
Section /o "Instalar o driver USB-SERIAL CH340 (WCH 3.9.2024.09)" SecDriver
  ${IfNot} ${RunningX64}
    DetailPrint "Este pacote de driver só vale para Windows 64 bits; opção ignorada."
  ${ElseIf} ${FileExists} "$WINDIR\System32\DriverStore\FileRepository\${CH340_PASTA_STORE}\*.*"
    DetailPrint "O pacote CH340 3.9.2024.09 já está no Driver Store; nada foi alterado."
  ${Else}
    SetOutPath "$PLUGINSDIR\ch340"
    File /r "..\driver\ch340\CH341*.*"
    DetailPrint "Adicionando o pacote ao Driver Store e vinculando-o aos dispositivos presentes..."
    nsExec::ExecToStack '"$SYSDIR\pnputil.exe" /add-driver "$PLUGINSDIR\ch340\CH341SER.INF" /install'
    Pop $CodSaida
    Pop $SaidaPnputil
    DetailPrint "pnputil: $SaidaPnputil"
    DetailPrint "pnputil saiu com código $CodSaida"
    ${If} $CodSaida != 0
      MessageBox MB_OK|MB_ICONEXCLAMATION \
        "O Windows recusou a instalação do driver (código $CodSaida).$\r$\n$\r$\n\
         O ${APPNAME} foi instalado normalmente. Instale o driver CH340 com o pacote\
         oficial da WCH ou pelo Gerenciador de Dispositivos e reconecte o adaptador."
    ${EndIf}
  ${EndIf}
SectionEnd

; ---------------------------------------------------------------------------
; Tarefa de logon elevada: quem faz é o próprio programa, com o mesmo código
; do checkbox "Iniciar com o Windows". O instalador já está elevado, então não
; aparece nova confirmação do Windows.
; ---------------------------------------------------------------------------
Section "Iniciar com o Windows (tarefa de logon elevada)" SecAutostart
  ExecWait '"$INSTDIR\${EXE}" --registrar-autostart' $CodSaida
  nsExec::ExecToStack '"$SYSDIR\schtasks.exe" /Query /TN ${APPNAME}'
  Pop $CodSaida
  ${If} $CodSaida != 0
    MessageBox MB_OK|MB_ICONEXCLAMATION \
      "O ${APPNAME} foi instalado, mas a tarefa de logon ${APPNAME} não ficou registrada.$\r$\n$\r$\n\
       Abra o ${APPNAME} na bandeja, em Propriedades, e marque 'Iniciar com o Windows'."
  ${Else}
    DetailPrint "Tarefa de logon ${APPNAME} registrada."
  ${EndIf}
SectionEnd

!insertmacro MUI_FUNCTION_DESCRIPTION_BEGIN
  !insertmacro MUI_DESCRIPTION_TEXT ${SecApp} \
    "O programa em si. Não pode ser desmarcado."
  !insertmacro MUI_DESCRIPTION_TEXT ${SecDriver} \
    "Adiciona ao Driver Store o pacote WCH 3.9.2024.09 para adaptadores USB-serial$\r$\n\
     CH340/CH341/CH341A/CH330 (VID 1A86). Marque apenas se o Windows ainda não$\r$\n\
     reconheceu o adaptador: se este pacote já estiver instalado, nada é alterado.$\r$\n\
     A desinstalação do ${APPNAME} nunca remove pacotes de driver."
  !insertmacro MUI_DESCRIPTION_TEXT ${SecAutostart} \
    "Cria a tarefa de logon que abre o ${APPNAME} oculto, na bandeja, já elevado e$\r$\n\
     sem a confirmação do Windows. Dá para desligar depois em Propriedades."
!insertmacro MUI_FUNCTION_DESCRIPTION_END

; ---------------------------------------------------------------------------
Section Uninstall
  ; Primeiro o programa remove a própria tarefa; por isso o executável ainda
  ; precisa existir nesta altura da desinstalação.
  ExecWait '"$INSTDIR\${EXE}" --remover-autostart' $CodSaida

  Delete "$SMPROGRAMS\${APPNAME}\${APPNAME}.lnk"
  Delete "$SMPROGRAMS\${APPNAME}\Desinstalar ${APPNAME}.lnk"
  RMDir "$SMPROGRAMS\${APPNAME}"
  Delete "$INSTDIR\${EXE}"
  Delete "$INSTDIR\LICENSE"
  Delete "$INSTDIR\COM2USB.INI"
  Delete "$INSTDIR\COM2USB.log"
  Delete "$INSTDIR\COM2USB.rec"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"
  DeleteRegKey HKLM "${UNINST_KEY}"

  ; De propósito: nenhum pnputil /delete-driver aqui. Remover pacote do Driver
  ; Store é operação destrutiva e pode afetar outros aparelhos da máquina.
  DetailPrint "Pacotes de driver não foram alterados pela desinstalação."
SectionEnd
