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
- npm

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
npm install
```

Run the web UI only:

```sh
npm run dev
```

Run the desktop app:

```sh
npm run tauri:dev
```

Tauri starts the Vite dev server automatically from `src-tauri/tauri.conf.json`, so you do not need to run `npm run dev` separately before `npm run tauri:dev`.

## Build

Build the frontend:

```sh
npm run build
```

Build the desktop app:

```sh
npm run tauri:build
```

Build output:

- Frontend: `dist/`
- Tauri: `src-tauri/target/`

## Troubleshooting

### `failed to run 'cargo metadata'`

If `npm run tauri:dev` prints an error like this:

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
npm run tauri:dev
```

### npm cache permission errors

If `npm install` reports a cache permission error, you can use a temporary cache directory:

```sh
npm_config_cache=/private/tmp/linglux-npm-cache npm install
```

For a long-term fix, repair the permissions of your local npm cache directory.

## References

- [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)
- [Rust installation](https://www.rust-lang.org/tools/install)
- [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
- [Microsoft Edge WebView2 Runtime](https://developer.microsoft.com/microsoft-edge/webview2/)
