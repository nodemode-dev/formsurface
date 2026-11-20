# src-tauri HASHOUT_LOG

Decisions, reasons and anything left out, newest first. Code carries no comments; the context lives here. Started in 0.64.0; earlier history is not backfilled.

## 0.64.0 (2026-11-20): the shell hands the window its key (`src/main.rs`)

- The engine is started with `FORMSURFACE_WINDOW_KEY_STDOUT=1`. The line reader takes the first `formsurface-window-key: ...` line (pattern `[A-Za-z0-9_-]{32,}`) and does not treat it as the address line. The main window is built with an initialization script that sets `window.__FORMSURFACE_WINDOW_KEY` only when the page's origin is the engine's own `http://127.0.0.1:<port>`, the same builder call as FormTools.
- On Windows the shell asks the engine to close with `POST /api/shutdown`, which now needs the key, so `Service` keeps the port and the key together and `ask_to_close` sends the header.
- Not compiled here; read against FormTools `src-tauri/src/main.rs` for every name and type. The orchestrator's shell check runs on the tip.
