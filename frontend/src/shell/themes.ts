const STORE = 'nodemode_ui_settings';
const TOKENS = ['--bg', '--surface', '--surface-2', '--border', '--surface-3', '--text', '--text-muted', '--text-faint', '--canvas-bg'] as const;
const OLED: Record<string, true> = { amoled: true };

export const THEMES: Array<[string, string, boolean, readonly string[]]> = [
  ['studio', 'Studio Dark', false, ['#111111', '#17151b', '#222127', '#2a2a2a', '#34333a', '#f7f7f7', '#a4a4a4', '#717072', '#1c1b1d']],
  ['nodemode-dark', 'NodeMode Dark', false, ['#0a0a0a', '#121218', '#1a1a24', '#2a2a35', '#3a3a48', '#e8e8ee', '#a8a8b8', '#6a6a7a', '#0a0a0a']],
  ['light', 'Light', true, ['#ecebe6', '#f7f6f1', '#e2e0d8', '#d1cdc1', '#beb9aa', '#26241d', '#4c4a40', '#8b887a', '#e7e5dd']],
  ['github-dark', 'GitHub Dark', false, ['#0d1117', '#161b22', '#21262d', '#30363d', '#3d444d', '#e6edf3', '#9198a1', '#656c76', '#0d1117']],
  ['midnight', 'Midnight Blue', false, ['#0b1020', '#101830', '#182242', '#25325c', '#324274', '#e4e9f7', '#9fabd0', '#63709a', '#0b1020']],
  ['forest', 'Forest', false, ['#0c130e', '#121c15', '#1a281e', '#2a4030', '#375441', '#e4efe7', '#a4bdac', '#6a8674', '#0c130e']],
  ['ember', 'Ember', false, ['#140d0a', '#1d1310', '#2a1c16', '#452e22', '#5a3d2e', '#f2e8e2', '#c7ab9c', '#8f7263', '#140d0a']],
  ['violet', 'Violet Haze', false, ['#100b1c', '#171129', '#221a3c', '#382c5e', '#493b78', '#ece6f9', '#b3a6d8', '#7a6da3', '#100b1c']],
  ['slate', 'Slate', false, ['#101418', '#171d23', '#212a32', '#32404c', '#415260', '#e7edf2', '#a3b2bf', '#6b7a87', '#101418']],
  ['mono', 'Mono Ink', false, ['#0e0e0e', '#161616', '#202020', '#333333', '#454545', '#f0f0f0', '#ababab', '#6e6e6e', '#0e0e0e']],
  ['sand', 'Sand (light)', true, ['#efe8da', '#f8f3e9', '#e6dcc8', '#d4c7ab', '#c2b28e', '#2a2418', '#59503c', '#8d8168', '#eae2d1']],
  ['ocean', 'Deep Ocean', false, ['#06131c', '#0a1c29', '#10293c', '#1d425e', '#285578', '#e2eef7', '#96b6cc', '#5b7d94', '#06131c']],
  ['rose', 'Rose Noir', false, ['#170b10', '#211117', '#301a22', '#4d2b38', '#653a49', '#f5e7ec', '#cfa5b3', '#966b7b', '#170b10']],
  ['terminal', 'Terminal Green', false, ['#050d05', '#0a140a', '#102010', '#1f3d1f', '#2b522b', '#d8f5d8', '#8fce8f', '#568a56', '#050d05']],
  ['contrast', 'High Contrast', false, ['#000000', '#0a0a0a', '#161616', '#555555', '#777777', '#ffffff', '#d8d8d8', '#9a9a9a', '#000000']],
  ['amoled', 'AMOLED Black', false, ['#000000', '#000000', '#141414', '#262626', '#3a3a3a', '#ededed', '#a6a6a6', '#858585', '#000000']],
  ['nineties', '90s', true, ['#c0c0c0', '#d4d0c8', '#b4b0a8', '#808080', '#ffffff', '#000000', '#202020', '#585858', '#008080']],
  ['duck', 'Duck', true, ['#e9e4f7', '#f4f1fc', '#d9d2ef', '#c7bde6', '#b3a6dc', '#2b2540', '#4f4770', '#7a7299', '#e2dcf2']],
  ['pro-dark', 'Pro Dark', false, ['#1f1f1f', '#2b2b2b', '#363636', '#4a4a4a', '#818181', '#ececec', '#bdbdbd', '#a0a0a0', '#181818']],
  ['pro-medium-grey', 'Pro Medium Gray', false, ['#3a3a3a', '#454545', '#505050', '#666666', '#a0a0a0', '#f4f4f4', '#d6d6d6', '#c3c3c3', '#2e2e2e']],
  ['pro-light-grey', 'Pro Light Gray', true, ['#b9b9b9', '#c9c9c9', '#aeaeae', '#8c8c8c', '#5a5a5a', '#121212', '#2a2a2a', '#414141', '#a4a4a4']],
  ['pro-light', 'Pro Light', true, ['#e3e3e3', '#f2f2f2', '#d6d6d6', '#b5b5b5', '#767676', '#1a1a1a', '#383838', '#545454', '#d0d0d0']],
  ['classic', 'Classic', true, ['#d1d4d9', '#e1e4e8', '#c2c6cc', '#9ea4ad', '#666c76', '#14171c', '#2c323a', '#4a515b', '#c0c4ca']],
  ['solarized-dark', 'Solarized Dark', false, ['#002b36', '#073642', '#0d4452', '#1b5563', '#768e97', '#fdf6e3', '#bcc6c6', '#9bb0b3', '#00212b']],
  ['adwaita-light', 'Adwaita Light', true, ['#ebebeb', '#fafafa', '#dedde0', '#c0bfbc', '#77767b', '#241f31', '#3d3846', '#5e5c64', '#f6f5f4']],
  ['adwaita-dark', 'Adwaita Dark', false, ['#242424', '#303030', '#3a3a3a', '#4a4a4a', '#858585', '#ffffff', '#deddda', '#b4b3b0', '#1e1e1e']],
  ['breeze-dark', 'Breeze Dark', false, ['#232629', '#31363b', '#3b4045', '#4d5359', '#828c94', '#eff0f1', '#c5cacd', '#a4adb3', '#1b1e20']],
  ['soft-light', 'Soft Light', true, ['#e8ebef', '#f4f6f8', '#dce0e6', '#c3c9d2', '#777d89', '#2b323c', '#454d59', '#5b6370', '#dfe3e8']],
  ['soft-dark', 'Soft Dark', false, ['#22262b', '#2b3036', '#353b43', '#454c56', '#7c8693', '#e1e6ec', '#b9c1cb', '#9ca6b2', '#1b1e22']],
];

interface Choice { theme?: string; accentColor?: string; intensity?: number; appearance?: number }

function read(): Choice {
  try {
    const raw = JSON.parse(localStorage.getItem(STORE) ?? 'null');
    return raw && typeof raw === 'object' ? (raw as Choice) : {};
  } catch {
    return {};
  }
}

export function currentTheme(): string {
  const id = read().theme;
  return THEMES.some(([key]) => key === id) ? (id as string) : 'studio';
}

export function applyTheme(id: string): void {
  const entry = THEMES.find(([key]) => key === id) ?? THEMES[0];
  const root = document.documentElement;
  const base = entry[0] === 'studio';
  TOKENS.forEach((name, i) => {
    if (base) root.style.removeProperty(name);
    else if (root.style.getPropertyValue(name) !== entry[3][i]) root.style.setProperty(name, entry[3][i]);
  });
  root.style.colorScheme = base ? '' : entry[2] ? 'light' : 'dark';
  root.style.background = base ? '' : entry[3][0];
  root.classList.toggle('theme-light', entry[2]);
  const oled = OLED[entry[0]] === true;
  if (oled !== root.hasAttribute('data-oled')) root.toggleAttribute('data-oled', oled);
}

export function setTheme(id: string): void {
  const choice = { appearance: 3, accentColor: '#f97316', intensity: 1, ...read(), theme: id };
  try {
    localStorage.setItem(STORE, JSON.stringify(choice));
  } catch {
    void 0;
  }
  applyTheme(id);
}
