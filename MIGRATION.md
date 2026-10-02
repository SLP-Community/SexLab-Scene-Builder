# Migration: Tauri/React → egui

## Status

Cutover on `feat/egui-rewrite`: the WebView (Tauri + React + Vite) stack has been removed from the workspace. The primary desktop app is `scene_builder_app` (egui + eframe).

| Component | Role |
|-----------|------|
| `crates/scene_builder_app` | Desktop UI |
| `crates/scene_builder_cli` | CLI |
| `crates/scene_builder_core` | Domain library |

## Why

Native GTK menus + WebKitGTK deadlocked under Tauri on Linux (menu callbacks, dialogs, second webview). egui draws UI without a WebView.

## Historical note

Earlier versions used Tauri 2 + React/antd/X6 under `src/` and `src-tauri/`. Those trees are gone on this branch; recover from `master` / `feat/ostim-rework` if you need the old UI for comparison.
