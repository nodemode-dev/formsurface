import init, { bake_high_detached } from './pkg/fs_engine.js';

export interface BakeJob {
  low: [Float32Array, Float32Array, Float32Array, Uint32Array];
  high: { positions: Float32Array; normals: Float32Array; indices: Uint32Array; parts: Uint32Array };
  size: number;
  front: number;
  rear: number;
  lowKeys: Uint32Array;
  highKeys: Uint32Array;
  aoRays: number;
  cage: Float32Array;
  offset: Float32Array;
}

export interface BakeResult {
  normal: Uint8Array;
  id: Uint8Array;
  height: Uint8Array;
  opacity: Uint8Array;
  ao: Uint8Array;
  bent: Uint8Array;
  hits: number;
  texels: number;
  size: number;
}

const ready = init();

self.onmessage = async (event: MessageEvent<BakeJob>) => {
  try {
    await ready;
    const { low, high, size, front, rear, lowKeys, highKeys, aoRays, cage, offset } = event.data;
    const [normal, id, hits, texels, height, opacity, ao, bent] = bake_high_detached(low[0], low[1], low[2], low[3], high.positions, high.normals, high.indices, high.parts, size, front, rear, lowKeys, highKeys, aoRays, cage, offset) as [Uint8Array, Uint8Array, number, number, Uint8Array, Uint8Array, Uint8Array, Uint8Array];
    const result: BakeResult = { normal, id, height, opacity, ao, bent, hits, texels, size };
    (self as unknown as Worker).postMessage({ ok: true, ...result }, [normal.buffer, id.buffer, height.buffer, opacity.buffer, ao.buffer, bent.buffer]);
  } catch (error) {
    (self as unknown as Worker).postMessage({ ok: false, error: error instanceof Error ? error.message : String(error) });
  }
};
