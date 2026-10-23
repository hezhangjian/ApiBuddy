import { build } from "esbuild";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
const output = mkdtempSync(join(tmpdir(), "apibuddy-tests-"));
try {
  const runtime = join(output, "cli-runtime.cjs");
  await build({ entryPoints: ["src/cli/runtime.ts"], bundle: true, platform: "node", format: "cjs", target: "node18", outfile: runtime });
  const files = [];
  for (const name of ["core", "service-cli"]) {
    const file = join(output, `${name}.test.mjs`);
    await build({ entryPoints: [`tests/${name}.test.ts`], bundle: true, platform: "node", format: "esm", outfile: file });
    files.push(file);
  }
  const result = spawnSync(process.execPath, ["--test", ...files], { stdio: "inherit", env: { ...process.env, APIBUDDY_TEST_CLI_RUNTIME: runtime } });
  process.exitCode = result.status ?? 1;
} finally { rmSync(output, { recursive: true, force: true }); }
