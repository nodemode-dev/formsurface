# Security policy

## Supported versions

Security fixes go into the newest release. Please update before reporting, and
check whether the problem is still there.

| Version | Supported |
| --- | --- |
| 0.64.x (latest release) | Yes |
| Anything older | No |

## Reporting a vulnerability

**Do not open a public issue for a security problem.** Use GitHub's private
reporting instead: on the repository's **Security** tab, choose **Report a
vulnerability**. Include what you found, the steps to reproduce it, the version and
platform, and what an attacker could do with it.

You can expect an acknowledgement within a week. We will keep you informed while it
is fixed and credit you in the release notes, unless you would rather not be named.

## What counts

FormSurface runs on your own machine, so the things that matter most are the ways
something outside it could get in:

- **The service** binds to loopback (127.0.0.1, port 8766 from source). Anything
  that makes it reachable from another machine, or lets a web page you visit call
  it: getting past the loopback check, the Host check or the Origin check.
- **Agent link** (off by default) and the MCP server (`./formsurface mcp`), the
  batch runner and the Python API behind it: an agent driving the editor while
  Agent link is off.
- **The agent dock and its programs** (a shell, Claude Code, Codex, Cursor and
  Antigravity): starting a shell or a command there from anywhere but the app's
  own window, or a tab's MCP token working after its tab has closed or reaching a
  shell tab.
- **Scripts**: the Python panel in the editor and `./formsurface script` running
  anything the person did not choose to run, or a workspace script reaching outside
  the workspace folder.
- **Files**: opening or importing a mesh (glTF, GLB, OBJ, FBX, USD, PLY, Collada),
  a project, an image, an environment, an OpenColorIO config or a shared
  intelligent material or mask file in a way that crashes the engine, exhausts
  memory or writes outside the workspace folder; and the project library and its
  Trash moving, restoring or deleting anything outside the projects folder.
- **Editing a layer in an external app**: starting a program other than the one
  the person chose, or reading a file back from outside the workspace.

Bugs that need someone to already control your machine, or to run a modified build
of FormSurface, are ordinary bugs: please open an issue for those.
