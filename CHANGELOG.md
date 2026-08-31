# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project uses
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Summary

Linglux is currently in pre-alpha development. No public release has been published yet. The current development snapshot establishes a desktop-first AI video workstation with a node-based workflow canvas, a timeline editor, managed local media, cancellable FFmpeg-backed tasks, an editor Agent, storyboard-to-video encoding, and local speech synthesis.

### Added

- Initialized the Vue 3, TypeScript, Vite, and Tauri desktop application with the node-based workflow workspace and local host bridge. ([PR #1]) Thanks to [@Bald-M].
- Added the professional timeline editor, media bin, preview monitor, clip inspector, undo/redo, managed media import, derivatives, project persistence, and FFmpeg export. ([PR #3]) Thanks to [@Bald-M].
- Added a persistent editor Agent that produces typed edit plans for explicit review and atomic application. ([PR #4]) Thanks to [@Bald-M].
- Added task-backed storyboard grid preview and row-major H.264 frame-sequence encoding, providing the initial implementation for the 24-frame, 24 FPS storyboard workflow. ([PR #4], [Issue #5]) Thanks to [@Bald-M].
- Added pinned local CosyVoice setup and managed speech synthesis on supported macOS Apple Silicon systems, providing the initial implementation for voice-over generation. ([PR #4], [Issue #6]) Thanks to [@Bald-M].
- Added beat-marker alignment, cancellable media tasks, task recovery, provider settings, and supporting regression tests. ([PR #4]) Thanks to [@Bald-M].
- Added ordered image-sequence review with natural sorting, drag reordering, aspect ratio and shot duration controls, continuous dynamic-comic timeline creation, and image asset add/delete actions. ([PR #25], [Issue #16]) Thanks to [@Bald-M].
- Added a shot-first dynamic-comic workspace with card-based editing, accessible drag-and-swap ordering, timeline synchronization, batch duration controls, duplication and deletion feedback, responsive details, and shared undo/redo history. ([Issue #17]) Thanks to [@Bald-M].
- Added reusable character voice profiles, per-shot speech settings and previews, lightweight local Kokoro synthesis, and durable editor autosave for dynamic-comic projects. ([PR #29], [Issue #18]) Thanks to [@Bald-M].

### Changed

- Replaced Nuxt UI with a local shadcn-vue-style component layer backed by Reka UI, preserving the existing desktop workstation layout and component behavior. ([PR #7]) Thanks to [@Bald-M].
- Moved Tailwind CSS integration directly into Vite and added local semantic theme tokens, global `Ui*` component registration, startup error feedback, and migration parity tests. ([PR #7]) Thanks to [@Bald-M].
- Refined workflow canvas spacing, typography, and responsive layout behavior. ([PR #2]) Thanks to [@wrx012].
- Expanded the editor mockup into a desktop media architecture with durable project packages, derivative caches, worker queues, and observable task events. ([PR #3]) Thanks to [@Bald-M].
- Standardized the frontend package workflow on pnpm and updated project documentation for the editor Agent and media-core boundaries. ([PR #4]) Thanks to [@Bald-M].
- Changed desktop provider API keys to session-only Rust host memory and removed operating-system credential-vault access, preventing recurring macOS Keychain authorization prompts during startup and use. Thanks to [@Bald-M].

[Unreleased]: https://github.com/wrx012/linglux/commits/main
[PR #1]: https://github.com/wrx012/linglux/pull/1
[PR #2]: https://github.com/wrx012/linglux/pull/2
[PR #3]: https://github.com/wrx012/linglux/pull/3
[PR #4]: https://github.com/wrx012/linglux/pull/4
[PR #7]: https://github.com/wrx012/linglux/pull/7
[PR #25]: https://github.com/wrx012/linglux/pull/25
[PR #29]: https://github.com/wrx012/linglux/pull/29
[Issue #5]: https://github.com/wrx012/linglux/issues/5
[Issue #6]: https://github.com/wrx012/linglux/issues/6
[Issue #16]: https://github.com/wrx012/linglux/issues/16
[Issue #17]: https://github.com/wrx012/linglux/issues/17
[Issue #18]: https://github.com/wrx012/linglux/issues/18
[@Bald-M]: https://github.com/Bald-M
[@wrx012]: https://github.com/wrx012
