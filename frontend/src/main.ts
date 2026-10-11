import init, { create_engine, type Engine } from './engine/pkg/fs_engine.js';
import { loadHighPoly, loadMeshFile, sampleHighPoly, sampleMesh, type FlatMesh } from './engine/meshes';
import type { BakeJob } from './engine/bake-worker';
import { VERSION } from './version';
import { LayerList, type LayerInfo } from './ui/layers';
import { TextureSetList, type TextureSetInfo } from './ui/sets';
import { TEMPLATES, halfToFloat, halfToUnit16, pack, sourcesOf, validateMaps, type MapSpec, type Pixels, type Template } from './ui/export-templates';
import { FORMAT_ALPHA, FORMAT_DEPTHS, checkFormat, encodeImage, gammaToLinear, sampleMax, type Format } from './ui/image-formats';
import { PADDING_MODES, applyPadding, nearestSource, uvMask, type Padding, type PaddingMode } from './ui/padding';
import { TemplateEditor, loadTemplates, saveTemplate } from './ui/template-editor';
import { api } from './ui/api';
import { AgentLink } from './ui/agent-link';
import { BUILT_IN_MATERIALS, SMART_MASKS, generatorPart, saveMask, saveMaterial, savedMasks, savedMaterials, type SmartMaterial } from './ui/smart';
import { parseHdr, type HdrImage } from './ui/hdr';
import { clearOverlay, drawPath, pathSamples, type Vec3 } from './ui/path';
import { fontLabel, loadFont, textImage } from './ui/text';

const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;
const canvas = $<HTMLCanvasElement>('canvas');
const notice = $('notice');
const statusEl = $('status');
let engine: Engine | undefined;
let mesh: FlatMesh | undefined;
let meshName = 'sphere';
type Tool = 'paint' | 'erase' | 'polyfill' | 'clone' | 'path';
let tool: Tool = 'paint';
const pathOverlay = $<HTMLCanvasElement>('path-overlay');
const pathPoints: Vec3[] = [];
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

function hexColor(v: string): [number, number, number] {
  if (!/^#[0-9a-fA-F]{6}$/.test(v)) throw new Error('Colors are #rrggbb');
  return [1, 3, 5].map((i) => Math.pow(parseInt(v.slice(i, i + 2), 16) / 255, 2.2)) as [number, number, number];
}

function hex(id: string): [number, number, number] {
  const v = $<HTMLInputElement>(id).value;
  const c = [1, 3, 5].map((i) => parseInt(v.slice(i, i + 2), 16) / 255);
  return c.map((x) => Math.pow(x, 2.2)) as [number, number, number];
}

function syncBrush(): void {
  engine?.set_brush(JSON.stringify({
    size_px: num('b-size'), hardness: num('b-hardness'), flow: num('b-flow'), opacity: num('b-opacity'), spacing: num('b-spacing'),
    mode: tool === 'erase' ? 'erase' : tool === 'clone' ? 'clone' : 'paint', mirror_x: false, backface_culling: checked('b-cull'), pressure_size: checked('b-psize'), pressure_flow: checked('b-pflow'),
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
  engine?.set_environment_view(num('d-env-rot'), num('d-env-blur'), checked('d-env-bg'));
}

const environments = new Map<string, HdrImage>();

function useEnvironment(name: string): void {
  const e = requireEngine();
  if (!name) { e.clear_environment(); $<HTMLSelectElement>('d-env').value = ''; return; }
  const image = environments.get(name);
  if (!image) throw new Error(`Environment ${name} is not loaded`);
  e.set_environment(name, image.width, image.height, image.rgb);
  const list = $<HTMLSelectElement>('d-env');
  if (![...list.options].some((o) => o.value === name)) list.add(new Option(name, name));
  list.value = name;
}

async function loadEnvironment(name: string, buffer: ArrayBuffer): Promise<{ width: number; height: number }> {
  const image = parseHdr(buffer);
  environments.set(name, image);
  useEnvironment(name);
  return { width: image.width, height: image.height };
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
  if (!engine) return;
  const sets = textureSets();
  const active = engine.active_texture_set();
  setList.render(sets, active);
  $<HTMLInputElement>('uv-tiles').checked = engine.uv_tiles();
  const select = $<HTMLSelectElement>('l-instance');
  const wanted = [['', 'Choose a texture set'], ...sets.filter((s) => s.index !== active).map((s) => [String(s.index), s.name])];
  if ([...select.options].map((o) => `${o.value}:${o.textContent}`).join('|') !== wanted.map(([v, t]) => `${v}:${t}`).join('|')) {
    select.replaceChildren(...wanted.map(([v, t]) => { const o = document.createElement('option'); o.value = v; o.textContent = t; return o; }));
  }
  const kind = layers().find((l) => l.id === engine!.active_layer())?.kind;
  select.disabled = sets.length < 2 || (kind !== 'fill' && kind !== 'filter');
}

function instanceLayer(id: number, target: number): number {
  const created = requireEngine().instance_layer(id, target);
  refreshLayers();
  return created;
}

$('l-instance').addEventListener('change', () => {
  if (!engine) return;
  const value = sel('l-instance');
  $<HTMLSelectElement>('l-instance').value = '';
  if (!value) return;
  try {
    instanceLayer(engine.active_layer(), Number(value));
    status(`Instanced into ${textureSets().find((s) => s.index === Number(value))?.name}: changes to its settings, mask generator, name, opacity or blend now update every copy`);
  } catch (error) {
    status(error instanceof Error ? error.message : String(error), true);
  }
});

function setUvTiles(on: boolean): void {
  const e = requireEngine();
  if (!e.set_uv_tiles(on)) throw new Error('Finish the stroke first');
  refreshLayers();
  status(on ? 'UV tiles on: the UDIM tiles of each material share the active tile\'s layer stack and strokes paint across them; other tiles\' previous paint was replaced' : 'UV tiles off: every tile keeps its own layer stack from now on');
}

$('uv-tiles').addEventListener('change', () => {
  try { setUvTiles(checked('uv-tiles')); } catch (error) { status(error instanceof Error ? error.message : String(error), true); }
});

const MASK_GENERATORS = ['cavity', 'edges', 'thin', 'top', 'light', 'position', 'noise', 'mask_builder', 'dirt', 'metal_edge_wear'];
let pickingId = false;

function maskFromSettings(id: number, settings: Record<string, unknown>): void {
  const e = requireEngine();
  if (!MASK_GENERATORS.includes(String(settings.generator))) throw new Error(`generator is one of ${MASK_GENERATORS.join(', ')}`);
  e.mask_generator(id, JSON.stringify({ pattern: 'solid', use_color: true, use_roughness: false, use_metallic: false, use_height: false, opacity: 1, ...settings }));
  e.set_paint_target(true);
  refreshLayers();
}

function maskFromGenerator(id: number, generator: string, level = 0.5, softness = 0.1, invert = false): void {
  maskFromSettings(id, { generator, generator_level: level, generator_softness: softness, generator_invert: invert });
}

function maskFromId(id: number, color: string, tolerance = 0.06): void {
  const e = requireEngine();
  if (!/^#[0-9a-fA-F]{6}$/.test(color)) throw new Error('color is a #rrggbb ID color');
  const [r, g, b] = [1, 3, 5].map((i) => parseInt(color.slice(i, i + 2), 16) / 255);
  e.mask_from_id(id, r, g, b, tolerance);
  e.set_paint_target(true);
  refreshLayers();
}

$('l-msrc').addEventListener('change', () => {
  if (!engine) return;
  const source = sel('l-msrc');
  $<HTMLSelectElement>('l-msrc').value = '';
  try {
    if (MASK_GENERATORS.includes(source)) {
      maskFromGenerator(engine.active_layer(), source);
      status(`Mask generated from ${source}${engine.is_baked() ? '' : '; bake mesh maps for it to follow the shape'}. It updates after every bake; strokes on it last until then.`);
    } else if (source === 'id') {
      pickingId = true;
      status('Click the part whose ID color should become the mask');
    } else if (source === 'image') {
      imageTarget = 'mask';
      imageInput.click();
    }
  } catch (error) {
    status(error instanceof Error ? error.message : String(error), true);
  }
});

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
  ['f-gnoise', 'generator_noise'], ['f-gscale', 'noise_scale'], ['f-bao', 'builder_ao'], ['f-bcurv', 'builder_curvature'], ['f-bnoise', 'builder_noise'],
];

function directionOf(yawDegrees: number, pitchDegrees: number): [number, number, number] {
  const [y, p] = [(yawDegrees * Math.PI) / 180, (pitchDegrees * Math.PI) / 180];
  return [Math.sin(y) * Math.cos(p), Math.sin(p), Math.cos(y) * Math.cos(p)];
}

const FILTER_FIELDS: [string, string][] = [
  ['x-bright', 'brightness'], ['x-contrast', 'contrast'], ['x-hue', 'hue'], ['x-sat', 'saturation'], ['x-gamma', 'gamma'], ['x-rough', 'roughness_offset'], ['x-metal', 'metallic_offset'],
  ['x-blur', 'blur'], ['x-sharp', 'sharpen'],
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
  const d = settings.generator_direction as number[] | undefined;
  if (d && d.length === 3) {
    $<HTMLInputElement>('f-gyaw').value = String(Math.round((Math.atan2(d[0], d[2]) * 180) / Math.PI));
    $<HTMLInputElement>('f-gpitch').value = String(Math.round((Math.asin(Math.max(-1, Math.min(1, d[1]))) * 180) / Math.PI));
    setOutput('f-gyaw'); setOutput('f-gpitch');
  }
  const c = settings.base_color as number[] | undefined;
  if (c) $<HTMLInputElement>('f-color').value = `#${c.map((v) => Math.round(Math.pow(Math.max(0, v), 1 / 2.2) * 255).toString(16).padStart(2, '0')).join('')}`;
}

function fillPayload(): Record<string, unknown> {
  const out: Record<string, unknown> = { base_color: hex('f-color'), generator_direction: directionOf(num('f-gyaw'), num('f-gpitch')) };
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

function setTool(next: Tool): void {
  tool = next;
  $('tool-paint').setAttribute('aria-pressed', String(next === 'paint'));
  $('tool-erase').setAttribute('aria-pressed', String(next === 'erase'));
  $('tool-polyfill').setAttribute('aria-pressed', String(next === 'polyfill'));
  $('tool-clone').setAttribute('aria-pressed', String(next === 'clone'));
  $('tool-path').setAttribute('aria-pressed', String(next === 'path'));
  if (next === 'clone' && engine && !engine.has_clone_source()) status('Clone: Ctrl+click the mesh to set the source, then paint where the copy should go');
  if (next === 'path') status('Path: click points on the surface, then Enter or Stroke path paints a curve through them with the current brush');
  if (next !== 'path') { pathPoints.length = 0; clearOverlay(pathOverlay); }
  syncBrush();
}

function setView2d(on: boolean): boolean {
  const e = requireEngine();
  if (!e.set_view_2d(on)) throw new Error('Finish the stroke first');
  $('view-2d').setAttribute('aria-pressed', String(on));
  if (on && tool === 'path') setTool('paint');
  status(on ? '2D view: the active texture set\'s UV layout. Strokes paint straight into the texture; U returns to 3D' : '3D view');
  return on;
}

function addPathPoint(x: number, y: number): boolean {
  const p = requireEngine().surface_point(x, y);
  if (p.length !== 3) return false;
  pathPoints.push([p[0], p[1], p[2]]);
  return true;
}

const nextFrame = () => new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));

async function strokePath(closed = checked('p-closed'), smooth = checked('p-smooth'), erase = checked('p-erase')): Promise<number> {
  const e = requireEngine();
  if (pathPoints.length < 2) throw new Error('A path needs at least two points on the surface');
  if (e.view_2d()) throw new Error('The path tool works in the 3D view');
  const screen = pathSamples(pathPoints, closed && pathPoints.length > 2, smooth).map((p) => e.screen_point(p[0], p[1], p[2]));
  const first = screen.findIndex((s) => s.length === 2);
  if (first < 0) throw new Error('The path is behind the camera');
  const keep = tool;
  if (erase) { tool = 'erase'; syncBrush(); }
  let landed = 0;
  try {
    if (!e.stroke_begin(screen[first][0], screen[first][1], 1)) throw new Error('The active layer cannot take paint: pick a paint layer or a mask');
    for (const s of screen.slice(first + 1)) {
      if (s.length !== 2) continue;
      e.stroke_to(s[0], s[1], 1);
      landed++;
      if (landed % 12 === 0) await nextFrame();
    }
    e.stroke_end();
  } finally {
    if (erase) { tool = keep; syncBrush(); }
  }
  return landed;
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
    if (sel('d-env')) useEnvironment(sel('d-env'));
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
  if (pickingId) {
    pickingId = false;
    try {
      const color = engine.id_color_at(x, y);
      maskFromId(engine.active_layer(), color);
      status(`Mask set to ID color ${color}`);
    } catch (error) {
      status(error instanceof Error ? error.message : String(error), true);
    }
    return;
  }
  if (tool === 'clone' && (event.ctrlKey || event.metaKey)) {
    try {
      engine.set_clone_source(x, y);
      status('Clone source set: the first stroke fixes the offset, later strokes keep it');
    } catch (error) {
      status(error instanceof Error ? error.message : String(error), true);
    }
    return;
  }
  if (tool === 'path') {
    if (engine.view_2d()) { status('The path tool works in the 3D view; press U to return to it', true); return; }
    if (addPathPoint(x, y)) status(`Path: ${pathPoints.length} point${pathPoints.length === 1 ? '' : 's'}; Enter strokes it, Backspace removes the last, Esc clears`);
    else status('Click on the mesh to place a path point', true);
    return;
  }
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
  if (!painting && tool === 'clone' && !engine.has_clone_source()) { status('Ctrl+click the mesh to set a clone source first', true); return; }
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
  if (!engine || (event.target as HTMLElement).tagName === 'INPUT' && !['range', 'checkbox'].includes((event.target as HTMLInputElement).type)) return;
  const key = event.key.toLowerCase();
  if ((event.ctrlKey || event.metaKey) && key === 'z') { event.preventDefault(); if (event.shiftKey) engine.redo(); else engine.undo(); return; }
  if ((event.ctrlKey || event.metaKey) && key === 'y') { event.preventDefault(); engine.redo(); return; }
  if (tool === 'path' && pathPoints.length && (key === 'enter' || key === 'backspace' || key === 'escape')) {
    event.preventDefault();
    if (key === 'enter') runPathStroke();
    else if (key === 'backspace') pathPoints.pop();
    else pathPoints.length = 0;
    if (!pathPoints.length) clearOverlay(pathOverlay);
    return;
  }
  if (key === 'b') setTool('paint');
  else if (key === 'l') setTool('path');
  else if (key === 'u') { try { setView2d(!engine.view_2d()); } catch (error) { status(error instanceof Error ? error.message : String(error), true); } }
  else if (key === 'p') setTool('polyfill');
  else if (key === 'c' && !event.ctrlKey && !event.metaKey) setTool('clone');
  else if (key === 't') setQuickMask(engine.quick_mask_mode() === 1 ? 2 : 1);
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
for (const id of ['d-exposure', 'd-normal', 'd-env-rot', 'd-env-blur']) $(id).addEventListener('input', () => { setOutput(id); syncView(); });
$('d-env-bg').addEventListener('change', syncView);
$('d-env').addEventListener('change', () => { try { useEnvironment(sel('d-env')); } catch (error) { status(error instanceof Error ? error.message : String(error), true); } });
const envInput = $<HTMLInputElement>('env-file');
$('d-env-load').addEventListener('click', () => envInput.click());
envInput.addEventListener('change', async () => {
  const file = envInput.files?.[0];
  envInput.value = '';
  if (!file) return;
  if (!engine) { status('The GPU engine is still starting; load the environment again in a moment', true); return; }
  try {
    const size = await loadEnvironment(file.name, await file.arrayBuffer());
    status(`Environment ${file.name} (${size.width} by ${size.height}) lights the model; rotate and blur it in Display`);
  } catch (error) {
    status(`Could not load ${file.name}: ${error instanceof Error ? error.message : String(error)}`, true);
  }
});
$('l-opacity').addEventListener('input', () => { setOutput('l-opacity'); updateActiveLayer({ opacity: num('l-opacity') }); });
$('l-blend').addEventListener('change', () => updateActiveLayer({ blend: $<HTMLSelectElement>('l-blend').value }));

$('tool-paint').addEventListener('click', () => setTool('paint'));
$('tool-erase').addEventListener('click', () => setTool('erase'));
$('tool-polyfill').addEventListener('click', () => setTool('polyfill'));
$('tool-clone').addEventListener('click', () => setTool('clone'));
$('tool-path').addEventListener('click', () => setTool('path'));
$('view-2d').addEventListener('click', () => { if (!engine) return; try { setView2d(!engine.view_2d()); } catch (error) { status(error instanceof Error ? error.message : String(error), true); } });

let pathBusy = false;

function runPathStroke(): void {
  if (pathBusy) return;
  pathBusy = true;
  strokePath().then((landed) => {
    pathPoints.length = 0;
    clearOverlay(pathOverlay);
    status(`Path stroked through ${landed} samples (Ctrl+Z undoes it)`);
  }, (error) => status(error instanceof Error ? error.message : String(error), true)).finally(() => { pathBusy = false; });
}
$('p-stroke').addEventListener('click', runPathStroke);
$('p-clear').addEventListener('click', () => { pathPoints.length = 0; clearOverlay(pathOverlay); status('Path cleared'); });

function drawPathOverlay(): void {
  if (!engine || !pathPoints.length) return;
  const e = engine;
  const ratio = pathOverlay.getBoundingClientRect().width ? pathOverlay.getBoundingClientRect().width / canvas.getBoundingClientRect().width : 1;
  const toOverlay = (p: Vec3): [number, number] | undefined => {
    const s = e.screen_point(p[0], p[1], p[2]);
    return s.length === 2 ? [s[0] * ratio, s[1] * ratio] : undefined;
  };
  const closed = checked('p-closed') && pathPoints.length > 2;
  drawPath(pathOverlay, pathSamples(pathPoints, closed, checked('p-smooth'), 16).map(toOverlay), pathPoints.map(toOverlay));
}

function useImage(target: 'alpha' | 'stencil' | 'fill' | 'mask', w: number, h: number, data: Uint8Array, label: string): void {
  const e = requireEngine();
  if (target === 'mask') {
    e.mask_from_image(e.active_layer(), w, h, data);
    e.set_paint_target(true);
    refreshLayers();
    status(`Mask loaded from ${label}: white shows the layer, mapped over the UV layout`);
    return;
  }
  if (target === 'fill') {
    const id = e.active_layer();
    if (layers().find((l) => l.id === id)?.kind !== 'fill') throw new Error('Select a fill layer first');
    e.set_fill_image(id, w, h, data);
    const option = $<HTMLSelectElement>('f-pattern').querySelector<HTMLOptionElement>('option[value="image"]')!;
    option.disabled = false;
    $<HTMLSelectElement>('f-pattern').value = 'image';
    syncFill();
    status(`Fill image ${label} loaded`);
    return;
  }
  if (target === 'alpha') {
    e.set_alpha(w, h, data);
    const option = $<HTMLSelectElement>('b-tip').querySelector<HTMLOptionElement>('option[value="image"]')!;
    option.disabled = false;
    $<HTMLSelectElement>('b-tip').value = 'image';
    status(`Brush alpha ${label} loaded`);
  } else {
    e.set_stencil(w, h, data);
    const on = $<HTMLInputElement>('s-on');
    on.disabled = false;
    on.checked = true;
    status(`Stencil ${label} loaded and enabled`);
  }
  syncBrush();
}

function useText(target: 'alpha' | 'stencil' | 'fill', text = $<HTMLInputElement>('t-text').value, font = sel('t-font'), bold = checked('t-bold')): { width: number; height: number } {
  const image = textImage(text, font, bold, target);
  if (target === 'fill') $<HTMLInputElement>('f-imgcolor').checked = false;
  useImage(target, image.w, image.h, image.data, `text "${text.trim().slice(0, 24)}" in ${fontLabel(font)}`);
  return { width: image.w, height: image.h };
}

for (const target of ['alpha', 'stencil', 'fill'] as const) {
  $(`t-${target}`).addEventListener('click', () => { try { useText(target); } catch (error) { status(error instanceof Error ? error.message : String(error), true); } });
}
const fontInput = $<HTMLInputElement>('font-file');
$('t-font-load').addEventListener('click', () => fontInput.click());
fontInput.addEventListener('change', async () => {
  const file = fontInput.files?.[0];
  fontInput.value = '';
  if (!file) return;
  try {
    const family = await loadFont(file);
    const select = $<HTMLSelectElement>('t-font');
    if (!select.querySelector(`option[value="${CSS.escape(family)}"]`)) select.add(new Option(file.name, family));
    select.value = family;
    status(`Font ${file.name} loaded`);
  } catch (error) {
    status(`Could not load ${file.name}: ${error instanceof Error ? error.message : String(error)}`, true);
  }
});

function downloadBlob(blob: Blob, name: string): void {
  const link = document.createElement('a');
  link.href = URL.createObjectURL(blob);
  link.download = name;
  link.click();
  setTimeout(() => URL.revokeObjectURL(link.href), 4000);
}

async function exportMesh(name = meshName.replace(/\.[^.]+$/, '')): Promise<string> {
  const obj = requireEngine().mesh_obj();
  if (await api.alive()) return api.saveMesh(name, obj);
  downloadBlob(new Blob([obj], { type: 'text/plain' }), `${name}.obj`);
  return 'your downloads';
}

async function exportMask(id: number, name: string): Promise<string> {
  const e = requireEngine();
  const size = e.texture_size();
  const blob = await pngOf(await (e.export_layer_mask(id) as Promise<Uint8Array>), size);
  if (await api.alive()) return api.saveMaps(name, new Map([[`mask_${id}.png`, blob]]));
  downloadBlob(blob, `${name}_mask_${id}.png`);
  return 'your downloads';
}

function flattenStack(name = 'Flattened'): number {
  const id = requireEngine().flatten_stack(name);
  if (!id) throw new Error('Finish the stroke first');
  refreshLayers();
  return id;
}

$('x-mesh').addEventListener('click', () => {
  exportMesh().then((where) => status(`Mesh written to ${where}`), (error) => status(error instanceof Error ? error.message : String(error), true));
});
$('l-mask-export').addEventListener('click', () => {
  if (!engine) return;
  exportMask(engine.active_layer(), meshName.replace(/\.[^.]+$/, '')).then((where) => status(`Mask written to ${where}`), (error) => status(error instanceof Error ? error.message : String(error), true));
});
$('l-flatten').addEventListener('click', () => {
  if (!window.confirm('Flatten replaces every layer of this texture set with one paint layer. Fill, filter and generator settings are lost and this cannot be undone. Flatten?')) return;
  try { flattenStack(); status('Flattened into one paint layer'); } catch (error) { status(error instanceof Error ? error.message : String(error), true); }
});

const QUICK_STATUS = [
  'Quick mask off: painting is not limited',
  'Quick mask edit: strokes and fills mark where paint may land (red is protected); the eraser protects',
  'Quick mask on: paint only lands outside the red areas; press T to edit it',
];

function setQuickMask(mode: number): number {
  const e = requireEngine();
  if (!e.set_quick_mask(mode)) throw new Error('Finish the stroke first');
  $<HTMLSelectElement>('qm-mode').value = String(e.quick_mask_mode());
  status(QUICK_STATUS[e.quick_mask_mode()]);
  return e.quick_mask_mode();
}

$('qm-mode').addEventListener('change', () => { try { setQuickMask(Number(sel('qm-mode'))); } catch (error) { status(error instanceof Error ? error.message : String(error), true); } });
$('qm-clear').addEventListener('click', () => { engine?.quick_mask_fill(false); status('Quick mask cleared: everything is protected until you paint it'); });
$('qm-all').addEventListener('click', () => { engine?.quick_mask_fill(true); status('Quick mask allows painting everywhere'); });
$('qm-invert').addEventListener('click', () => { engine?.quick_mask_invert(); status('Quick mask inverted'); });
$('tool-mirror').addEventListener('click', () => {
  $<HTMLSelectElement>('b-sym').value = mirror ? 'off' : 'mirror';
  syncBrush();
});
for (const id of ['b-angle', 'b-density', 'b-lazy', 'b-jsize', 'b-jangle', 'b-jpos', 'b-jflow', 'b-radial', 's-scale', 's-angle', 's-x', 's-y']) $(id).addEventListener('input', () => { setOutput(id); syncBrush(); });
for (const id of ['b-align', 'b-space', 'b-tip', 'b-sym', 'b-axis', 'b-follow', 's-on']) $(id).addEventListener('change', syncBrush);
const imageInput = $<HTMLInputElement>('image-file');
let imageTarget: 'alpha' | 'stencil' | 'fill' | 'mask' = 'alpha';
$('b-alpha-load').addEventListener('click', () => { imageTarget = 'alpha'; imageInput.click(); });
$('s-load').addEventListener('click', () => { imageTarget = 'stencil'; imageInput.click(); });
imageInput.addEventListener('change', async () => {
  const file = imageInput.files?.[0];
  imageInput.value = '';
  if (!file || !engine) return;
  try {
    const { w, h, data } = await imageRgba(file, imageTarget !== 'fill');
    useImage(imageTarget, w, h, data, file.name);
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
for (const id of ['f-gyaw', 'f-gpitch']) $(id).addEventListener('input', () => { setOutput(id); syncFill(); });
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
let savedTemplates: Template[] = [];
function addTemplateOption(template: Template): void {
  const list = $<HTMLSelectElement>('x-template');
  if ([...list.options].some((o) => o.value === template.name)) return;
  list.add(new Option(TEMPLATES.includes(template) ? template.label : `${template.label} (saved)`, template.name));
}
for (const template of TEMPLATES) addTemplateOption(template);
async function refreshTemplates(select?: string): Promise<void> {
  savedTemplates = await loadTemplates();
  for (const template of savedTemplates) addTemplateOption(template);
  if (select) { $<HTMLSelectElement>('x-template').value = select; describeTemplate(); }
}
const templateEditor = new TemplateEditor($('te-maps'));
$('te-add').addEventListener('click', () => { try { templateEditor.addMap(); } catch (error) { status(error instanceof Error ? error.message : String(error), true); } });
$('te-load').addEventListener('click', () => {
  const template = allTemplates().find((t) => t.name === sel('x-template'));
  if (!template || !template.maps.length) { status('Pick a packed template (not separate PBR maps) to start from', true); return; }
  templateEditor.load(template);
  $<HTMLInputElement>('te-name').value = TEMPLATES.includes(template) ? `${template.name}_custom` : template.name;
});
$('te-save').addEventListener('click', async () => {
  try {
    const name = $<HTMLInputElement>('te-name').value.trim();
    const template = { name, label: name, maps: validateMaps(templateEditor.maps()) };
    if (TEMPLATES.some((t) => t.name === name)) throw new Error(`${name} is a built-in template; choose another name`);
    const file = await saveTemplate(template);
    await refreshTemplates(name);
    status(`Template ${name} saved to ${file}`);
  } catch (error) {
    status(error instanceof Error ? error.message : String(error), true);
  }
});
void refreshTemplates();
function syncDepths(): void {
  const format = sel('x-format') as Format;
  const allowed = FORMAT_DEPTHS[format] ?? [8];
  const depth = $<HTMLSelectElement>('x-depth');
  for (const option of depth.options) option.disabled = !allowed.includes(Number(option.value));
  if (!allowed.includes(Number(depth.value))) depth.value = String(allowed[allowed.length > 1 && allowed.includes(16) ? allowed.indexOf(16) : 0]);
  const transparent = $<HTMLSelectElement>('x-pad').querySelector<HTMLOptionElement>('option[value="transparent"]')!;
  transparent.disabled = !FORMAT_ALPHA[format];
  if (transparent.disabled && sel('x-pad') === 'transparent') $<HTMLSelectElement>('x-pad').value = 'infinite';
  $<HTMLInputElement>('x-pad-width').disabled = sel('x-pad') === 'infinite';
}
$('x-format').addEventListener('change', syncDepths);
$('x-pad').addEventListener('change', syncDepths);
$('x-pad-width').addEventListener('input', () => setOutput('x-pad-width'));
syncDepths();
function describeTemplate(): void {
  const template = allTemplates().find((t) => t.name === sel('x-template'));
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

let smartMasks: Record<string, Record<string, unknown>> = { ...SMART_MASKS };

async function refreshMasks(select?: string): Promise<void> {
  smartMasks = { ...SMART_MASKS, ...(await savedMasks()) };
  const list = $<HTMLSelectElement>('sm-mask');
  for (const name of Object.keys(smartMasks)) if (![...list.options].some((o) => o.value === name)) list.add(new Option(SMART_MASKS[name] ? name : `${name} (saved)`, name));
  if (select) list.value = select;
}

function applySmartMask(id: number, name: string): Record<string, unknown> {
  const e = requireEngine();
  const preset = smartMasks[name];
  if (!preset) throw new Error(`No smart mask named ${name}. Smart masks: ${Object.keys(smartMasks).join(', ')}`);
  const raw = e.fill_settings(id);
  if (!raw) {
    maskFromSettings(id, preset);
    return { applied_as: 'layer mask', ...preset };
  }
  e.set_fill(id, JSON.stringify({ ...JSON.parse(raw), ...preset }));
  refreshLayers();
  return { applied_as: 'fill generator', ...(JSON.parse(e.fill_settings(id)) as Record<string, unknown>) };
}

function captureSmartMask(id: number): Record<string, unknown> {
  const e = requireEngine();
  const fill = e.fill_settings(id);
  const sources = [fill, e.mask_generator_settings(id)].filter((s) => s).map((s) => generatorPart(JSON.parse(s) as Record<string, unknown>));
  const found = sources.find((s) => s.generator && s.generator !== 'none');
  if (!found) throw new Error('Give the layer a generator (fill layer) or a generator mask (Mask from) first; that setup is what a smart mask saves');
  return found;
}

async function saveSmartMask(id: number, name: string): Promise<Record<string, unknown>> {
  const settings = captureSmartMask(id);
  if (SMART_MASKS[name]) throw new Error(`${name} is a built-in smart mask; choose another name`);
  const file = await saveMask(name, settings);
  await refreshMasks(name);
  return { file, settings };
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
  try { if (engine) { const r = applySmartMask(engine.active_layer(), sel('sm-mask')); status(`Applied smart mask ${sel('sm-mask')} as ${r.applied_as}`); } } catch (error) { status(error instanceof Error ? error.message : String(error), true); }
});
$('sm-mask-save').addEventListener('click', async () => {
  try {
    if (!engine) return;
    const name = $<HTMLInputElement>('sm-mask-name').value.trim();
    if (!name) throw new Error('Name the smart mask first');
    await saveSmartMask(engine.active_layer(), name);
    status(`Saved smart mask ${name}; it applies to any layer from the list`);
  } catch (error) { status(error instanceof Error ? error.message : String(error), true); }
});
void refreshMasks();

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
  let matchNote = '';
  if (checked('k-match') && mesh) matchNote = `; ${e.set_match_names(mesh.parts, JSON.stringify(mesh.partNames), JSON.stringify(highMesh.partNames))}`;
  else e.clear_match_names();
  const started = performance.now();
  try {
    for (const set of sets) {
      const [positions, normals, uvs, indices, size] = e.texture_set_mesh(set.index) as [Float32Array, Float32Array, Float32Array, Uint32Array, number];
      $('k-high-state').textContent = `Baking ${set.name} in the background (${sets.indexOf(set) + 1} of ${sets.length}); you can keep working`;
      const high = { positions: highMesh.positions.slice(), normals: highMesh.normals.slice(), indices: highMesh.indices.slice(), parts: highMesh.parts.slice() };
      const keys = e.texture_set_match_keys(set.index) as Uint32Array[];
      const result = await bakeInWorker({ low: [positions, normals, uvs, indices], high, size, front: num('k-front'), rear: num('k-rear'), lowKeys: keys[0] ?? new Uint32Array(), highKeys: keys[1] ?? new Uint32Array() });
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
  const text = `${hits} of ${texels} texels hit the high-poly mesh across ${sets.length} texture set${sets.length === 1 ? '' : 's'} in ${((performance.now() - started) / 1000).toFixed(1)} s, baked in a worker${matchNote}`;
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
  if (kind === 'pair') setHigh(sampleHighPoly('pair'), 'two bumpy spheres (left_high, right_high)');
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

async function channelPixels(e: Engine, channel: number, max: number): Promise<Pixels> {
  if (max === 255) return await e.export_channel(channel) as Uint8Array;
  const raw = await e.export_channel_wide(channel) as Uint8Array;
  return max === 65535 ? halfToUnit16(raw) : halfToFloat(raw);
}

interface ExportChoice {
  format: Format;
  depth: number;
  padding: Padding;
}

function exportChoice(format = sel('x-format'), depth = Number(sel('x-depth')), mode = sel('x-pad'), width = num('x-pad-width'), color = hex('x-pad-color')): ExportChoice {
  const checked = checkFormat(format, depth);
  if (!PADDING_MODES.includes(mode as PaddingMode)) throw new Error(`padding is one of ${PADDING_MODES.join(', ')}`);
  if (mode === 'transparent' && !FORMAT_ALPHA[checked]) throw new Error(`${checked.toUpperCase()} has no alpha channel, so transparent padding cannot be written; choose infinite, color or diffusion`);
  const gamma = color.map((v) => Math.pow(v, 1 / 2.2)) as [number, number, number];
  return { format: checked, depth, padding: { mode: mode as PaddingMode, width: Math.max(0, Math.min(256, Math.round(width))), color: gamma } };
}

function allTemplates(): Template[] {
  return [...TEMPLATES, ...savedTemplates.filter((t) => !TEMPLATES.some((b) => b.name === t.name))];
}

async function renderMaps(templateName = sel('x-template'), custom?: MapSpec[], choice: ExportChoice = exportChoice()): Promise<Map<string, Blob>> {
  const { format, depth, padding } = choice;
  const max = sampleMax(format, depth);
  const maps = new Map<string, Blob>();
  if (!engine) return maps;
  const e = engine;
  const template = custom ? { name: 'custom', label: 'Custom', maps: custom } : allTemplates().find((t) => t.name === templateName);
  if (!template) throw new Error(`Unknown export template ${templateName}. Templates: ${allTemplates().map((t) => t.name).join(', ')}`);
  const size = e.texture_size();
  const sets = textureSets();
  const original = e.active_texture_set();
  const write = async (name: string, px: Pixels, alpha: boolean, linearize: boolean, src: Int32Array) => {
    const transparent = applyPadding(px, size, src, padding, max);
    if (linearize && px instanceof Float32Array) gammaToLinear(px);
    maps.set(`${name}.${format}`, await encodeImage(format, px, size, size, alpha || transparent, depth));
  };
  try {
    for (const set of sets) {
      if (!e.select_texture_set(set.index)) throw new Error('Finish the stroke before exporting');
      const prefix = sets.length > 1 ? `${set.material.replace(/[^A-Za-z0-9_-]/g, '_')}_` : '';
      const suffix = set.tiled ? `.${set.udim}` : '';
      const [, , uvs, indices] = e.texture_set_mesh(set.index) as [Float32Array, Float32Array, Float32Array, Uint32Array, number];
      const src = nearestSource(uvMask(uvs, indices, size), size, padding.mode === 'infinite' ? Infinity : padding.width);
      if (template.maps.length) {
        const channels = new Map<number, Pixels>();
        for (const c of sourcesOf(template.maps)) {
          if (c >= 5 && !e.is_baked()) continue;
          channels.set(c, await channelPixels(e, c, max));
        }
        for (const map of template.maps) await write(`${prefix}${map.suffix}${suffix}`, pack(map, channels, size, max), !!map.a, map.rgb === 'basecolor', src);
        continue;
      }
      const wanted = [0, 1, 2, 3, 4, ...(e.is_baked() ? [5, 6, 7, 8, 9] : []), ...(e.is_high_baked() ? [10, 11] : [])];
      for (const i of wanted) await write(`${prefix}${CHANNELS[i]}${suffix}`, await channelPixels(e, i, max), false, i === 0, src);
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
      link.download = `${stem}_${channel}`;
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
    quick_mask: ['off', 'edit', 'on'][e.quick_mask_mode()],
    uv_tiles: e.uv_tiles(),
    view: e.view_2d() ? '2d' : '3d',
    path_points: pathPoints.length,
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
  load_mesh: async ({ file }) => {
    const name = String(file);
    const response = await fetch(`/api/meshes/${encodeURIComponent(name)}`);
    if (!response.ok) throw new Error(`${name} is not in the workspace meshes folder`);
    applyMesh(await loadMeshFile(new File([await response.arrayBuffer()], name)), name);
    return snapshotState();
  },
  load_sample: ({ kind }) => { applyMesh(sampleMesh(String(kind)), String(kind)); return snapshotState(); },
  camera: ({ yaw, pitch, zoom }) => { requireEngine().set_orbit((Number(yaw) * Math.PI) / 180, (Number(pitch) * Math.PI) / 180, Number(zoom ?? 1)); return JSON.parse(requireEngine().camera_json()); },
  brush: (args) => {
    if (args.eraser !== undefined) setTool(args.eraser ? 'erase' : 'paint');
    if (args.tool === 'paint' || args.tool === 'erase' || args.tool === 'clone') setTool(args.tool);
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
  smart_list: async () => { await refreshLibrary(); await refreshMasks(); return { materials: library.map((m) => ({ name: m.name, built_in: !!m.builtIn, layers: m.layers.map((l) => l.name) })), masks: Object.keys(smartMasks), saved_masks: Object.keys(smartMasks).filter((n) => !SMART_MASKS[n]) }; },
  save_smart_mask: async ({ id, name }) => saveSmartMask(Number(id), String(name ?? '')),
  environment: async ({ file, studio, rotation_degrees, blur, background, exposure }) => {
    const e = requireEngine();
    if (studio) useEnvironment('');
    else if (file) {
      const name = String(file);
      if (environments.has(name)) useEnvironment(name);
      else {
        const response = await fetch(`/api/environments/${encodeURIComponent(name)}`);
        if (!response.ok) throw new Error(`${name} is not in the workspace environments folder (Radiance .hdr files)`);
        await loadEnvironment(name, await response.arrayBuffer());
      }
    }
    setControl('d-env-rot', rotation_degrees); setControl('d-env-blur', blur); setControl('d-env-bg', background, 'change'); setControl('d-exposure', exposure);
    return JSON.parse(e.environment_json()) as Record<string, unknown>;
  },
  smart_apply: async ({ name }) => { await refreshLibrary(); return { layer_ids: applySmartMaterial(String(name)), baked: requireEngine().is_baked(), layers: layers() }; },
  smart_mask: ({ id, name }) => ({ settings: applySmartMask(Number(id), String(name)) }),
  smart_save: async ({ name }) => { const file = await saveMaterial(captureSmartMaterial(String(name))); await refreshLibrary(); return { file }; },
  bake_high: async (args) => {
    const { file, sample, front, rear } = args;
    if (args.match_by_name !== undefined && args.match_by_name !== null) $<HTMLInputElement>('k-match').checked = Boolean(args.match_by_name);
    if (sample === 'pair') {
      applyMesh(sampleMesh('pair'), 'pair');
      setHigh(sampleHighPoly('pair'), 'two bumpy spheres (left_high, right_high)');
    } else if (sample) {
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
  instance_layer: ({ id, texture_set }) => {
    const sets = textureSets();
    const target = sets.find((s) => s.name === String(texture_set) || s.material === String(texture_set) || s.index === Number(texture_set));
    if (!target) throw new Error(`No texture set ${texture_set}. Sets: ${sets.map((s) => s.name).join(', ')}`);
    return { id: instanceLayer(Number(id), target.index), texture_set: target.name };
  },
  export_mesh: async ({ name }) => ({ folder: await exportMesh(name ? String(name) : undefined), format: 'obj' }),
  flatten: ({ name }) => ({ id: flattenStack(String(name || 'Flattened')), layers: layers() }),
  export_mask: async ({ id, name }) => ({ folder: await exportMask(Number(id), String(name || meshName.replace(/\.[^.]+$/, ''))) }),
  clone_source: ({ x, y }) => {
    const e = requireEngine();
    const [w, h] = e.viewport_size();
    e.set_clone_source(Number(x) * w, Number(y) * h);
    setTool('clone');
    return { tool: 'clone', source_set: true };
  },
  view_2d: ({ on, center_u, center_v, zoom }) => {
    const e = requireEngine();
    if (on !== undefined && on !== null && Boolean(on) !== e.view_2d()) setView2d(Boolean(on));
    if (e.view_2d() && (center_u !== undefined || center_v !== undefined || zoom !== undefined)) {
      e.frame_mesh();
      const [w, h] = e.viewport_size();
      const z = Math.max(0.25, Math.min(32, Number(zoom ?? 1)));
      if (z !== 1) e.zoom((1 - z) * 1000);
      const [u, v] = e.uv_at(w / 2, h / 2);
      const scale = (e.uv_at(w / 2 + 100, h / 2)[0] - u) / 100;
      e.pan(-(Number(center_u ?? 0.5) - u) / scale, -(Number(center_v ?? 0.5) - v) / scale);
    }
    return { view: e.view_2d() ? '2d' : '3d' };
  },
  paint_path: async ({ points, closed, smooth, erase }) => {
    const e = requireEngine();
    if (!Array.isArray(points) || points.length < 2) throw new Error('points is a list of at least two [x, y] pairs in 0 to 1');
    if (e.view_2d()) throw new Error('The path tool works in the 3D view; call formsurface_view_2d with on false first');
    const [w, h] = e.viewport_size();
    pathPoints.length = 0;
    let missed = 0;
    for (const p of points as unknown[]) {
      const [x, y] = Array.isArray(p) ? p.map(Number) : [NaN, NaN];
      if (!addPathPoint(x * w, y * h)) missed++;
    }
    try {
      const samples = await strokePath(Boolean(closed), smooth === undefined ? true : Boolean(smooth), Boolean(erase));
      return { control_points: pathPoints.length, missed_points: missed, samples, layer: e.active_layer() };
    } finally {
      pathPoints.length = 0;
      clearOverlay(pathOverlay);
    }
  },
  text_resource: ({ text, font, bold, target }) => {
    const where = String(target ?? 'alpha');
    if (where !== 'alpha' && where !== 'stencil' && where !== 'fill') throw new Error('target is alpha, stencil or fill');
    const family = String(font ?? 'sans-serif');
    if (!['sans-serif', 'serif', 'monospace'].includes(family) && !$<HTMLSelectElement>('t-font').querySelector(`option[value="${CSS.escape(family)}"]`)) throw new Error('font is sans-serif, serif, monospace or a font the person loaded in the Text section');
    const size = useText(where, String(text ?? ''), family, bold === undefined ? true : Boolean(bold));
    return { target: where, image: size, brush: snapshotState().brush };
  },
  uv_tiles: ({ on }) => {
    if (on !== undefined && on !== null) setUvTiles(Boolean(on));
    return { uv_tiles: requireEngine().uv_tiles(), texture_sets: textureSets() };
  },
  mask_source: ({ id, source, color, x, y, tolerance, level, softness, invert }) => {
    const e = requireEngine();
    const layerId = Number(id);
    if (MASK_GENERATORS.includes(String(source))) {
      maskFromGenerator(layerId, String(source), Number(level ?? 0.5), Number(softness ?? 0.1), Boolean(invert));
      return { layers: layers(), baked: e.is_baked() };
    }
    if (source === 'id_color') {
      let pick = color ? String(color) : '';
      if (!pick) {
        const [w, h] = e.viewport_size();
        pick = e.id_color_at(Number(x) * w, Number(y) * h);
      }
      maskFromId(layerId, pick, Number(tolerance ?? 0.06));
      return { color: pick, layers: layers() };
    }
    throw new Error(`source is one of ${[...MASK_GENERATORS, 'id_color'].join(', ')}`);
  },
  quick_mask: ({ mode, action }) => {
    const e = requireEngine();
    if (action === 'clear') e.quick_mask_fill(false);
    else if (action === 'allow_all') e.quick_mask_fill(true);
    else if (action === 'invert') e.quick_mask_invert();
    else if (action !== undefined && action !== null) throw new Error('action is clear, allow_all or invert');
    const modes = ['off', 'edit', 'on'];
    if (mode !== undefined && mode !== null) {
      const code = modes.indexOf(String(mode));
      if (code < 0) throw new Error('mode is off, edit or on');
      setQuickMask(code);
    }
    return { mode: modes[e.quick_mask_mode()] };
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
  export: async ({ name, template, maps: custom, bit_depth, format, padding, padding_width, padding_color }) => {
    const given = (v: unknown) => v !== undefined && v !== null;
    const choice = exportChoice(given(format) ? String(format) : sel('x-format'), given(bit_depth) ? Number(bit_depth) : Number(sel('x-depth')),
      given(padding) ? String(padding) : sel('x-pad'), given(padding_width) ? Number(padding_width) : num('x-pad-width'),
      given(padding_color) ? hexColor(String(padding_color)) : hex('x-pad-color'));
    const maps = await renderMaps(template ? String(template) : sel('x-template'), custom ? validateMaps(custom) : undefined, choice);
    const folder = await api.saveMaps(String(name || meshName.replace(/\.[^.]+$/, '')), maps);
    return { folder, template: custom ? 'custom' : template ?? sel('x-template'), format: choice.format, bit_depth: choice.depth, padding: choice.padding.mode, padding_width: choice.padding.width, maps: [...maps.keys()] };
  },
  export_templates: () => ({ templates: allTemplates().map((t) => ({ name: t.name, label: t.label, maps: t.maps, saved: !TEMPLATES.includes(t) })) }),
  save_export_template: async ({ name, label, maps: list }) => {
    const template = { name: String(name ?? ''), label: String(label || name || ''), maps: validateMaps(list) };
    if (TEMPLATES.some((t) => t.name === template.name)) throw new Error(`${template.name} is a built-in template; choose another name`);
    const file = await saveTemplate(template);
    await refreshTemplates(template.name);
    return { file, templates: allTemplates().map((t) => t.name) };
  },
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
  drawPathOverlay();
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
