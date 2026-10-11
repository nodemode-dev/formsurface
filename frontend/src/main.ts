import init, { create_engine, type Engine } from './engine/pkg/fs_engine.js';
import { loadHighPoly, loadMeshFile, sampleHighPoly, sampleMesh, type FlatMesh } from './engine/meshes';
import type { BakeJob } from './engine/bake-worker';
import { VERSION } from './version';
import { LayerList, type LayerInfo } from './ui/layers';
import { TextureSetList, type TextureSetInfo } from './ui/sets';
import { TEMPLATES, encodePng, halfToUnit16, pack, sourcesOf, validateMaps, type MapSpec, type Pixels } from './ui/export-templates';
import { api } from './ui/api';
import { AgentLink } from './ui/agent-link';
import { BUILT_IN_MATERIALS, SMART_MASKS, saveMaterial, savedMaterials, type SmartMaterial } from './ui/smart';

const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;
const canvas = $<HTMLCanvasElement>('canvas');
const notice = $('notice');
const statusEl = $('status');
let engine: Engine | undefined;
let mesh: FlatMesh | undefined;
let meshName = 'sphere';
let tool: 'paint' | 'erase' | 'polyfill' = 'paint';
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
    mode: tool === 'erase' ? 'erase' : 'paint', mirror_x: false, backface_culling: checked('b-cull'), pressure_size: checked('b-psize'), pressure_flow: checked('b-pflow'),
    alignment: sel('b-align'), size_space: sel('b-space'), angle: num('b-angle'), follow_path: checked('b-follow'),
    jitter_size: num('b-jsize'), jitter_angle: num('b-jangle'), jitter_position: num('b-jpos'), jitter_flow: num('b-jflow'),
    lazy_px: num('b-lazy') * Math.min(window.devicePixelRatio || 1, 2), tip: sel('b-tip'), spray_density: num('b-density'),
    symmetry: sel('b-sym'), symmetry_axis: Number(sel('b-axis')), radial_count: num('b-radial'),
    stencil: checked('s-on'), stencil_scale: num('s-scale'), stencil_angle: num('s-angle'), stencil_x: num('s-x'), stencil_y: num('s-y'),
  }));
  mirror = sel('b-sym') === 'mirror';
  $('tool-mirror').setAttribute('aria-pressed', String(mirror));
}

function sel(id: string): string {
  return $<HTMLSelectElement>(id).value;
}

async function imageRgba(file: File, gray = true): Promise<{ w: number; h: number; data: Uint8Array }> {
  const bitmap = await createImageBitmap(file);
  const scale = Math.min(1, 2048 / Math.max(bitmap.width, bitmap.height));
  const w = Math.max(1, Math.round(bitmap.width * scale));
  const h = Math.max(1, Math.round(bitmap.height * scale));
  const surface = new OffscreenCanvas(w, h);
  const context = surface.getContext('2d')!;
  context.drawImage(bitmap, 0, 0, w, h);
  const pixels = context.getImageData(0, 0, w, h).data;
  for (let i = 0; gray && i < pixels.length; i += 4) {
    const l = (pixels[i] * 0.2126 + pixels[i + 1] * 0.7152 + pixels[i + 2] * 0.0722) * (pixels[i + 3] / 255);
    pixels[i] = pixels[i + 1] = pixels[i + 2] = Math.round(l);
    pixels[i + 3] = 255;
  }
  return { w, h, data: new Uint8Array(pixels.buffer) };
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

const setList = new TextureSetList($('texture-sets'), (index) => { selectTextureSet(index); });

function textureSets(): TextureSetInfo[] {
  return engine ? JSON.parse(engine.texture_sets_json()) as TextureSetInfo[] : [];
}

function refreshSets(): void {
  if (engine) setList.render(textureSets(), engine.active_texture_set());
}

function selectTextureSet(index: number): TextureSetInfo {
  const e = requireEngine();
  const set = textureSets().find((s) => s.index === index);
  if (!set) throw new Error(`No texture set ${index}`);
  if (!e.select_texture_set(index)) throw new Error('Finish the stroke before switching texture sets');
  const base = JSON.parse(e.base_json()) as { base_color: number[]; roughness: number; metallic: number };
  $<HTMLInputElement>('m-color').value = `#${base.base_color.map((v) => Math.round(Math.pow(Math.max(0, v), 1 / 2.2) * 255).toString(16).padStart(2, '0')).join('')}`;
  $<HTMLInputElement>('m-rough').value = String(base.roughness);
  $<HTMLInputElement>('m-metal').value = String(base.metallic);
  setOutput('m-rough'); setOutput('m-metal');
  bakeState();
  refreshLayers();
  status(`Texture set ${set.name}: strokes, layers, bakes and the base material now apply to it`);
  return set;
}

function refreshLayers(): void {
  if (!engine) return;
  const all = layers();
  const active = engine.active_layer();
  layerList.render(all, active);
  const layer = all.find((l) => l.id === active);
  if (layer) {
    $<HTMLSelectElement>('l-blend').value = layer.blend;
    $<HTMLSelectElement>('l-blend').disabled = layer.kind === 'folder';
    $('l-blend').title = layer.kind === 'folder' ? 'Folders pass through: each layer inside keeps its own blend mode' : '';
    $<HTMLInputElement>('l-opacity').value = String(layer.opacity);
    setOutput('l-opacity');
    const maskSelect = $<HTMLSelectElement>('l-mask');
    maskSelect.value = layer.mask ? 'keep' : 'none';
    const target = $<HTMLSelectElement>('l-target');
    target.disabled = !layer.mask || layer.kind === 'fill';
    target.value = layer.kind === 'fill' ? 'mask' : engine.paint_target_is_mask() ? 'mask' : 'layer';
    const isFill = layer.kind === 'fill';
    $('fill-section').setAttribute('aria-disabled', String(!isFill));
    if (isFill) loadFillControls(JSON.parse(engine.fill_settings(layer.id)) as Record<string, unknown>);
    const isFilter = layer.kind === 'filter';
    $('filter-section').setAttribute('aria-disabled', String(!isFilter));
    if (isFilter) loadFilterControls(JSON.parse(engine.filter_settings(layer.id)) as Record<string, number>);
    if (isFilter) target.disabled = true;
    syncParentOptions(all, layer);
  }
  refreshSets();
}

function syncParentOptions(all: LayerInfo[], layer: LayerInfo): void {
  const select = $<HTMLSelectElement>('l-parent');
  const blocked = new Set<number>([layer.id]);
  for (let grew = true; grew;) {
    grew = false;
    for (const l of all) if (l.parent && blocked.has(l.parent) && !blocked.has(l.id)) { blocked.add(l.id); grew = true; }
  }
  const wanted = [['0', 'None (top level)'], ...all.filter((l) => l.kind === 'folder' && !blocked.has(l.id)).map((f) => [String(f.id), f.name])];
  const current = [...select.options].map((o) => `${o.value}:${o.textContent}`).join('|');
  if (current !== wanted.map(([v, t]) => `${v}:${t}`).join('|')) {
    select.replaceChildren(...wanted.map(([v, t]) => { const o = document.createElement('option'); o.value = v; o.textContent = t; return o; }));
  }
  select.value = String(layer.parent ?? 0);
}

const FILL_FIELDS: [string, string][] = [
  ['f-proj', 'projection'], ['f-axis', 'axis'], ['f-pattern', 'pattern'], ['f-imgcolor', 'image_color'], ['f-scale', 'scale'], ['f-rot', 'rotation'],
  ['f-ox', 'offset_x'], ['f-oy', 'offset_y'], ['f-sharp', 'sharpness'], ['f-rough', 'roughness'], ['f-metal', 'metallic'], ['f-height', 'height'],
  ['f-use-color', 'use_color'], ['f-use-rough', 'use_roughness'], ['f-use-metal', 'use_metallic'], ['f-use-height', 'use_height'], ['f-opacity', 'opacity'],
  ['f-gen', 'generator'], ['f-glevel', 'generator_level'], ['f-gsoft', 'generator_softness'], ['f-ginvert', 'generator_invert'],
];

const FILTER_FIELDS: [string, string][] = [
  ['x-bright', 'brightness'], ['x-contrast', 'contrast'], ['x-hue', 'hue'], ['x-sat', 'saturation'], ['x-gamma', 'gamma'], ['x-rough', 'roughness_offset'], ['x-metal', 'metallic_offset'],
];

function syncFilter(): void {
  if (!engine) return;
  const layer = layers().find((l) => l.id === engine!.active_layer());
  if (!layer || layer.kind !== 'filter') return;
  const out: Record<string, number> = {};
  for (const [id, key] of FILTER_FIELDS) out[key] = num(id);
  engine.set_filter(layer.id, JSON.stringify(out));
}

function loadFilterControls(settings: Record<string, number>): void {
  for (const [id, key] of FILTER_FIELDS) {
    if (settings[key] === undefined) continue;
    $<HTMLInputElement>(id).value = String(settings[key]);
    setOutput(id);
  }
}

function bakeState(): void {
  const baked = !!engine?.is_baked();
  $('k-state').textContent = baked ? 'Baked: AO, curvature, thickness, position and world normal are available to generators and export' : 'Not baked: generators other than From top need baked maps';
}

function loadFillControls(settings: Record<string, unknown>): void {
  for (const [id, key] of FILL_FIELDS) {
    const input = $<HTMLInputElement>(id);
    const value = settings[key];
    if (value === undefined) continue;
    if (input.type === 'checkbox') input.checked = Boolean(value); else input.value = String(value);
    setOutput(id);
  }
  const c = settings.base_color as number[] | undefined;
  if (c) $<HTMLInputElement>('f-color').value = `#${c.map((v) => Math.round(Math.pow(Math.max(0, v), 1 / 2.2) * 255).toString(16).padStart(2, '0')).join('')}`;
}

function fillPayload(): Record<string, unknown> {
  const out: Record<string, unknown> = { base_color: hex('f-color') };
  for (const [id, key] of FILL_FIELDS) {
    const input = $<HTMLInputElement>(id);
    out[key] = input.type === 'checkbox' ? input.checked : input.tagName === 'SELECT' && key !== 'axis' ? input.value : Number(input.value);
  }
  return out;
}

function syncFill(): void {
  if (!engine) return;
  const layer = layers().find((l) => l.id === engine!.active_layer());
  if (!layer || layer.kind !== 'fill') return;
  engine.set_fill(layer.id, JSON.stringify(fillPayload()));
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

function setTool(next: 'paint' | 'erase' | 'polyfill'): void {
  tool = next;
  $('tool-paint').setAttribute('aria-pressed', String(next === 'paint'));
  $('tool-erase').setAttribute('aria-pressed', String(next === 'erase'));
  $('tool-polyfill').setAttribute('aria-pressed', String(next === 'polyfill'));
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

let meshGeneration = 0;

function applyMesh(next: FlatMesh, name: string): void {
  if (!engine) return;
  meshGeneration++;
  mesh = next;
  meshName = name;
  const info = engine.set_mesh(next.positions, next.normals, next.uvs, next.indices, next.materials, JSON.stringify(next.materialNames));
  $('mesh-info').textContent = `${name} · ${info}`;
  bakeState();
  refreshLayers();
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
  if (tool === 'polyfill') {
    try {
      const count = engine.polygon_fill(x, y, Number(sel('pf-mode')));
      status(`Filled ${count} triangle${count === 1 ? '' : 's'} with the paint material (Ctrl+Z undoes it)`);
    } catch (error) {
      status(error instanceof Error ? error.message : String(error), true);
    }
    return;
  }
  painting = engine.stroke_begin(x, y, pressure(event));
  const activeKind = layers().find((l) => l.id === engine!.active_layer())?.kind;
  if (!painting && (activeKind === 'fill' || activeKind === 'filter' || activeKind === 'folder')) status(`A ${activeKind} is painted through its mask: choose a mask in the layer properties first.`, true);
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
  else if (key === 'p') setTool('polyfill');
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
$('tool-polyfill').addEventListener('click', () => setTool('polyfill'));
$('tool-mirror').addEventListener('click', () => {
  $<HTMLSelectElement>('b-sym').value = mirror ? 'off' : 'mirror';
  syncBrush();
});
for (const id of ['b-angle', 'b-density', 'b-lazy', 'b-jsize', 'b-jangle', 'b-jpos', 'b-jflow', 'b-radial', 's-scale', 's-angle', 's-x', 's-y']) $(id).addEventListener('input', () => { setOutput(id); syncBrush(); });
for (const id of ['b-align', 'b-space', 'b-tip', 'b-sym', 'b-axis', 'b-follow', 's-on']) $(id).addEventListener('change', syncBrush);
const imageInput = $<HTMLInputElement>('image-file');
let imageTarget: 'alpha' | 'stencil' | 'fill' = 'alpha';
$('b-alpha-load').addEventListener('click', () => { imageTarget = 'alpha'; imageInput.click(); });
$('s-load').addEventListener('click', () => { imageTarget = 'stencil'; imageInput.click(); });
imageInput.addEventListener('change', async () => {
  const file = imageInput.files?.[0];
  imageInput.value = '';
  if (!file || !engine) return;
  try {
    const { w, h, data } = await imageRgba(file, imageTarget !== 'fill');
    if (imageTarget === 'fill') {
      const id = engine.active_layer();
      engine.set_fill_image(id, w, h, data);
      const option = $<HTMLSelectElement>('f-pattern').querySelector<HTMLOptionElement>('option[value="image"]')!;
      option.disabled = false;
      $<HTMLSelectElement>('f-pattern').value = 'image';
      syncFill();
      status(`Fill image ${file.name} loaded`);
      return;
    }
    if (imageTarget === 'alpha') {
      engine.set_alpha(w, h, data);
      const option = $<HTMLSelectElement>('b-tip').querySelector<HTMLOptionElement>('option[value="image"]')!;
      option.disabled = false;
      $<HTMLSelectElement>('b-tip').value = 'image';
      status(`Brush alpha ${file.name} loaded`);
    } else {
      engine.set_stencil(w, h, data);
      const on = $<HTMLInputElement>('s-on');
      on.disabled = false;
      on.checked = true;
      status(`Stencil ${file.name} loaded and enabled`);
    }
    syncBrush();
  } catch (error) {
    status(error instanceof Error ? error.message : String(error), true);
  }
});
$('undo').addEventListener('click', () => engine?.undo());
$('redo').addEventListener('click', () => engine?.redo());
$('frame').addEventListener('click', () => engine?.frame_mesh());
$('layer-add').addEventListener('click', () => {
  if (!engine) return;
  engine.add_layer(`Layer ${layers().length + 1}`);
  refreshLayers();
});
$('layer-add-fill').addEventListener('click', () => {
  if (!engine) return;
  engine.add_fill_layer(`Fill ${layers().length + 1}`);
  refreshLayers();
});
$('l-mask').addEventListener('change', () => {
  if (!engine) return;
  const id = engine.active_layer();
  const choice = sel('l-mask');
  if (choice === 'none') engine.set_mask(id, false);
  else if (choice === 'white' || choice === 'black') engine.fill_mask(id, choice === 'white');
  if (choice !== 'none') engine.set_paint_target(true);
  refreshLayers();
});
$('l-target').addEventListener('change', () => { engine?.set_paint_target(sel('l-target') === 'mask'); });
for (const [id] of FILL_FIELDS) $(id).addEventListener(id.startsWith('f-use') || id === 'f-imgcolor' || $(id).tagName === 'SELECT' ? 'change' : 'input', () => { setOutput(id); syncFill(); });
$('f-color').addEventListener('input', syncFill);
for (const [id] of FILTER_FIELDS) $(id).addEventListener('input', () => { setOutput(id); syncFilter(); });
$('layer-add-folder').addEventListener('click', () => {
  if (!engine) return;
  engine.add_folder(`Folder ${layers().filter((l) => l.kind === 'folder').length + 1}`);
  refreshLayers();
});
$('l-parent').addEventListener('change', () => {
  if (!engine) return;
  if (!engine.set_parent(engine.active_layer(), Number(sel('l-parent')))) status('A folder cannot go inside itself or its own subfolders.', true);
  refreshLayers();
});
$('layer-add-filter').addEventListener('click', () => {
  if (!engine) return;
  engine.add_filter_layer(`Filter ${layers().length + 1}`);
  refreshLayers();
});
let library: SmartMaterial[] = [...BUILT_IN_MATERIALS];

function fillOption(select: HTMLSelectElement, names: string[]): void {
  const existing = [...select.options].map((o) => o.value);
  for (const name of names) {
    if (existing.includes(name)) continue;
    const option = document.createElement('option');
    option.value = name;
    option.textContent = name;
    select.appendChild(option);
  }
}

async function refreshLibrary(): Promise<void> {
  const saved = await savedMaterials();
  library = [...BUILT_IN_MATERIALS, ...saved.filter((m) => !BUILT_IN_MATERIALS.some((b) => b.name === m.name))];
  fillOption($<HTMLSelectElement>('sm-list'), library.map((m) => m.name));
}
fillOption($<HTMLSelectElement>('sm-mask'), Object.keys(SMART_MASKS));
for (const template of TEMPLATES) {
  const option = document.createElement('option');
  option.value = template.name;
  option.textContent = template.label;
  $<HTMLSelectElement>('x-template').appendChild(option);
}
function describeTemplate(): void {
  const template = TEMPLATES.find((t) => t.name === sel('x-template'));
  $('x-maps').textContent = !template ? '' : template.maps.length
    ? `Writes ${template.maps.map((m) => m.suffix).join(', ')} per texture set; AO is white until baked.`
    : 'Writes basecolor, roughness, metallic, height and normal per texture set, plus every baked map.';
}
$('x-template').addEventListener('change', describeTemplate);
describeTemplate();
void refreshLibrary();

function applySmartMaterial(name: string): number[] {
  const e = requireEngine();
  const material = library.find((m) => m.name === name);
  if (!material) throw new Error(`No smart material named ${name}`);
  e.add_folder(material.name);
  const ids: number[] = [];
  for (const layer of material.layers) {
    const id = layer.kind === 'filter' ? e.add_filter_layer(layer.name) : e.add_fill_layer(layer.name);
    const current = JSON.parse(layer.kind === 'filter' ? e.filter_settings(id) : e.fill_settings(id)) as Record<string, unknown>;
    const merged = JSON.stringify({ ...current, ...layer.settings });
    if (layer.kind === 'filter') e.set_filter(id, merged); else e.set_fill(id, merged);
    if (layer.mask) e.fill_mask(id, layer.mask === 'white');
    const info = layers().find((l) => l.id === id);
    if (info && (layer.opacity !== undefined || layer.blend)) e.update_layer(JSON.stringify({ ...info, opacity: layer.opacity ?? info.opacity, blend: layer.blend ?? info.blend }));
    ids.push(id);
  }
  refreshLayers();
  return ids;
}

function captureSmartMaterial(name: string): SmartMaterial {
  const e = requireEngine();
  const captured = layers().filter((l) => l.kind === 'fill' || l.kind === 'filter').map((l) => ({
    kind: l.kind as 'fill' | 'filter',
    name: l.name,
    settings: JSON.parse(l.kind === 'filter' ? e.filter_settings(l.id) : e.fill_settings(l.id)) as Record<string, unknown>,
    opacity: l.opacity,
    blend: l.blend,
  }));
  if (!captured.length) throw new Error('Add fill or filter layers before saving a smart material');
  return { name, layers: captured };
}

function applySmartMask(id: number, name: string): Record<string, unknown> {
  const e = requireEngine();
  const preset = SMART_MASKS[name];
  if (!preset) throw new Error(`No smart mask named ${name}`);
  const raw = e.fill_settings(id);
  if (!raw) throw new Error('Smart masks apply to fill layers');
  e.set_fill(id, JSON.stringify({ ...JSON.parse(raw), ...preset }));
  refreshLayers();
  return JSON.parse(e.fill_settings(id)) as Record<string, unknown>;
}

$('sm-apply').addEventListener('click', () => {
  try { const ids = applySmartMaterial(sel('sm-list')); status(`Applied ${sel('sm-list')}: ${ids.length} layers in a folder of that name${engine?.is_baked() ? '' : '. Bake mesh maps for its generators to take effect'}`); } catch (error) { status(error instanceof Error ? error.message : String(error), true); }
});
$('sm-save').addEventListener('click', async () => {
  try {
    const name = $<HTMLInputElement>('sm-name').value.trim();
    if (!name) throw new Error('Name the smart material first');
    await saveMaterial(captureSmartMaterial(name));
    await refreshLibrary();
    status(`Saved smart material ${name}`);
  } catch (error) { status(error instanceof Error ? error.message : String(error), true); }
});
$('sm-mask-apply').addEventListener('click', () => {
  try { if (engine) applySmartMask(engine.active_layer(), sel('sm-mask')); status(`Applied smart mask ${sel('sm-mask')}`); } catch (error) { status(error instanceof Error ? error.message : String(error), true); }
});

let highMesh: FlatMesh | undefined;
let baking = false;

function setHigh(next: FlatMesh, name: string): string {
  const info = requireEngine().set_high_mesh(next.positions, next.normals, next.indices, next.parts);
  highMesh = next;
  $('k-high-state').textContent = `High-poly ${name}: ${info}`;
  return info;
}

function bakeInWorker(job: BakeJob): Promise<{ normal: Uint8Array; id: Uint8Array; hits: number; texels: number; size: number }> {
  return new Promise((resolve, reject) => {
    const worker = new Worker(new URL('./engine/bake-worker.ts', import.meta.url), { type: 'module' });
    worker.onmessage = (event) => {
      worker.terminate();
      if (event.data.ok) resolve(event.data); else reject(new Error(event.data.error));
    };
    worker.onerror = (event) => { worker.terminate(); reject(new Error(event.message || 'The bake worker failed to start')); };
    worker.postMessage(job);
  });
}

async function bakeHigh(): Promise<string> {
  const e = requireEngine();
  if (!highMesh) throw new Error('Load a high-poly mesh first');
  if (baking) throw new Error('A high-poly bake is already running');
  baking = true;
  const button = $<HTMLButtonElement>('k-bake-high');
  button.disabled = true;
  const generation = meshGeneration;
  const sets = textureSets();
  let hits = 0;
  let texels = 0;
  const started = performance.now();
  try {
    for (const set of sets) {
      const [positions, normals, uvs, indices, size] = e.texture_set_mesh(set.index) as [Float32Array, Float32Array, Float32Array, Uint32Array, number];
      $('k-high-state').textContent = `Baking ${set.name} in the background (${sets.indexOf(set) + 1} of ${sets.length}); you can keep working`;
      const high = { positions: highMesh.positions.slice(), normals: highMesh.normals.slice(), indices: highMesh.indices.slice(), parts: highMesh.parts.slice() };
      const result = await bakeInWorker({ low: [positions, normals, uvs, indices], high, size, front: num('k-front'), rear: num('k-rear') });
      if (generation !== meshGeneration || engine !== e) throw new Error('The mesh changed during the bake; bake again');
      e.apply_high_bake(set.index, result.size, result.normal, result.id);
      hits += result.hits;
      texels += result.texels;
    }
  } finally {
    baking = false;
    button.disabled = false;
    refreshLayers();
  }
  const text = `${hits} of ${texels} texels hit the high-poly mesh across ${sets.length} texture set${sets.length === 1 ? '' : 's'} in ${((performance.now() - started) / 1000).toFixed(1)} s, baked in a worker`;
  $('k-high-state').textContent = `Baked normal and ID: ${text}`;
  return text;
}
for (const id of ['k-front', 'k-rear']) $(id).addEventListener('input', () => setOutput(id));
const highInput = $<HTMLInputElement>('high-file');
$('k-high-load').addEventListener('click', () => highInput.click());
highInput.addEventListener('change', async () => {
  const file = highInput.files?.[0];
  highInput.value = '';
  if (!file || !engine) return;
  try { setHigh(await loadHighPoly(file.name, await file.arrayBuffer()), file.name); } catch (error) { status(error instanceof Error ? error.message : String(error), true); }
});
$('k-high-sample').addEventListener('click', () => {
  if (!engine) return;
  applyMesh(sampleMesh('sphere'), 'sphere');
  setHigh(sampleHighPoly(), 'bumpy sphere (two parts)');
  status('Low-poly sphere loaded with a bumpy two-part high-poly sphere; press Bake normal and ID');
});
$('k-bake-high').addEventListener('click', () => {
  bakeHigh().then((text) => status(text), (error) => status(error instanceof Error ? error.message : String(error), true));
});
$('k-bake').addEventListener('click', () => {
  if (!engine) return;
  try {
    status('Baking mesh maps on the GPU');
    engine.bake_maps(Number(sel('k-dirs')));
    bakeState();
    status('Mesh maps baked');
  } catch (error) {
    status(error instanceof Error ? error.message : String(error), true);
  }
});
$('f-image-load').addEventListener('click', () => { imageTarget = 'fill'; imageInput.click(); });
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
  status('Texture size changed: the texture sets were rebuilt and their layers cleared.');
  void boot();
});

const CHANNELS = ['basecolor', 'roughness', 'metallic', 'height', 'normal', 'ambient_occlusion', 'curvature', 'thickness', 'position', 'world_normal', 'id', 'normal_baked'];

async function pngOf(bytes: Uint8Array, size: number): Promise<Blob> {
  const surface = new OffscreenCanvas(size, size);
  const context = surface.getContext('2d')!;
  context.putImageData(new ImageData(new Uint8ClampedArray(bytes.buffer as ArrayBuffer, bytes.byteOffset, bytes.byteLength), size, size), 0, 0);
  return surface.convertToBlob({ type: 'image/png' });
}

async function channelPixels(e: Engine, channel: number, wide: boolean): Promise<Pixels> {
  return wide ? halfToUnit16(await e.export_channel_wide(channel) as Uint8Array) : await e.export_channel(channel) as Uint8Array;
}

async function renderMaps(templateName = sel('x-template'), custom?: MapSpec[], depth = Number(sel('x-depth'))): Promise<Map<string, Blob>> {
  const wide = depth === 16;
  const maps = new Map<string, Blob>();
  if (!engine) return maps;
  const e = engine;
  const template = custom ? { name: 'custom', label: 'Custom', maps: custom } : TEMPLATES.find((t) => t.name === templateName);
  if (!template) throw new Error(`Unknown export template ${templateName}. Templates: ${TEMPLATES.map((t) => t.name).join(', ')}`);
  const size = e.texture_size();
  const sets = textureSets();
  const original = e.active_texture_set();
  try {
    for (const set of sets) {
      if (!e.select_texture_set(set.index)) throw new Error('Finish the stroke before exporting');
      const prefix = sets.length > 1 ? `${set.material.replace(/[^A-Za-z0-9_-]/g, '_')}_` : '';
      const suffix = set.tiled ? `.${set.udim}` : '';
      if (template.maps.length) {
        const channels = new Map<number, Pixels>();
        for (const c of sourcesOf(template.maps)) {
          if (c >= 5 && !e.is_baked()) continue;
          channels.set(c, await channelPixels(e, c, wide));
        }
        for (const map of template.maps) maps.set(`${prefix}${map.suffix}${suffix}`, await encodePng(pack(map, channels, size, wide ? 65535 : 255), size, size, !!map.a));
        continue;
      }
      const wanted = [0, 1, 2, 3, 4, ...(e.is_baked() ? [5, 6, 7, 8, 9] : []), ...(e.is_high_baked() ? [10, 11] : [])];
      for (const i of wanted) {
        const pixels = await channelPixels(e, i, wide);
        maps.set(`${prefix}${CHANNELS[i]}${suffix}`, wide ? await encodePng(pixels, size, size, false) : await pngOf(pixels as Uint8Array, size));
      }
    }
  } finally {
    e.select_texture_set(original);
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

async function viewportPng(): Promise<Blob> {
  if (!engine) throw new Error('The GPU engine is not running');
  const [w, h] = engine.viewport_size();
  const bytes = await engine.snapshot() as Uint8Array;
  const surface = new OffscreenCanvas(w, h);
  surface.getContext('2d')!.putImageData(new ImageData(new Uint8ClampedArray(bytes.buffer as ArrayBuffer, bytes.byteOffset, bytes.byteLength), w, h), 0, 0);
  return surface.convertToBlob({ type: 'image/png' });
}

$('screenshot').addEventListener('click', async () => {
  if (!engine) return;
  try {
    const link = document.createElement('a');
    link.href = URL.createObjectURL(await viewportPng());
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

function requireEngine(): Engine {
  if (!engine) throw new Error('The GPU engine is not running in the editor');
  return engine;
}

function setControl(id: string, value: unknown, event = 'input'): void {
  if (value === undefined || value === null) return;
  const input = $<HTMLInputElement>(id);
  if (input.type === 'checkbox') input.checked = Boolean(value);
  else input.value = String(value);
  setOutput(id);
  input.dispatchEvent(new Event(event));
}

function snapshotState(): Record<string, unknown> {
  const e = requireEngine();
  return {
    mesh: meshName,
    mesh_info: $('mesh-info').textContent,
    texture_size: e.texture_size(),
    texture_sets: textureSets(),
    active_texture_set: e.active_texture_set(),
    layers: layers(),
    active_layer: e.active_layer(),
    tool,
    brush: {
      size_px: num('b-size'), hardness: num('b-hardness'), flow: num('b-flow'), stroke_opacity: num('b-opacity'), spacing: num('b-spacing'), mirror_x: mirror, backface_culling: checked('b-cull'),
      alignment: sel('b-align'), size_space: sel('b-space'), angle: num('b-angle'), follow_path: checked('b-follow'), tip: sel('b-tip'), spray_density: num('b-density'),
      lazy_px: num('b-lazy'), jitter_size: num('b-jsize'), jitter_angle: num('b-jangle'), jitter_position: num('b-jpos'), jitter_flow: num('b-jflow'),
      symmetry: sel('b-sym'), symmetry_axis: Number(sel('b-axis')), radial_count: num('b-radial'), stencil: checked('s-on'),
    },
    paint_material: { base_color: $<HTMLInputElement>('c-color').value, roughness: num('c-rough'), metallic: num('c-metal'), height: num('c-height'), paint_color: checked('c-use-color'), paint_roughness: checked('c-use-rough'), paint_metallic: checked('c-use-metal'), paint_height: checked('c-use-height') },
    camera: JSON.parse(e.camera_json()),
    viewport: e.viewport_size(),
  };
}

async function blobBase64(blob: Blob): Promise<string> {
  const bytes = new Uint8Array(await blob.arrayBuffer());
  let text = '';
  for (let i = 0; i < bytes.length; i += 0x8000) text += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  return btoa(text);
}

const agentLink = new AgentLink({
  status: () => snapshotState(),
  load_sample: ({ kind }) => { applyMesh(sampleMesh(String(kind)), String(kind)); return snapshotState(); },
  camera: ({ yaw, pitch, zoom }) => { requireEngine().set_orbit((Number(yaw) * Math.PI) / 180, (Number(pitch) * Math.PI) / 180, Number(zoom ?? 1)); return JSON.parse(requireEngine().camera_json()); },
  brush: (args) => {
    if (args.eraser !== undefined) setTool(args.eraser ? 'erase' : 'paint');
    if (args.mirror_x !== undefined && Boolean(args.mirror_x) !== mirror) $('tool-mirror').click();
    setControl('b-size', args.size_px); setControl('b-hardness', args.hardness); setControl('b-flow', args.flow);
    setControl('b-opacity', args.opacity); setControl('b-spacing', args.spacing); setControl('b-cull', args.backface_culling, 'change');
    setControl('b-align', args.alignment, 'change'); setControl('b-space', args.size_space, 'change'); setControl('b-angle', args.angle);
    setControl('b-follow', args.follow_path, 'change'); setControl('b-tip', args.tip, 'change'); setControl('b-density', args.spray_density);
    setControl('b-lazy', args.lazy_px); setControl('b-jsize', args.jitter_size); setControl('b-jangle', args.jitter_angle);
    setControl('b-jpos', args.jitter_position); setControl('b-jflow', args.jitter_flow); setControl('b-sym', args.symmetry, 'change');
    setControl('b-axis', args.symmetry_axis, 'change'); setControl('b-radial', args.radial_count);
    return snapshotState().brush as Record<string, unknown>;
  },
  material: (args) => {
    setControl('c-color', args.base_color); setControl('c-rough', args.roughness); setControl('c-metal', args.metallic); setControl('c-height', args.height);
    setControl('c-use-color', args.use_color); setControl('c-use-rough', args.use_roughness); setControl('c-use-metal', args.use_metallic); setControl('c-use-height', args.use_height);
    return snapshotState().paint_material as Record<string, unknown>;
  },
  base: (args) => { setControl('m-color', args.base_color); setControl('m-rough', args.roughness); setControl('m-metal', args.metallic); return {}; },
  add_layer: ({ name }) => { const id = requireEngine().add_layer(String(name || `Layer ${layers().length + 1}`)); refreshLayers(); return { id, layers: layers() }; },
  update_layer: (args) => {
    const e = requireEngine();
    const layer = layers().find((l) => l.id === Number(args.id));
    if (!layer) throw new Error(`No layer with id ${args.id}`);
    const next = { ...layer, ...(args.name !== undefined ? { name: String(args.name) } : {}), ...(args.visible !== undefined ? { visible: Boolean(args.visible) } : {}), ...(args.opacity !== undefined ? { opacity: Number(args.opacity) } : {}), ...(args.blend !== undefined ? { blend: String(args.blend) } : {}) };
    e.update_layer(JSON.stringify(next));
    if (args.active) e.select_layer(layer.id);
    refreshLayers();
    return { layers: layers(), active_layer: e.active_layer() };
  },
  add_fill_layer: (args) => {
    const e = requireEngine();
    const id = e.add_fill_layer(String(args.name || `Fill ${layers().length + 1}`));
    const current = JSON.parse(e.fill_settings(id)) as Record<string, unknown>;
    const patch = (args.settings ?? {}) as Record<string, unknown>;
    if (typeof patch.base_color === 'string') patch.base_color = [1, 3, 5].map((i) => Math.pow(parseInt((patch.base_color as string).slice(i, i + 2), 16) / 255, 2.2));
    e.set_fill(id, JSON.stringify({ ...current, ...patch }));
    refreshLayers();
    return { id, settings: JSON.parse(e.fill_settings(id)), layers: layers() };
  },
  set_fill: (args) => {
    const e = requireEngine();
    const id = Number(args.id);
    const raw = e.fill_settings(id);
    if (!raw) throw new Error(`Layer ${id} is not a fill layer`);
    const patch = (args.settings ?? {}) as Record<string, unknown>;
    if (typeof patch.base_color === 'string') patch.base_color = [1, 3, 5].map((i) => Math.pow(parseInt((patch.base_color as string).slice(i, i + 2), 16) / 255, 2.2));
    e.set_fill(id, JSON.stringify({ ...JSON.parse(raw), ...patch }));
    refreshLayers();
    return { id, settings: JSON.parse(e.fill_settings(id)) };
  },
  smart_list: async () => { await refreshLibrary(); return { materials: library.map((m) => ({ name: m.name, built_in: !!m.builtIn, layers: m.layers.map((l) => l.name) })), masks: Object.keys(SMART_MASKS) }; },
  smart_apply: async ({ name }) => { await refreshLibrary(); return { layer_ids: applySmartMaterial(String(name)), baked: requireEngine().is_baked(), layers: layers() }; },
  smart_mask: ({ id, name }) => ({ settings: applySmartMask(Number(id), String(name)) }),
  smart_save: async ({ name }) => { const file = await saveMaterial(captureSmartMaterial(String(name))); await refreshLibrary(); return { file }; },
  bake_high: async ({ file, sample, front, rear }) => {
    if (sample) {
      applyMesh(sampleMesh('sphere'), 'sphere');
      setHigh(sampleHighPoly(), 'bumpy sphere (two parts)');
    } else if (file) {
      const response = await fetch(`/api/meshes/${encodeURIComponent(String(file))}`);
      if (!response.ok) throw new Error(`High-poly ${file} is not in the workspace meshes folder`);
      setHigh(await loadHighPoly(String(file), await response.arrayBuffer()), String(file));
    }
    if (front !== undefined) setControl('k-front', front);
    if (rear !== undefined) setControl('k-rear', rear);
    return { result: await bakeHigh(), maps: ['id', 'normal_baked'] };
  },
  bake: ({ directions }) => {
    const e = requireEngine();
    e.bake_maps(Number(directions ?? 64));
    bakeState();
    return { baked: e.is_baked(), maps: ['ambient_occlusion', 'curvature', 'thickness', 'position', 'world_normal'] };
  },
  add_filter_layer: (args) => {
    const e = requireEngine();
    const id = e.add_filter_layer(String(args.name || `Filter ${layers().length + 1}`));
    const patch = (args.settings ?? {}) as Record<string, unknown>;
    e.set_filter(id, JSON.stringify({ ...JSON.parse(e.filter_settings(id)), ...patch }));
    refreshLayers();
    return { id, settings: JSON.parse(e.filter_settings(id)), layers: layers() };
  },
  set_filter: (args) => {
    const e = requireEngine();
    const id = Number(args.id);
    const raw = e.filter_settings(id);
    if (!raw) throw new Error(`Layer ${id} is not a filter layer`);
    e.set_filter(id, JSON.stringify({ ...JSON.parse(raw), ...((args.settings ?? {}) as Record<string, unknown>) }));
    refreshLayers();
    return { id, settings: JSON.parse(e.filter_settings(id)) };
  },
  mask: ({ id, action, paint_target }) => {
    const e = requireEngine();
    const layerId = Number(id);
    if (action === 'remove') e.set_mask(layerId, false);
    else if (action === 'add_white' || action === 'add_black') e.fill_mask(layerId, action === 'add_white');
    if (paint_target === 'mask' || paint_target === 'layer') e.set_paint_target(paint_target === 'mask');
    refreshLayers();
    return { layers: layers(), paints_mask: e.paint_target_is_mask() };
  },
  polygon_fill: ({ x, y, mode }) => {
    const e = requireEngine();
    const modes = ['triangle', 'polygon', 'mesh', 'uv_chunk'];
    const code = modes.indexOf(String(mode ?? 'mesh'));
    if (code < 0) throw new Error(`mode is one of ${modes.join(', ')}`);
    const [w, h] = e.viewport_size();
    const filled = e.polygon_fill(Number(x) * w, Number(y) * h, code);
    return { filled_triangles: filled, mode: modes[code], layer: e.active_layer() };
  },
  texture_sets: () => ({ texture_sets: textureSets(), active_texture_set: requireEngine().active_texture_set() }),
  select_texture_set: ({ index, name }) => {
    const all = textureSets();
    const match = name !== undefined && name !== null ? all.find((s) => s.name === String(name) || s.material === String(name)) : all.find((s) => s.index === Number(index));
    if (!match) throw new Error(`No texture set ${name ?? index}. Sets: ${all.map((s) => s.name).join(', ')}`);
    const set = selectTextureSet(match.index);
    return { active_texture_set: set, layers: layers(), active_layer: requireEngine().active_layer() };
  },
  add_folder: ({ name, layer_ids }) => {
    const e = requireEngine();
    const id = e.add_folder(String(name || `Folder ${layers().filter((l) => l.kind === 'folder').length + 1}`));
    const moved = ((layer_ids ?? []) as number[]).filter((child) => e.set_parent(Number(child), id));
    e.select_layer(id);
    refreshLayers();
    return { id, moved, layers: layers() };
  },
  set_parent: ({ id, folder_id }) => {
    const e = requireEngine();
    if (!e.set_parent(Number(id), Number(folder_id ?? 0))) throw new Error('That folder does not exist, or it is the layer itself or one of its subfolders');
    refreshLayers();
    return { layers: layers() };
  },
  move_layer: ({ id, offset }) => { requireEngine().move_layer(Number(id), Number(offset)); refreshLayers(); return { layers: layers() }; },
  remove_layer: ({ id }) => {
    if (!requireEngine().remove_layer(Number(id))) throw new Error('A texture set keeps at least one layer');
    refreshLayers();
    return { layers: layers() };
  },
  fill: () => { if (!requireEngine().fill_layer()) throw new Error('Load a mesh before filling'); return { filled_layer: requireEngine().active_layer() }; },
  stroke: ({ points, pressure }) => {
    const e = requireEngine();
    const list = points as [number, number][];
    const [w, h] = e.viewport_size();
    const p = Number(pressure ?? 1);
    let hits = 0;
    list.forEach(([u, v], i) => {
      const x = u * w;
      const y = v * h;
      if (e.hover(x, y)) hits++;
      if (i === 0) e.stroke_begin(x, y, p); else e.stroke_to(x, y, p);
    });
    e.stroke_end();
    e.clear_hover();
    return { points: list.length, points_on_surface: hits, layer: e.active_layer() };
  },
  undo: ({ steps }) => { let n = 0; for (let i = 0; i < Number(steps ?? 1); i++) if (requireEngine().undo()) n++; return { undone: n }; },
  redo: ({ steps }) => { let n = 0; for (let i = 0; i < Number(steps ?? 1); i++) if (requireEngine().redo()) n++; return { redone: n }; },
  screenshot: async () => ({ png_base64: await blobBase64(await viewportPng()) }),
  export: async ({ name, template, maps: custom, bit_depth }) => {
    const depth = bit_depth === undefined || bit_depth === null ? Number(sel('x-depth')) : Number(bit_depth);
    if (depth !== 8 && depth !== 16) throw new Error('bit_depth is 8 or 16');
    const maps = await renderMaps(template ? String(template) : sel('x-template'), custom ? validateMaps(custom) : undefined, depth);
    const folder = await api.saveMaps(String(name || meshName.replace(/\.[^.]+$/, '')), maps);
    return { folder, template: custom ? 'custom' : template ?? sel('x-template'), bit_depth: depth, maps: [...maps.keys()] };
  },
  export_templates: () => ({ templates: TEMPLATES.map((t) => ({ name: t.name, label: t.label, maps: t.maps })) }),
}, (state) => {
  $('agent-link').setAttribute('aria-pressed', String(state !== 'off'));
  $('agent-label').textContent = state === 'linked' ? 'Agent link on' : state === 'waiting' ? 'Agent link wait' : 'Agent link off';
  if (state === 'waiting') status('Agent link: waiting for the FormSurface service on 127.0.0.1:8766');
  if (state === 'linked') status('Agent link on: MCP agents can now edit this document');
});
$('agent-link').addEventListener('click', () => agentLink.toggle());

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
