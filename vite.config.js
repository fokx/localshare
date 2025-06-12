import {defineConfig} from "vite";
import {sveltekit} from "@sveltejs/kit/vite";
import tailwindcss from '@tailwindcss/vite';

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// https://vitejs.dev/config/
export default defineConfig(async () => ({
    plugins: [tailwindcss(), sveltekit()],

    // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
    //
    // 1. prevent vite from obscuring rust errors
    clearScreen: false,
    // 2. tauri expects a fixed port, fail if that port is not available
    server: {
        port: 1420,
        strictPort: true,
        host: host || false,
        hmr: host
            ? {
                protocol: "ws",
                host,
                port: 1421,
            }
            : undefined,
        watch: {
            // 3. tell vite to ignore watching `src-tauri`
            ignored: ["**/src-tauri/**"],
        },
        fs: {
            allow: [
                '../plugins-workspace/plugins/sql/dist-js/**',
                '../plugins-workspace/plugins/sql/dist-js/*',
                '../flowbite-svelte/dist/**',
                '../flowbite-svelte/dist/*',
            ]
        }
    },
    // the following will lead to runtime error (only) in production build on Android:
    // Uncaught (in promise) TypeError: Failed to resolve module specifier "@tauri-apps/api/core". Relative references must start with either "/", "./", or "../".
    // build: {
        // rollupOptions: {
            // external: [
            //     '@tauri-apps/api',
            //     '@tauri-apps/api/core'
            // ]
        // }
    // }
}));
