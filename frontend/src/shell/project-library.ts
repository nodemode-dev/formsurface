export interface ProjectSummary { id: string; title: string; mesh: string; sets: number | null; layers: number | null; template: string; bytes: number; modified: string; recent: string; saved: boolean; thumb: boolean; meshStored: boolean }
export interface CreatedProject extends ProjectSummary { sample: string }
export interface TrashedProject { entry: string; id: string; title: string; layers: number | null; deletedAt: number; thumb: boolean }
export interface TemplateSummary { id: string; name: string; category: string; tags: string[]; description: string; sample: string; source: string }
export interface LibraryState { counts: { projects: number; templates: number; trash: number }; settings: { pinned: string[]; recentHidden: string[] } }

async function call<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(path, init);
  if (!response.ok) {
    const body = await response.json().catch(() => ({})) as { detail?: unknown };
    throw new Error(typeof body.detail === 'string' ? body.detail : `FormSurface service: ${response.status}`);
  }
  return response.json() as Promise<T>;
}

const json = (method: string, body: unknown): RequestInit => ({ method, headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) });
const at = (id: string): string => `/api/projects/${encodeURIComponent(id)}`;

export const library = {
  templates: () => call<{ categories: string[]; templates: TemplateSummary[]; default: string }>('/api/project-templates'),
  projects: () => call<{ projects: ProjectSummary[] }>('/api/projects?detail=1'),
  trash: () => call<{ projects: TrashedProject[] }>('/api/projects/trash'),
  state: () => call<LibraryState>('/api/project-library'),
  setSettings: (patch: Partial<LibraryState['settings']>) => call<LibraryState['settings']>('/api/project-library/settings', json('POST', patch)),
  create: (body: { title?: string; template?: string; mesh?: string }) => call<CreatedProject>('/api/projects', json('POST', body)),
  remove: (id: string) => call<{ ok: boolean; entry: string }>(at(id), { method: 'DELETE' }),
  duplicate: (id: string) => call<ProjectSummary>(`${at(id)}/duplicate`, { method: 'POST' }),
  opened: (id: string) => call<ProjectSummary>(`${at(id)}/opened`, { method: 'POST' }),
  putInfo: (id: string, info: { title?: string; mesh?: string; sets?: number; layers?: number; template?: string }) => call<ProjectSummary>(`${at(id)}/info`, json('PUT', info)),
  putThumb: (id: string, image: Blob) => call<{ ok: boolean }>(`${at(id)}/thumb`, { method: 'PUT', body: image }),
  putMesh: (id: string, file: File) => call<{ ok: boolean; bytes: number }>(`${at(id)}/mesh?file=${encodeURIComponent(file.name)}`, { method: 'PUT', body: file }),
  meshUrl: (id: string): string => `${at(id)}/mesh`,
  thumbUrl: (id: string, version: string): string => `${at(id)}/thumb?v=${encodeURIComponent(version)}`,
  restore: (entry: string) => call<ProjectSummary>(`/api/projects/trash/${encodeURIComponent(entry)}/restore`, { method: 'POST' }),
  purge: (entry: string) => call<{ ok: boolean }>(`/api/projects/trash/${encodeURIComponent(entry)}`, { method: 'DELETE' }),
  emptyTrash: () => call<{ ok: boolean; removed: number }>('/api/projects/trash', { method: 'DELETE' }),
};
