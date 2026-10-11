import init, { bake_high_detached } from './pkg/fs_engine.js';

export interface BakeJob {
  low: [Float32Array, Float32Array, Float32Array, Uint32Array];
  high: { positions: Float32Array; normals: Float32Array; indices: Uint32Array; parts: Uint32Array };
  size: number;
  front: number;
  rear: number;
  lowKeys: Uint32Array;
  highKeys: Uint32Array;
}

const ready = init();

self.onmessage = async (event: MessageEvent<BakeJob>) => {
  try {
    await ready;
    const { low, high, size, front, rear, lowKeys, highKeys } = event.data;
    const [normal, id, hits, texels] = bake_high_detached(low[0], low[1], low[2], low[3], high.positions, high.normals, high.indices, high.parts, size, front, rear, lowKeys, highKeys) as [Uint8Array, Uint8Array, number, number];
    (self as unknown as Worker).postMessage({ ok: true, normal, id, hits, texels, size }, [normal.buffer, id.buffer]);
  } catch (error) {
    (self as unknown as Worker).postMessage({ ok: false, error: error instanceof Error ? error.message : String(error) });
  }
};
