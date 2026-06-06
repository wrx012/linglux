# AGENTS.md

Guidance for coding agents working in this repository.

## Project Overview

Linglux is a desktop-first AI agent for video creation, editing, enhancement, and automation. The product direction is documented in `DESIGN.md`; read it before making feature or architecture changes.

The current app is a Vue 3 + TypeScript + Vite frontend wrapped by Tauri 2. It presents a dark, node-based AI workflow canvas where users can combine sources, text, image generation, video diffusion, 3D world, audio, storyboard, and automation nodes. Keep changes aligned with professional video tooling, workflow automation, and desktop app ergonomics.

## Current Implementation Notes

- `src/App.vue` is still the main prototype surface. It contains workflow node data, drag/pan logic, context menu handling, inspector state, camera controls, API key panel state, and generation simulations.
- Workflow nodes are data-driven through `NodeType`, `CanvasNode`, `nodeDefinitions`, `nodePaletteSections`, and `modelOptionsByGroup`. Extend those structures instead of duplicating hardcoded node markup.
- The central canvas supports node dragging, canvas panning, node creation, node deletion, dynamic wires, and responsive scaling. Preserve pointer cleanup and keyboard/panel close behavior when editing interactions.
- Image generation calls the Tauri command `create_video_plan` as a host bridge smoke test. Video generation is currently simulated with a timeout.
- API key settings use Tauri commands in desktop mode and fall back to browser `localStorage` for web preview.
- Camera controls build prompt cues from camera body, lens, focal length, aperture, and lens effect selections.

## Tech Stack

- Frontend: Vue 3 single-file components with `<script setup lang="ts">`
- Styling: Tailwind CSS v4 through `@tailwindcss/vite`
- Icons: `@lucide/vue`
- Desktop shell: Tauri 2
- Rust crate: `src-tauri`, Rust edition 2021, minimum Rust `1.77.2`
- Package manager: npm with `package-lock.json`
- Required runtime: Node.js 20 or later; Rust toolchain for desktop work

## Repository Layout

- `README.md`: setup, development, build, and troubleshooting guide
- `DESIGN.md`: product goals, architecture, target data model, module roadmap, and design principles
- `src/`: Vue frontend source
- `src/App.vue`: current main UI and interaction prototype
- `src/style.css`: Tailwind import, base styles, transitions, and shared utilities
- `src/assets/linglux-logo-no-text.png`: current app logo asset
- `src-tauri/`: Tauri application shell and Rust commands
- `src-tauri/src/lib.rs`: command definitions, API key settings persistence, and Tauri setup
- `src-tauri/tauri.conf.json`: Tauri dev/build, window, security, and bundle configuration
- `vite.config.ts`: Vite, Vue, Tailwind, and Tauri-facing build settings

Generated or local-only paths such as `node_modules/`, `dist/`, `src-tauri/target/`, and OS metadata files must not be committed. Do not edit generated Tauri schema files under `src-tauri/gen/` by hand.

## Development Commands

Install dependencies:

```sh
npm install
```

Run the web UI only:

```sh
npm run dev
```

`npm run dev` serves Vite on `127.0.0.1:1420` with a strict port. If the port is already in use, stop the existing process rather than silently switching ports.

Run the desktop app:

```sh
npm run tauri:dev
```

Tauri starts the Vite dev server automatically from `src-tauri/tauri.conf.json`; do not run a separate Vite server before `npm run tauri:dev` unless you are intentionally debugging the frontend alone.

Build the frontend:

```sh
npm run build
```

Build the desktop app:

```sh
npm run tauri:build
```

## Validation

Before handing off code changes, run the narrowest reliable validation for the files touched:

- Frontend or shared TypeScript changes: `npm run build`
- Tauri/Rust changes: `cargo check --manifest-path src-tauri/Cargo.toml`
- Packaging, window, bundle, or permission changes: `npm run tauri:build`
- Documentation-only changes: no build is required unless the docs change executable commands or configuration assumptions

There is currently no dedicated lint or test script in `package.json`. Do not invent one without adding the supporting tooling intentionally.

## Frontend Conventions

- Prefer Vue 3 Composition API with `<script setup lang="ts">`.
- Keep TypeScript strict-compatible; avoid `any` unless there is a clear boundary reason.
- Use `ref`, `reactive`, and `computed` for local state and derived UI values.
- Use `@lucide/vue` icons for common UI actions.
- Keep component state and handler names descriptive and product-oriented.
- Preserve accessibility basics: semantic landmarks, useful `aria-label`s, `type="button"` on non-submit buttons, and labels for form controls.
- Keep the UI responsive across the desktop canvas layout and the narrow-screen fallback already present in `App.vue`.
- If `App.vue` grows further, split along the module names suggested in `DESIGN.md`: `Sidebar`, `WorkflowCanvas`, `WorkflowNode`, `InspectorPanel`, `StatusFooter`, and `GenerationControls`.

## Styling Conventions

- Use Tailwind utility classes for component-local styling.
- Keep shared CSS in `src/style.css` for base rules, transitions, and reusable utilities only.
- The current visual language is a restrained dark professional workstation UI with green/teal accents, compact controls, grid/canvas surfaces, and dense inspector panels.
- Avoid unrelated palette rewrites, marketing-style landing sections, or broad visual redesigns unless explicitly requested.
- Keep fixed-format UI stable with explicit dimensions or responsive constraints so labels, hover states, dynamic text, and generated node content do not shift layout.

## Tauri, Rust, and Security

- Register frontend-callable commands in `src-tauri/src/lib.rs` with `#[tauri::command]` and `tauri::generate_handler!`.
- Current commands are `create_video_plan`, `load_api_key_settings`, `save_api_key_settings`, and `clear_api_key_settings`.
- Keep command payloads explicit and serializable with `serde` when structured data is needed.
- API key settings are sensitive. Never log full keys, render full keys outside intentional input fields, or include them in task logs. Mask keys in UI summaries.
- Desktop API key settings are persisted under the Tauri app config directory as `api-key-settings.json`; web preview falls back to `localStorage`.
- Avoid expanding Tauri permissions in `src-tauri/capabilities/default.json` unless a feature genuinely requires it. Add the narrowest permission possible.
- Keep desktop window configuration in `src-tauri/tauri.conf.json`. The current main window is `1440x780` with a `960x640` minimum size.
- Prefer small Rust commands that expose stable application capabilities to the Vue frontend instead of embedding desktop-specific assumptions in UI code.

## Product and Architecture Direction

- Treat nodes as business units, jobs as runtime units, and artifacts as reusable results, matching `DESIGN.md`.
- Frontend should express intent and immediate UI state; the Tauri host should own local files, queues, secure settings, export work, and platform capabilities.
- Do not treat the current model names as a complete backend contract. When implementing real providers, add explicit request/response types and error handling.
- Long-running generation and export work should be cancellable, retryable, and observable through progress/log state.
- File system, shell, network, and secret-storage capabilities must be justified by user-visible product value and scoped narrowly.

## Dependency and Config Policy

- Use npm, not pnpm or yarn, unless the project intentionally changes package managers.
- Keep `package-lock.json` in sync with `package.json`.
- Do not add new dependencies for simple UI, state, or data transformations that the existing stack can handle.
- If adding a dependency, explain why it is worth the extra desktop bundle and maintenance cost.
- Keep Vite and Tauri dev URLs aligned. The app currently expects `http://127.0.0.1:1420`.

## Change Management

- Keep changes narrowly scoped to the requested feature or fix.
- Do not rewrite existing user-facing Chinese/English copy casually; preserve the product tone unless copy changes are requested.
- Do not revert unrelated uncommitted changes.
- Avoid broad refactors while the app is still in prototype shape unless they directly reduce risk for the requested work.
- When a change affects both frontend and Tauri, update both sides in the same pass and validate the bridge with the relevant command.
