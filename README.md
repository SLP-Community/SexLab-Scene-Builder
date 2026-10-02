# SexLab Scene Builder

Desktop tool to author SexLab animation packs (`.slsb.json`) and export SLSB / SLAL / FNIS layouts used by [SexLab P+](https://github.com/Scrabx3/SexLabpp).

## Run (egui)

```bash
cargo run -p scene_builder_app
```

Release build:

```bash
cargo build -p scene_builder_app --release
```

## CLI

```bash
cargo run -p scene_builder_cli -- --help
```

## Crates

| Crate | Role |
|-------|------|
| `scene_builder_app` | Native egui + eframe desktop UI |
| `scene_builder_cli` | convert / build / export-slal / generate-behaviors |
| `scene_builder_core` | Shared package IR, import/export, behavior gen |

See [`crates/scene_builder_app/README.md`](crates/scene_builder_app/README.md) for UI details.

## Linux dependencies

GUI builds need GTK / X11 (or Wayland) development packages, for example on Debian/Ubuntu:

```bash
sudo apt-get install -y libgtk-3-dev libxcb-render0-dev libxcb-shape0-dev \
  libxcb-xfixes0-dev libxkbcommon-dev libglib2.0-dev pkg-config
```

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) / Cursor + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
