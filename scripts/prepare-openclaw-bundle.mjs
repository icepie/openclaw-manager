#!/usr/bin/env node
// 为 openclaw-manager 准备离线安装 bundle
// 从 npm registry 拉取 openclaw，打包 node + npm + tgz + prefix snapshot

import fs from "node:fs";
import fsp from "node:fs/promises";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const projectRoot = path.resolve(__dirname, "..");
const bundleDir = path.resolve(projectRoot, "src-tauri", "bundle", "resources", "openclaw-bundle");
const tempDir = path.resolve(projectRoot, ".tmp", "openclaw-bundle");

const OPENCLAW_MIN_NODE = "22.16.0";
const OPENCLAW_SPEC = process.env.OPENCLAW_VERSION
  ? `openclaw@${process.env.OPENCLAW_VERSION}`
  : "openclaw@latest";
const RUN_MAX_BUFFER = Number(process.env.OPENCLAW_BUNDLE_RUN_MAX_BUFFER || 128 * 1024 * 1024);
const REQUESTED_NODE_ARCH = normalizeNodeArch(process.env.OPENCLAW_BUNDLE_NODE_ARCH);

function normalizeNodeArch(rawArch) {
  const value = String(rawArch ?? "").trim().toLowerCase();
  if (!value) return null;
  if (value === "x64" || value === "x86_64" || value === "amd64") return "x64";
  if (value === "arm64" || value === "aarch64") return "arm64";
  if (value === "ia32" || value === "x86") return "ia32";
  throw new Error(`Unsupported OPENCLAW_BUNDLE_NODE_ARCH value: ${rawArch}`);
}

function run(cmd, args, opts = {}) {
  const useShell = process.platform === "win32" && cmd === "npm";
  const result = spawnSync(cmd, args, {
    cwd: opts.cwd,
    env: opts.env ?? process.env,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
    shell: useShell,
    windowsHide: true,
    maxBuffer: RUN_MAX_BUFFER,
  });
  if (result.error) throw new Error(`${cmd} ${args.join(" ")} failed\n${String(result.error)}`);
  if (result.status !== 0) {
    const detail = [result.stdout, result.stderr].map((s) => (s ?? "").trim()).filter(Boolean).join("\n");
    throw new Error(`${cmd} ${args.join(" ")} failed${detail ? `\n${detail}` : ""}`);
  }
  return (result.stdout ?? "").trim();
}

function parseVersion(v) {
  const m = String(v).trim().match(/^v?(\d+)\.(\d+)\.(\d+)/);
  if (!m) return null;
  return { major: Number(m[1]), minor: Number(m[2]), patch: Number(m[3]) };
}

function versionGte(left, right) {
  const a = parseVersion(left);
  const b = parseVersion(right);
  if (!a || !b) return false;
  if (a.major !== b.major) return a.major > b.major;
  if (a.minor !== b.minor) return a.minor > b.minor;
  return a.patch >= b.patch;
}

function ensureFile(p, label) {
  if (!fs.existsSync(p) || !fs.statSync(p).isFile())
    throw new Error(`${label} not found: ${p}`);
}

async function ensureCleanDir(p) {
  await fsp.rm(p, { recursive: true, force: true });
  await fsp.mkdir(p, { recursive: true });
}

async function ensureUserWritableRecursive(rootDir) {
  const queue = [rootDir];
  while (queue.length > 0) {
    const current = queue.pop();
    const stat = await fsp.lstat(current);
    await fsp.chmod(current, stat.mode | 0o200);
    if (!stat.isDirectory()) continue;
    for (const child of await fsp.readdir(current))
      queue.push(path.join(current, child));
  }
}

function resolveNpmDir() {
  const npmRoot = run("npm", ["root", "-g"]);
  const candidate = path.join(npmRoot, "npm");
  if (fs.existsSync(candidate)) return candidate;
  const prefix = run("npm", ["config", "get", "prefix"]);
  const extra = process.platform === "win32"
    ? path.join(prefix, "node_modules", "npm")
    : path.join(prefix, "lib", "node_modules", "npm");
  if (fs.existsSync(extra)) return extra;
  throw new Error("Unable to locate npm directory for offline bundle");
}

function resolveInstalledOpenclaw(prefix) {
  const candidates = process.platform === "win32"
    ? [
        path.join(prefix, "bin", "openclaw.cmd"),
        path.join(prefix, "bin", "openclaw.exe"),
        path.join(prefix, "node_modules", "openclaw", "openclaw.mjs"),
        path.join(prefix, "node_modules", ".bin", "openclaw.cmd"),
      ]
    : [
        path.join(prefix, "bin", "openclaw"),
        path.join(prefix, "node_modules", "openclaw", "openclaw.mjs"),
        path.join(prefix, "node_modules", ".bin", "openclaw"),
      ];
  for (const c of candidates) {
    try { if (fs.statSync(c).isFile()) return c; } catch {}
  }
  throw new Error(`bundled openclaw executable not found under: ${prefix}`);
}

function npmPack(args, cwd) {
  const raw = run("npm", ["pack", "--json", ...args], { cwd });
  let parsed;
  try { parsed = JSON.parse(raw); } catch (e) {
    throw new Error(`Failed to parse npm pack --json output: ${e}\n${raw}`);
  }
  const record = Array.isArray(parsed) ? parsed[0] : parsed;
  const filename = record?.filename;
  if (!filename) throw new Error(`npm pack --json missing filename: ${raw}`);
  const packedFile = path.resolve(cwd, filename);
  ensureFile(packedFile, "packed openclaw tarball");
  return { filename, packedFile, version: record?.version ?? null };
}

async function resolveBundledNodeRuntime() {
  // 允许通过环境变量指定已有 node 二进制（CI 交叉编译场景）
  const customNode = process.env.OPENCLAW_BUNDLE_NODE;
  if (customNode && fs.existsSync(customNode)) {
    const ver = run(customNode, ["-v"]);
    let arch = process.arch;
    try { arch = run(customNode, ["-p", "process.arch"]).trim() || process.arch; } catch {}
    if (versionGte(ver, OPENCLAW_MIN_NODE))
      return { nodePath: customNode, nodeVersion: ver, nodeSource: "env:OPENCLAW_BUNDLE_NODE", nodeArch: arch };
  }

  console.log(`[bundle] provisioning portable node@${OPENCLAW_MIN_NODE} runtime...`);
  if (REQUESTED_NODE_ARCH) console.log(`[bundle] requested node arch: ${REQUESTED_NODE_ARCH}`);

  const nodeProvisionPrefix = path.join(tempDir, "node-runtime");
  await ensureCleanDir(nodeProvisionPrefix);
  const installEnv = { ...process.env };
  if (REQUESTED_NODE_ARCH) installEnv.npm_config_arch = REQUESTED_NODE_ARCH;

  run("npm", [
    "install", "--prefix", nodeProvisionPrefix,
    "--no-audit", "--no-fund", "--loglevel=error",
    `node@${OPENCLAW_MIN_NODE}`,
  ], { env: installEnv });

  const bundledNodePath = process.platform === "win32"
    ? path.join(nodeProvisionPrefix, "node_modules", "node", "bin", "node.exe")
    : path.join(nodeProvisionPrefix, "node_modules", "node", "bin", "node");
  ensureFile(bundledNodePath, "bundled node runtime");

  const effectiveArch = REQUESTED_NODE_ARCH ?? process.arch;
  let bundledNodeVersion;
  if (effectiveArch !== process.arch) {
    console.log(`[bundle] skip executing cross-arch node (${effectiveArch}) on host ${process.arch}`);
    bundledNodeVersion = `v${OPENCLAW_MIN_NODE}`;
  } else {
    bundledNodeVersion = run(bundledNodePath, ["-v"]);
    if (!versionGte(bundledNodeVersion, OPENCLAW_MIN_NODE))
      throw new Error(`bundled node ${bundledNodeVersion} does not satisfy >=${OPENCLAW_MIN_NODE}`);
  }

  return {
    nodePath: bundledNodePath,
    nodeVersion: bundledNodeVersion,
    nodeSource: `npm:node@${OPENCLAW_MIN_NODE}`,
    nodeArch: effectiveArch,
  };
}

async function resolveBundledNpmDir() {
  const npmProvisionPrefix = path.join(tempDir, "npm-runtime");
  await ensureCleanDir(npmProvisionPrefix);
  run("npm", ["install", "--prefix", npmProvisionPrefix, "--no-audit", "--no-fund", "--loglevel=error", "npm@10"]);
  const npmDir = path.join(npmProvisionPrefix, "node_modules", "npm");
  ensureFile(path.join(npmDir, "bin", "npm-cli.js"), "bundled npm cli");
  return npmDir;
}

async function main() {
  if (process.env.OPENCLAW_DESKTOP_SKIP_BUNDLE_PREP === "1") {
    console.log("[bundle] skip prepare because OPENCLAW_DESKTOP_SKIP_BUNDLE_PREP=1");
    return;
  }

  await ensureCleanDir(bundleDir);
  await ensureCleanDir(tempDir);

  // 从 npm registry 拉取 openclaw tgz
  console.log(`[bundle] fetching ${OPENCLAW_SPEC} from npm registry...`);
  const packed = npmPack([OPENCLAW_SPEC], tempDir);
  console.log(`[bundle] openclaw version: ${packed.version}, file: ${packed.filename}`);

  const runtime = await resolveBundledNodeRuntime();

  // 复制 tgz
  const bundledTgz = path.join(bundleDir, "openclaw.tgz");
  await fsp.copyFile(packed.packedFile, bundledTgz);

  // 复制 node runtime
  console.log("[bundle] copying node runtime...");
  const nodeDir = path.join(bundleDir, "node");
  await ensureCleanDir(nodeDir);
  const nodeTarget = path.join(nodeDir, process.platform === "win32" ? "node.exe" : "node");
  await fsp.copyFile(runtime.nodePath, nodeTarget);
  if (process.platform !== "win32") await fsp.chmod(nodeTarget, 0o755);
  ensureFile(nodeTarget, "bundled node runtime");

  // 复制 npm
  console.log("[bundle] copying npm...");
  let npmDir;
  try {
    npmDir = resolveNpmDir();
  } catch (e) {
    console.log(`[bundle] system npm dir unavailable (${e.message}), provisioning standalone npm...`);
    npmDir = await resolveBundledNpmDir();
  }
  const npmTarget = path.join(bundleDir, "npm");
  await fsp.rm(npmTarget, { recursive: true, force: true });
  await fsp.cp(npmDir, npmTarget, {
    recursive: true,
    filter: (src) => path.basename(src) !== ".npmrc",
  });

  // 预热 npm cache + 生成 prefix snapshot
  console.log("[bundle] warming offline npm cache...");
  const cacheDir = path.join(bundleDir, "npm-cache");
  const installPrefix = path.join(tempDir, "install-prefix");
  await fsp.mkdir(cacheDir, { recursive: true });
  let prefixAvailable = false;
  const bundledPrefix = path.join(bundleDir, "prefix");
  await fsp.rm(bundledPrefix, { recursive: true, force: true });

  try {
    run("npm", [
      "install", "--prefix", installPrefix, bundledTgz,
      "--cache", cacheDir,
      "--no-audit", "--no-fund", "--loglevel=error",
    ]);

    console.log("[bundle] snapshot installed prefix...");
    await fsp.cp(installPrefix, bundledPrefix, { recursive: true, dereference: true });

    if (process.env.OPENCLAW_BUNDLE_SKIP_VERIFY === "1") {
      console.log("[bundle] skip prefix verification (OPENCLAW_BUNDLE_SKIP_VERIFY=1)");
    } else if (runtime.nodeArch !== process.arch) {
      console.log(`[bundle] skip prefix verification for cross-arch ${runtime.nodeArch} on ${process.arch}`);
    } else {
      console.log("[bundle] verifying bundled prefix...");
      const verifyPrefix = path.join(tempDir, "verify-prefix");
      await fsp.cp(bundledPrefix, verifyPrefix, { recursive: true });
      const verifyBin = resolveInstalledOpenclaw(verifyPrefix);
      const verifyEnv = { ...process.env, PATH: `${path.dirname(nodeTarget)}${path.delimiter}${process.env.PATH || ""}` };
      if (process.platform === "win32") {
        run("cmd", ["/C", verifyBin, "--version"], { env: verifyEnv });
      } else {
        run(verifyBin, ["--version"], { env: verifyEnv });
      }
      await fsp.rm(verifyPrefix, { recursive: true, force: true });
    }
    prefixAvailable = true;
  } catch (e) {
    console.warn(`[bundle] WARN: prefix snapshot failed, fallback to npm-cache mode: ${e.message}`);
    await fsp.rm(bundledPrefix, { recursive: true, force: true });
    run("npm", ["cache", "add", bundledTgz, "--cache", cacheDir, "--loglevel=error"]);
  } finally {
    await fsp.rm(installPrefix, { recursive: true, force: true });
  }

  const npmCli = path.join(bundleDir, "npm", "bin", "npm-cli.js");
  const manifest = {
    name: "openclaw-offline-bundle",
    generatedAt: new Date().toISOString(),
    openclawVersion: packed.version ?? "unknown",
    openclawSource: `npm-registry:${OPENCLAW_SPEC}`,
    nodeVersion: runtime.nodeVersion,
    nodeSource: runtime.nodeSource,
    nodePlatform: `${process.platform}-${runtime.nodeArch}`,
    prefixAvailable,
    files: {
      openclawTgz: "openclaw.tgz",
      npmCache: "npm-cache",
      node: path.relative(bundleDir, nodeTarget),
      npmCli: path.relative(bundleDir, npmCli),
      prefix: prefixAvailable ? "prefix" : null,
    },
  };
  await fsp.writeFile(path.join(bundleDir, "manifest.json"), JSON.stringify(manifest, null, 2), "utf8");

  await ensureUserWritableRecursive(bundleDir);
  await fsp.rm(tempDir, { recursive: true, force: true });
  console.log("[bundle] ready:", bundleDir);
}

main().catch((e) => {
  console.error("[bundle] failed:", e instanceof Error ? e.message : String(e));
  process.exit(1);
});
