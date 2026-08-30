# AGENTS.md

Guidance for coding agents working in this repository.

## First Principles

Linglux is a desktop-first AI agent for video creation, editing, enhancement, and automation. Keep every change aligned with professional video tooling, node-based workflow automation, local media durability, and desktop app ergonomics.

Read `DESIGN.md` before feature, architecture, data-model, provider, export, storage, media-core, or Tauri changes. Read `MEDIA_CORE.md` before changing managed media import, project manifests, derivative cache, FFmpeg execution, task queues, cancellation, or export behavior. Treat `DESIGN.md` as product and architecture intent, `MEDIA_CORE.md` as media implementation guidance, `AGENTS.md` as execution policy, and `CHANGELOG.md` as the user-visible change record.

The design docs are the product north star, but source code may be ahead of documentation. Verify current implementation before editing and update docs when the task includes documentation work.

Keep changes narrow. Do not revert unrelated uncommitted work, generated output, or local experiments. Generated or local-only paths such as `node_modules/`, `dist/`, `src-tauri/target/`, `src-tauri/gen/`, OS metadata files, and development export output under `output/` must not be edited by hand or committed unless explicitly requested.

## Current Architecture

Linglux is a Vue 3 + TypeScript + Vite frontend wrapped by Tauri 2. Styling uses Tailwind CSS v4 through its Vite plugin, UI primitives live in the local shadcn-vue style component layer under `src/components/ui`, icons come from `@lucide/vue`, and the package manager is pnpm with `pnpm-lock.yaml`.

Required runtimes are Node.js 20 or later for frontend work and a Rust toolchain for desktop work. The Tauri crate uses Rust edition 2021 and declares minimum Rust `1.77.2`.

The app currently has two primary workspaces:

- Workflow workspace in `src/App.vue`: dark node canvas, node palette, graph state, node drag/pan behavior, camera prompt controls, API key/settings UI, generation simulations, artifacts, and workspace switching.
- Editor workspace under `src/components/editor/`: media bin, preview monitor, timeline, clip inspector, export dialog, local edit-session state, undo/redo, import handling, preview playback, task-backed save/export bridge calls, and web-preview fallbacks.

The desktop media layer lives in Rust under `src-tauri/crates/linglux-media-core`. It owns managed project media, manifest persistence, derivative cache, task journals, import, thumbnail/waveform/proxy generation, storyboard-frame encoding, local TTS, FFmpeg process execution, and queue/cancellation primitives.

The current implementation is still a prototype in the image/video-provider sense. The editor Agent performs real OpenAI-compatible model calls and desktop keys use the OS credential vault, but image/video generation providers, general provider queues, retry policy, cost reporting, full project browser UX, and durable cross-project asset reuse are not complete. Managed media import, project manifest save/load, media derivatives, task events, cancellation hooks, storyboard encoding, local TTS, and FFmpeg export are real implementation paths and should not be described as purely simulated.

## Important Files

- `DESIGN.md`: product goals, architecture boundaries, current implementation snapshot, target data model, roadmap, and design principles.
- `MEDIA_CORE.md`: media-core layout, task model, import/derivative/export responsibilities, storage policy, and validation notes.
- `CHANGELOG.md`: unreleased and released user-visible changes with PR, Issue, and contributor attribution.
- `README.md`: setup, development, build, troubleshooting, and platform requirements.
- `package.json`: pnpm scripts and dependency source of truth.
- `vite.config.ts`: Vue, Tailwind CSS, and dev server configuration.
- `src/main.ts`: Vue app entry and local UI component registration.
- `src/components/ui/index.ts`: local shadcn-vue style primitives, variants, and overlays.
- `src/components/ui/migrationParity.test.ts`: shared `Ui*` interface compatibility tests; keep product-specific behavior tests in their owning module.
- `src/App.vue`: app shell and workflow workspace. It owns workflow nodes/edges, canvas interaction state, API key/settings panel state, camera controls, `activeWorkspace`, `editSession`, and generated/exported artifacts.
- `src/components/editor/LingluxEditor.vue`: editor workspace orchestration. It owns the editable project clone, selection, playback state, history/future stacks, imported object URLs, managed import/derivative tasks, Agent plan application, timeline operations, save/export actions, shortcuts, and child component wiring.
- `src/components/editor/AgentChatPanel.vue`: persistent AI conversation, task cancellation, typed plan preview, explicit apply/reject controls, and desktop-only provider messaging.
- `src/components/editor/StoryboardToVideoDialog.vue`: storyboard-grid detection, crop preview, frame/FPS controls, and cancellable frame-sequence generation UI.
- `src/components/editor/MediaBin.vue`: media import, native dialog bridge, web file fallback, search/filter tabs, multi-select, preset audio/text assets, asset drag start, and audio preview.
- `src/components/editor/PreviewMonitor.vue`: responsive 16:9 preview frame, native video playback clock, timeline audio sync, caption overlay rendering, playhead synchronization, and preview fallback states.
- `src/components/editor/TimelinePanel.vue`: track controls, virtualized ruler/clips, playhead/ruler, clip context menu, drag/drop targets, split/trim/delete actions, zoom, snapping, magnetic main track, waveform and beat-marker display, and visibility/audio toggles.
- `src/components/editor/AudioWaveform.vue`: canvas waveform rendering for audio and video clips with peak overlays and played-region feedback.
- `src/components/editor/InspectorPanel.vue`: selected clip parameter editing, including transform/audio/effect controls and text clip styling.
- `src/components/editor/ExportDialog.vue`: export preset selection, task progress, cancellation, reveal/open-location, and completion UI.
- `src/types/editor.ts`: frontend editor data contracts, including media tasks, managed media fields, text style, beat markers, and export results.
- `src/lib/editorProject.ts`: editor defaults, export presets, project/session helpers, clip factory, duration/timecode utilities, normalization, cloning, and primary-track gap closing.
- `src/lib/editorAgent.ts`: sanitized Agent project snapshots, typed plan validation, dry-run preview, and deterministic edit-plan execution.
- `src/lib/beatMarkerAlignment.ts`: beat-marker alignment helpers covered by Vitest.
- `src/lib/tts.ts`: frontend TTS validation, status labels, and supported voice/emotion options.
- `src/style.css`: Tailwind import, base rules, transitions, canvas/editor utilities, sockets, scrollbars, and shared range styling.
- `src-tauri/src/lib.rs`: Tauri commands, API key settings persistence, media-core setup, project save/load, import/derivative/export task bridges, FFmpeg export orchestration, and reveal-file guard.
- `src-tauri/crates/linglux-media-core/src/project_store.rs`: project directory and manifest persistence.
- `src-tauri/crates/linglux-media-core/src/task.rs`: task snapshots, journals, state transitions, and cancellation.
- `src-tauri/crates/linglux-media-core/src/import.rs`: desktop media import, supported extensions, fingerprinting, clone/hard-link/copy strategy, and progress.
- `src-tauri/crates/linglux-media-core/src/media.rs`: metadata probing, thumbnail/waveform/proxy generation, warnings, and cache pruning.
- `src-tauri/crates/linglux-media-core/src/ffmpeg.rs`: FFmpeg/FFprobe discovery and process execution.
- `src-tauri/crates/linglux-media-core/src/storyboard.rs`: validated storyboard crops and cancellable H.264 frame-sequence encoding.
- `src-tauri/crates/linglux-media-core/src/tts.rs`: pinned local CosyVoice setup, synthesis lifecycle, and managed WAV import.
- `src-tauri/src/agent.rs`: provider settings, credential-vault integration, bounded tool-call loop, conversation sidecar, and Agent task cancellation.
- `src-tauri/tauri.conf.json`: Tauri dev/build URLs, window settings, asset protocol, security, and bundle configuration.
- `src-tauri/capabilities/default.json`: current Tauri permissions.

## Development Commands

Install dependencies:

```sh
pnpm install
```

Run the web UI only:

```sh
pnpm dev
```

`pnpm dev` serves Vite on `127.0.0.1:1420` with `--strictPort`. If that port is already in use, stop the existing process instead of silently switching ports.

Run the desktop app:

```sh
pnpm tauri:dev
```

Tauri starts the Vite dev server from `src-tauri/tauri.conf.json`; do not run a separate Vite server first unless intentionally debugging the frontend alone.

Build the frontend:

```sh
pnpm build
```

Build the desktop app:

```sh
pnpm tauri:build
```

## Validation

Before handing off code changes, run the narrowest reliable validation for the files touched:

- Frontend or shared TypeScript changes: `pnpm build`.
- Frontend logic covered by unit tests: `pnpm test`.
- Shared UI module or theme changes: run both `pnpm test` and `pnpm build`.
- Tauri command, Rust bridge, or host setup changes: `cargo check --manifest-path src-tauri/Cargo.toml`.
- Media-core import, project store, task, cache, derivative, or FFmpeg wrapper changes: `cargo test --manifest-path src-tauri/Cargo.toml -p linglux-media-core`.
- Packaging, window, bundle, permission, asset protocol, or release changes: `pnpm tauri:build`.
- Frontend and Tauri bridge changes: validate both sides with the relevant frontend build and Rust check.
- FFmpeg export or derivative behavior changes: also exercise the path on a machine where `ffmpeg` and `ffprobe` are discoverable.
- Documentation-only changes: no build is required unless executable commands, configuration assumptions, or generated examples changed.

There is currently no lint script in `package.json`. Do not invent one unless you intentionally add and configure the supporting tooling.

## Frontend Conventions

Prefer Vue 3 Composition API with `<script setup lang="ts">`. Keep TypeScript strict-compatible and avoid `any` unless it is a deliberate boundary with a clear reason.

Use `ref`, `reactive`, and `computed` for local state and derived values. Keep state and handler names descriptive and product-oriented.

Use the globally registered local `Ui*` components for common buttons, dialogs, popovers, dropdowns, cards, progress, badges, inputs, selects, switches, tooltips, and modals. Treat this module as the stable application interface: keep Reka UI and other interaction adapters inside `src/components/ui`, preserve shared theme tokens in `src/style.css`, and extend the existing interface before creating a product-local wrapper.

When changing the UI module, preserve model events, named slots, focus behavior, dismissal semantics, overlay stacking, keyboard access, and caller-provided `class`/`ui` overrides. Keep migration parity tests focused on the shared interface rather than unrelated editor or workflow behavior.

Use `@lucide/vue` icons for common UI actions. Preserve accessibility basics: semantic landmarks, useful `aria-label`s, `type="button"` on non-submit buttons, dialog attributes, labels for form controls, and keyboard/pointer cleanup.

Preserve the existing responsive desktop-workstation layout and narrow-screen fallbacks. Fixed-format controls such as canvas nodes, editor panels, timeline tracks, preview frames, toolbar buttons, waveform canvases, and sliders should have stable dimensions or responsive constraints so dynamic text and hover states do not shift the layout.

Do not casually rewrite existing Chinese or English product copy. If copy changes are requested, preserve the professional Linglux tone: concise, cinematic, creator-tool oriented, and suitable for a serious desktop workstation.

## Workflow Workspace Rules

Workflow nodes are data-driven through `NodeType`, `CanvasNode`, `nodeDefinitions`, `nodePaletteSections`, and model option maps in `src/App.vue`. Extend those structures instead of duplicating hardcoded node markup.

The workflow canvas supports node dragging, canvas panning, node creation, node deletion, dynamic wires, context menus, responsive scaling, and panel close behavior. When changing interactions, preserve pointer cleanup, `Escape` behavior, bounds clamping, selected-node behavior, and responsive canvas sizing.

Image generation still calls the Tauri command `create_video_plan` as a host bridge smoke test. Video generation remains simulated in the current workflow UI. Do not treat the current model list, token estimates, or generated workflow artifacts as a backend provider contract.

Camera controls build prompt cues from camera body, lens, focal length, aperture, and lens effect selections. Preserve the `Camera:` prefix replacement behavior so repeated camera applications do not stack duplicate camera lines.

The editor can return completed export results to the workflow as artifacts. Preserve this bridge when changing editor completion or artifact handling.

## Editor Workspace Rules

Editor data types live in `src/types/editor.ts`; editor helpers and presets live in `src/lib/editorProject.ts`. Add shared editor concepts there before wiring them through multiple components.

`LingluxEditor.vue` clones the incoming session project and owns local edit state. Mutating operations should capture history through the existing history helpers before edits and mark the project dirty after edits when the change affects the project.

Keep project normalization in `src/lib/editorProject.ts` and Rust serde defaults aligned. New fields should load safely from older projects and from web-preview fallback data.

Keep media import browser-safe and desktop-safe:

- Desktop imports should pass file paths to Tauri and let media-core manage bytes.
- Web preview imports may use object URLs and browser metadata extraction.
- Detect asset types defensively from MIME/extension.
- Read metadata asynchronously.
- Create thumbnails and waveforms defensively.
- Revoke object URLs through the existing cleanup path.
- Never pass large media bytes through Tauri JSON IPC.

Timeline drag/drop uses custom Linglux data-transfer types and pointer-driven drag state for the media bin. Preserve compatibility checks between asset types and track types when adding tracks or media kinds.

The primary video track supports magnetic behavior through `mainTrackMagnetEnabled` and project normalization. Preserve gap closing and insertion semantics when moving, trimming, splitting, deleting, or inserting primary-track clips.

Timeline rendering uses virtualization and stable sizing for ruler ticks, clips, tracks, waveforms, beat markers, and drag previews. Preserve these performance constraints when adding timeline features.

Preview playback can delegate timing to the native video element when a video clip is active and also manages timeline audio elements. Preserve the preview clock handoff, audio sync, scrub synchronization, throttled playhead events, error fallback, and cleanup on unmount.

Save and export call Tauri commands with web-preview fallbacks. Keep fallback behavior when adding host-backed editor features unless the feature is explicitly desktop-only and the UI communicates that constraint.

Export currently uses `start_export` with task events, not the old mock-only export path. Preserve cancellation, progress, output path, manifest path, warnings, and reveal/open-location behavior.

## Media Core Rules

Read `MEDIA_CORE.md` before changing this area.

The media-core root lives under the Tauri app data directory as `media-core/`. Projects use `<project-id>.linglux/` directories with `manifest.json`, `manifest.json.bak`, `media/`, and `proxies/`. Cache data lives under `cache/thumbnails/` and `cache/waveforms/`. Task journals live under `tasks/`.

Project save writes a temporary next manifest, syncs it, backs up the previous manifest, then replaces the active manifest. Do not replace this with direct overwrite behavior.

Task states include `queued`, `running`, `cancelling`, `cancelled`, `succeeded`, `failed`, and `interrupted`. Preserve journal recovery semantics for unfinished tasks.

Exports run on a single export worker. Import and derivative work share media workers. Keep long-running work observable through task events and cancellable through task handles.

Storyboard encoding uses the media worker pool and accepts only managed source images inside the current project. Preserve crop bounds validation, the 240-frame/120-FPS limits, row-major ordering, temporary-file cleanup, and FFmpeg cancellation behavior.

TTS setup and synthesis share a dedicated single worker. The first implementation is macOS Apple Silicon-only, installs pinned CosyVoice/runtime revisions under `media-core/tts/`, and imports only completed WAV output through the normal managed-media path. Do not enable voice cloning or accept arbitrary runtime/model paths without an explicit security and product review.

Derivative generation should prefer cached artifacts where valid and should report warnings for unavailable FFmpeg, skipped streams, unsupported media, or fallback behavior. Do not make cache misses or derivative failures corrupt the project manifest.

Import should keep the clonefile/hard-link/copy fallback strategy and progress updates. Keep supported extension checks centralized in media-core.

FFmpeg and FFprobe are discovered from PATH and common system locations. If packaging sidecars later, update discovery, docs, and validation together.

## Tauri, Rust, and Security

Register frontend-callable commands in `src-tauri/src/lib.rs` with `#[tauri::command]` and `tauri::generate_handler!`.

Current commands include:

- `create_video_plan`
- `load_agent_provider_settings`
- `save_agent_provider_settings`
- `clear_agent_provider_settings`
- `start_editor_agent_turn`
- `cancel_editor_agent_turn`
- `load_agent_conversation`
- `update_agent_plan_state`
- `clear_agent_conversation`
- `create_edit_session`
- `load_edit_project`
- `save_edit_project`
- `start_import_media`
- `start_storyboard_to_video`
- `start_media_derivatives`
- `start_export`
- `cancel_media_task`
- `get_media_task`
- `list_media_tasks`
- `get_tts_status`
- `start_tts_setup`
- `start_speech_synthesis`
- `reveal_export_file`

When changing editor request/response payloads, keep `src/types/editor.ts`, frontend invoke calls, and Rust serde structs compatible. If a new frontend field is required, either mirror it in Rust or make the bridge tolerant with explicit defaults.

Large media bytes should stay out of Tauri JSON IPC. Send paths or small typed payloads, use media-core for local file ownership, and use Tauri asset conversion for renderable media URLs.

API key settings are sensitive. Keys are stored through the Rust host in the OS credential vault and must never be returned to the WebView after saving. Never log full keys, render full keys outside intentional input fields, or include keys in task logs, project files, Agent conversations, artifacts, export records, manifests, or error messages. Mask keys in summaries.

Desktop provider metadata is persisted under the Tauri app config directory while API keys are stored in the operating-system credential vault and never returned to the WebView. A legacy plaintext key in `api-key-settings.json` is migrated out of that JSON file; if the vault is unavailable, a newly entered key is session-only. Web preview does not call Agent providers or retain a usable provider key.

Avoid expanding `src-tauri/capabilities/default.json` unless a user-visible feature genuinely needs it. Add the narrowest possible permission. File system, shell, network, dialog, asset protocol, and secret-storage capabilities must be justified by product value and scoped tightly.

Keep desktop window and dev URL settings aligned with `src-tauri/tauri.conf.json`, `vite.config.ts`, and `package.json`. The app currently expects `http://127.0.0.1:1420`.

`reveal_export_file` should remain guarded to known export locations. Do not introduce arbitrary path reveal/open behavior.

## Product Direction

Treat nodes as business units, jobs as runtime units, and artifacts as reusable results. Frontend code should express creative intent and immediate UI state; the Tauri host and media-core should own local files, queues, secure settings, export work, and platform capabilities.

Long-running generation, import, derivative, save, and export work should be cancellable where practical, retryable where safe, and observable through progress and logs.

When implementing real providers, add explicit request/response types, error normalization, cost/progress reporting, and cancellation boundaries.

Prefer stable application capabilities exposed through small Rust commands or service adapters instead of scattering desktop-specific assumptions through Vue components.

## Dependency and Config Policy

Use pnpm, not npm or yarn, unless the project intentionally changes package managers. Keep `pnpm-lock.yaml` in sync with `package.json`.

Do not add new dependencies for simple UI, state, timing, formatting, or data transformations that the existing stack can handle. If adding a dependency, explain why it is worth the desktop bundle size and maintenance cost.

The local shadcn-vue component layer is already part of the current UI stack. Do not add parallel component libraries for ordinary controls.

Do not broaden Tailwind, the local UI layer, Vite, TypeScript, Tauri, or Cargo workspace configuration unless the requested change requires it. Keep Vite and Tauri dev URLs aligned.

Do not hand-edit generated Tauri schemas under `src-tauri/gen/`.

## Change Management

Keep changes scoped to the requested feature or fix. Avoid broad refactors while the app is still in prototype shape unless they directly reduce risk or unblock the requested work.

Follow `CONTRIBUTING.md` when preparing Issues and pull requests. Its title, form, validation, screenshot, changelog, sensitive-data, and repository-hygiene requirements are enforced by repository workflows; do not duplicate or weaken those rules here.

If a change affects both frontend and Tauri, update both sides in the same pass and validate the bridge. If source code and docs disagree, prefer the current source for implementation details and update docs when the task includes documentation work.

Keep `CHANGELOG.md` under `Unreleased` while the project is pre-alpha and no release is being cut. Record notable user-visible changes with their verified PR and Issue links, and add `Thanks to @author` only when the GitHub contributor is known. Do not invent a release version, date, tag, PR, Issue, or attribution.

When editing UI, preserve the restrained dark professional workstation language: compact controls, dense panels, canvas/grid surfaces, green/teal and blue accents, and clear hierarchy. Avoid marketing-style landing sections, unrelated palette rewrites, decorative cards, or broad visual redesigns unless explicitly requested.

Before final handoff, summarize what changed, mention validation performed or why it was not needed, and call out any relevant existing dirty worktree files you did not touch.
