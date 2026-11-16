# Home and Projects

Home is the start screen over the project library. It follows the Form family's Home: the same regions and behaviour as formdeck's, under formsurface's own `fs-` class prefix.

| Where | What lives there |
| --- | --- |
| `frontend/src/shell/home.ts` | the Home screen: side panel, Start, Recent, Templates and Trash pages, card menu, confirm bar |
| `frontend/src/shell/project-library.ts` | the editor's client for the library routes |
| `frontend/src/shell/demos.ts` | the demos list, empty in 0.61.0 |
| `frontend/src/shell/toast.ts` | undo toasts |
| `frontend/src/styles/home.css` | Home's styles, on the family tokens in `frontend/src/styles/tokens.css` |
| `backend/app/project_library.py`, `backend/app/projects.py` | the library service and its routes |
| `frontend/src/shell/app-shell.ts`, `frontend/src/shell/native-menu.ts` | the one menu list, and the native macOS menu built from it |

## What the person sees

- Home shows at launch while no project is open, and when the project is closed with File, Close Project. Both are preferences (Show Home Screen at Launch, Show Home Screen When the Last Document Closes), on by default.
- Esc leaves Home only when a project is open. The brand in the title bar, File, Home and Alt+Ctrl+H (Alt+Cmd+H on macOS) bring it back.
- Recent lists saved projects with a preview, newest first. A card's menu has Open, Duplicate, Pin to list, Remove from list and Delete. Delete moves the project to Trash and shows an undo toast.
- Trash lists deleted projects with Restore and Delete permanently, and Empty Trash; the last two ask in a confirm bar first.
- Templates start a new project on a mesh the app makes itself: Sphere, Capsule, Torus, Torus Knot, Sphere with a second UV channel, Figure with Three Texture Sets, Sphere Pair with High-Poly.
- A mesh dropped on Home, or chosen from the hero, opens as a new unsaved project. Saving stores the mesh with the project, so it opens again later.

## Where the data lives

`projects/` in the workspace folder holds one folder per project: the project file, a summary, a preview picture and the mesh. Trash is the `.trash` folder inside `projects/`. Nothing outside `projects/` is moved or deleted by Home.

## The menu bar and quitting

One list in `frontend/src/shell/app-shell.ts` feeds the menu bar in the window, the command palette and, in the desktop app on macOS, the native menu bar. The order is File, Edit, Tools, Layer, Bake, Agent, View, Window, Help. Menus in Window (View menu, macOS desktop only) keeps the bar in the window. In the desktop app Quit, Cmd+Q and the window's close button ask first only when the open document has changes that are not saved; the window title marks such a document with a dot.

## Contract for changes here

- Home is built once and patched in place: set text, classes and hidden state, or patch a list by key. Never rebuild a live container, and reserve the box of anything that loads later (previews, counts).
- New Home, menu and shell styles use the family token names; the app's older token names keep their meaning outside Home.
- A new library route validates the project name, writes atomically, and never follows a path outside `projects/`.
- A Demos section and a Help, Demos submenu appear only when the list in `frontend/src/shell/demos.ts` is not empty.
- A new menu item is a command first; a bare-key or Alt-only shortcut is never registered as a native accelerator.
