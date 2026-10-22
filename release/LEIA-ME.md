# Binários compilados

Espelho versionado dos pacotes publicados. Estes são os mesmos bytes dos anexos da
Release `v1.0.0`, gerados a partir das fontes deste repositório.

| Arquivo | Bytes | SHA-256 |
|---|---|---|
| `COM2USB_1.0.0_setup.exe` | 3.729.217 | `19EF764ACC346019F9EBA35D4FB9DAD4D53D71816E5841A55DD911522C6BBAE7` |
| `COM2USB.exe` | 6.685.184 | `C0661119C1B9AD0670C952FB5C54714BE0C2EDF74D24E856B1660C7F3710385A` |

Download direto por estes caminhos (`main`):

```
https://github.com/gorinformaticadev/COM2USB/raw/main/release/COM2USB_1.0.0_setup.exe
https://github.com/gorinformaticadev/COM2USB/raw/main/release/COM2USB.exe
```

Ou pela página da release, que é o caminho preferível porque carrega as notas da versão:
<https://github.com/gorinformaticadev/COM2USB/releases/latest>.

## Como são gerados

`npm run instalador` compila o aplicativo (`tauri build --no-bundle`) e empacota o
instalador NSIS; no fim ele copia os dois arquivos para cá, então **nunca edite ou
substitua estes arquivos à mão** — o que vale é o build.

A compilação NSIS é determinística: empacotar o mesmo executável produz bytes idênticos,
então o SHA-256 acima é uma propriedade do build, não do momento da compilação.

O instalador embute o pacote de driver WCH 3.9.2024.09 sem modificação; procedência e
hashes em `../driver/ch340/AVISO.md`.

## Conferir depois de baixar

```powershell
Get-FileHash -Algorithm SHA256 .\COM2USB_1.0.0_setup.exe
```
