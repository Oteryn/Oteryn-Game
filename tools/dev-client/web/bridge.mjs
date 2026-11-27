// Development-only local presentation IPC. This is NOT a Game transport profile.
import { createServer as httpServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { spawn } from 'node:child_process';
import { pathToFileURL } from 'node:url';

export const MAX_RECORD = 4096 * 4096 * 4 + 16;
const MAX_QUADS = 81920;
const staticFiles = new Map([
  ['/', ['public/index.html', 'text/html; charset=utf-8']],
  ['/client.mjs', ['public/client.mjs', 'text/javascript; charset=utf-8']],
  ['/scene.mjs', ['public/scene.mjs', 'text/javascript; charset=utf-8']],
  ['/style.css', ['public/style.css', 'text/css; charset=utf-8']],
  ['/vendor/three.module.js', ['node_modules/three/build/three.module.js', 'text/javascript; charset=utf-8']],
  ['/vendor/three.core.js', ['node_modules/three/build/three.core.js', 'text/javascript; charset=utf-8']],
]);

/** A single bounded allocation after checking the length, even with fragmented pipe reads. */
export class RecordDecoder {
  constructor(accept) { this.accept = accept; this.prefix = Buffer.alloc(4); this.used = 0; this.body = undefined; }
  push(chunk) {
    let offset = 0;
    while (offset < chunk.length) {
      const target = this.body ?? this.prefix;
      const n = Math.min(target.length - this.used, chunk.length - offset);
      chunk.copy(target, this.used, offset, offset + n); offset += n; this.used += n;
      if (this.used !== target.length) continue;
      this.used = 0;
      if (!this.body) {
        const length = this.prefix.readUInt32LE();
        if (length < 16 || length > MAX_RECORD) throw Error('IPC record size');
        this.body = Buffer.allocUnsafe(length);
      } else {
        const body = this.body; this.body = undefined; this.accept(body);
      }
    }
  }
  finish() { if (this.used || this.body) throw Error('truncated IPC record'); }
}

/** Retains one committed scene and at most one incoming atlas, never an unbounded replay. */
export class SceneStore {
  status = 'starting'; frame = undefined; current = undefined; pending = undefined;
  lastFrame = 0; lastRecord = performance.now();
  accept(body) {
    if (this.status === 'unavailable') throw Error('closed presentation');
    const kind = body.toString('latin1', 0, 4); // ASCII decoding would mask corrupt high bits.
    if (kind === 'OTAT') {
      const id = body.readUInt32LE(4), w = body.readUInt32LE(8), h = body.readUInt32LE(12);
      if (!id || id <= (this.current?.readUInt32LE(4) ?? 0) || this.pending ||
          !w || !h || w > 4096 || h > 4096 || body.length !== 16 + w * h * 4) throw Error('invalid atlas');
      this.pending = body;
    } else if (kind === 'OTFR') {
      if (body.length < 24) throw Error('short frame');
      const id = body.readUInt32LE(4), atlasId = body.readUInt32LE(8);
      const w = body.readUInt32LE(12), h = body.readUInt32LE(16), count = body.readUInt32LE(20);
      const atlas = this.pending ?? this.current;
      if (!atlas || atlas.readUInt32LE(4) !== atlasId || id <= this.lastFrame ||
          !w || !h || w > 65536 || h > 65536 || count > MAX_QUADS || body.length !== 24 + count * 32) throw Error('invalid frame');
      for (let at = 24; at < body.length; at += 32) {
        for (let n = 0; n < 8; n++) {
          const v = body.readFloatLE(at + n * 4);
          if (!Number.isFinite(v) || (n < 2 && Math.abs(v) > 131072) ||
              (n >= 2 && n < 4 && (v < 0 || v > 65536)) ||
              (n >= 4 && (v < 0 || v > 1))) throw Error('invalid quad');
        }
        if (body.readFloatLE(at + 16) > body.readFloatLE(at + 24) ||
            body.readFloatLE(at + 20) > body.readFloatLE(at + 28)) throw Error('invalid UV rectangle');
      }
      this.current = atlas; this.pending = undefined; this.frame = body;
      this.lastFrame = id; this.status = 'playing';
    } else throw Error('unknown IPC record');
    this.lastRecord = performance.now();
  }
  atlas(id) { return this.current?.readUInt32LE(4) === id ? this.current : undefined; }
  fail() { this.status = 'unavailable'; this.frame = this.current = this.pending = undefined; }
}

export function inputByte(direction) {
  const byte = new Map([['north', 'N'], ['east', 'E'], ['south', 'S'], ['west', 'W']]).get(direction);
  if (!byte) throw Error('invalid direction');
  return byte;
}

export function createServer(store, send, stop, {
  now = () => performance.now(), onView = () => {}, onConnect = () => 503, assets,
} = {}) {
  let lastInput = -Infinity;
  const server = httpServer({ maxHeaderSize: 8192, requestTimeout: 5000, headersTimeout: 5000 }, async (req, res) => {
    res.setHeader('Cache-Control', 'no-store');
    res.setHeader('X-Content-Type-Options', 'nosniff');
    res.setHeader('Cross-Origin-Resource-Policy', 'same-origin');
    res.setHeader('Content-Security-Policy', "default-src 'self'; script-src 'self'; style-src 'self'; connect-src 'self'; img-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'");
    const reply = (code, value = '', type = 'text/plain; charset=utf-8') => {
      res.writeHead(code, { 'Content-Type': type, 'Content-Length': Buffer.byteLength(value) }); res.end(value);
    };
    const authority = `127.0.0.1:${server.address()?.port}`, origin = `http://${authority}`;
    if (req.headers.host !== authority || (req.headers.origin && req.headers.origin !== origin) ||
        (req.headers['sec-fetch-site'] && !['same-origin', 'none'].includes(req.headers['sec-fetch-site']))) return reply(403);
    const path = req.url;
    if (req.method === 'GET' && staticFiles.has(path)) {
      const [file, type] = staticFiles.get(path);
      try { return reply(200, assets?.get(path) ?? await readFile(new URL(file, import.meta.url)), type); }
      catch { return reply(503, 'Static dependency unavailable. Install the pinned npm dependency.'); }
    }
    if (req.headers['x-oteryn-view'] !== '1') return reply(403);
    if (req.method === 'GET' && path === '/status') {
      onView(); return reply(200, JSON.stringify({ state: store.status }), 'application/json');
    }
    if (req.method === 'GET' && path === '/frame') {
      onView(); return store.frame ? reply(200, store.frame, 'application/octet-stream') : reply(409);
    }
    if (req.method === 'GET' && /^\/atlas\/[1-9][0-9]{0,9}$/.test(path)) {
      const bytes = store.atlas(Number(path.slice(7)));
      return bytes ? reply(200, bytes, 'application/octet-stream') : reply(410);
    }
    if (req.method !== 'POST' || !['/input', '/connect', '/disconnect'].includes(path)) return reply(404);
    if (req.headers.origin !== origin) return reply(403);
    if (req.headers['content-type'] !== 'application/json') return reply(415);
    if (path !== '/input') {
      // A ready/close signal carries no endpoint, credential or other configuration.
      if (req.headers['transfer-encoding'] || Number(req.headers['content-length'] ?? 0) !== 0) return reply(413);
      if (path === '/disconnect') { store.fail(); stop(); return reply(204); }
      return reply(store.status === 'unavailable' ? 409 : onConnect());
    }
    if (store.status !== 'playing') return reply(409);
    try {
      // Also bound chunked bodies; Content-Length is not trusted.
      let body = '', size = 0;
      for await (const chunk of req) {
        size += chunk.length;
        if (size > 256) return reply(413);
        body += chunk.toString('utf8');
      }
      const value = JSON.parse(body);
      if (!value || typeof value !== 'object' || Object.keys(value).join() !== 'direction') return reply(400);
      const byte = inputByte(value.direction);
      // Reading a slow request can yield to disconnect/native failure. Recheck before dispatch.
      if (store.status !== 'playing') return reply(409);
      if (now() - lastInput < 100) return reply(429);
      if (!send(byte)) return reply(503);
      lastInput = now(); reply(202);
    } catch { if (!res.headersSent) reply(400); }
  });
  server.maxConnections = 8;
  return server;
}

/** Check dependency presence/version before any native process can read an admission grant. */
async function prepareAssets(root) {
  const manifest = JSON.parse(await readFile(new URL('package.json', root), 'utf8'));
  const installed = JSON.parse(await readFile(new URL('node_modules/three/package.json', root), 'utf8'));
  if (installed.version !== manifest.dependencies.three) throw Error('pinned Three.js version unavailable');
  const assets = new Map();
  for (const [path, [file]] of staticFiles) assets.set(path, await readFile(new URL(file, root)));
  return assets;
}

// The executable and arguments are operator-supplied once, never accepted over HTTP.
export async function launch(binary, args, { root = new URL('./', import.meta.url), log = console.log } = {}) {
  const assets = await prepareAssets(root);
  const store = new SceneStore(); store.status = 'idle';
  let child, started = false, stopped = false, startedAt = 0, lastView = performance.now();
  const stop = () => {
    if (stopped) return;
    stopped = true; store.fail(); child?.stdin.destroy(); child?.kill();
  };
  // Invoked only after the page has loaded Three.js and initialized WebGL2. Reloading
  // an already active page attaches to the same host; a terminal session cannot restart.
  const activate = () => {
    if (stopped) return 409;
    if (started) return 200;
    started = true; startedAt = lastView = performance.now(); store.status = 'starting';
    try {
      child = spawn(binary, args, { shell: false, windowsHide: true, stdio: ['pipe', 'pipe', 'ignore'] });
      const decoder = new RecordDecoder(bytes => store.accept(bytes));
      child.stdout.on('data', chunk => { try { decoder.push(chunk); } catch { stop(); } });
      child.stdout.on('end', () => { try { decoder.finish(); } catch { /* unavailable below */ } stop(); });
      child.stdin.on('error', stop); child.stdout.on('error', stop); child.on('error', stop);
      child.on('exit', stop);
      return 202;
    } catch { stop(); return 503; }
  };
  const server = createServer(store, byte => {
    if (!child || stopped || child.stdin.destroyed || child.stdin.writableLength > 0) return false;
    child.stdin.write(byte); return true;
  }, stop, { assets, onConnect: activate, onView: () => { lastView = performance.now(); } });
  const watchdog = setInterval(() => {
    const now = performance.now();
    const idleLimit = store.status === 'playing' ? 15000 : 90000;
    if (now - lastView > idleLimit ||
        (store.status === 'starting' && now - startedAt > 90000) ||
        (store.status === 'playing' && now - store.lastRecord > 10000)) stop();
  }, 1000);
  watchdog.unref();
  const shutdown = () => { server.closeAllConnections(); server.close(); };
  server.on('close', () => {
    clearInterval(watchdog); stop();
    process.removeListener('SIGINT', shutdown); process.removeListener('SIGTERM', shutdown);
  });
  server.on('error', () => { stop(); clearInterval(watchdog); });
  server.listen(0, '127.0.0.1', () => log(`OTERYN development view: http://127.0.0.1:${server.address().port}/`));
  process.once('SIGINT', shutdown); process.once('SIGTERM', shutdown);
  return server;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const [binary, ...args] = process.argv.slice(2);
  if (!binary || args.length !== 5) {
    console.error('Usage: node bridge.mjs HOST_BINARY LOOPBACK_GAME_ADDR TLS_NAME ROOT_DER GRANT_FILE CHARACTER_HEX');
    process.exitCode = 2;
  } else {
    launch(binary, args).catch(() => {
      console.error('Client files unavailable. Run npm ci --ignore-scripts before starting the native session.');
      process.exitCode = 1;
    });
  }
}
