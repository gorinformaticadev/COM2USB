// Gera icons/icon.png, icons/tray.png e icons/icon.ico (quadrado verde com "S").
const fs = require("fs");
const path = require("path");
const zlib = require("zlib");

function crc32(buf) {
  let table = crc32.table;
  if (!table) {
    table = crc32.table = [];
    for (let n = 0; n < 256; n++) {
      let c = n;
      for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
      table[n] = c >>> 0;
    }
  }
  let c = 0xffffffff;
  for (const b of buf) c = table[(c ^ b) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const body = Buffer.concat([Buffer.from(type), data]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(body));
  return Buffer.concat([len, body, crc]);
}

// bitmap RGBA: quadrado verde com barras brancas; se `off`, sobrepõe distintivo com X
function bitmap(size, off) {
  const px = Buffer.alloc(size * size * 4);
  const set = (x, y, r, g, b, a) => {
    const i = (y * size + x) * 4;
    px[i] = r; px[i + 1] = g; px[i + 2] = b; px[i + 3] = a;
  };
  const rad = Math.max(3, Math.floor(size / 5));
  const inside = (x, y) => {
    if (x < rad && y < rad) return (x - rad) ** 2 + (y - rad) ** 2 <= rad * rad;
    if (x >= size - rad && y < rad) return (x - (size - rad - 1)) ** 2 + (y - rad) ** 2 <= rad * rad;
    if (x < rad && y >= size - rad) return (x - rad) ** 2 + (y - (size - rad - 1)) ** 2 <= rad * rad;
    if (x >= size - rad && y >= size - rad)
      return (x - (size - rad - 1)) ** 2 + (y - (size - rad - 1)) ** 2 <= rad * rad;
    return true;
  };
  for (let y = 0; y < size; y++)
    for (let x = 0; x < size; x++)
      if (inside(x, y)) set(x, y, 0x1f, 0x8a, 0x3c, 255);

  // barras verticais brancas (código de barras)
    const bars = [0.22, 0.3, 0.42, 0.5, 0.62, 0.78];
  for (let i = 0; i < bars.length; i++) {
    const w = i % 2 === 0 ? 0.06 : 0.03;
    const x0 = Math.floor(bars[i] * size);
    const x1 = Math.min(size - 1, Math.floor((bars[i] + w) * size));
    const y0 = Math.floor(size * 0.25);
    const y1 = Math.floor(size * 0.75);
    for (let y = y0; y <= y1; y++) for (let x = x0; x <= x1; x++) if (inside(x, y)) set(x, y, 255, 255, 255, 255);
  }
  if (off) {
    // distintivo vermelho com X branco no canto inferior direito
    const bx0 = Math.floor(size * 0.5), by0 = Math.floor(size * 0.5);
    const r = Math.max(2, Math.floor(size * 0.46));
    const cx = bx0 + (size - bx0) / 2 - 0.5, cy = by0 + (size - by0) / 2 - 0.5;
    const rad = (size - bx0) / 2;
    for (let y = by0; y < size; y++)
      for (let x = bx0; x < size; x++) {
        const dx = x - cx, dy = y - cy;
        if (dx * dx + dy * dy <= rad * rad) {
          set(x, y, 200, 30, 30, 255);
          const t = Math.max(1, Math.floor(size * 0.05));
          const d1 = Math.abs(dx - dy), d2 = Math.abs(dx + dy);
          if (d1 <= t && Math.abs(dx) <= rad * 0.62 && Math.abs(dy) <= rad * 0.62) set(x, y, 255, 255, 255, 255);
          if (d2 <= t && Math.abs(dx) <= rad * 0.62 && Math.abs(dy) <= rad * 0.62) set(x, y, 255, 255, 255, 255);
        }
      }
    void r;
  }
  return px;
}

function png(size, off) {
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(size, 0);
  ihdr.writeUInt32BE(size, 4);
  ihdr[8] = 8; ihdr[9] = 6; // RGBA8
  const raw = Buffer.alloc(size * (size * 4 + 1));
  for (let y = 0; y < size; y++) {
    raw[y * (size * 4 + 1)] = 0;
    bitmap(size, off).copy(raw, y * (size * 4 + 1) + 1, y * size * 4, (y + 1) * size * 4);
  }
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", ihdr),
    chunk("IDAT", zlib.deflateSync(raw, { level: 9 })),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

function ico(pngBuf, size) {
  const header = Buffer.alloc(6);
  header.writeUInt16LE(0, 0);
  header.writeUInt16LE(1, 2);
  header.writeUInt16LE(1, 4);
  const entry = Buffer.alloc(16);
  entry[0] = size === 256 ? 0 : size;
  entry[1] = size === 256 ? 0 : size;
  entry[2] = 0; entry[3] = 0;
  entry.writeUInt16LE(1, 4);
  entry.writeUInt16LE(32, 6);
  entry.writeUInt32LE(pngBuf.length, 8);
  entry.writeUInt32LE(22, 12);
  return Buffer.concat([header, entry, pngBuf]);
}

const out = path.join(__dirname, "src-tauri", "icons");
fs.mkdirSync(out, { recursive: true });
const png256 = png(256, false);
fs.writeFileSync(path.join(out, "icon.png"), png256);
fs.writeFileSync(path.join(out, "tray.png"), png(32, false));
fs.writeFileSync(path.join(out, "tray-off.png"), png(32, true));
fs.writeFileSync(path.join(out, "icon.ico"), ico(png256, 256));
console.log("icones gerados em", out);
