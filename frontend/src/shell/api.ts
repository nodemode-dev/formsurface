export class ApiError extends Error {
  constructor(message: string, readonly status: number) { super(message); }
}

export const api = {
  url(path: string): string { return path; },
  async cmd<T = unknown>(id: string, params: Record<string, unknown> = {}): Promise<T> {
    const res = await fetch(`/api/commands/${encodeURIComponent(id)}`, {
      method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(params),
    });
    const body = await res.json().catch(() => ({})) as { result?: T; detail?: string; error?: string };
    if (!res.ok) throw new ApiError(body.detail || body.error || `${id} failed (${res.status})`, res.status);
    return body.result as T;
  },
};

export type Api = typeof api;
