// Real HTTP + child-process pipes; the child below is explicitly NOT a Game server.
import test from 'node:test';
import assert from 'node:assert/strict';
import { once } from 'node:events';
import { request as rawRequest } from 'node:http';
import { mkdtemp, mkdir, writeFile, readFile, rm, access } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';
import { setImmediate as yieldNow } from 'node:timers/promises';
import { launch, SceneStore, createServer } from '../bridge.mjs';
import { decodeFrame, decodeAtlas } from '../public/scene.mjs';

const bytes = buffer => buffer.buffer.slice(buffer.byteOffset, buffer.byteOffset + buffer.byteLength);
function atlas() {
  const b = Buffer.alloc(20); b.write('OTAT');
  [1, 1, 1].forEach((v, i) => b.writeUInt32LE(v, 4 + i * 4));
  b.fill(255, 16); return b;
}
function frame() {
  const b = Buffer.alloc(56); b.write('OTFR');
  [1, 1, 480, 352, 1].forEach((v, i) => b.writeUInt32LE(v, 4 + i * 4));
  [0, 0, 32, 32, 0, 0, 1, 1].forEach((v, i) => b.writeFloatLE(v, 24 + i * 4));
  return b;
}

test('bridge and browser both reject high-bit aliases of IPC magic', () => {
  const badAtlas = atlas(); badAtlas[0] |= 128;
  assert.throws(() => new SceneStore().accept(badAtlas), /unknown|invalid/);
  assert.throws(() => decodeAtlas(bytes(badAtlas)));
  const store = new SceneStore(); store.accept(atlas());
  const badFrame = frame(); badFrame[0] |= 128;
  assert.throws(() => store.accept(badFrame), /unknown|invalid/);
  assert.throws(() => decodeFrame(bytes(badFrame)));
});

async function fixture(t) {
  const root = await mkdtemp(join(tmpdir(), 'oteryn-threejs-'));
  t.after(() => rm(root, { recursive: true, force: true }));
  const metadata = JSON.parse(await readFile(new URL('../package.json', import.meta.url)));
  await mkdir(join(root, 'public'), { recursive: true });
  await mkdir(join(root, 'node_modules/three/build'), { recursive: true });
  await writeFile(join(root, 'package.json'), JSON.stringify(metadata));
  await writeFile(join(root, 'node_modules/three/package.json'), JSON.stringify({ version: metadata.dependencies.three }));
  // Only dependency-presence tests, never passed off as Three.js/WebGL execution.
  for (const file of ['public/index.html', 'public/client.mjs', 'public/scene.mjs', 'public/style.css',
    'node_modules/three/build/three.module.js', 'node_modules/three/build/three.core.js']) {
    await writeFile(join(root, file), '/* test-only static fixture */');
  }
  const marker = join(root, 'starts.txt');
  const child = join(root, 'synthetic-host.mjs');
  await writeFile(child, `
    import { appendFileSync } from 'node:fs';
    appendFileSync(process.argv[2], 'start\\n');
    let id = 0, x = 0;
    function emit(body) {
      const prefix = Buffer.alloc(4); prefix.writeUInt32LE(body.length);
      process.stdout.write(prefix); process.stdout.write(body);
    }
    emit(Buffer.from(${JSON.stringify([...atlas()])}));
    function draw() {
      const b = Buffer.from(${JSON.stringify([...frame()])});
      b.writeUInt32LE(++id, 4); b.writeFloatLE(x, 24); emit(b);
    }
    draw();
    process.stdin.on('data', chunk => {
      for (const byte of chunk) {
        if (byte === 69) x += 32;
        if (byte === 78) { process.stdout.write(Buffer.from([1, 0, 0, 0])); continue; }
        draw();
      }
    });
    process.stdin.on('end', () => process.exit(0));
  `);
  return { root, marker, child, options: { root: pathToFileURL(root + '/'), log: () => {} } };
}
async function close(server) {
  if (!server.listening) await once(server, 'listening');
  server.closeAllConnections();
  await new Promise(resolve => server.close(resolve));
}
async function serve(t, f, binary = process.execPath, args = [f.child, f.marker]) {
  const server = await launch(binary, args, f.options);
  t.after(() => close(server));
  if (!server.listening) await once(server, 'listening');
  const origin = `http://127.0.0.1:${server.address().port}`;
  const headers = { 'X-Oteryn-View': '1', 'Content-Type': 'application/json', Origin: origin };
  return {
    get: path => fetch(origin + path, { headers, signal: AbortSignal.timeout(5000) }),
    post: (path, body, extra = {}) => fetch(origin + path, {
      method: 'POST', headers: { ...headers, ...extra }, body, signal: AbortSignal.timeout(5000),
    }),
  };
}
async function until(read, condition) {
  const end = performance.now() + 10000;
  do {
    const value = await read();
    if (condition(value)) return value;
    await yieldNow();
  } while (performance.now() < end);
  assert.fail('child/HTTP condition did not become true within its deadline');
}
async function nextFrame(client, id) {
  return until(async () => {
    const response = await client.get('/frame');
    return response.ok ? decodeFrame(await response.arrayBuffer()) : null;
  }, frame => frame?.id >= id);
}

test('native host starts only after renderer-ready, carries input, and never respawns after close', async t => {
  const f = await fixture(t), client = await serve(t, f);
  assert.equal((await (await client.get('/status')).json()).state, 'idle');
  await assert.rejects(access(f.marker));
  assert.equal((await client.post('/input', '{"direction":"east"}')).status, 409);
  assert.equal((await client.post('/connect', undefined, { Origin: 'https://other.example' })).status, 403);
  await assert.rejects(access(f.marker));
  assert.equal((await client.post('/connect', '{"endpoint":"evil"}')).status, 413);
  await assert.rejects(access(f.marker));
  assert.equal((await client.post('/connect')).status, 202);
  assert.equal((await nextFrame(client, 1)).quads[0], 0);
  assert.equal((await client.post('/input', '{"direction":"east"}')).status, 202);
  assert.equal((await nextFrame(client, 2)).quads[0], 32);
  assert.equal((await client.post('/connect')).status, 200);
  assert.equal(await readFile(f.marker, 'utf8'), 'start\n');
  assert.equal((await client.post('/disconnect')).status, 204);
  assert.equal((await client.get('/frame')).status, 409);
  assert.equal((await client.post('/connect')).status, 409);
  assert.equal(await readFile(f.marker, 'utf8'), 'start\n');
});

test('missing renderer dependency or wrong installed version fails before spawning the host', async t => {
  const f = await fixture(t);
  await rm(join(f.root, 'node_modules/three/build/three.core.js'));
  let server;
  try {
    await assert.rejects(async () => { server = await launch(process.execPath, [f.child, f.marker], f.options); });
    await assert.rejects(access(f.marker));
  } finally { if (server) await close(server); }
  await writeFile(join(f.root, 'node_modules/three/build/three.core.js'), '/* fixture */');
  await writeFile(join(f.root, 'node_modules/three/package.json'), '{"version":"0.0.0"}');
  await assert.rejects(launch(process.execPath, [f.child, f.marker], f.options));
  await assert.rejects(access(f.marker));
});

test('malformed native pipe clears a previously playable frame and cannot restart', async t => {
  const f = await fixture(t), client = await serve(t, f);
  assert.equal((await client.post('/connect')).status, 202);
  await nextFrame(client, 1);
  assert.equal((await client.post('/input', '{"direction":"north"}')).status, 202);
  await until(async () => (await (await client.get('/status')).json()).state, state => state === 'unavailable');
  assert.equal((await client.get('/frame')).status, 409);
  assert.equal((await client.get('/atlas/1')).status, 410);
  assert.equal((await client.post('/connect')).status, 409);
});

test('native executable failure reports unavailable without manufacturing admission', async t => {
  const f = await fixture(t), client = await serve(t, f, join(f.root, 'missing-binary'), []);
  assert.equal((await client.post('/connect')).status, 202);
  await until(async () => (await (await client.get('/status')).json()).state, state => state === 'unavailable');
  assert.equal((await client.get('/frame')).status, 409);
  assert.equal((await client.post('/connect')).status, 409);
});


test('input whose body finishes after disconnect is not dispatched', async t => {
  const store = new SceneStore(); store.accept(atlas()); store.accept(frame());
  const sent = [];
  const server = createServer(store, byte => { sent.push(byte); return true; }, () => {});
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  t.after(() => close(server));
  const origin = `http://127.0.0.1:${server.address().port}`;
  const body = '{"direction":"east"}';
  const headers = { Origin: origin, 'Content-Type': 'application/json', 'X-Oteryn-View': '1' };
  // Wait for the handler to start reading, not an arbitrary sleep.
  const entered = once(server, 'request');
  let request;
  const result = new Promise((resolve, reject) => {
    request = rawRequest(origin + '/input', {
      method: 'POST', headers: { ...headers, 'Content-Length': Buffer.byteLength(body) },
    }, response => { response.resume(); resolve(response.statusCode); });
    request.on('error', reject); request.flushHeaders();
  });
  t.after(() => request.destroy());
  await entered;
  assert.equal((await fetch(origin + '/disconnect', { method: 'POST', headers })).status, 204);
  request.end(body);
  assert.equal(await result, 409);
  assert.deepEqual(sent, []);
});
