async function post(path: string, body: FormData): Promise<{ folder: string }> {
  const response = await fetch(path, { method: 'POST', body });
  if (!response.ok) throw new Error(`FormSurface service: ${response.status} ${await response.text()}`);
  return response.json();
}

export const api = {
  async alive(): Promise<boolean> {
    try {
      const response = await fetch('/api/health', { signal: AbortSignal.timeout(800) });
      return response.ok;
    } catch {
      return false;
    }
  },

  async saveMaps(stem: string, maps: Map<string, Blob>): Promise<string> {
    const form = new FormData();
    for (const [channel, blob] of maps) form.append('maps', blob, `${stem}_${channel}.png`);
    return (await post(`/api/exports/${encodeURIComponent(stem)}`, form)).folder;
  },

  async saveProject(name: string, document: unknown, maps: Map<string, Blob>): Promise<string> {
    const form = new FormData();
    form.append('document', JSON.stringify(document));
    for (const [channel, blob] of maps) form.append('maps', blob, `${channel}.png`);
    return (await post(`/api/projects/${encodeURIComponent(name)}`, form)).folder;
  },
};
