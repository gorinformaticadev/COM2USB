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
| Bandeja (tray) | Ícone verde com código de barras. **Sem o X = ativo (gravando); com X vermelho = inativo.** Menu: Ativar/Desativar, Propriedades..., Sair. |
| Captura serial | Lê a porta configurada (ex.: `COM4 9600 sem 8 2`). Uma leitura termina em CR/LF ou após 150 ms de silêncio. |
| Arquivo de leituras | Cada leitura vira uma linha em `COM2USB.rec`: `AAAA-MM-DD HH:MM:SS<TAB>código<TAB>DUP?`. |
| Duplicatas | Leitura idêntica dentro de 2 s é marcada `DUP` (opcional). |
| Configuração | Janela Propriedades, persistida em `COM2USB.INI` ao lado do executável (formato INI simples). |
| Autostart | Tarefa agendada `COM2USB` (gatilho no logon, privilégio **mais elevado**): inicia **sem UAC e sem prompt**, oculto na bandeja. |
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

### Passos

```bash
# 1. instalar dependências do frontend/CLI
npm install

# 2. (opcional) regenerar os ícones PNG/ICO
npm run icons

# 3. desenvolvimento (recompilação automática)
npm run dev

# 4. build final + instalador NSIS
npm run build
```

O instalador é gerado em:

```
src-tauri/target/release/bundle/nsis/COM2USB_1.0.0_x64-setup.exe
```

### Onde baixar o build pronto

Os instaladores compilados ficam publicados nas **Releases** deste repositório Git
(anchor `v1.0.0`). Baixe o `COM2USB_1.0.0_x64-setup.exe` da release mais recente,
instale (o instalador pede UAC uma única vez) e marque **"Iniciar com o Windows"**
na tela de Propriedades para ativar o autostart elevado.

## Estrutura do repositório

```
src/                 # frontend da janela Propriedades (HTML/CSS/JS puro)
src-tauri/
  src/main.rs        # bandeja, comandos IPC, tarefa agendada, plugins
  src/serial.rs      # thread de leitura serial + gravação do COM2USB.rec
  src/config.rs      # parser/gravador do COM2USB.INI
  build.rs           # embed do manifesto Windows (asInvoker)
  tauri.conf.json    # identificação do produto (COM2USB/Pluggor/GPL)
gen-icons.js         # gerador dos ícones (PNG/ICO, estados on/off)
LICENSE              # GPL-3.0
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
```

## Solução de problemas

- **Botões não fazem nada / lista de portas vazia:** confirme que a porta existe no
  Gerenciador de Dispositivos; o erro de abertura aparece na janela e no tooltip da bandeja.
- **"Iniciar com o Windows" avisa que precisa de administrador:** execute o COM2USB
  uma vez via botão direito → *Executar como administrador* e marque a opção, ou
  instale pelo instalador (elevado). A partir daí a tarefa agendada inicia elevado
  automaticamente, sem prompts.
- **Portas COM ≥ 10:** funcionam; o prefixo `\\.\` é aplicado automaticamente.

## Licença

Copyright (C) 2026 PLUGGOR TECNOLOGIA LTDA

Este programa é software livre: você pode redistribuí-lo e/ou modificá-lo sob os
termos da **GNU General Public License versão 3**, publicada pela Free Software
Foundation. Veja o arquivo [LICENSE](LICENSE).
