import test from 'node:test';
import assert from 'node:assert/strict';
import { decodeAtlas, decodeFrame, boundedBytes, MAX_FRAME_BYTES } from '../public/scene.mjs';
const arrayBuffer = b => b.buffer.slice(b.byteOffset, b.byteOffset + b.byteLength);

test('browser decoder preserves top-left pixel rectangles and raw RGBA cell orientation', () => {
  const atlas = Buffer.alloc(24); atlas.write('OTAT'); atlas.writeUInt32LE(7, 4);
  atlas.writeUInt32LE(1, 8); atlas.writeUInt32LE(2, 12);
  atlas.set([255, 0, 0, 255, 0, 0, 255, 255], 16);
  const parsed = decodeAtlas(arrayBuffer(atlas));
  assert.equal(parsed.id, 7); assert.deepEqual([...parsed.pixels], [255, 0, 0, 255, 0, 0, 255, 255]);
  const frame = Buffer.alloc(56); frame.write('OTFR');
  [1, 7, 480, 352, 1].forEach((v, i) => frame.writeUInt32LE(v, 4 + i * 4));
  const quads = [32, 64, 32, 32, 0, 0.5, 1, 1];
  quads.forEach((v, i) => frame.writeFloatLE(v, 24 + i * 4));
  assert.deepEqual([...decodeFrame(arrayBuffer(frame)).quads], quads);
  frame.writeFloatLE(Infinity, 24); assert.throws(() => decodeFrame(arrayBuffer(frame)));
});
test('browser rejects truncated/oversized records and unrecognized magic', () => {
  assert.throws(() => decodeFrame(new ArrayBuffer(3)));
  assert.throws(() => decodeFrame(new ArrayBuffer(MAX_FRAME_BYTES + 1)));
  assert.throws(() => decodeAtlas(new ArrayBuffer(16)));
});
test('HTTP body is bounded by an exact declared length, including chunked data', async () => {
  assert.equal((await boundedBytes(new Response('abc', { headers: { 'Content-Length': '3' } }), 4)).byteLength, 3);
  await assert.rejects(boundedBytes(new Response('abcd', { headers: { 'Content-Length': '3' } }), 4));
  await assert.rejects(boundedBytes(new Response('ab', { headers: { 'Content-Length': '3' } }), 4));
  await assert.rejects(boundedBytes(new Response('abc'), 4));
});
