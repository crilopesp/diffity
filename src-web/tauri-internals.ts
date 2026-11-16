/**
 * Tauri's IPC for the web build: every `@tauri-apps/api` call goes through `window.__TAURI_INTERNALS__`, so
 * defining it here, before the app loads, runs the unchanged app against `diffity-server`.
 *
 * - Commands → `POST /api/invoke/<command>`; the server runs them.
 * - Events (`listen`) and `Channel` messages ← `/api/events` WebSocket.
 * - What belongs to the browser window (window state, opening links) is answered here.
 */

type Callback = (data: unknown) => void;

const callbacks = new Map<number, Callback>();
/** event name → ids of the callbacks listening to it */
const listeners = new Map<string, Set<number>>();

function transformCallback(callback?: Callback, once = false): number {
  const id = window.crypto.getRandomValues(new Uint32Array(1))[0];
  callbacks.set(id, (data) => {
    if (once) {
      callbacks.delete(id);
    }
    callback?.(data);
  });
  return id;
}

function runCallback(id: number, data: unknown) {
  callbacks.get(id)?.(data);
}

function dispatchEvent(event: string, payload: unknown) {
  for (const id of listeners.get(event) ?? []) {
    runCallback(id, { event, id, payload });
  }
}

interface ListenArgs {
  event: string;
  handler: number;
}

interface UnlistenArgs {
  event: string;
  eventId: number;
}

/** Answers for the browser window itself; `undefined` sends the command to the server. */
function browserCommand(cmd: string, args: Record<string, unknown>): { value: unknown } | undefined {
  if (cmd === 'plugin:event|listen') {
    const { event, handler } = args as unknown as ListenArgs;
    if (!listeners.has(event)) {
      listeners.set(event, new Set());
    }
    listeners.get(event)!.add(handler);
    return { value: handler };
  }
  if (cmd === 'plugin:event|unlisten') {
    const { event, eventId } = args as unknown as UnlistenArgs;
    listeners.get(event)?.delete(eventId);
    return { value: null };
  }
  if (cmd === 'plugin:event|emit' || cmd === 'plugin:event|emit_to') {
    dispatchEvent(args.event as string, args.payload);
    return { value: null };
  }
  if (cmd === 'plugin:opener|open_url') {
    window.open(args.url as string, '_blank', 'noopener');
    return { value: null };
  }
  if (cmd === 'plugin:webview|create_webview_window' || cmd === 'plugin:window|create') {
    // A new app window (e.g. another repository) is a new tab.
    const options = (args.options ?? {}) as { url?: string };
    window.open(new URL(options.url ?? '', window.location.href).href, '_blank');
    return { value: null };
  }
  if (cmd.startsWith('plugin:window|') || cmd.startsWith('plugin:webview|')) {
    return { value: windowAnswer(cmd) };
  }
  return undefined;
}

/** A browser tab is one visible, focused, unmaximized window that the page cannot move or resize. */
function windowAnswer(cmd: string): unknown {
  const name = cmd.slice(cmd.indexOf('|') + 1);
  if (name.startsWith('get_all_')) {
    // Other tabs are not this page's windows.
    return [];
  }
  if (name === 'is_visible' || name === 'is_focused' || name === 'is_enabled') {
    return document.visibilityState === 'visible';
  }
  if (name.startsWith('is_')) {
    return false;
  }
  if (name === 'scale_factor') {
    return window.devicePixelRatio;
  }
  if (name === 'title') {
    return document.title;
  }
  return null;
}

async function invoke(cmd: string, args: Record<string, unknown> = {}): Promise<unknown> {
  const local = browserCommand(cmd, args);
  if (local) {
    return local.value;
  }
  const response = await fetch(`/api/invoke/${encodeURIComponent(cmd)}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    // A `Channel` serializes to `__CHANNEL__:<id>`; its messages come back over the WebSocket.
    body: JSON.stringify(args ?? {}),
  });
  const body: unknown = await response.json().catch(() => null);
  if (!response.ok) {
    // Tauri rejects with the command's error value; the app reads `code` and `message` from it.
    throw body ?? { code: 'internal', message: `${cmd} failed (${response.status})` };
  }
  return body;
}

interface ServerMessage {
  event?: string;
  payload?: unknown;
}

const RECONNECT_MS = 1000;

function connectEvents() {
  const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
  const socket = new WebSocket(`${protocol}//${window.location.host}/api/events`);
  socket.onmessage = (message) => {
    const data = JSON.parse(String(message.data)) as ServerMessage;
    if (data.event) {
      dispatchEvent(data.event, data.payload);
    }
  };
  socket.onclose = () => window.setTimeout(connectEvents, RECONNECT_MS);
}

const label = 'main';

Object.assign(window, {
  isTauri: true,
  __TAURI_INTERNALS__: {
    invoke,
    transformCallback,
    unregisterCallback: (id: number) => callbacks.delete(id),
    runCallback,
    callbacks,
    convertFileSrc: (path: string) => path,
    metadata: { currentWindow: { label }, currentWebview: { windowLabel: label, label } },
  },
  __TAURI_EVENT_PLUGIN_INTERNALS__: {
    unregisterListener: (event: string, id: number) => {
      listeners.get(event)?.delete(id);
      callbacks.delete(id);
    },
  },
});

connectEvents();
