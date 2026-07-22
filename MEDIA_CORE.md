# Linglux Media Core

Linglux keeps the Vue layer responsible for interaction state and creative intent. Durable media work now lives in the independent Rust crate at `src-tauri/crates/linglux-media-core` and is exposed to the UI through small Tauri commands.

## Runtime boundary

```text
Vue editor
  ├─ native file picker / desktop path drop
  ├─ project edits and visible timeline state
  └─ Tauri commands + task event channels
        └─ Rust adapter (`src-tauri/src/lib.rs`)
              └─ Media Core
                    ├─ ProjectStore
                    ├─ TaskManager + journal
                    ├─ fixed worker queues
                    ├─ managed media import
                    ├─ proxy / thumbnail / waveform cache
                    └─ cancellable FFmpeg process runner
```

Large media bytes never pass through JSON IPC. Desktop imports send paths to Rust. On macOS, same-volume assets enter the project package through APFS clone-on-write; other supported filesystems use a hard link when possible, with a fixed-size streaming copy fallback for removable or incompatible volumes. The web-only preview keeps its object-URL fallback.

## Project and cache layout

The root is the Tauri app-data directory under `media-core/`:

```text
media-core/
  projects/<project-id>.linglux/
    manifest.json
    manifest.json.bak
    media/
    proxies/
  cache/
    thumbnails/
    waveforms/
  tasks/
```

`manifest.json.next` is synchronized before replacing the active manifest. The previous valid manifest is retained as a backup. Each save increments a revision. Reopening the editor from the same workflow node restores that node's saved project.

Derivative cache keys use the imported media fingerprint and derivative parameters. The shared cache is pruned to a 2 GiB budget after derivative jobs.

## Task model

Long-running work has these states:

```text
queued → running → succeeded
                 ↘ failed
                 ↘ cancelling → cancelled
```

Unfinished journal entries are restored as `interrupted` after a process restart. Export uses one worker; import and derivative work share two media workers. Progress, status text, errors, results, and cancellation travel through Tauri channels rather than UI polling.

Frontend-callable commands:

- `start_import_media`
- `start_media_derivatives`
- `start_export`
- `cancel_media_task`
- `get_media_task`
- `list_media_tasks`
- `save_edit_project`
- `load_edit_project`

## Editor performance changes

- Undo and redo store entity-level before/after patches instead of retaining complete project clones. History is capped at 128 entries and an estimated 8 MiB; continuous edits on the same clip are coalesced.
- Timeline clips and ruler marks render only inside the visible horizontal range with one viewport of overscan.
- Export, media copy, probing, thumbnails, proxies, and waveforms run outside the UI thread.
- Desktop assets become selectable as soon as their managed file entries exist. Initial background work is limited to metadata, thumbnails, and standalone-audio waveforms.
- Video proxy and video-waveform generation starts on demand when a managed video is previewed or placed on the timeline, instead of eagerly transcoding every imported video.

## Validation targets

Use the following gates when changing this boundary:

```sh
npm run build
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml -p linglux-media-core
npm run tauri:build
```

Runtime performance should also be checked with multi-gigabyte imports, at least 10,000 timeline clips, cancellation during proxy generation and export, and restart recovery while a task is running.

## Current operational requirements

FFmpeg and FFprobe must be available on the host `PATH`. Linglux reports a task failure when they are missing. Bundled sidecars and platform-specific hardware encoders remain release-engineering work; the command and task boundary is designed so those can be added without moving media bytes back into the frontend.
