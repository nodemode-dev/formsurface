import serverPy from '../../../mcp/server.py?raw';
import projectPy from '../../../mcp/formsurface_api/project.py?raw';
import textureSetPy from '../../../mcp/formsurface_api/textureset.py?raw';
import layerStackPy from '../../../mcp/formsurface_api/layerstack.py?raw';
import bakingPy from '../../../mcp/formsurface_api/baking.py?raw';
import exportPy from '../../../mcp/formsurface_api/export.py?raw';
import resourcePy from '../../../mcp/formsurface_api/resource.py?raw';
import displayPy from '../../../mcp/formsurface_api/display.py?raw';
import initPy from '../python/api_init.py?raw';
import eventPy from '../python/event.py?raw';
import uiPy from '../python/ui.py?raw';
import httpxPy from '../python/httpx.py?raw';
import pydanticPy from '../python/pydantic.py?raw';
import mcpServerPy from '../python/mcpserver.py?raw';

type Handler = (args: Record<string, unknown>) => unknown;
type PyCallable = ((...args: unknown[]) => unknown) & { callPromising?: (...args: unknown[]) => Promise<unknown>; destroy?: () => void };

interface Pyodide {
  FS: { mkdirTree(path: string): void; writeFile(path: string, data: string): void };
  runPythonAsync(code: string): Promise<unknown>;
  setStdout(options: { batched: (line: string) => void }): void;
  setStderr(options: { batched: (line: string) => void }): void;
}

const FILES: [string, string][] = [
  ['server.py', serverPy], ['httpx.py', httpxPy], ['pydantic.py', pydanticPy],
  ['mcp/__init__.py', ''], ['mcp/server/__init__.py', ''], ['mcp/server/mcpserver/__init__.py', 'from ._impl import Image, MCPServer\n'],
  ['mcp/server/mcpserver/_impl.py', mcpServerPy], ['mcp/server/mcpserver/exceptions.py', 'from ._impl import ToolError\n'],
  ['formsurface_api/__init__.py', initPy], ['formsurface_api/project.py', projectPy], ['formsurface_api/textureset.py', textureSetPy],
  ['formsurface_api/layerstack.py', layerStackPy], ['formsurface_api/baking.py', bakingPy], ['formsurface_api/export.py', exportPy],
  ['formsurface_api/resource.py', resourcePy], ['formsurface_api/display.py', displayPy], ['formsurface_api/event.py', eventPy], ['formsurface_api/ui.py', uiPy],
];

const ROOT = '/home/pyodide/formsurface';
const listeners = new Map<string, Set<PyCallable>>();
let loading: Promise<Pyodide> | undefined;
let log: (line: string, error?: boolean) => void = () => undefined;
let onButtons: (labels: string[]) => void = () => undefined;
const buttons = new Map<string, PyCallable>();

function call(fn: PyCallable, ...args: unknown[]): Promise<unknown> {
  return Promise.resolve(fn.callPromising ? fn.callPromising(...args) : fn(...args));
}

export function emitPython(name: string, details: Record<string, unknown> = {}): void {
  const set = listeners.get(name);
  if (!set?.size) return;
  const payload = JSON.stringify(details);
  for (const fn of set) call(fn, payload).catch((error) => log(`${name} handler: ${error instanceof Error ? error.message : String(error)}`, true));
}

export function pythonButtons(): string[] {
  return [...buttons.keys()];
}

export async function clickPythonButton(label: string): Promise<void> {
  const fn = buttons.get(label);
  if (!fn) throw new Error(`No plugin button ${label}`);
  await call(fn);
}

export function setPythonOutput(write: (line: string, error?: boolean) => void, buttonsChanged: (labels: string[]) => void): void {
  log = write;
  onButtons = buttonsChanged;
}

export function pythonLoaded(): boolean {
  return loading !== undefined;
}

async function boot(handlers: Record<string, Handler>): Promise<Pyodide> {
  const g = globalThis as Record<string, unknown>;
  g.formsurfaceAgent = async (command: string, args: string) => {
    const handler = handlers[command];
    if (!handler) throw new Error(`Unknown command ${command}`);
    return JSON.stringify(await handler(JSON.parse(args) as Record<string, unknown>) ?? null);
  };
  g.formsurfaceOn = (name: string, fn: PyCallable) => { if (!listeners.has(name)) listeners.set(name, new Set()); listeners.get(name)!.add(fn); };
  g.formsurfaceOff = (name: string, fn: PyCallable) => { listeners.get(name)?.delete(fn); };
  g.formsurfaceLog = (line: string) => log(line);
  g.formsurfaceButton = (label: string, fn: PyCallable | null) => {
    if (fn) buttons.set(label, fn); else buttons.delete(label);
    onButtons(pythonButtons());
  };
  const { loadPyodide } = await import('pyodide');
  const indexURL = import.meta.env.DEV ? new URL('/node_modules/pyodide/', location.origin).href : new URL('/pyodide/', location.origin).href;
  const py = await loadPyodide({ indexURL }) as unknown as Pyodide;
  py.setStdout({ batched: (line) => log(line) });
  py.setStderr({ batched: (line) => log(line, true) });
  for (const dir of ['mcp/server/mcpserver', 'formsurface_api']) py.FS.mkdirTree(`${ROOT}/${dir}`);
  for (const [path, text] of FILES) py.FS.writeFile(`${ROOT}/${path}`, text);
  await py.runPythonAsync(`import sys\nsys.path.insert(0, ${JSON.stringify(ROOT)})\nimport formsurface_api`);
  return py;
}

export const PYTHON_UNSUPPORTED = 'Python scripts need JavaScript Promise Integration (WebAssembly stack switching), which this window\'s web engine does not have. The formsurface MCP tools reach the same commands.';

export function pythonSupported(): boolean {
  try { new WebAssembly.Module(new Uint8Array([0, 97, 115, 109, 1, 0, 0, 0])); } catch { return false; }
  return 'Suspending' in WebAssembly || 'Suspender' in WebAssembly;
}

export function loadPython(handlers: Record<string, Handler>): Promise<Pyodide> {
  if (!pythonSupported()) return Promise.reject(new Error(PYTHON_UNSUPPORTED));
  loading ??= boot(handlers).catch((error) => { loading = undefined; throw error; });
  return loading;
}

export async function runPython(handlers: Record<string, Handler>, code: string): Promise<string> {
  const py = await loadPython(handlers);
  const result = await py.runPythonAsync(code);
  return result === undefined || result === null ? '' : String(result);
}
