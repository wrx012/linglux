import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import ui from "@nuxt/ui/vite";

const BIGINT_ZERO_COMPARISON = /([A-Za-z_$][\w$]*) !== 0n/g;

function legacyWebKitBigIntCompatibility() {
  return {
    name: "linglux-legacy-webkit-bigint-compatibility",
    enforce: "pre" as const,
    transform(code: string, id: string) {
      if (!id.includes("/tailwind-variants/")) return null;

      const compatibleCode = code.replace(
        BIGINT_ZERO_COMPARISON,
        'typeof $1 !== "bigint"',
      );

      return compatibleCode === code ? null : { code: compatibleCode, map: null };
    },
  };
}

export default defineConfig({
  plugins: [
    legacyWebKitBigIntCompatibility(),
    vue(),
    ui({
      colorMode: false,
      router: false,
      theme: {
        defaultVariants: {
          color: "primary",
          size: "sm",
        },
      },
      ui: {
        colors: {
          primary: "emerald",
          secondary: "blue",
          neutral: "zinc",
        },
        button: {
          slots: {
            base: "font-bold",
          },
        },
        input: {
          slots: {
            base: "font-semibold",
          },
        },
        select: {
          slots: {
            base: "font-semibold",
          },
        },
      },
    }),
  ],
  clearScreen: false,
  server: {
    strictPort: true,
    port: 1420,
  },
  envPrefix: ["VITE_", "TAURI_"],
  optimizeDeps: {
    // Keep this dependency in Vite's normal transform pipeline. Its distributed
    // source contains BigInt literals that older macOS WebViews cannot parse.
    exclude: ["tailwind-variants"],
  },
  build: {
    target: process.env.TAURI_ENV_PLATFORM === "windows" ? "chrome105" : "safari13",
    minify: !process.env.TAURI_ENV_DEBUG ? "oxc" : false,
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
  },
});
