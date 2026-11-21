// Presentation-only binary formats; neither file contains gameplay authority or credentials.
export const MAX_QUADS = 81920;
export const MAX_ATLAS_BYTES = 16 + 4096 * 4096 * 4;
export const MAX_FRAME_BYTES = 24 + MAX_QUADS * 32;
const magic = (view, name) => [...name].every((c, i) => view.getUint8(i) === c.charCodeAt(0));

export function decodeAtlas(buffer) {
  if (!(buffer instanceof ArrayBuffer) || buffer.byteLength < 16 || buffer.byteLength > MAX_ATLAS_BYTES) throw Error('Rozmiar atlasu');
  const v = new DataView(buffer), id = v.getUint32(4, true), width = v.getUint32(8, true), height = v.getUint32(12, true);
  if (!magic(v, 'OTAT') || !id || !width || !height || width > 4096 || height > 4096 ||
      buffer.byteLength !== 16 + width * height * 4) throw Error('Nieprawidłowy atlas');
  return { id, width, height, pixels: new Uint8Array(buffer, 16) };
}

export function decodeFrame(buffer) {
  if (!(buffer instanceof ArrayBuffer) || buffer.byteLength < 24 || buffer.byteLength > MAX_FRAME_BYTES) throw Error('Rozmiar sceny');
  const v = new DataView(buffer), id = v.getUint32(4, true), atlasId = v.getUint32(8, true);
  const width = v.getUint32(12, true), height = v.getUint32(16, true), count = v.getUint32(20, true);
  if (!magic(v, 'OTFR') || !id || !atlasId || !width || !height || width > 65536 || height > 65536 ||
      count > MAX_QUADS || buffer.byteLength !== 24 + count * 32) throw Error('Nieprawidłowa scena');
  const quads = new Float32Array(count * 8);
  for (let i = 0; i < quads.length; i++) {
    const value = v.getFloat32(24 + i * 4, true), field = i % 8;
    if (!Number.isFinite(value) || (field < 2 && Math.abs(value) > 131072) ||
        (field >= 2 && field < 4 && (value < 0 || value > 65536)) ||
        (field >= 4 && (value < 0 || value > 1))) throw Error('Nieprawidłowy sprite');
    quads[i] = value;
  }
  for (let i = 0; i < quads.length; i += 8) {
    if (quads[i + 4] > quads[i + 6] || quads[i + 5] > quads[i + 7]) throw Error('Odwrócone UV');
  }
  return { id, atlasId, width, height, count, quads };
}

export async function boundedBytes(response, maximum) {
  const length = Number(response.headers.get('content-length'));
  if (!Number.isSafeInteger(length) || length < 1 || length > maximum) throw Error('Limit odpowiedzi');
  const reader = response.body.getReader();
  const bytes = new Uint8Array(length); let used = 0;
  try {
    while (true) {
      const { value, done } = await reader.read();
      if (done) break;
      if (used + value.length > length) throw Error('Limit odpowiedzi');
      bytes.set(value, used); used += value.length;
    }
    if (used !== length) throw Error('Niepełna odpowiedź');
    return bytes.buffer;
  } finally { await reader.cancel().catch(() => {}); reader.releaseLock(); }
}
