async function post(path: string, body: FormData): Promise<{ folder: string }> {
  const response = await fetch(path, { method: 'POST', body });
  if (!response.ok) throw new Error(`FormSurface service: ${response.status} ${await response.text()}`);
  return response.json();
}

export function safeStem(name: string): string {
  return name.replace(/[^A-Za-z0-9 _.-]/g, '_').slice(0, 40).replace(/^[. ]+|[. ]+$/g, '') || 'untitled';
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

  async saveMesh(name: string, obj: string): Promise<string> {
    const stem = safeStem(name);
    const form = new FormData();
    form.append('mesh', new Blob([obj], { type: 'text/plain' }), `${stem}.obj`);
    return (await post(`/api/exports/${encodeURIComponent(stem)}/mesh`, form)).folder;
  },

  async saveMaps(name: string, maps: Map<string, Blob>): Promise<string> {
    const stem = safeStem(name);
    const form = new FormData();
    for (const [file, blob] of maps) form.append('maps', blob, `${stem}_${file}`);
    return (await post(`/api/exports/${encodeURIComponent(stem)}`, form)).folder;
  },

  async listScripts(): Promise<string[]> {
    const response = await fetch('/api/scripts');
    if (!response.ok) throw new Error(`FormSurface service: ${response.status} ${await response.text()}`);
    return (await response.json()).scripts as string[];
  },

  async readScript(name: string): Promise<string> {
    const response = await fetch(`/api/scripts/${encodeURIComponent(name)}`);
    if (!response.ok) throw new Error(`FormSurface service: ${response.status} ${await response.text()}`);
    return response.text();
  },

  async saveProject(name: string, document: unknown, maps: Map<string, Blob>): Promise<string> {
    const form = new FormData();
    form.append('document', JSON.stringify(document));
    for (const [file, blob] of maps) form.append('maps', blob, file);
    return (await post(`/api/projects/${encodeURIComponent(name)}`, form)).folder;
  },
};
