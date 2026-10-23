import { cpSync, mkdtempSync, symlinkSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import { build } from "esbuild";
const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const channel = process.argv[2] === "stable" ? "stable" : "dev";
const staging = mkdtempSync(join(tmpdir(), "apibuddy-electrobun-"));
function run(command, args, cwd) {
  const result = spawnSync(command, args, { cwd, stdio: "inherit", env: process.env });
  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
}
// Resolve relative data paths before moving the build to its temporary workspace.
for (const name of ["APIBUDDY_DIR", "APIABC_DIR", "APIX_DIR"]) {
  if (process.env[name] && !process.env[name].startsWith("~")) process.env[name] = resolve(process.env[name]);
}
run("hutch", ["electrobun", "prepare"], root);
for (const name of ["src", "electrobun.config.ts", "hutch.config.ts", "hutch.lock", "package.json", "postcss.config.js", "tailwind.config.js", "tsconfig.json", "vite.config.ts"]) {
  cpSync(join(root, name), join(staging, name), { recursive: true });
}
cpSync(join(root, ".hutch"), join(staging, ".hutch"), { recursive: true });
symlinkSync(join(root, "node_modules"), join(staging, "node_modules"), "dir");
console.log(`ApiBuddy build workspace: ${staging}`);
await build({ entryPoints: [join(staging, "src/cli/runtime.ts")], bundle: true, platform: "node", format: "cjs", target: "node18", outfile: join(staging, "generated/cli-runtime.cjs") });
run("pnpm", ["exec", "vite", "build"], staging);
run("hutch", ["electrobun", "build", `--env=${channel}`], staging);
if (channel === "dev") {
  if (process.platform === "darwin") {
    const app = join(staging, "build", `dev-macos-${process.arch}`, "ApiBuddy-dev.app");
    const environment = ["APIBUDDY_DIR", "APIBUDDY_HMR", "APIABC_DIR", "APIX_DIR"].flatMap((name) => process.env[name] === undefined ? [] : ["--env", `${name}=${process.env[name]}`]);
    run("open", ["-n", "-W", ...environment, app], staging);
  } else {
    run("hutch", ["electrobun", "run"], staging);
  }
}
