import { spawn } from "node:child_process";
import { createServer } from "node:net";
import { setTimeout as delay } from "node:timers/promises";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../", import.meta.url));
const backendPort = port("REVIEWIZARD_BACKEND_PORT", 7777);
const frontendPort = port("REVIEWIZARD_FRONTEND_PORT", 5173);
const apiURL = `http://127.0.0.1:${backendPort}`;
const uiURL = `http://127.0.0.1:${frontendPort}`;
const children = [];
const shutdown = new AbortController();
let stopping = false;

function port(name, fallback) {
  const value = process.env[name] || String(fallback);
  if (!/^\d+$/.test(value) || Number(value) < 1 || Number(value) > 65535) {
    throw new Error(`${name} must be a port between 1 and 65535`);
  }
  return Number(value);
}

async function available(port) {
  const server = createServer();
  await new Promise((resolve, reject) => {
    server.once("error", () => reject(new Error(`Port ${port} is unavailable. Stop the app using it or choose a different port.`)));
    server.listen(port, "127.0.0.1", resolve);
  });
  await new Promise((resolve, reject) => server.close((error) => error ? reject(error) : resolve()));
}

function launch(name, task, env = {}) {
  const child = spawn("mise", ["run", task], {
    cwd: root,
    env: { ...process.env, ...env },
    detached: process.platform !== "win32",
    stdio: ["ignore", "pipe", "pipe"],
  });
  const entry = { name, child, output: "", exited: false };
  children.push(entry);
  for (const stream of [child.stdout, child.stderr]) {
    stream.on("data", (data) => { entry.output = (entry.output + data).slice(-16000); });
  }
  child.once("error", (error) => { entry.exited = true; void fail(`${name} could not start: ${error.message}`); });
  child.once("exit", (code, signal) => {
    entry.exited = true;
    if (!stopping) void fail(`${name} stopped unexpectedly (${signal || `exit ${code}`}).`);
  });
}

function signalChild(entry, signal) {
  if (!entry.child.pid) return;
  try {
    if (process.platform === "win32") {
      // Include descendants such as Cargo's API process and pnpm's Vite process.
      spawn("taskkill", ["/pid", String(entry.child.pid), "/T", "/F"], { stdio: "ignore" });
    } else {
      process.kill(-entry.child.pid, signal);
    }
  } catch (error) {
    if (error.code !== "ESRCH") console.error(error.message);
  }
}

async function stop(code) {
  if (stopping) return;
  stopping = true;
  process.exitCode = code;
  shutdown.abort();
  for (const entry of children) signalChild(entry, "SIGTERM");
  const deadline = Date.now() + 5000;
  while (children.some((entry) => !entry.exited) && Date.now() < deadline) await delay(50);
  for (const entry of children.filter((entry) => !entry.exited)) signalChild(entry, "SIGKILL");
}

async function fail(message) {
  if (stopping) return;
  console.error(`\n${message}`);
  for (const entry of children) {
    if (entry.output.trim()) console.error(`\n${entry.name}:\n${entry.output.trim()}`);
  }
  await stop(1);
}

async function ready(url, timeout) {
  const deadline = Date.now() + timeout;
  while (!stopping && Date.now() < deadline) {
    try {
      const response = await fetch(url, {
        signal: AbortSignal.any([shutdown.signal, AbortSignal.timeout(1500)]),
        redirect: "error",
      });
      await response.body?.cancel();
      if (response.ok) return;
    } catch {
      // The service may still be compiling or binding its listener.
    }
    await delay(250, undefined, { signal: shutdown.signal });
  }
  throw new Error(`Timed out waiting for ${url}`);
}

process.once("SIGINT", () => { void stop(0); });
process.once("SIGTERM", () => { void stop(0); });
process.once("SIGHUP", () => { void stop(0); });

try {
  await available(backendPort);
  await available(frontendPort);
  shutdown.signal.throwIfAborted();
  if (backendPort === frontendPort) throw new Error("API and UI must use different ports.");
  console.log("Starting reviewizard…");
  launch("API", "dev:backend");
  await ready(`${apiURL}/healthz`, 300000);
  if (!stopping) {
    launch("UI", "dev:frontend", { REVIEWIZARD_API_URL: apiURL });
    await ready(uiURL, 60000);
    if (!stopping) console.log(`\nreviewizard is running at ${uiURL}\nPress Ctrl+C to stop.`);
  }
} catch (error) {
  if (!stopping) await fail(error.message);
}
