# Linglux Design

Linglux 是一个 desktop-first 的 AI 视频创作与剪辑工作台。它把「节点式 AI 工作流」和「传统时间线剪辑器」放在同一个桌面应用里：前者负责生成、增强和自动化，后者负责把素材落到时间线中做可控编辑、预览、保存与导出。

本文档描述当前源码实际状态和下一阶段设计方向。源码可能比早期产品设想更靠前；修改功能前以源码为准，并在文档中补齐差异。

项目当前处于 **pre-alpha / Unreleased** 阶段。`package.json` 中的版本用于本地构建标识，不代表已经发布；公开版本、日期和迁移说明只在实际发布时写入 `CHANGELOG.md`。

## Product North Star

Linglux 面向希望在本地桌面环境中完成专业视频创作的用户：

- 用节点组织 AI 任务、参数、上下游资产和复用结果。
- 用时间线完成视频、音频、字幕、图片和生成素材的精细编辑。
- 用 Tauri/Rust 承接本地文件、长任务、缓存、导出和桌面能力。
- 让前端保持即时、清晰、创作导向，把重型媒体处理留给宿主端。

设计上的核心约束是：**创作意图在前端表达，媒体事实在本地宿主保存，长任务必须可观察、可取消、可恢复。**

## Current Snapshot

当前应用已经具备两套主要工作区：

- 工作流工作区：位于 `src/App.vue`，包含深色节点画布、节点面板、模型/API Key 设置、镜头语言控制、生成模拟、工件列表和工作区切换。
- 剪辑器工作区：位于 `src/components/editor/`，包含媒体库、预览监视器、时间线、检查器、导出弹窗、撤销/重做、媒体导入、桌面保存和 FFmpeg 导出。

相比早期原型，剪辑器已经接入了一个真实的本地媒体内核：

- Rust crate `src-tauri/crates/linglux-media-core` 负责项目清单、托管媒体、缓存、任务队列、任务日志、导入、派生资源和 FFmpeg 进程执行。
- Tauri 命令通过 Channel 推送长任务事件，前端不再把大媒体字节塞进 JSON IPC。
- 桌面导入会把用户选择的文件复制或克隆到 app data 下的项目目录，并生成 `file://`/asset protocol 可访问地址。
- 视频、图片和音频派生资源包括缩略图、波形峰值和按需生成的视频代理文件。
- 导出已经从纯模拟推进到 FFmpeg 流程，能够输出文件和旁路 manifest，并把完成结果回写到工作流。
- 剪辑器已经接入真实的对话式 Agent：模型通过受控 Tool Calls 读取净化后的工程元数据、返回结构化剪辑计划，用户确认后才由本地时间线事务执行。
- 托管的分镜拼图可在前端识别网格并校对裁切框，再由 Rust/FFmpeg 按行优先顺序生成可取消的 H.264 MP4，结果回到工程素材库。
- macOS Apple Silicon 上可按需安装固定版本的 CosyVoice 运行时和模型，在本地合成中文配音并导入为托管 WAV 素材。

仍属于原型阶段的能力：

- 工作流侧的图像/视频 AI provider 调用仍主要是模拟或 host bridge smoke test；剪辑 Agent 的聊天 provider 已是真实调用。
- 真实项目打开/最近项目列表、素材缺失恢复、跨项目资产复用和完整文件浏览还未产品化。
- 密钥仅保存在 Rust 宿主的当前进程内存中，避免桌面开发构建反复触发系统凭据授权；provider 成本统计、失败重试策略和任务日志 UI 还未完成。
- FFmpeg 依赖当前通过本机 PATH 或常见安装目录发现，尚未作为 bundle sidecar 完整交付。

## Technology Stack

Frontend:

- Vue 3 + TypeScript + Vite。
- Vue Composition API，主要组件使用 `<script setup lang="ts">`。
- 本地 shadcn-vue 风格组件层提供基础 UI 组件，交互 primitives 基于 Reka UI，变体由 CVA 管理。
- `@lucide/vue` 用于常见动作图标。
- Tailwind CSS v4 直接通过 Vite 参与样式构建。
- pnpm 和 `pnpm-lock.yaml` 是当前包管理约定。

Desktop host:

- Tauri 2。
- Rust edition 2021，主 crate 最低 Rust 版本声明为 `1.77.2`。
- `tauri-plugin-dialog` 用于原生打开文件对话框。
- `protocol-asset` 用于安全访问 app data 下的托管媒体。
- `linglux-media-core` workspace crate 承接媒体核心能力。

External runtime:

- Node.js 20+ 用于前端开发和构建。
- Rust toolchain 用于 Tauri 和 media-core。
- FFmpeg/FFprobe 用于派生资源和导出。

## Repository Map

关键文件和目录：

- `README.md`: 项目启动、开发、构建、故障排查和平台要求。
- `CHANGELOG.md`: 尚未发布及已发布的用户可见变更，并关联对应 PR、Issue 和贡献者。
- `DESIGN.md`: 当前产品和架构设计文档。
- `AGENTS.md`: 给代码代理的仓库操作指南。
- `MEDIA_CORE.md`: 媒体内核设计、目录布局、任务模型和验证方法。
- `package.json`: pnpm scripts 与前端/Tauri 依赖。
- `vite.config.ts`: Vite、Vue 与 dev server 配置。
- `src/main.ts`: Vue app 入口，注册本地 shadcn-vue 组件层。
- `src/components/ui/index.ts`: 本地 UI primitives、变体与浮层组件。
- `src/App.vue`: 应用外壳和工作流工作区。
- `src/style.css`: 全局样式、Tailwind import、画布/编辑器工具样式。
- `src/types/editor.ts`: 剪辑器前端数据契约。
- `src/lib/editorProject.ts`: 项目默认值、session helper、clip factory、时间线工具和归一化逻辑。
- `src/components/editor/LingluxEditor.vue`: 剪辑器编排层。
- `src/components/editor/MediaBin.vue`: 媒体库、导入、筛选、多选、拖拽和预设素材。
- `src/components/editor/PreviewMonitor.vue`: 预览监视器、视频时钟、音频同步和字幕叠加。
- `src/components/editor/TimelinePanel.vue`: 时间线、轨道、虚拟化、拖拽、吸附、磁性主轨和波形显示。
- `src/components/editor/AudioWaveform.vue`: Canvas 波形渲染。
- `src/components/editor/InspectorPanel.vue`: 选中 clip 参数编辑。
- `src/components/editor/ExportDialog.vue`: 导出预设、进度、取消和打开位置。
- `src-tauri/src/lib.rs`: Tauri 命令、API key 设置、media-core 桥接、保存、导入、派生资源和导出。
- `src-tauri/tauri.conf.json`: Tauri dev/build URL、窗口、安全和 bundle 配置。
- `src-tauri/capabilities/default.json`: 当前 Tauri 权限。
- `src-tauri/crates/linglux-media-core/`: 媒体核心 Rust crate。
- `output/`: 开发环境导出产物目录，属于本地生成结果而不是源代码。

## High-Level Architecture

```mermaid
flowchart TD
  App["src/App.vue\nWorkflow shell"] --> Workflow["Node workflow\nCanvas, palette, camera, API settings"]
  App --> Editor["LingluxEditor.vue\nTimeline editor orchestration"]
  App --> UI["Local Ui* module\nStable app-facing interface"]

  Editor --> Bin["MediaBin\nImport, search, presets"]
  Editor --> Preview["PreviewMonitor\nVideo, audio, captions"]
  Editor --> Timeline["TimelinePanel\nTracks, clips, waveform"]
  Editor --> Inspector["InspectorPanel\nClip parameters"]
  Editor --> ExportDialog["ExportDialog\nPreset, progress, location"]
  Editor --> UI

  UI --> Reka["Reka UI\nInternal interaction adapter"]
  UI --> Tokens["Tailwind + semantic tokens\nInternal styling implementation"]

  Editor --> Tauri["Tauri commands\ninvoke + Channel<TaskEvent>"]
  Agent["Editor Agent panel\nChat + plan confirmation"] --> Tauri
  Workflow --> Tauri

  Tauri --> Store["ProjectStore\nmanifest.json + backup"]
  Tauri --> Tasks["TaskManager\njournal + cancellation"]
  Tauri --> Import["Import pipeline\nclone, hard link, copy"]
  Tauri --> Derivatives["Media derivatives\nthumbnail, waveform, proxy"]
  Tauri --> Storyboard["Storyboard encoder\ngrid crops to H.264 MP4"]
  Tauri --> TTS["Local TTS\npinned CosyVoice to managed WAV"]
  Tauri --> Export["FFmpeg export\nsegments, captions, audio mix"]
  Tauri --> Provider["Agent provider runtime\nDeepSeek / OpenAI-compatible"]

  Store --> AppData["App data/media-core"]
  Import --> AppData
  Derivatives --> AppData
  Export --> DevOutput["output/ in dev\napp data output in packaged app"]
```

The architectural boundary is intentionally simple:

- Vue owns interactive state, visual editing decisions, optimistic UI, keyboard shortcuts and browser fallback behavior.
- Tauri owns desktop APIs, local paths, app data directories, long-running work and command registration.
- `linglux-media-core` owns durable media facts: project manifests, managed files, cache, task state, import and derivative generation.
- FFmpeg owns real media transforms, but all process execution is wrapped by Rust task handles so progress and cancellation can be surfaced.

## Workflow Workspace

The workflow workspace is the entry surface for generation and automation.

Current responsibilities:

- Render a node canvas with pan, drag, selection, context menu and dynamic wires.
- Maintain workflow nodes and edges in local Vue state.
- Define node categories through `NodeType`, `CanvasNode`, `nodeDefinitions`, `nodePaletteSections` and model option maps.
- Expose generation controls for image/video tasks.
- Build camera prompt cues from camera body, lens, focal length, aperture and lens effect settings.
- Persist API key settings through Tauri commands on desktop and localStorage fallback on web preview.
- Open the editor workspace with an edit session seeded from a source node.
- Receive editor export results and turn them back into workflow artifacts.

Important interaction rules:

- Workflow nodes should remain data-driven; add new node types by extending definitions rather than duplicating markup.
- Canvas interaction changes must preserve pointer cleanup, Escape behavior, bounds clamping, selected-node behavior and responsive sizing.
- Camera prompt application preserves the `Camera:` prefix replacement behavior so repeated application does not stack duplicate camera lines.
- The current model lists and token estimates are UI/product scaffolding, not stable backend contracts.

Target direction:

- Nodes should become business units: provider configuration, prompt assembly, media transforms, validation and artifact reuse.
- Jobs should become runtime units: queued, cancellable, retryable, observable and cost-aware.
- Artifacts should become durable reusable results, not just in-memory demo outputs.

## Editor Workspace

`LingluxEditor.vue` is the editor orchestration layer. It clones the incoming session project and owns:

- Current editable project.
- Selection state for clips and assets.
- Multi-select asset state and media library preview state.
- Playback/playhead/scrubbing state.
- Timeline zoom, snapping and magnetic main track state.
- Shortcut bindings.
- Dirty/save state.
- Import, derivative and export task status.
- Object URL lifecycle for web fallback imports.
- Project history and future stacks for undo/redo.
- Agent conversation UI, editor-version staleness checks and atomic plan application.

Child components remain focused:

- `MediaBin.vue` handles media import UI, tabs/search, preset assets, native dialog bridge, multi-select, marquee selection and media drag start.
- `PreviewMonitor.vue` handles active visual preview, native video clock handoff, audio element sync, subtitle overlays and fallback states.
- `TimelinePanel.vue` handles track rendering, clip rendering, virtualized rulers/clips, drag/drop, trim/split/delete, zoom, snapping, magnet controls and waveform/beat display.
- `InspectorPanel.vue` edits selected clip transform, audio/effect settings and text style.
- `ExportDialog.vue` manages export preset selection, progress, cancellation, reveal and completion.
- `AudioWaveform.vue` renders waveform peaks in a stable canvas surface.
- `AgentChatPanel.vue` renders persistent conversation, cancellable request state, plan previews and explicit apply/reject controls.
- `StoryboardToVideoDialog.vue` detects storyboard grids, previews crop geometry, and submits typed frame-sequence tasks.

### Conversational Editing Agent

The editor Agent is a bounded domain agent rather than GUI automation:

1. Vue creates a sanitized snapshot containing IDs, names, types, durations, track/clip timing, locks, selection and playhead.
2. The Rust host loads the provider key from current-session memory and runs at most four model rounds and twelve read-only tool calls.
3. The model may inspect assets, timeline and selection, then either ask a clarification or call `propose_edit_plan`.
4. Vue dry-runs the typed plan against a project clone. Plans use integer milliseconds and carry the editor version they were generated from.
5. The user confirms the plan. All affected tracks are applied as one existing history transaction, so one undo restores the pre-Agent state.

The first operation set is `addAssetRange`, `keepClipSourceRange`, `removeClipSourceRange`, `splitClipAtTimeline`, `deleteClip` and `moveClip`. Raw media, local paths, URLs, thumbnails, waveforms and fingerprints are never sent to the provider. Ambiguous names or time expressions must produce a clarification instead of a guessed edit.

Provider endpoints must use HTTPS. Agent task results are delivered live to the editor but omitted from task journals, keeping persisted conversation content inside the bounded project sidecar.

### Editor Data Shape

The editor frontend data contract lives in `src/types/editor.ts`.

Key concepts:

- `EditorSession`: session wrapper used when entering the editor.
- `EditorProject`: durable project object with mode, assets, tracks, duration, resolution, main track magnet preference and update time. Dynamic-comic projects additionally carry shot metadata.
- `MediaAsset`: project asset with type, URL, optional managed file path, proxy path, fingerprint, thumbnail, waveform peaks, duration and dimensions.
- `TimelineTrack`: video, overlay, audio or caption track with visibility/media enable flags.
- `TimelineClip`: time-ranged asset or text clip with transform, opacity, volume/mute, effects, text style, beat markers and visibility.
- `TextClipStyle`: font, size, color, line metrics and optional background box settings for caption/text clips.
- `MediaTaskSnapshot` and `MediaTaskEvent`: long-task state shared between Tauri and Vue.

`src/lib/editorProject.ts` provides:

- Export presets.
- Timeline constants and zoom limits.
- Default text style.
- Seeded demo edit sessions.
- Clip factories.
- Project cloning and normalization.
- Duration/timecode formatting.
- Primary-track gap closing when magnet mode is enabled.

Data normalization is important because Rust manifests, previous frontend state and web fallback objects may not always include newer fields. New fields should be introduced with explicit defaults in both TypeScript helpers and Rust serde models.

### Dynamic-comic domain model

Dynamic-comic projects use `EditorProject.mode = "dynamicComic"` and store their creative intent in `EditorProject.dynamicComic.shots`. Normal timeline projects use `mode = "timeline"`; projects saved before the mode field existed normalize to this mode and keep their existing behavior.

A `DynamicComicShot` is the durable story-editing unit. It owns a stable ID, explicit order, intended duration, normalized focal point, optional character and emotion, dialogue, pauses, start-to-end camera motion, transition choice and sound-effect asset references. A shot may link to the existing media and timeline model through `visualAssetId` and `visualClipId`.

Shots do not replace timeline clips or duplicate managed media facts. The shot model records authoring intent for storyboard-oriented workflows, while timeline clips remain the source of render timing and the media core remains the owner of durable local files. Later shot-first tools must synchronize shot edits into the existing timeline rather than introduce a second rendering timeline.

Dynamic-comic defaults and compatibility live behind the `normalizeEditorProject` interface in the frontend and serde defaults in the Tauri adapter. Callers should use normalized projects instead of independently filling missing shot fields. Camera motion is stored now as a start frame, end frame, preset and easing, but preview controls and FFmpeg rendering are separate follow-up capabilities.

### History Model

The editor uses scoped project history entries rather than blind whole-app snapshots for every small interaction.

Design intent:

- Capture state before project mutations.
- Mark the project dirty after meaningful changes.
- Coalesce continuous edits where appropriate.
- Keep memory bounded.
- Preserve undo/redo for timeline, asset, track and project-level operations.

New editing operations should integrate with the existing history helpers instead of introducing isolated rollback state.

### Media Import

There are two import paths.

Desktop path:

1. User chooses files through `@tauri-apps/plugin-dialog` or drops files onto the Tauri webview.
2. Frontend calls `start_import_media` with file paths and receives task events through a `Channel<TaskEvent>`.
3. Rust imports supported media into the project media directory.
4. Frontend creates managed `MediaAsset` records using managed path, fingerprint and `convertFileSrc`.
5. Frontend requests derivatives through `start_media_derivatives`.
6. Assets receive thumbnails, waveform peaks and eventually video proxy paths.

Web preview fallback:

1. User imports browser `File` objects.
2. Frontend creates object URLs.
3. Metadata, thumbnails and waveforms are generated defensively in the browser.
4. Object URLs are revoked through the existing cleanup path.

Supported import classes include common video, image, audio and caption/text extensions. Actual validation happens in Rust for desktop imports and in browser helpers for web preview.

Design rules:

- Do not pass large media bytes over Tauri JSON IPC.
- Preserve web fallback for UI development and browser preview.
- Keep object URL cleanup reliable.
- Derivative generation should tolerate missing FFmpeg and return warnings/errors through task state, not crash the editor.
- Managed paths and proxy paths are desktop facts; UI should use `convertFileSrc` for display.

### Timeline

The timeline is the main precision surface.

Current behaviors:

- Tracks support video, overlay, audio and caption media kinds.
- Empty timelines show a compact primary track by default.
- Displayed tracks are filtered to active/compatible tracks unless content or dragging state requires more context.
- Clips and ruler ticks are virtualized horizontally for performance.
- Timeline has zoom controls, wheel/scroll interaction and stable pixel-per-second constraints.
- Clip placement respects track/media compatibility.
- Primary video track can operate in magnetic mode: clips are kept contiguous by closing gaps.
- First visual asset on an empty timeline anchors to the primary video track.
- Audio clips can display waveforms and generated beat markers.
- Split logic preserves source offsets and splits beat markers for audio.

Design rules:

- Do not break drag cleanup, playhead scrub cleanup or global pointer listeners.
- Keep fixed-format timeline controls dimensionally stable.
- Preserve snapping and magnetic primary-track behavior when adding insert/move operations.
- Keep waveform rendering bounded and performant.
- Track-level visibility and audio enablement should affect preview/export consistently.

### Preview

The preview monitor represents timeline playback, not just a thumbnail.

Current behaviors:

- Uses a 16:9 responsive preview frame.
- Chooses active visual content from overlay/video tracks or from media-library preview.
- Delegates timing to the native video element when a video clip is active.
- Synchronizes playhead updates while avoiding excessive event churn.
- Manages audio elements for active timeline audio clips.
- Corrects audio drift during playback and handles scrubbing.
- Renders caption/text overlays with the clip text style.
- Provides fallback states for missing, unsupported or failed preview sources.

Design rules:

- Preserve native video clock handoff.
- Keep audio and video state cleanup on unmount.
- Avoid UI changes that occlude preview content or make scrub feedback ambiguous.
- When proxy media exists, preview should prefer the proxy where appropriate.

### Inspector

The inspector edits selected clip properties.

Current editable surfaces:

- Position, scale, rotation and opacity.
- Volume and mute for audio/video clips with audio.
- Effect intensity.
- Text content and typography for caption/text clips.
- Text background box size, offset, color and corner radius.

Design rules:

- Inspector edits should use typed patch events and participate in history.
- Text-style defaults must be compatible with both frontend rendering and Rust export serialization.
- Do not add inspector-only fields without adding defaults and serialization behavior.

### Export

Export is now a task-backed desktop flow with web fallback.

Desktop flow:

1. User chooses an export preset in `ExportDialog.vue`.
2. Frontend calls `start_export` with the current project and preset.
3. Rust creates a media task and runs FFmpeg work under task control.
4. Exported media is written to `output/` during development or app data output in packaged builds.
5. A manifest JSON is written beside the output.
6. Frontend receives the `EditorExportResult`, can reveal the file location and can send the artifact back to the workflow.

The FFmpeg export path currently supports:

- Visual segment rendering from video/image clips.
- Caption/text burn-in using FFmpeg drawtext-style filters.
- Audio mixing for audio-capable clips.
- MP4/MOV H.264 + AAC output and WebM VP9 + Opus output.
- Warnings for skipped or missing audio cases.
- Cancellation through task handles.

Design rules:

- Export should reject empty/non-exportable timelines clearly.
- Reveal/open-location must stay scoped to known output locations.
- Export progress, task state, output path, manifest path and warnings must remain visible to the UI.
- Web preview fallback can simulate export, but desktop behavior should prefer the real task-backed path.

## Media Core

The media core is documented in more detail in `MEDIA_CORE.md`. This section summarizes the product architecture contract.

The root directory is under Tauri app data:

```text
media-core/
  projects/
    <project-id>.linglux/
      manifest.json
      manifest.json.bak
      media/
      proxies/
  cache/
    thumbnails/
    waveforms/
  tasks/
```

Core modules:

- `project_store.rs`: project directories, manifest save/load, backup fallback and revision increments.
- `task.rs`: task snapshots, states, journal persistence and cancellation.
- `executor.rs`: separate export and media queues.
- `import.rs`: supported file validation, same-volume clone/hard-link optimization and buffered copy fallback.
- `media.rs`: metadata probing, thumbnail, waveform and proxy generation.
- `ffmpeg.rs`: FFmpeg/FFprobe discovery and process execution with progress parsing.
- `cache.rs`: derivative cache pathing, stats, namespace clearing and pruning.

Task model:

- Normal states: `queued`, `running`, `succeeded`, `failed`.
- Cancellation states: `cancelling`, `cancelled`.
- Recovery state: unfinished journal entries restore as `interrupted`.

Queue model:

- Exports use one worker.
- Import and derivative jobs share media workers.
- Long work should publish events rather than blocking the frontend.

Persistence model:

- Project save writes `manifest.json.next`, syncs it, rotates the previous manifest to `manifest.json.bak`, then replaces active manifest.
- Project load can fall back to backup if the active manifest is unreadable.
- Revisions increment on save.
- Large media files live in `media/`; generated video proxies live in `proxies/`; derivative cache lives under `cache/`.

Cache model:

- Thumbnails and waveform JSON are keyed by content fingerprint.
- Proxy files are project-local.
- Cache pruning currently targets a bounded size.

## Tauri Boundary

Frontend-callable commands are registered in `src-tauri/src/lib.rs`.

Current command surface:

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
- `reveal_export_file`
- `get_tts_status`
- `start_tts_setup`
- `start_speech_synthesis`

Design rules:

- Command payloads must stay compatible with `src/types/editor.ts`.
- New Rust serde fields should be optional or have defaults unless old projects are migrated.
- New frontend-required fields should be mirrored in Rust deliberately.
- Long commands should use task snapshots/events and cancellation where possible.
- Do not use raw OS paths directly in UI media elements; convert managed paths with Tauri asset helpers.
- Keep web fallback behavior for frontend-only development unless a feature is explicitly desktop-only.

### Local AI Voiceover

The editor audio panel can synthesize Mandarin voiceover locally on macOS Apple Silicon. The first release uses the pinned Apache-2.0 CosyVoice-300M-Instruct model with its built-in Chinese male and female speakers. Runtime and model files are installed on demand under `media-core/tts/`; generated WAV files enter the normal managed-media, waveform, timeline, save, preview, and export paths.

Setup and synthesis use the existing observable task model and a dedicated single-worker speech queue so the model is never loaded concurrently. Vue sends only typed text/voice/emotion/speed intent. Rust owns validated paths, downloads, checksums, the Python worker, cancellation, temporary-file cleanup, and project import. Voice cloning and browser synthesis are intentionally out of scope.

### Storyboard Frame Sequence

The storyboard dialog analyzes an imported image in the WebView, proposes row/column separators and crop rectangles, and lets the user correct margins, gaps, top trim, frame count, and FPS. The Rust boundary accepts only a source already contained in the current project package, validates every crop, limits requests to 240 frames and 120 FPS, and delegates encoding to the cancellable FFmpeg runner.

Frames are read in row-major order and scaled to the smallest even crop size before H.264/YUV420p encoding. The completed MP4 is written directly into managed project media and then follows the normal metadata, derivative, timeline, save, preview, and export paths. This is deterministic frame-sequence assembly, not an AI video provider.

## Security And Permissions

Current Tauri security posture:

- `src-tauri/capabilities/default.json` grants `core:default` and `dialog:allow-open`.
- Asset protocol is enabled only for `$APPDATA/**`.
- Provider metadata is persisted under the Tauri app config directory while API keys remain only in Rust host memory for the current process; the WebView receives only masked key state.
- A legacy plaintext `api-key-settings.json` key is migrated into current-session memory and removed from the JSON payload.
- Web preview does not call Agent providers and does not persist a usable API key.
- Export reveal is guarded so arbitrary paths are not opened.

Security rules:

- Never log full API keys.
- Never include API keys in project manifests, task logs, artifacts, export manifests or error strings.
- Mask keys in UI summaries.
- Do not broaden Tauri permissions unless a user-visible feature requires it.
- Scope file system, shell, network and secret-storage access narrowly.
- Keep provider secrets in Rust host memory for the current session; JSON sidecars, localStorage, task journals, logs, and project/export data may contain metadata or masked state only.

## UI Module And Interaction Direction

Linglux should feel like a dense professional desktop workstation:

- Dark restrained UI.
- Compact controls.
- Clear hierarchy.
- Canvas/grid surfaces.
- Teal/green and blue accents.
- Stable panel dimensions.
- No marketing-style landing sections inside the app surface.

`src/components/ui/index.ts` is the application-facing UI module. Its globally registered `Ui*` components are the stable interface used by workflow and editor callers; Reka UI, CVA, class merging, semantic tokens and overlay mechanics are implementation details behind that seam. This keeps interaction fixes local and prevents third-party component contracts from spreading through product modules.

UI module rules:

- Use Vue Composition API and TypeScript types.
- Use the local `Ui*` interface for dialogs, popovers, buttons and form controls.
- Keep direct Reka UI imports inside the UI module unless a new interaction cannot be expressed by the existing interface.
- Extend an existing `Ui*` interface before adding one-off wrappers in product modules.
- Preserve named slots, model events, focus behavior, dismissal rules and overlay ordering when changing an adapter.
- Cover shared interface behavior in `src/components/ui/migrationParity.test.ts` rather than asserting unrelated product behavior there.
- Use lucide icons for common tool actions.
- Preserve semantic labels, useful `aria-label`s and button `type="button"`.
- Avoid broad palette rewrites unless the product direction explicitly changes.
- Keep text from overflowing buttons, tabs, cards and timeline controls.

## Data And Migration Principles

Project and media data will evolve. Current principles:

- `EditorProject` is the frontend source of editing truth.
- Media-core manifest is the durable desktop representation.
- Rust and TypeScript models currently mirror many fields; keep them synchronized.
- New fields need defaults in `normalizeEditorProject` and Rust serde defaults.
- Old projects should load with sensible fallbacks.
- Manifests should prefer explicit schema versions and revisions.
- Task records should be useful for recovery and debugging without leaking secrets or large payloads.

## Known Gaps

Product gaps:

- Image/video generation provider integration is not implemented end-to-end; the editor Agent provider path is real.
- Node outputs are not yet durable project assets in a fully general way.
- Project browser, recent files and missing media recovery are not complete.
- Export queue UI and historical task log UI are minimal.
- Provider cost/progress accounting is not implemented.
- Credential-vault storage is implemented, but provider-specific recovery, migration diagnostics and lifecycle UX remain incomplete.

Technical gaps:

- FFmpeg is currently discovered from the system rather than guaranteed through bundled sidecars.
- Rust and TypeScript models are duplicated manually.
- Export filter graph coverage is still narrow compared with a professional NLE.
- Timeline performance needs continued validation with large projects.
- Audio waveform and beat detection are lightweight approximations.
- Cache cleanup and task journal retention need product policy.
- Generated development output under `output/` should not be treated as source.

## Roadmap

Near-term architecture work:

1. Harden media-core integration for save/load/import/export edge cases.
2. Add project browser and recent project flow.
3. Improve task log UI with retry, cancellation and recoverable errors.
4. Bundle or configure FFmpeg sidecars for packaged desktop builds.
5. Extend the normalized Agent provider contract to image/video generation jobs.
6. Harden credential-vault migration, recovery, and provider-specific secret lifecycle.
7. Expand export coverage for transforms, opacity, track enablement and text styling.
8. Expand focused tests beyond the current Agent executor, TTS helpers, beat alignment, storyboard encoder, and media-core coverage.

Medium-term product work:

1. Promote generated workflow artifacts into managed project assets.
2. Add reusable node/job/artifact library.
3. Support durable asset relinking and project portability.
4. Add provider cost estimation and job observability.
5. Add richer timeline editing: transitions, keyframes, nested sequences and proxy policy controls.
6. Add collaboration/export manifest formats only after local project semantics are stable.

## Validation

Use the narrowest reliable validation for touched areas:

```sh
pnpm build
```

Frontend and shared TypeScript validation.

```sh
pnpm test
```

Frontend unit tests for Agent plan execution, TTS helpers, beat-marker alignment, and shared UI interface parity.

```sh
cargo check --manifest-path src-tauri/Cargo.toml
```

Tauri command, Rust bridge and host validation.

```sh
cargo test --manifest-path src-tauri/Cargo.toml -p linglux-media-core
```

Media-core unit tests.

```sh
pnpm tauri:build
```

Packaging, window, bundle, permission and full desktop build validation.

FFmpeg-related changes should also be exercised on a machine where `ffmpeg` and `ffprobe` are discoverable.

Documentation-only edits do not require a build unless commands, configuration assumptions or executable examples changed.

## Design Principles

- Keep the editor usable as a desktop workstation first.
- Keep media files local, durable and recoverable.
- Keep long work visible, cancellable and resumable.
- Keep frontend state immediate and expressive.
- Keep Tauri commands small and typed.
- Keep permissions narrow.
- Keep provider integrations explicit rather than hidden in UI components.
- Keep third-party UI adapters behind the local `Ui*` interface.
- Prefer real source state over stale docs, then update docs to match.
