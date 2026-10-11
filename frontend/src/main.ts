import init, { create_engine, type Engine } from './engine/pkg/fs_engine.js';
import { loadMeshFile, sampleMesh, type FlatMesh } from './engine/meshes';
import { VERSION } from './version';
import { LayerList, type LayerInfo } from './ui/layers';
import { api } from './ui/api';

const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;
const canvas = $<HTMLCanvasElement>('canvas');
const notice = $('notice');
const statusEl = $('status');
let engine: Engine | undefined;
let mesh: FlatMesh | undefined;
let meshName = 'sphere';
let tool: 'paint' | 'erase' = 'paint';
let mirror = false;
let painting = false;
let nav: 'orbit' | 'pan' | undefined;
let lastX = 0;
let lastY = 0;
let booting = false;

$('version').textContent = `v${VERSION}`;

function status(text: string, error = false): void {
  statusEl.textContent = text;
  statusEl.classList.toggle('error', error);
}

function num(id: string): number {
  return Number($<HTMLInputElement>(id).value);
}

function checked(id: string): boolean {
  return $<HTMLInputElement>(id).checked;
}

function hex(id: string): [number, number, number] {
  const v = $<HTMLInputElement>(id).value;
  const c = [1, 3, 5].map((i) => parseInt(v.slice(i, i + 2), 16) / 255);
  return c.map((x) => Math.pow(x, 2.2)) as [number, number, number];
}

function syncBrush(): void {
  engine?.set_brush(JSON.stringify({
    size_px: num('b-size'), hardness: num('b-hardness'), flow: num('b-flow'), opacity: num('b-opacity'), spacing: num('b-spacing'),
    mode: tool, mirror_x: mirror, backface_culling: checked('b-cull'), pressure_size: checked('b-psize'), pressure_flow: checked('b-pflow'),
  }));
}

function syncChannels(): void {
  engine?.set_channels(JSON.stringify({
    base_color: hex('c-color'), roughness: num('c-rough'), metallic: num('c-metal'), height: num('c-height'),
    use_color: checked('c-use-color'), use_roughness: checked('c-use-rough'), use_metallic: checked('c-use-metal'), use_height: checked('c-use-height'),
  }));
}

function syncBase(): void {
  const c = hex('m-color');
  engine?.set_base(c[0], c[1], c[2], num('m-rough'), num('m-metal'));
}

function syncView(): void {
  engine?.set_view(num('d-exposure'), num('d-normal'));
}

const layerList = new LayerList($('layers'), {
  select: (id) => { engine?.select_layer(id); refreshLayers(); },
  toggle: (layer) => { engine?.update_layer(JSON.stringify({ ...layer, visible: !layer.visible })); refreshLayers(); },
  move: (id, offset) => { engine?.move_layer(id, offset); refreshLayers(); },
});

function layers(): LayerInfo[] {
  return engine ? JSON.parse(engine.layers_json()) as LayerInfo[] : [];
}

function refreshLayers(): void {
  if (!engine) return;
  const all = layers();
  const active = engine.active_layer();
  layerList.render(all, active);
  const layer = all.find((l) => l.id === active);
  if (layer) {
    $<HTMLSelectElement>('l-blend').value = layer.blend;
    $<HTMLInputElement>('l-opacity').value = String(layer.opacity);
    setOutput('l-opacity');
  }
}

function setOutput(id: string): void {
  const input = $<HTMLInputElement>(id);
  const out = document.querySelector<HTMLOutputElement>(`output[for="${id}"]`);
  if (out) out.textContent = input.step && Number(input.step) < 1 ? Number(input.value).toFixed(2) : input.value;
}

function updateActiveLayer(patch: Partial<LayerInfo>): void {
  if (!engine) return;
  const layer = layers().find((l) => l.id === engine!.active_layer());
  if (layer) engine.update_layer(JSON.stringify({ ...layer, ...patch }));
  refreshLayers();
}

function setTool(next: 'paint' | 'erase'): void {
  tool = next;
  $('tool-paint').setAttribute('aria-pressed', String(next === 'paint'));
  $('tool-erase').setAttribute('aria-pressed', String(next === 'erase'));
  syncBrush();
}

function resize(): void {
  const rect = canvas.getBoundingClientRect();
  const ratio = Math.min(window.devicePixelRatio || 1, 2);
  const w = Math.max(1, Math.round(rect.width * ratio));
  const h = Math.max(1, Math.round(rect.height * ratio));
  if (canvas.width !== w || canvas.height !== h) {
    canvas.width = w;
    canvas.height = h;
    engine?.resize(w, h);
  }
}

function local(event: PointerEvent | WheelEvent): [number, number] {
  const rect = canvas.getBoundingClientRect();
  return [(event.clientX - rect.left) * (canvas.width / rect.width), (event.clientY - rect.top) * (canvas.height / rect.height)];
}

function pressure(event: PointerEvent): number {
  return event.pointerType === 'mouse' ? 1 : Math.max(0.05, event.pressure || 1);
}

function applyMesh(next: FlatMesh, name: string): void {
  if (!engine) return;
  mesh = next;
  meshName = name;
  const info = engine.set_mesh(next.positions, next.normals, next.uvs, next.indices);
  $('mesh-info').textContent = `${name} · ${info}`;
  status(next.skipped ? `Loaded ${name}. ${next.skipped} part(s) without UVs were skipped.` : `Loaded ${name}`);
}

async function boot(): Promise<void> {
  if (booting) return;
  booting = true;
  try {
    resize();
    engine?.free();
    engine = undefined;
    engine = await create_engine(canvas, Number($<HTMLSelectElement>('tex-size').value));
    $('gpu-info').textContent = `${engine.backend()} · ${engine.texture_size()} px`;
    syncBrush(); syncChannels(); syncBase(); syncView();
    applyMesh(mesh ?? sampleMesh('sphere'), mesh ? meshName : 'sphere');
    refreshLayers();
    notice.hidden = true;
  } catch (error) {
    notice.hidden = false;
    notice.textContent = `The GPU engine could not start: ${error instanceof Error ? error.message : String(error)}. FormSurface needs a browser with WebGPU (Chrome, Edge, Safari 26 or Firefox on Windows and macOS).`;
    status('GPU engine unavailable', true);
  } finally {
    booting = false;
  }
}

canvas.addEventListener('contextmenu', (e) => e.preventDefault());
canvas.addEventListener('pointerdown', (event) => {
  if (!engine) return;
  canvas.focus();
  canvas.setPointerCapture(event.pointerId);
  [lastX, lastY] = [event.clientX, event.clientY];
  if (event.button === 1 || (event.altKey && event.button === 2)) { nav = 'pan'; return; }
  if (event.button === 2 || (event.altKey && event.button === 0)) { nav = 'orbit'; return; }
  if (event.button !== 0) return;
  const [x, y] = local(event);
  painting = engine.stroke_begin(x, y, pressure(event));
});
canvas.addEventListener('pointermove', (event) => {
  if (!engine) return;
  if (nav) {
    const dx = event.clientX - lastX;
    const dy = event.clientY - lastY;
    [lastX, lastY] = [event.clientX, event.clientY];
    if (nav === 'orbit') engine.orbit(dx, dy); else engine.pan(dx, dy);
    return;
  }
  if (painting) {
    for (const sample of event.getCoalescedEvents?.() ?? [event]) {
      const [x, y] = local(sample);
      engine.stroke_to(x, y, pressure(sample));
    }
  } else {
    const [x, y] = local(event);
    engine.hover(x, y);
  }
});
const endPointer = () => {
  if (painting) engine?.stroke_end();
  painting = false;
  nav = undefined;
};
canvas.addEventListener('pointerup', endPointer);
canvas.addEventListener('pointercancel', endPointer);
canvas.addEventListener('pointerleave', () => { if (!painting) engine?.clear_hover(); });
canvas.addEventListener('wheel', (event) => { event.preventDefault(); engine?.zoom(event.deltaY); }, { passive: false });

window.addEventListener('keydown', (event) => {
  if (!engine || (event.target as HTMLElement).tagName === 'INPUT' && (event.target as HTMLInputElement).type !== 'range') return;
  const key = event.key.toLowerCase();
  if ((event.ctrlKey || event.metaKey) && key === 'z') { event.preventDefault(); if (event.shiftKey) engine.redo(); else engine.undo(); return; }
  if ((event.ctrlKey || event.metaKey) && key === 'y') { event.preventDefault(); engine.redo(); return; }
  if (key === 'b') setTool('paint');
  else if (key === 'e') setTool('erase');
  else if (key === 'm') $('tool-mirror').click();
  else if (key === 'f') engine.frame_mesh();
  else if (key === '[' || key === ']') {
    const input = $<HTMLInputElement>('b-size');
    input.value = String(Math.max(2, Math.min(400, Math.round(num('b-size') * (key === ']' ? 1.15 : 0.87)))));
    setOutput('b-size'); syncBrush();
  }
});

for (const id of ['b-size', 'b-hardness', 'b-flow', 'b-opacity', 'b-spacing']) $(id).addEventListener('input', () => { setOutput(id); syncBrush(); });
for (const id of ['b-cull', 'b-psize', 'b-pflow']) $(id).addEventListener('change', syncBrush);
for (const id of ['c-rough', 'c-metal', 'c-height']) $(id).addEventListener('input', () => { setOutput(id); syncChannels(); });
for (const id of ['c-color', 'c-use-color', 'c-use-rough', 'c-use-metal', 'c-use-height']) $(id).addEventListener('input', syncChannels);
for (const id of ['m-rough', 'm-metal']) $(id).addEventListener('input', () => { setOutput(id); syncBase(); });
$('m-color').addEventListener('input', syncBase);
for (const id of ['d-exposure', 'd-normal']) $(id).addEventListener('input', () => { setOutput(id); syncView(); });
$('l-opacity').addEventListener('input', () => { setOutput('l-opacity'); updateActiveLayer({ opacity: num('l-opacity') }); });
$('l-blend').addEventListener('change', () => updateActiveLayer({ blend: $<HTMLSelectElement>('l-blend').value }));

$('tool-paint').addEventListener('click', () => setTool('paint'));
$('tool-erase').addEventListener('click', () => setTool('erase'));
$('tool-mirror').addEventListener('click', () => {
  mirror = !mirror;
  $('tool-mirror').setAttribute('aria-pressed', String(mirror));
  syncBrush();
});
$('undo').addEventListener('click', () => engine?.undo());
$('redo').addEventListener('click', () => engine?.redo());
$('frame').addEventListener('click', () => engine?.frame_mesh());
$('layer-add').addEventListener('click', () => {
  if (!engine) return;
  engine.add_layer(`Layer ${layers().length + 1}`);
  refreshLayers();
});
$('layer-delete').addEventListener('click', () => {
  if (!engine) return;
  if (!engine.remove_layer(engine.active_layer())) status('A texture set keeps at least one layer.', true);
  refreshLayers();
});

const sampleMenu = $('sample-menu');
$('sample-toggle').addEventListener('click', () => {
  sampleMenu.hidden = !sampleMenu.hidden;
  $('sample-toggle').setAttribute('aria-expanded', String(!sampleMenu.hidden));
});
sampleMenu.addEventListener('click', (event) => {
  const kind = (event.target as HTMLElement).dataset.sample;
  if (!kind) return;
  sampleMenu.hidden = true;
  $('sample-toggle').setAttribute('aria-expanded', 'false');
  applyMesh(sampleMesh(kind), kind);
});
document.addEventListener('pointerdown', (event) => {
  if (!sampleMenu.hidden && !(event.target as HTMLElement).closest('.menu')) sampleMenu.hidden = true;
});

const fileInput = $<HTMLInputElement>('mesh-file');
$('open-mesh').addEventListener('click', () => fileInput.click());
fileInput.addEventListener('change', async () => {
  const file = fileInput.files?.[0];
  fileInput.value = '';
  if (!file) return;
  try {
    status(`Reading ${file.name}`);
    applyMesh(await loadMeshFile(file), file.name);
  } catch (error) {
    status(error instanceof Error ? error.message : String(error), true);
  }
});

$('tex-size').addEventListener('change', () => {
  status('Texture size changed: the texture set was rebuilt and its layers cleared.');
  void boot();
});

const CHANNELS = ['basecolor', 'roughness', 'metallic', 'height', 'normal'];

async function pngOf(bytes: Uint8Array, size: number): Promise<Blob> {
  const surface = new OffscreenCanvas(size, size);
  const context = surface.getContext('2d')!;
  context.putImageData(new ImageData(new Uint8ClampedArray(bytes.buffer as ArrayBuffer, bytes.byteOffset, bytes.byteLength), size, size), 0, 0);
  return surface.convertToBlob({ type: 'image/png' });
}

async function renderMaps(): Promise<Map<string, Blob>> {
  const maps = new Map<string, Blob>();
  if (!engine) return maps;
  const size = engine.texture_size();
  for (let i = 0; i < CHANNELS.length; i++) {
    const bytes = await engine.export_channel(i) as Uint8Array;
    maps.set(CHANNELS[i], await pngOf(bytes, size));
  }
  return maps;
}

$('export').addEventListener('click', async () => {
  try {
    status('Exporting maps');
    const maps = await renderMaps();
    const stem = meshName.replace(/\.[^.]+$/, '');
    if (await api.alive()) {
      const folder = await api.saveMaps(stem, maps);
      status(`Exported ${maps.size} maps to ${folder}`);
      return;
    }
    for (const [channel, blob] of maps) {
      const link = document.createElement('a');
      link.href = URL.createObjectURL(blob);
      link.download = `${stem}_${channel}.png`;
      link.click();
      setTimeout(() => URL.revokeObjectURL(link.href), 4000);
    }
    status(`Downloaded ${maps.size} maps (service not running)`);
  } catch (error) {
    status(error instanceof Error ? error.message : String(error), true);
  }
});

$('screenshot').addEventListener('click', async () => {
  if (!engine) return;
  try {
    const [w, h] = engine.viewport_size();
    const bytes = await engine.snapshot() as Uint8Array;
    const surface = new OffscreenCanvas(w, h);
    surface.getContext('2d')!.putImageData(new ImageData(new Uint8ClampedArray(bytes.buffer as ArrayBuffer, bytes.byteOffset, bytes.byteLength), w, h), 0, 0);
    const link = document.createElement('a');
    link.href = URL.createObjectURL(await surface.convertToBlob({ type: 'image/png' }));
    link.download = `${meshName.replace(/\.[^.]+$/, '')}_viewport.png`;
    link.click();
    setTimeout(() => URL.revokeObjectURL(link.href), 4000);
    status('Viewport screenshot saved');
  } catch (error) {
    status(error instanceof Error ? error.message : String(error), true);
  }
});

$('save-project').addEventListener('click', async () => {
  try {
    if (!(await api.alive())) { status('Start the FormSurface service (./formsurface) to save projects.', true); return; }
    const maps = await renderMaps();
    const folder = await api.saveProject(meshName.replace(/\.[^.]+$/, ''), { mesh: meshName, layers: layers(), texture_size: engine?.texture_size() }, maps);
    status(`Saved to ${folder}`);
  } catch (error) {
    status(error instanceof Error ? error.message : String(error), true);
  }
});

let frames = 0;
let lastFps = performance.now();
function tick(now: number): void {
  resize();
  engine?.frame();
  frames++;
  if (now - lastFps > 1000) {
    $('fps').textContent = `${Math.round((frames * 1000) / (now - lastFps))} fps`;
    frames = 0;
    lastFps = now;
  }
  requestAnimationFrame(tick);
}

await init();
await boot();
requestAnimationFrame(tick);
