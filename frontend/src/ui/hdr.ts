export interface HdrImage {
  width: number;
  height: number;
  rgb: Float32Array;
}

export function parseHdr(buffer: ArrayBuffer): HdrImage {
  const bytes = new Uint8Array(buffer);
  let pos = 0;
  const line = (): string => {
    let s = '';
    while (pos < bytes.length && bytes[pos] !== 10) s += String.fromCharCode(bytes[pos++]);
    pos++;
    return s;
  };
  if (!line().startsWith('#?')) throw new Error('Not a Radiance HDR file (it should start with #?RADIANCE)');
  let format = '';
  for (;;) {
    if (pos >= bytes.length) throw new Error('The HDR header has no end');
    const l = line();
    if (l === '') break;
    if (l.startsWith('FORMAT=')) format = l.slice(7).trim();
  }
  if (format && format !== '32-bit_rle_rgbe') throw new Error(`Only RGBE HDR files are supported, not ${format}`);
  const res = line().trim().split(/\s+/);
  if (res.length !== 4 || res[0] !== '-Y' || res[2] !== '+X') throw new Error(`Unsupported HDR orientation ${res.join(' ')}; FormSurface reads -Y height +X width`);
  const height = Number(res[1]);
  const width = Number(res[3]);
  if (!(width > 0 && height > 0 && width <= 16384 && height <= 8192)) throw new Error('The HDR image is empty or larger than 16384 by 8192');
  const rgb = new Float32Array(width * height * 3);
  const scan = new Uint8Array(width * 4);
  for (let y = 0; y < height; y++) {
    if (pos + 4 > bytes.length) throw new Error('The HDR file ends early');
    if (width >= 8 && width < 0x8000 && bytes[pos] === 2 && bytes[pos + 1] === 2 && !(bytes[pos + 2] & 0x80)) {
      if (((bytes[pos + 2] << 8) | bytes[pos + 3]) !== width) throw new Error('An HDR scanline has the wrong width');
      pos += 4;
      for (let c = 0; c < 4; c++) {
        let x = 0;
        while (x < width) {
          let count = bytes[pos++];
          if (count > 128) {
            count -= 128;
            if (x + count > width) throw new Error('A run in the HDR file is too long');
            const v = bytes[pos++];
            for (let k = 0; k < count; k++) scan[(x++) * 4 + c] = v;
          } else {
            if (count === 0 || x + count > width) throw new Error('A run in the HDR file is malformed');
            for (let k = 0; k < count; k++) scan[(x++) * 4 + c] = bytes[pos++];
          }
        }
      }
    } else {
      scan.set(bytes.subarray(pos, pos + width * 4));
      pos += width * 4;
    }
    for (let x = 0; x < width; x++) {
      const e = scan[x * 4 + 3];
      const i = (y * width + x) * 3;
      if (!e) { rgb[i] = rgb[i + 1] = rgb[i + 2] = 0; continue; }
      const f = Math.pow(2, e - 136);
      rgb[i] = (scan[x * 4] + 0.5) * f;
      rgb[i + 1] = (scan[x * 4 + 1] + 0.5) * f;
      rgb[i + 2] = (scan[x * 4 + 2] + 0.5) * f;
    }
  }
  return { width, height, rgb };
}
