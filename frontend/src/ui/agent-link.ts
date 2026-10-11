export type Handler = (args: Record<string, unknown>) => Promise<unknown> | unknown;

export class AgentLink {
  private socket?: WebSocket;
  private wanted = false;
  private retry = 0;

  constructor(private readonly handlers: Record<string, Handler>, private readonly onState: (state: 'off' | 'waiting' | 'linked') => void) {}

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
    socket.onmessage = async (event) => {
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
    };
  }
}
