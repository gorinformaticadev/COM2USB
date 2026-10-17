# Pacote do driver USB-SERIAL CH340 incluído no instalador

Estes arquivos são o pacote de driver **WCH (Nanjing Qinheng Microelectronics) / wch.cn**,
versão `3.9.2024.09` (arquivo de origem: `CH341SER.INF`), copiado **sem nenhuma modificação**
do Driver Store desta máquina, onde está publicado como `oem1.inf` e é o driver em uso pelo
dispositivo `USB\VID_1A86&PID_7523` (USB-SERIAL CH340).

Pasta de origem:

```
C:\Windows\System32\DriverStore\FileRepository\ch341ser.inf_amd64_34a3206305b40d57
```

A cópia foi conferida por SHA-256 contra a origem (`driver/confere.ps1`): os seis arquivos
são idênticos byte a byte.

O sétimo arquivo daquela pasta, `ch341ser.PNF`, **não** foi copiado: é o INF pré-compilado que
o próprio Windows gera ao registrar o pacote, não uma peça entregue pelo fabricante.

| Arquivo | SHA-256 |
|---|---|
| CH341SER.INF | `FB573086EE1B6F65D68A330CC4B48350439E3327C85292FC455B3588DFF053A3` |
| CH341SER.CAT | `B44F551843D0E36319AB4E911D4579A866B175201980DAA9E8973919AD3FB1FE` |
| CH341S64.SYS | `A8CB7CF5050F9D14106B27D45D50DFB85F6CB119C0E871E673743AD62ECAEED0` |
| CH341PORTSA64.DLL | `16AD9A058DF6B58CE8259E086275B81F663F43202A2B7F0DA7A634C356203E1D` |
| CH341PTA64.DLL | `9F8B6DBE2B81A720184F0A0A269E17A90ED6DA9ECBB3FA3D9FA32D2B95AB1ADC` |
| CH341PT.DLL | `4FAE54A4CC5CBFD5BD0A59B6AFF3F888BEBEB3C78521DADCDD1247B40012C580` |

## Licença e escopo

O COM2USB é GPL-3.0; **estes arquivos não são**. Eles pertencem à WCH e são distribuídos
aqui apenas como conveniência para instalar o adaptador COM-USB em máquinas que ainda não o
reconhecem. Prefira sempre o pacote oficial do fabricante
(<https://www.wch-ic.com/downloads/CH341SER_EXE.html>).

No instalador, a instalação deste driver é uma **opção desmarcada por padrão**. Ela só é
aplicada se o usuário a marcar, e o que o instalador faz é exatamente uma chamada:

```
pnputil /add-driver CH341SER.inf /install
```

Sempre é possível reverter pelo Gerenciador de Dispositivos ou por
`pnputil /delete-driver oemNN.inf /uninstall`. O desinstalador do COM2USB **não remove**
pacotes de driver — remover Driver Store é operação destrutiva e fica nas mãos do operador.
