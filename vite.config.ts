import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import ui from "@nuxt/ui/vite";

export default defineConfig({
  plugins: [
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
  build: {
    target: process.env.TAURI_ENV_PLATFORM === "windows" ? "chrome105" : "safari13",
    minify: !process.env.TAURI_ENV_DEBUG ? "oxc" : false,
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
  },
});
