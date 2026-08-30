# Linglux

Linglux is an AI agent for video creation, editing, enhancement, and automation, designed to make professional video production accessible to everyone.

## Stack

- Vue 3
- TypeScript
- Vite
- Tailwind CSS
- Tauri 2

## Requirements

For web development:

- Node.js 20 or later
- pnpm 11 or later

For desktop development with Tauri:

- Rust toolchain, including `cargo` and `rustc`
- Platform-specific system dependencies for macOS or Windows

## macOS Setup

Install Xcode Command Line Tools:

```sh
xcode-select --install
```

Install Rust:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

Verify the installation:

```sh
cargo --version
rustc --version
```

If you also plan to target iOS, install the full Xcode app from the Mac App Store or Apple Developer website and open it once to finish setup.

## Windows Setup

Install Node.js 20 or later from the Node.js website, or use PowerShell with `winget`:

```powershell
winget install --id OpenJS.NodeJS.LTS -e
```

Install Microsoft C++ Build Tools:

1. Download Microsoft C++ Build Tools from Visual Studio.
2. Open the installer.
3. Select the `Desktop development with C++` workload.
4. Finish the installation and restart the terminal.

Install Microsoft Edge WebView2 Runtime if it is not already present. WebView2 is already included on Windows 10 version 1803 and later, but older or stripped-down systems may still need the Evergreen Runtime installer.

Install Rust with `winget`:

```powershell
winget install --id Rustlang.Rustup -e
```

During Rust setup, use the MSVC Rust toolchain. After installation, open a new terminal and verify:

```powershell
cargo --version
rustc --version
```

If `cargo` is still not found, make sure Rust's cargo bin directory is available in `PATH`. For the current PowerShell session, you can try:

```powershell
$env:Path += ";$env:USERPROFILE\.cargo\bin"
cargo --version
```

If you build MSI installers on Windows and see errors around `light.exe`, enable the Windows optional feature named `VBSCRIPT`, then restart if Windows asks you to.

## Development

Install dependencies:

```sh
pnpm install
```

Run the web UI only:

```sh
pnpm dev
```

Run the desktop app:

```sh
pnpm tauri:dev
```

Tauri starts the Vite dev server automatically from `src-tauri/tauri.conf.json`, so you do not need to run `pnpm dev` separately before `pnpm tauri:dev`.

## Build

Build the frontend:

```sh
pnpm build
```

Build the desktop app:

```sh
pnpm tauri:build
```

Run frontend unit tests:

```sh
pnpm test
```

Build output:

- Frontend: `dist/`
- Tauri: `src-tauri/target/`

## AI Editing Agent

The editor includes a right-side Linglux Agent panel in the desktop app. It translates natural-language instructions into a typed edit plan, shows the plan for confirmation, and then applies it directly to project/timeline data. It does not read the screen or control the mouse.

To configure it:

1. Open **Settings → Model Service**.
2. Choose DeepSeek, OpenAI, OpenRouter, or a custom HTTPS OpenAI-compatible endpoint.
3. Confirm the Base URL and editable chat model ID.
4. Enter an API key. Desktop keys remain in Rust host memory for the current app session only, are never returned to the WebView, and are cleared when the app exits.

DeepSeek defaults to `https://api.deepseek.com` and `deepseek-v4-flash`. Provider calls and persistent chat are desktop-only; the web preview renders the UI without calling a remote model.

Example instruction:

```text
把 x.mp4 加到主视频轨，1:03 前面的不要，3:02 后面的不要
```

The Agent proposes keeping source range `01:03–03:02`. After confirmation, the resulting edit is one undoable timeline transaction. Conversation history is stored in the project package as `agent-conversation.json` and is not included in export manifests.

## Storyboard Image to Video

The desktop editor can turn a managed storyboard/contact-sheet image into a short MP4. Select an imported image, open the storyboard animation dialog, verify the automatically detected grid and crop boxes, then choose the frame count and FPS. Linglux crops frames in row-major order, normalizes them to a common even-sized canvas, encodes H.264 with FFmpeg, and adds the result back to the project media bin.

The source image must already be imported into the current project. A task accepts 1–240 frames and 1–120 FPS, is cancellable, and requires a discoverable local `ffmpeg` executable.

## Local AI Voiceover

AI voiceover is currently available only in the desktop app on macOS Apple Silicon. Open the editor's audio panel, install the local model when prompted (about 3.4 GB including runtime and model), then choose the built-in Mandarin male or female voice. Once installed, synthesis runs offline and generated WAV files are managed with the project like imported audio.

The feature uses [CosyVoice-300M-Instruct](https://github.com/QwenAudio/CosyVoice), distributed under Apache License 2.0. Linglux pins the runtime, source, and model revisions instead of following floating releases. Initial setup requires network access; voice cloning and browser synthesis are not enabled.

## Troubleshooting

### `failed to run 'cargo metadata'`

If `pnpm tauri:dev` prints an error like this:

```text
failed to run command cargo metadata --no-deps --format-version 1: No such file or directory
```

The current terminal cannot find `cargo`. Rust is either not installed, or the terminal has not loaded Rust's environment variables yet.

Check first:

```sh
cargo --version
```

On macOS, install or reload Rust with:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

On Windows, install Rust with:

```powershell
winget install --id Rustlang.Rustup -e
```

Then open a new terminal and run:

```sh
pnpm tauri:dev
```

### pnpm store permission errors

If `pnpm install` reports a store permission error, you can use a temporary store directory:

```sh
pnpm install --store-dir /private/tmp/linglux-pnpm-store
```

For a long-term fix, repair the permissions of the store reported by `pnpm store path`.

## References

- [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)
- [Rust installation](https://www.rust-lang.org/tools/install)
- [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
- [Microsoft Edge WebView2 Runtime](https://developer.microsoft.com/microsoft-edge/webview2/)
