// Gera o conjunto de ícones a partir do modelo do projeto (icon-model.png).
// Estado desabilitado = mesma arte + selo "X" vermelho no canto inferior direito.
const fs = require('node:fs');
const path = require('node:path');
const sharp = require('sharp');

const ROOT = __dirname;
const MODEL = path.join(ROOT, 'icon-model.png');
const OUT = path.join(ROOT, 'src-tauri', 'icons');
const TRAY_PX = 64;
const ICO_SIZES = [16, 24, 32, 48, 64, 128, 256];

const badgeSvg = (px) => {
  const r = px * 0.42;
  const c = px / 2;
  const arm = r * 0.46;
  const stroke = px * 0.075;
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${px}" height="${px}">
  <circle cx="${c}" cy="${c}" r="${r}" fill="#d40000" stroke="#ffffff" stroke-width="${stroke}"/>
  <g stroke="#ffffff" stroke-width="${stroke}" stroke-linecap="round">
    <line x1="${c - arm}" y1="${c - arm}" x2="${c + arm}" y2="${c + arm}"/>
    <line x1="${c + arm}" y1="${c - arm}" x2="${c - arm}" y2="${c + arm}"/>
  </g>
</svg>`;
};

// PNG empacotado em ICO (formato aceito pelo Windows desde o Vista).
function toIco(images) {
  const count = images.length;
  const header = Buffer.alloc(6 + count * 16);
  header.writeUInt16LE(0, 0);
  header.writeUInt16LE(1, 2);
  header.writeUInt16LE(count, 4);
  let offset = header.length;
  images.forEach(({ buf, px }, i) => {
    const entry = 6 + i * 16;
    header.writeUInt8(px >= 256 ? 0 : px, entry);
    header.writeUInt8(px >= 256 ? 0 : px, entry + 1);
    header.writeUInt8(0, entry + 2);
    header.writeUInt8(0, entry + 3);
    header.writeUInt16LE(1, entry + 4);
    header.writeUInt16LE(32, entry + 6);
    header.writeUInt32LE(buf.length, entry + 8);
    header.writeUInt32LE(offset, entry + 12);
    offset += buf.length;
  });
  return Buffer.concat([header, ...images.map((i) => i.buf)]);
}

async function main() {
  fs.mkdirSync(OUT, { recursive: true });

  const trimmed = await sharp(MODEL).trim().png().toBuffer();
  const { width, height } = await sharp(trimmed).metadata();
  const side = Math.max(width, height);
  const padX = Math.floor((side - width) / 2);
  const padY = Math.floor((side - height) / 2);

  const square = await sharp(trimmed)
    .extend({
      top: padY,
      bottom: side - height - padY,
      left: padX,
      right: side - width - padX,
      background: { r: 0, g: 0, b: 0, alpha: 0 },
    })
    .png()
    .toBuffer();

  const resize = (px) =>
    sharp(square, { fastShrinkOnLoad: false }).resize(px, px, { kernel: 'lanczos3' });

  const badgePx = Math.round(TRAY_PX * 0.6);
  const badge = await sharp(Buffer.from(badgeSvg(badgePx)), { density: 300 })
    .resize(badgePx, badgePx)
    .png()
    .toBuffer();

  await resize(256).toFile(path.join(OUT, 'icon.png'));
  await resize(TRAY_PX).toFile(path.join(OUT, 'tray.png'));
  await resize(TRAY_PX).composite([{ input: badge, gravity: 'southeast' }]).png()
    .toFile(path.join(OUT, 'tray-off.png'));

  const images = [];
  for (const px of ICO_SIZES) {
    images.push({ px, buf: await resize(px).png().toBuffer() });
  }
  fs.writeFileSync(path.join(OUT, 'icon.ico'), toIco(images));

  console.log(`modelo ${width}x${height} -> arte quadrada de ${side}px; ícones gerados em src-tauri/icons`);
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
