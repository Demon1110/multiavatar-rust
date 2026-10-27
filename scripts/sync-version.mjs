// 同步版本号：Cargo.toml（唯一来源）→ wasm/package.json
// 跨平台（Windows/Linux npm hook 均可跑）；幂等；只替换顶层 "version" 字段，其余内容原样保留。
// 用法：node scripts/sync-version.mjs
import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");

const cargo = readFileSync(join(root, "Cargo.toml"), "utf8");
const m = cargo.match(/^version\s*=\s*"([^"]+)"/m);
if (!m) {
  console.error("sync-version: Cargo.toml 未找到 version");
  process.exit(1);
}
const ver = m[1];

const pkgPath = join(root, "wasm", "package.json");
const raw = readFileSync(pkgPath, "utf8");
if (!/"version"\s*:/.test(raw)) {
  console.error("sync-version: wasm/package.json 未找到 version 字段");
  process.exit(1);
}
writeFileSync(pkgPath, raw.replace(/("version"\s*:\s*)"[^"]*"/, `$1"${ver}"`), "utf8");

console.log(`synced: wasm/package.json version -> ${ver}`);
