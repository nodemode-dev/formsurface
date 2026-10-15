/**
 * A docked HSV picker under the base color: a hue ring around a saturation and value square, drawn on one canvas.
 * Dragging writes the color input and fires input events while moving and one change event on release (so the recent
 * colors strip records commits, not drags). Changes made elsewhere move the markers. The block has a fixed height,
 * so nothing around it shifts when it draws.
 */

const SIZE = 140;
const RING_OUT = 68;
const RING_IN = 55;
const SQUARE = 70;

function hexToRgb(hex: string): [number, number, number] {
  const n = parseInt(hex.slice(1), 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

function rgbToHex(r: number, g: number, b: number): string {
  return `#${[r, g, b].map((v) => Math.round(Math.min(255, Math.max(0, v))).toString(16).padStart(2, '0')).join('')}`;
}

function rgbToHsv(r: number, g: number, b: number): [number, number, number] {
  const [rn, gn, bn] = [r / 255, g / 255, b / 255];
  const max = Math.max(rn, gn, bn), min = Math.min(rn, gn, bn), d = max - min;
  let h = 0;
  if (d > 0) {
    if (max === rn) h = ((gn - bn) / d) % 6;
    else if (max === gn) h = (bn - rn) / d + 2;
    else h = (rn - gn) / d + 4;
    h *= 60;
    if (h < 0) h += 360;
  }
  return [h, max === 0 ? 0 : d / max, max];
}

function hsvToRgb(h: number, s: number, v: number): [number, number, number] {
  const c = v * s, x = c * (1 - Math.abs(((h / 60) % 2) - 1)), m = v - c;
  const [r, g, b] = h < 60 ? [c, x, 0] : h < 120 ? [x, c, 0] : h < 180 ? [0, c, x] : h < 240 ? [0, x, c] : h < 300 ? [x, 0, c] : [c, 0, x];
  return [(r + m) * 255, (g + m) * 255, (b + m) * 255];
}

export function mountColorWheel(input: HTMLInputElement, after: Element): void {
  const box = document.createElement('div');
  box.className = 'fs-wheel';
  const canvas = document.createElement('canvas');
  canvas.tabIndex = 0;
  canvas.setAttribute('role', 'application');
  canvas.setAttribute('aria-label', 'Color wheel: drag the ring for hue and the square for saturation and value; arrow keys adjust');
  canvas.style.width = `${SIZE}px`;
  canvas.style.height = `${SIZE}px`;
  box.appendChild(canvas);
  after.after(box);
  const ctx = canvas.getContext('2d');
  if (!ctx) return;
  let [h, s, v] = rgbToHsv(...hexToRgb(input.value || '#000000'));
  let mode: 'hue' | 'square' | null = null;
  let own = false;
  const c = SIZE / 2;
  const half = SQUARE / 2;

  const draw = () => {
    const dpr = Math.max(1, Math.round(window.devicePixelRatio || 1));
    if (canvas.width !== SIZE * dpr) { canvas.width = SIZE * dpr; canvas.height = SIZE * dpr; }
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, SIZE, SIZE);
    const ring = ctx.createConicGradient(0, c, c);
    for (let k = 0; k <= 6; k++) ring.addColorStop(k / 6, `hsl(${k * 60}, 100%, 50%)`);
    ctx.beginPath();
    ctx.arc(c, c, RING_OUT, 0, Math.PI * 2);
    ctx.arc(c, c, RING_IN, 0, Math.PI * 2, true);
    ctx.fillStyle = ring;
    ctx.fill();
    const [hr, hg, hb] = hsvToRgb(h, 1, 1);
    ctx.fillStyle = rgbToHex(hr, hg, hb);
    ctx.fillRect(c - half, c - half, SQUARE, SQUARE);
    const white = ctx.createLinearGradient(c - half, 0, c + half, 0);
    white.addColorStop(0, 'rgba(255,255,255,1)');
    white.addColorStop(1, 'rgba(255,255,255,0)');
    ctx.fillStyle = white;
    ctx.fillRect(c - half, c - half, SQUARE, SQUARE);
    const black = ctx.createLinearGradient(0, c - half, 0, c + half);
    black.addColorStop(0, 'rgba(0,0,0,0)');
    black.addColorStop(1, 'rgba(0,0,0,1)');
    ctx.fillStyle = black;
    ctx.fillRect(c - half, c - half, SQUARE, SQUARE);
    const marker = (x: number, y: number) => {
      ctx.lineWidth = 2;
      ctx.strokeStyle = '#000';
      ctx.beginPath(); ctx.arc(x, y, 5, 0, Math.PI * 2); ctx.stroke();
      ctx.lineWidth = 1.25;
      ctx.strokeStyle = '#fff';
      ctx.beginPath(); ctx.arc(x, y, 5, 0, Math.PI * 2); ctx.stroke();
    };
    const a = (h * Math.PI) / 180;
    const mid = (RING_OUT + RING_IN) / 2;
    marker(c + Math.cos(a) * mid, c + Math.sin(a) * mid);
    marker(c - half + s * SQUARE, c - half + (1 - v) * SQUARE);
  };

  const emit = (kind: 'input' | 'change') => {
    own = true;
    input.value = rgbToHex(...hsvToRgb(h, s, v));
    input.dispatchEvent(new Event(kind, { bubbles: true }));
    own = false;
  };

  const at = (event: PointerEvent) => {
    const r = canvas.getBoundingClientRect();
    const x = ((event.clientX - r.left) / r.width) * SIZE - c;
    const y = ((event.clientY - r.top) / r.height) * SIZE - c;
    if (mode === 'hue') {
      h = ((Math.atan2(y, x) * 180) / Math.PI + 360) % 360;
    } else if (mode === 'square') {
      s = Math.min(1, Math.max(0, (x + half) / SQUARE));
      v = Math.min(1, Math.max(0, 1 - (y + half) / SQUARE));
    }
    draw();
    emit('input');
  };

  canvas.addEventListener('pointerdown', (event) => {
    const r = canvas.getBoundingClientRect();
    const x = ((event.clientX - r.left) / r.width) * SIZE - c;
    const y = ((event.clientY - r.top) / r.height) * SIZE - c;
    const d = Math.hypot(x, y);
    mode = d >= RING_IN - 3 && d <= RING_OUT + 3 ? 'hue' : Math.abs(x) <= half + 3 && Math.abs(y) <= half + 3 ? 'square' : null;
    if (!mode) return;
    canvas.setPointerCapture(event.pointerId);
    event.preventDefault();
    at(event);
  });
  canvas.addEventListener('pointermove', (event) => { if (mode) at(event); });
  const release = () => { if (!mode) return; mode = null; emit('change'); };
  canvas.addEventListener('pointerup', release);
  canvas.addEventListener('pointercancel', release);
  canvas.addEventListener('keydown', (event) => {
    const step = event.shiftKey ? 10 : 2;
    if (event.key === 'ArrowLeft') h = (h - step + 360) % 360;
    else if (event.key === 'ArrowRight') h = (h + step) % 360;
    else if (event.key === 'ArrowUp') v = Math.min(1, v + step / 100);
    else if (event.key === 'ArrowDown') v = Math.max(0, v - step / 100);
    else return;
    event.preventDefault();
    event.stopPropagation();
    draw();
    emit('input');
  });
  canvas.addEventListener('keyup', (event) => { if (event.key.startsWith('Arrow')) emit('change'); });

  const follow = () => {
    if (own) return;
    const [nh, ns, nv] = rgbToHsv(...hexToRgb(input.value || '#000000'));
    // keep the hue on greys, where it is undefined, so the ring does not jump back to red
    if (ns > 0 && nv > 0) h = nh;
    s = ns;
    v = nv;
    draw();
  };
  input.addEventListener('input', follow);
  input.addEventListener('change', follow);
  draw();
}
