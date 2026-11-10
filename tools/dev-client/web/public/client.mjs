import { MAX_QUADS, MAX_ATLAS_BYTES, MAX_FRAME_BYTES, decodeAtlas, decodeFrame, boundedBytes } from './scene.mjs';
const canvas = document.querySelector('canvas'), status = document.querySelector('#status');
const details = document.querySelector('#details'), disconnect = document.querySelector('#disconnect');
const headers = { 'X-Oteryn-View': '1' };
let running = true, admitted = false, held = null, sending = false, draw = null, cleanup = () => {};
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));
const request = (path, options = {}) => fetch(path, { ...options, headers: { ...headers, ...options.headers }, cache: 'no-store', signal: AbortSignal.timeout(5000) });
function stop(message) {
  running = admitted = false; held = null; clearInterval(repeat); status.textContent = message;
  canvas.hidden = true; details.textContent = ''; disconnect.disabled = true;
  cleanup(); cleanup = () => {};
}
async function closeHost() {
  try { await request('/disconnect', { method: 'POST', headers: { 'Content-Type': 'application/json' } }); } catch { /* no credential replay */ }
}
async function step(direction) {
  if (!admitted || !running || sending || document.hidden) return;
  sending = true;
  try {
    const result = await request('/input', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ direction }) });
    if (![202, 429].includes(result.status)) throw Error('Wejście niedostępne');
  } catch { stop('Połączenie przerwane. Uruchom ponownie klienta lokalnego.'); }
  finally { sending = false; }
}
const keys = new Map([['ArrowUp', 'north'], ['KeyW', 'north'], ['ArrowRight', 'east'], ['KeyD', 'east'], ['ArrowDown', 'south'], ['KeyS', 'south'], ['ArrowLeft', 'west'], ['KeyA', 'west']]);
window.addEventListener('keydown', event => {
  if (event.ctrlKey || event.altKey || event.metaKey || event.target.matches('input, textarea, [contenteditable]')) return;
  const direction = keys.get(event.code);
  if (direction && admitted) { event.preventDefault(); if (!event.repeat) { held = event.code; void step(direction); } }
});
window.addEventListener('keyup', event => { if (held === event.code) held = null; });
window.addEventListener('blur', () => { held = null; });
document.addEventListener('visibilitychange', () => { held = null; });
const repeat = setInterval(() => { if (held) void step(keys.get(held)); }, 150);
window.addEventListener('pagehide', () => { held = null; clearInterval(repeat); });
for (const button of document.querySelectorAll('[data-direction]')) button.addEventListener('click', () => void step(button.dataset.direction));
disconnect.addEventListener('click', () => { stop('Sesja lokalna zamknięta.'); void closeHost(); });
canvas.addEventListener('webglcontextlost', event => { event.preventDefault(); stop('Utracono kontekst GPU.'); void closeHost(); });

async function boot() {
  try {
    const T = await import('/vendor/three.module.js');
    const context = canvas.getContext('webgl2', { alpha: false, antialias: false });
    if (!context) throw Error('Ta przeglądarka nie udostępnia WebGL2.');
    const renderer = new T.WebGLRenderer({ canvas, context, antialias: false });
    renderer.setPixelRatio(1); renderer.setClearColor(0x000000);
    const camera = new T.OrthographicCamera(0, 1, 0, 1, 0.1, 10); camera.position.z = 1;
    const scene = new T.Scene();
    const geometry = new T.InstancedBufferGeometry();
    geometry.setAttribute('position', new T.Float32BufferAttribute([0,0,0, 1,0,0, 1,1,0, 0,0,0, 1,1,0, 0,1,0], 3));
    const instances = new T.InstancedInterleavedBuffer(new Float32Array(MAX_QUADS * 8), 8, 1);
    instances.setUsage(T.DynamicDrawUsage);
    geometry.setAttribute('rect', new T.InterleavedBufferAttribute(instances, 4, 0));
    geometry.setAttribute('atlasUV', new T.InterleavedBufferAttribute(instances, 4, 4));
    geometry.instanceCount = 0;
    const material = new T.ShaderMaterial({
      uniforms: { atlas: { value: null } }, transparent: true, depthTest: false, depthWrite: false,
      vertexShader: `attribute vec4 rect; attribute vec4 atlasUV; varying vec2 texCoord;
        void main() { vec2 p = position.xy; texCoord = mix(atlasUV.xy, atlasUV.zw, p);
        gl_Position = projectionMatrix * modelViewMatrix * vec4(rect.xy + p * rect.zw, 0.0, 1.0); }`,
      fragmentShader: `uniform sampler2D atlas; varying vec2 texCoord;
        void main() { vec4 c = texture2D(atlas, texCoord); if (c.a == 0.0) discard; gl_FragColor = c; }`,
      side: T.DoubleSide,
    });
    const mesh = new T.Mesh(geometry, material); mesh.frustumCulled = false; scene.add(mesh);
    let texture = null, atlasId = 0, frameId = 0;
    cleanup = () => { clearInterval(repeat); geometry.dispose(); material.dispose(); texture?.dispose(); renderer.dispose(); };
    draw = async frame => {
      if (frame.id <= frameId) return;
      if (frame.width > renderer.capabilities.maxTextureSize || frame.height > renderer.capabilities.maxTextureSize) throw Error('Widok przekracza limit GPU');
      if (frame.atlasId !== atlasId) {
        const response = await request(`/atlas/${frame.atlasId}`);
        if (response.status === 410) return; // Fetch a fresh pair, never mix atlas generations.
        if (!response.ok) throw Error('Atlas niedostępny');
        const atlas = decodeAtlas(await boundedBytes(response, MAX_ATLAS_BYTES));
        if (atlas.id !== frame.atlasId || atlas.width > renderer.capabilities.maxTextureSize || atlas.height > renderer.capabilities.maxTextureSize) throw Error('Atlas niezgodny z GPU');
        if (!running) return;
        const next = new T.DataTexture(atlas.pixels, atlas.width, atlas.height, T.RGBAFormat);
        next.magFilter = next.minFilter = T.NearestFilter; next.generateMipmaps = false;
        next.flipY = false; next.needsUpdate = true;
        const previous = texture; texture = next; material.uniforms.atlas.value = next; previous?.dispose();
        atlasId = atlas.id;
      }
      if (!running) return;
      camera.right = frame.width; camera.bottom = frame.height; camera.updateProjectionMatrix();
      if (canvas.width !== frame.width || canvas.height !== frame.height) renderer.setSize(frame.width, frame.height, false);
      instances.array.set(frame.quads); instances.clearUpdateRanges(); instances.addUpdateRange(0, frame.count * 8); instances.needsUpdate = true;
      geometry.instanceCount = frame.count;
      renderer.render(scene, camera); canvas.hidden = false; frameId = frame.id; admitted = true;
      status.textContent = 'Sesja Rust · scena serwera';
      details.textContent = `${frame.width} × ${frame.height} · ${frame.count} sprite’ów · atlas ${atlasId}`;
    };
    while (running) {
      if (document.hidden) { await sleep(250); continue; }
      const response = await request('/frame');
      if (!running) break;
      if (response.status === 409) {
        const healthResponse = await request('/status');
        if (!healthResponse.ok) throw Error('Adapter niedostępny');
        const health = await healthResponse.json();
        if (health.state !== 'starting') throw Error('Sesja niedostępna lub zakończona');
        status.textContent = 'Oczekiwanie na przyjęcie do gry i mapę serwera…';
      } else {
        if (!response.ok) throw Error('Adapter niedostępny');
        await draw(decodeFrame(await boundedBytes(response, MAX_FRAME_BYTES)));
      }
      await sleep(50);
    }
  } catch (error) {
    if (!running) return;
    stop(`Klient niedostępny: ${error instanceof Error ? error.message : 'błąd prezentacji'}`);
    await closeHost();
  }
}
void boot();
