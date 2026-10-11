---
name: release
description: FormSurface versions, commits, deliverable zips, the planning sidecar, parity grading and the skill packs. Load at the end of every turn that changes the repo, and for any parity question.
---

# Release

## Every turn

1. Commit each coherent change as it lands, authored `nodemode-ai <mackin@nodemode.ai>` through `git -c user.name=... -c user.email=...`, message `type(scope): summary (x.y.z)` with a body saying what changed, why and what ran.
2. Last commit bumps the version in `VERSION`, `frontend/package.json`, `frontend/package-lock.json` (two fields), `frontend/src/version.ts`, `native/Cargo.toml`, `backend/app/main.py`, the launcher usage line, the README badge and the harness metadata.
3. Zip the repo as `formsurface_vX.Y.Z.zip` with `.git`, without `node_modules`, `native/target`, `frontend/src/engine/pkg`, `venv`, `workspace`.
4. Rebuild the sidecar folder `formsurface_planning_docs_sidecar` with DEV_PLAN, BUILD_NOTES (version log with real hashes), HASHOUT_LOG, TEST_PLAN, research, parity and evidence, zipped as `formsurface_planning_docs_sidecar_vX.Y.Z.zip`.
5. Scan markdown for emoji and em dashes, and CSS for raw z-index values.

## Parity

Rows come from Substance 3D Painter's documented features (Experience League), never from acceptance sentences the build wrote for itself. Full means the Painter workflow works at production resolution in FormSurface; partial means a narrower version exists; none means absent. Grade from the running build, not from plans, and attach the evidence. Rival cells cite the rival's own documentation or are marked unknown. The consolidated FormAI matrix carries FormSurface as `PAGES.surface`, `SCORES.surface` and its `META` entry.

The FormSurface page must look and behave like every other page in the tracker. It is generated, never hand-built: `parity/tools/parity_build.py` in the sidecar turns the graded rows into `parity.json`, and `parity/tools/surface_page.py` renders `parity.json` into the FormCut comparison page (`template_formcut_page.html`: timeline lanes, verdict cards, filterable line-by-line table, beyond-baseline chart, improvement lists, method, and the frame bridge script that reports height to the tracker). Embed it with every `</` escaped as `</`, set `SCORES.surface` to the three lanes shown (FormSurface, ArmorPaint, Blender), then open the tracker at `#surface` beside `#cut` and compare before shipping.

## Skill packs

Validate both packs with the god-tier validator before shipping and keep `.agents/skills/formsurface-painting` identical to `.claude/skills/formsurface-painting`.
