import { fileURLToPath } from "node:url";
import { defineConfig, loadEnv } from "vite";
import react, { reactCompilerPreset } from "@vitejs/plugin-react";
import babel from "@rolldown/plugin-babel";
import tailwindcss from "@tailwindcss/vite";

// React Compiler via Babel (plugin-react has had no `babel` option since v6); target "18" needs react-compiler-runtime.
// Not the native `react({ compiler: true })`: DiffView's memo comparator is tuned against this compiler's bailouts.
const reactCompiler = babel({
  presets: [reactCompilerPreset({ target: "18" })],
});

// The frontend is served separately; dev proxies API requests to Rust.
export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), "REVIEWIZARD_");
  return {
    plugins: [tailwindcss(), react(), reactCompiler],
    resolve: {
      alias: { "@": fileURLToPath(new URL("./src", import.meta.url)) },
    },
    build: {
      outDir: "dist",
      emptyOutDir: true,
    },
    server: {
      proxy: {
        "/api": env.REVIEWIZARD_API_URL || "http://127.0.0.1:7777",
      },
    },
  };
});
