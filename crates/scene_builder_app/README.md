# scene_builder_app

Native egui + eframe UI for SexLab Scene Builder. This is the primary desktop app.

## Run

From the repository root:

```bash
cargo run -p scene_builder_app
```

Release build:

```bash
cargo build -p scene_builder_app --release
```

## Features

| Area | Status |
|------|--------|
| Menu bar (File / Tools / View / Help) | Implemented |
| Import SLAL pack (folder) | Locates JSON, walks FNIS_*_List.txt + sources*.txt for AnimObjects |
| Package load / save / dirty title | Implemented |
| Background export with progress | Implemented |
| Pack metadata + scene list | Implemented |
| Scene tags + furniture flags | Implemented |
| Stage editor (tags / positions tabs / Extra / validation) | Master parity (SFW/NSFW presets, Basic/Sequence, SLAL flags) |
| Graph canvas (pan / zoom / select / drag) | Left-drag empty pans; drag card moves; zoom-to-cursor |
| Graph edge connect + orth routing | Implemented (L/Z bends, wrap vertices, arrowheads) |
| Graph toolbox (undo/redo, center/fit/arrange, lock, zoom) | Implemented |
| Soft grid | Major lines + dots, low contrast |
| Stage editor | Blocking in-app modal (dims/blocks main UI) |
| Auto-arrange stacked / zeroed imports | Implemented (`graph_layout`) |
| Theme (charcoal / mint / antd accents) | Matched to former React `theme.js` / `App.css` |
| Asset library | Not in core `Package` on this branch |

## Linux notes

Needs a working OpenGL/EGL stack and windowing (X11 or Wayland). File dialogs use GTK via `rfd`.
