# COM2USB 1.0.0

Recriação do SEALST em Tauri: leitor de código de barras em porta serial digitando o
código no aplicativo em foco, morando na bandeja do sistema. GPL-3.0, PLUGGOR TECNOLOGIA LTDA.

**Anexos desta release** (Windows 10/11 x64):

| Arquivo | Bytes | SHA-256 |
|---|---|---|
| `COM2USB_1.0.0_setup.exe` — instalador NSIS | 3.729.217 | `19EF764ACC346019F9EBA35D4FB9DAD4D53D71816E5841A55DD911522C6BBAE7` |
| `COM2USB.exe` — portable, sem instalador | 6.685.184 | `C0661119C1B9AD0670C952FB5C54714BE0C2EDF74D24E856B1660C7F3710385A` |

## Instalar

1. Execute o instalador (ele pede elevação uma única vez).
2. Na tela *Escolha de componentes*, marque **"Instalar o driver USB-SERIAL CH340"**
   somente se o Windows ainda não reconheceu o adaptador. A opção vem **desmarcada**
   e, se o pacote WCH 3.9.2024.09 já estiver no Driver Store da máquina, nada é alterado.
3. Deixe **"Iniciar com o Windows"** marcado: cria a tarefa de logon que abre o programa
   oculto, já elevado, sem exibir confirmação do Windows.
4. O programa fica na bandeja: **sem o X = ativo**, **com X vermelho = inativo**.
   Fechar a janela apenas a oculta; para encerrar use o menu **Sair**.
5. Abra **Propriedades** e escolha a porta do leitor (a padrão é COM4).

## O que há nesta versão

- Leitura serial em thread própria, com dono exclusivo do handle da porta: `Desativar`
  libera a porta de verdade e `Ativar` não empilha dois leitores sobre a mesma COM.
- Fallback **"sem reprogramação de linha"**: se o driver recusar o `SetCommState`
  (`ERROR_GEN_FAILURE` em alguns CH340), o programa continua lendo do mesmo jeito e o
  tooltip da bandeja avisa do modo.
- Keyboard wedge: cada código é digitado na janela em foco via `SendInput`, com Enter
  opcional, e o botão **Testar digitação** permite exercitar o caminho com 3 s de espera.
- Leituras repetidas dentro de 2 s são marcadas `DUP` (e não são digitadas de novo);
  tudo vai para `COM2USB.rec`.
- Autostart elevado por tarefa agendada, sem janela de console e sem UAC visível.
- `COM2USB.log` ao lado do executável registra inicialização, elevação, bandeja e falhas
  de ativação da porta.
- Erros sempre em modal central, fechável pelo OK.

## Notas

- O `COM2USB.exe` da tabela acima é o mesmo binário que o instalador copia: serve como
  **portable**, gravando `COM2USB.INI`/`.log`/`.rec` na pasta onde for colocado, e sem
  instalar o driver CH340 (pressupõe que a máquina já reconhece o adaptador).
- Os pacotes de driver **não** são removidos pela desinstalação.
- O instalador inclui o driver WCH 3.9.2024.09 sem modificação; procedência e SHA-256 em
  `driver/ch340/AVISO.md` do repositório.
