export type Handler = (args: Record<string, unknown>) => Promise<unknown> | unknown;

export class AgentLink {
  private socket?: WebSocket;
  private wanted = false;
  private retry = 0;
  private queue: Promise<void> = Promise.resolve();

  constructor(readonly handlers: Record<string, Handler>, private readonly onState: (state: 'off' | 'waiting' | 'linked') => void) {}

  get enabled(): boolean {
    return this.wanted;
  }

  toggle(): void {
    this.wanted = !this.wanted;
    if (this.wanted) this.connect();
    else {
      clearTimeout(this.retry);
      this.socket?.close();
      this.socket = undefined;
      this.onState('off');
    }
  }

  private connect(): void {
    if (!this.wanted) return;
    this.onState('waiting');
    const scheme = location.protocol === 'https:' ? 'wss' : 'ws';
    const socket = new WebSocket(`${scheme}://${location.host}/api/live`);
    this.socket = socket;
    socket.onopen = () => this.onState('linked');
    socket.onclose = () => {
      if (this.socket !== socket) return;
      this.socket = undefined;
      if (this.wanted) {
        this.onState('waiting');
        this.retry = window.setTimeout(() => this.connect(), 2000);
      }
    };
    // Commands run one at a time in arrival order. An async command (a long path, a render, an export) yields to the
    // browser while it works; without the queue the next command ran inside those gaps, so an agent that gave up on a
    // slow paint_path could move the camera, undo or switch layers in the middle of an open stroke.
    socket.onmessage = (event) => {
      this.queue = this.queue.then(async () => {
        let id = '';
        try {
          const message = JSON.parse(event.data as string) as { id: string; command: string; args: Record<string, unknown> };
          id = message.id;
          const handler = this.handlers[message.command];
          if (!handler) throw new Error(`Unknown command ${message.command}`);
          const result = await handler(message.args ?? {});
          socket.send(JSON.stringify({ id, ok: true, result: result ?? {} }));
        } catch (error) {
          socket.send(JSON.stringify({ id, ok: false, error: error instanceof Error ? error.message : String(error) }));
        }
      });
    };
  }
}
