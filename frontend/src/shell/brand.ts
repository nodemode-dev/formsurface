export function brandMark(size: number, label = 'FormSurface'): string {
  return `<img class="fs-mark" src="/formsurface-logo.png" width="${size}" height="${size}" alt="${label}" draggable="false">`;
}
