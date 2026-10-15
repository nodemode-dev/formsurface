/**
 * Export file names from a template with tags, as Painter's output templates and game engines expect:
 * $mesh, $textureSet, $map, $udim (1001), $u and $v (1-based tile column and row), $size (pixels).
 * An empty template keeps FormSurface's own names. When a tiled set's template has no $udim, $u or $v, the tile is
 * appended as .1001 so tiles never overwrite each other.
 */

export const NAME_TAGS = ['$mesh', '$textureSet', '$map', '$udim', '$u', '$v', '$size'] as const;

export interface NameParts {
  mesh: string;
  textureSet: string;
  map: string;
  udim: number;
  tiled: boolean;
  size: number;
}

const clean = (s: string) => s.replace(/[^A-Za-z0-9_.-]/g, '_');

export function checkNameTemplate(template: string): string {
  const t = template.trim();
  if (!t) return '';
  if (t.length > 120) throw new Error('A file name template is at most 120 characters');
  if (/[\\/:*?"<>|]/.test(t)) throw new Error('A file name template cannot contain \\ / : * ? " < > |');
  if (!t.includes('$map')) throw new Error('A file name template needs $map, or every map would get the same name');
  const unknown = t.match(/\$[A-Za-z]+/g)?.filter((tag) => !(NAME_TAGS as readonly string[]).includes(tag)) ?? [];
  if (unknown.length) throw new Error(`Unknown tag ${unknown[0]}; tags are ${NAME_TAGS.join(', ')}`);
  return t;
}

export function fileName(template: string, p: NameParts): string {
  const tile = p.udim - 1001;
  const values: Record<string, string> = {
    $mesh: clean(p.mesh.replace(/\.[^.]+$/, '')),
    $textureSet: clean(p.textureSet),
    $map: clean(p.map),
    $udim: String(p.udim),
    $u: String((tile % 10) + 1),
    $v: String(Math.floor(tile / 10) + 1),
    $size: String(p.size),
  };
  // longest tags first, so $udim is never read as $u followed by "dim"
  const tags = [...NAME_TAGS].sort((a, b) => b.length - a.length);
  let out = template;
  for (const tag of tags) out = out.split(tag).join(values[tag]);
  if (p.tiled && !/\$udim|\$u|\$v/.test(template)) out += `.${p.udim}`;
  return out;
}
