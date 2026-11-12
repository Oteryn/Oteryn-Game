import test from 'node:test';
import { request as rawRequest } from 'node:http';
import assert from 'node:assert/strict';
import { RecordDecoder, SceneStore, createServer, inputByte, MAX_RECORD } from '../bridge.mjs';

function atlas(id = 1) {
  const b = Buffer.alloc(20); b.write('OTAT'); b.writeUInt32LE(id, 4);
  b.writeUInt32LE(1, 8); b.writeUInt32LE(1, 12); b.fill(255, 16); return b;
}
function frame(id = 1, atlasId = 1) {
  const b = Buffer.alloc(56); b.write('OTFR'); b.writeUInt32LE(id, 4);
  b.writeUInt32LE(atlasId, 8); b.writeUInt32LE(32, 12); b.writeUInt32LE(32, 16);
  b.writeUInt32LE(1, 20);
  [0, 0, 32, 32, 0, 0, 1, 1].forEach((v, i) => b.writeFloatLE(v, 24 + i * 4));
  return b;
}
function record(body) {
  const prefix = Buffer.alloc(4); prefix.writeUInt32LE(body.length);
  return Buffer.concat([prefix, body]);
}

test('IPC decoder accepts split prefixes, split bodies and multiple records', () => {
  const rows = []; const decoder = new RecordDecoder(b => rows.push(b));
  const bytes = Buffer.concat([record(atlas()), record(frame())]);
  for (const byte of bytes) decoder.push(Buffer.of(byte));
  decoder.finish(); assert.deepEqual(rows, [atlas(), frame()]);
});
test('IPC decoder rejects oversize length before allocating and truncated EOF', () => {
  const d = new RecordDecoder(() => {}); const prefix = Buffer.alloc(4);
  prefix.writeUInt32LE(MAX_RECORD + 1); assert.throws(() => d.push(prefix));
  const truncated = new RecordDecoder(() => {}); truncated.push(Buffer.of(20));
  assert.throws(() => truncated.finish());
});
test('only complete atlas/frame pairs become playable, failures clear retained state', () => {
  const s = new SceneStore(); assert.equal(s.status, 'starting');
  assert.throws(() => s.accept(frame())); s.accept(atlas());
  assert.equal(s.status, 'starting'); s.accept(frame()); assert.equal(s.status, 'playing');
  s.accept(atlas(2)); assert.equal(s.atlas(1).readUInt32LE(4), 1);
  s.accept(frame(2, 2)); assert.equal(s.atlas(1), undefined);
  s.fail(); assert.equal(s.frame, undefined); assert.equal(s.atlas(2), undefined);
  assert.equal(s.status, 'unavailable'); assert.throws(() => s.accept(atlas(3)));
});
test('malformed UVs, NaNs, stale frames and unrelated atlas generations fail closed', () => {
  const s = new SceneStore(); s.accept(atlas()); s.accept(frame());
  assert.throws(() => s.accept(frame())); assert.throws(() => s.accept(frame(2, 8)));
  const nan = frame(2); nan.writeFloatLE(NaN, 24); assert.throws(() => s.accept(nan));
  const uv = frame(2); uv.writeFloatLE(2, 40); assert.throws(() => s.accept(uv));
  const badAtlas = atlas(2); badAtlas.writeUInt32LE(4097, 8);
  assert.throws(() => s.accept(badAtlas));
});
test('input is an exact cardinal-direction allowlist, never coordinates or executable text', () => {
  assert.equal(inputByte('north'), 'N'); assert.equal(inputByte('west'), 'W');
  for (const value of ['N', 'north\nquit', '__proto__', null, 1, {}, 'teleport']) {
    assert.throws(() => inputByte(value));
  }
});
test('HTTP enforces exact Host, Origin, JSON and view header, with bounded input', async t => {
  const s = new SceneStore(); s.accept(atlas()); s.accept(frame());
  const writes = []; let stopped = false;
  const server = createServer(s, value => { writes.push(value); return true; }, () => { stopped = true; }, { now: () => 1234 });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  t.after(() => { server.closeAllConnections(); server.close(); });
  const origin = `http://127.0.0.1:${server.address().port}`;
  const request = (path, options = {}) => fetch(origin + path, options);
  const headers = { Origin: origin, 'Content-Type': 'application/json', 'X-Oteryn-View': '1' };
  assert.equal((await request('/frame')).status, 403);
  assert.equal((await request('/frame', { headers: { 'X-Oteryn-View': '1' } })).status, 200);
  const hostileHost = await new Promise((resolve, reject) => {
    const req = rawRequest(origin + '/status', { headers: { Host: 'attacker.example', 'X-Oteryn-View': '1' } }, res => { res.resume(); resolve(res.statusCode); });
    req.on('error', reject); req.end();
  });
  assert.equal(hostileHost, 403);
  assert.equal((await request('/input', { method: 'POST', headers: { ...headers, Origin: 'https://attacker.example' }, body: '{"direction":"north"}' })).status, 403);
  assert.equal((await request('/input', { method: 'POST', headers: { ...headers, 'Content-Type': 'text/plain' }, body: 'N' })).status, 415);
  assert.equal((await request('/input', { method: 'POST', headers, body: '{"direction":"north","x":9}' })).status, 400);
  assert.equal((await request('/input', { method: 'POST', headers, body: ' '.repeat(257) })).status, 413);
  assert.equal((await request('/input', { method: 'POST', headers, body: '{"direction":"north"}' })).status, 202);
  assert.deepEqual(writes, ['N']);
  assert.equal((await request('/input', { method: 'POST', headers, body: '{"direction":"east"}' })).status, 429);
  assert.equal((await request('/atlas/1', { headers: { 'X-Oteryn-View': '1' } })).status, 200);
  assert.equal((await request('/atlas/9', { headers: { 'X-Oteryn-View': '1' } })).status, 410);
  assert.equal((await request('/../../grant', { headers })).status, 404);
  assert.equal((await request('/disconnect', { method: 'POST', headers })).status, 204);
  assert.equal(stopped, true);
});
test('HTTP refuses input without a validated scene', async t => {
  const server = createServer(new SceneStore(), () => { throw Error('must not send'); }, () => {});
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  t.after(() => { server.closeAllConnections(); server.close(); });
  const origin = `http://127.0.0.1:${server.address().port}`;
  const result = await fetch(origin + '/input', { method: 'POST', headers: {
    Origin: origin, 'Content-Type': 'application/json', 'X-Oteryn-View': '1'
  }, body: '{"direction":"north"}' });
  assert.equal(result.status, 409);
});
