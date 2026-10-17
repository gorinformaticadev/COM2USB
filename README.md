# COM2USB

**Capturador serial de leitor de código de barras para Windows — bandeja do sistema.**

O COM2USB mantém uma porta serial (normalmente o port COM virtual criado pelo driver
serial-USB de um leitor de código de barras) aberta em segundo plano, registra cada
leitura em arquivo e controla tudo pela bandeja do sistema — sem janela de console,
sem intervenção do usuário no dia a dia.

- **Versão:** 1.0.0 (versão final)
- **Desenvolvido em:** 2026
- **Proprietário:** PLUGGOR TECNOLOGIA LTDA
- **Licença:** GPL-3.0-only (ver arquivo [LICENSE](LICENSE))

## Funcionalidades

| Recurso | Comportamento |
|---|---|
| Bandeja (tray) | Código de barras com laser vermelho. **Sem o X = ativo (gravando); com X vermelho = inativo.** Menu: Ativar/Desativar, Propriedades..., Sair. |
| Captura serial | Lê a porta configurada (ex.: `COM4 9600 sem 8 2`). Uma leitura termina em CR/LF ou após 150 ms de silêncio. |
| Digitação no app em foco | Cada código é **digitado no aplicativo que estiver em foco** (teclado virtual via `SendInput`), exatamente como o SEALST fazia com `keybd_event`: o leitor não precisa de driver de teclado, e o sistema de vendas/planilha/editor recebe o texto sem o COM2USB roubar o foco. Enter opcional ao final. |
| Arquivo de leituras | Cada leitura vira uma linha em `COM2USB.rec`: `AAAA-MM-DD HH:MM:SS<TAB>código<TAB>DUP?`. |
| Duplicatas | Leitura idêntica dentro de 2 s é marcada `DUP` (opcional) e **não** é digitada de novo. |
| Configuração | Janela Propriedades, persistida em `COM2USB.INI` ao lado do executável (formato INI simples). |
| Autostart | Tarefa agendada `COM2USB` (gatilho no logon com 15 s de atraso, privilégio **mais elevado**, sem limite de execução, até 3 novas tentativas em falha, funciona na bateria): inicia **sem UAC e sem prompt**, oculto na bandeja. O registro da tarefa pede UAC **uma única vez**, só quando o app não está elevado. |
| Diagnóstico | `COM2USB.log` ao lado do executável registra inicialização, estado da bandeja, elevação e falhas de ativação da porta — inclusive panics. |
| Ciclo de vida | Fechar a janela apenas a oculta; o app só encerra pelo menu **Sair**. |

## Requisitos do sistema (execução)

- Windows 10/11 x64
- WebView2 Runtime (já incluso no Windows 11; o instalador baixa se necessário)
- Leitor de código de barras com interface serial-USB (driver que expõe porta COM)

## Como fazer o build

### Pré-requisitos de compilação

1. **Rust** (toolchain MSVC): <https://rustup.rs> — confirme com `rustc --version`
2. **Visual Studio Build Tools 2022** com a workload *Desktop development with C++*
   (inclui Windows SDK)
3. **Node.js 18+**
4. **NSIS 3.x** (`makensis.exe`) — só para o passo 4. O script usa o `makensis` do PATH ou,
   na falta dele, o NSIS que o próprio Tauri mantém em `%LOCALAPPDATA%\tauri\NSIS\Bin`.
   O bootstrapper silencioso do WebView2 também é reaproveitado desse cache
   (ou baixado de <https://go.microsoft.com/fwlink/p/?LinkId=2124703>).

Antes do passo 4, feche o COM2USB pelo menu **Sair** da bandeja: com o app aberto o `cargo`
não consegue sobrescrever `target\release\COM2USB.exe` ("os error 5"), e o script aborta com
uma mensagem dizendo exatamente isso. Para empacotar um executável já compilado em outro
lugar, use `-SemBuild -Exe <caminho>`.

### Passos

```bash
# 1. instalar dependências do frontend/CLI
npm install

# 2. regenerar os ícones (icon.png/icon.ico do app, tray.png/tray-off.png da bandeija)
npm run icons

# 3. desenvolvimento (recompilação automática)
npm run dev

# 4. instalador oficial (compila o app com `tauri build --no-bundle` e roda o makensis)
npm run instalador
```

(equivale a `powershell -NoProfile -ExecutionPolicy Bypass -File installer\gerar-instalador.ps1`)

O instalador é gerado em:

```
installer/dist/COM2USB_1.0.0_setup.exe
```

`npm run build` (o empacotador embutido do Tauri) **não** é o caminho oficial: ele gera um
instalador genérico em `src-tauri/target/release/bundle/nsis/` sem a opção de driver e sem
a tarefa de logon elevada. O script acima existe exatamente para empacotar as três coisas
(programa, driver opcional, autostart) num pacote só.

### Os drivers

Os drivers **não são compilados**: `driver/ch340/` guarda o pacote original da WCH
(`CH341SER.INF` + `.SYS` + `.DLL` + `.CAT`) extraído do Driver Store, que `gerar-instalador.ps1`
apenas valida e anexa ao NSIS. Para reconfirmar que nenhum byte foi alterado em relação à origem:

```bash
powershell -NoProfile -ExecutionPolicy Bypass -File driver\confere.ps1
```

O script imprime `IDENTICO` por arquivo (SHA-256 origem × cópia) e é somente-leitura — não toca
no Driver Store. O instalador incorpora o pacote, mas **não o instala** a menos que a opção
"Instalar o driver USB-SERIAL CH340" seja marcada na tela de componentes.

## O instalador

O instalador pede elevação **uma única vez** (o UAC é o do próprio instalador). Páginas:
bem-vindo, licença GPL-3.0, pasta de destino e **escolha de componentes**:

| Componente | Estado inicial | O que faz |
|---|---|---|
| Programa COM2USB | sempre marcado (não dá para desmarcar) | copia `COM2USB.exe` + `LICENSE`, cria atalhos no menu Iniciar e as chaves de desinstalação. Instala o WebView2 Runtime silencioso se a máquina ainda não tiver. |
| **Instalar o driver USB-SERIAL CH340 (WCH 3.9.2024.09)** | **desmarcado** | se marcado, roda `pnputil /add-driver CH341SER.INF /install` com o pacote que está em `driver/ch340`. Se esse exato pacote já estiver no Driver Store, **nada é alterado**. |
| Iniciar com o Windows (tarefa de logon elevada) | marcado | chama o próprio programa (`COM2USB.exe --registrar-autostart`), que cria a tarefa `\COM2USB` com o mesmo código do checkbox de Propriedades. Como o instalador já está elevado, não aparece segunda confirmação. |

Arquivos criados em `C:\Program Files\COM2USB`: `COM2USB.exe`, `COM2USB.INI`
(configuração), `COM2USB.log` (diagnóstico de inicialização), `COM2USB.rec`
(leituras), `LICENSE`, `uninstall.exe`. Os três arquivos de dados recebem permissão
de escrita para o grupo *Usuários*, para que uma instância aberta sem elevação
consiga salvar configuração. Reinstalar **preserva** esses três arquivos: a porta
escolhida e o histórico de leituras são dados do usuário, não peças do pacote.

O pacote do driver incluído é o arquivo **original da WCH**, copiado sem modificação
do Driver Store e conferido por SHA-256 — procedência e hashes em
[`driver/ch340/AVISO.md`](driver/ch340/AVISO.md). A **desinstalação não remove
pacotes de driver**: mexer no Driver Store é operação destrutiva e fica a critério
do operador (`pnputil /enum-drivers` / `pnputil /delete-driver oemNN.inf /uninstall`).

## Baixar o build pronto

Os binários compilados são anexos da **Release** deste repositório (nada de binário
dentro da árvore do git). Link permanente para a release mais recente:

| Arquivo | Para quem | Download direto |
|---|---|---|
| `COM2USB_1.0.0_setup.exe` (3.729.217 bytes) | instalação normal: programa + atalhos + chaves do Painel de Controle + tarefa de logon elevada, com a opção de driver CH340 **desmarcada** | <https://github.com/gorinformaticadev/com2usb/releases/latest/download/COM2USB_1.0.0_setup.exe> |
| `COM2USB.exe` (6.685.184 bytes) | portable: um arquivo só, sem instalador, para quem quer só rodar | <https://github.com/gorinformaticadev/com2usb/releases/latest/download/COM2USB.exe> |

Página da release (notas, data e histórico das versões):
<https://github.com/gorinformaticadev/com2usb/releases/latest>.

SHA-256 dos dois anexos de `v1.0.0`, para conferir depois de baixar:

```
19EF764ACC346019F9EBA35D4FB9DAD4D53D71816E5841A55DD911522C6BBAE7  COM2USB_1.0.0_setup.exe
C0661119C1B9AD0670C952FB5C54714BE0C2EDF74D24E856B1660C7F3710385A  COM2USB.exe
```

```powershell
Get-FileHash -Algorithm SHA256 .\COM2USB_1.0.0_setup.exe
```

O **portable** é o mesmo executável do instalador (manifesto `asInvoker`), então ele
grava `COM2USB.INI`, `COM2USB.log` e `COM2USB.rec` **na pasta onde estiver**: escolha um
lugar com permissão de escrita (a raiz de `C:\` e `Program Files` sem instalar não
servem). Duas coisas que só o instalador faz: colocar o driver CH340, se você marcar a
opção, e registrar a tarefa de logon elevada — no portable, marque **"Iniciar com o
Windows"** na tela de Propriedades, que cria a tarefa apontando para o caminho desse
arquivo (aí o Windows pede a confirmação do UAC uma vez).

## Estrutura do repositório

```
src/                 # frontend da janela Propriedades (HTML/CSS/JS puro)
src-tauri/
  src/main.rs        # bandeja, comandos IPC, tarefa agendada, plugins, CLI do instalador
  src/serial.rs      # thread de leitura serial + gravação do COM2USB.rec
  src/config.rs      # parser/gravador do COM2USB.INI + ícones embutidos da bandeija
  src/wedge.rs       # injeção de teclado (SendInput) que digita o código no app em foco
  build.rs           # embed do manifesto Windows (asInvoker)
  capabilities/      # ACL de comandos liberados para a janela
  tauri.conf.json    # identificação do produto (COM2USB/Pluggor/GPL)
  icons/             # gerados pelo gen-icons.js (icon.png, icon.ico, tray*.png)
driver/
  ch340/             # pacote WCH 3.9.2024.09 (originais, não GPL) + AVISO.md de procedência
  confere.ps1        # SHA-256 origem × cópia, somente-leitura
installer/
  com2usb.nsi        # script NSIS: componentes, opção de driver desmarcada, desinstalador
  gerar-instalador.ps1  # compila o app e roda o makensis (cache/ e dist/ são ignorados)
  sonda-string.ps1   # confere quais recursos estão embutidos num .exe compilado
gen-icons.js         # gera os ícones a partir de icon-model.png (arte + selo X do estado inativo)
icon-model.png       # arte-mestre do ícone (código de barras + laser), recortada e quadrada pelo gen-icons.js
LICENSE              # GPL-3.0
RELEASE-NOTES.md     # texto publicado na release do GitHub
```

## Formato do COM2USB.INI

```ini
[Settings]
Porta=COM4          ; porta serial do leitor
Velocidade=9600     ; baud rate
Paridade=sem        ; sem | impar | par
BitsDados=8
StopsBits=2
Ativo=1             ; 1 = começa gravando ao iniciar o app
Gravar=1            ; 1 = escreve COM2USB.rec
Deduplicar=1        ; 1 = marca leituras repetidas em 2s como DUP
Digitar=1           ; 1 = digita o código no aplicativo em foco
Enter=1             ; 1 = envia Enter depois do código (confirma o campo)
```

## Solução de problemas

- **A porta do leitor não aparece na lista:** a lista é recarregada sempre que a janela
  Propriedades ganha foco — desconecte/conecte o USB e clique novamente na janela.
  Uma porta salva no INI que não está presente hoje aparece marcada como
  `(não detectada)`, e não é confundida com uma porta real.
- **"Iniciar com o Windows":** se o COM2USB já estiver elevado, a tarefa é criada na hora;
  caso contrário o Windows mostra **uma única** confirmação de UAC (negar deixa a opção
  desmarcada). Depois de criada, a tarefa `COM2USB` inicia o app elevado a cada logon,
  sem nenhum prompt.
- **Portas COM ≥ 10:** funcionam; o prefixo `\\.\` é aplicado automaticamente.
- **"Um dispositivo conectado ao sistema não está funcionando" ao ativar:** é
  `ERROR_GEN_FAILURE` (31) — o driver do adaptador USB-serial parou de aceitar
  *programação de linha* (`SetCommState`), embora continue entregando bytes. O app
  então abre a porta **sem reprogramar** (o tooltip da bandeja avisa desse modo) e
  segue lendo; recupere o driver reconectando o cabo USB ou desativando/reativando a
  porta no Gerenciador de Dispositivos.
- **O código aparece em `COM2USB.rec` mas não é digitado:** o destino é sempre a janela
  **em foco**. Se o foco estiver na própria janela Propriedades do COM2USB, a digitação é
  ignorada de propósito (o código iria para o formulário). Clique no campo de destino e
  leia de novo; o botão **Testar digitação** faz o mesmo com 3 s para você trocar de janela.
- **Digitação bloqueada contra outro programa:** o Windows não permite que um processo
  standard digite em uma janela de um processo **elevado**. Use o COM2USB elevado
  (autostart já faz isso) ou abra o programa de destino sem administração.
- **Iniciar com o Windows não aparenta ter funcionado:** consulte `COM2USB.log` ao lado do
  executável. Uma linha `iniciando` sem `pronto` em seguida significa que o processo morreu
  no arranque (o `panic:` logado diz o motivo); `bandeja indisponível` é o explorer ainda
  não estar pronto — o app tenta 5 vezes e continua capturando mesmo sem bandeja.

## Licença

Copyright (C) 2026 PLUGGOR TECNOLOGIA LTDA

Este programa é software livre: você pode redistribuí-lo e/ou modificá-lo sob os
termos da **GNU General Public License versão 3**, publicada pela Free Software
Foundation. Veja o arquivo [LICENSE](LICENSE).
