#!/usr/bin/env python3
"""Build the shared NSH Rust UI for the desktop WebView, then build its native host."""
import json
import os
from pathlib import Path
import sys
import shutil
import tempfile
import subprocess

root = Path(__file__).resolve().parent.parent
env = os.environ.copy()
target = Path(env.get("CARGO_TARGET_DIR", str(root / "target"))).resolve()
tools = target / "webview-tools"

def run(*args):
    subprocess.run([str(a) for a in args], cwd=root, env=env, check=True)

# Use the project's pinned NSH checkout; no globally installed NSH version is assumed.
metadata = json.loads(subprocess.check_output(["cargo", "metadata", "--locked", "--format-version", "1"], cwd=root, env=env))
nsh_package = next(p for p in metadata["packages"] if p["name"] == "nsh-core")
nsh_source = Path(nsh_package["manifest_path"]).parent.parent.parent
source_id = nsh_package["source"] or str(nsh_source)
stamp = tools / "nsh-source.txt"
suffix = ".exe" if os.name == "nt" else ""
cli = Path(env.get("NSH_CLI", str(tools / ("bin/nsh" + suffix)))).expanduser().resolve()
if "NSH_CLI" in env and not cli.is_file():
    raise SystemExit(f"NSH_CLI does not exist: {cli}")
if not cli.is_file() or ("NSH_CLI" not in env and (not stamp.is_file() or stamp.read_text() != source_id)):
    run("cargo", "install", "--force", "--locked", "--config", "profile.release.lto=false", "--config", "profile.release.codegen-units=16", "--path", nsh_source / "crates/nsh-cli", "--root", tools, "--target-dir", target / "webview-tools-build")
    stamp.write_text(source_id)
bindgen = Path(env.get("NSH_WASM_BINDGEN", str(tools / ("bin/wasm-bindgen" + suffix)))).expanduser().resolve()
if "NSH_WASM_BINDGEN" in env and not bindgen.is_file():
    raise SystemExit(f"NSH_WASM_BINDGEN does not exist: {bindgen}")
if not bindgen.is_file():
    run("cargo", "install", "wasm-bindgen-cli", "--version", "0.2.130", "--locked", "--root", tools)
if subprocess.check_output([str(bindgen), "--version"], text=True).strip() != "wasm-bindgen 0.2.130":
    raise SystemExit("NSH_WASM_BINDGEN must be wasm-bindgen 0.2.130")
env["NSH_WASM_BINDGEN"] = str(bindgen)
if sys.platform == "darwin":
    for prefix in [Path("/opt/homebrew/opt/llvm/bin"), Path("/usr/local/opt/llvm/bin")]:
        if (prefix / "clang").is_file():
            env.setdefault("CC_wasm32_unknown_unknown", str(prefix / "clang"))
            env.setdefault("AR_wasm32_unknown_unknown", str(prefix / "llvm-ar"))
            break
run("rustup", "target", "add", "wasm32-unknown-unknown")
with tempfile.TemporaryDirectory(prefix="apibuddy-web-package-") as temporary:
    run(cli, "package", "--target", "web", "--out", temporary)
    pages = list(Path(temporary).glob("*/index.html"))
    if len(pages) != 1:
        raise SystemExit("NSH did not produce exactly one packaged app")
    shutil.copytree(pages[0].parent, root / "dist/web", dirs_exist_ok=True)
run("cargo", "build", "--locked", "--release", "-p", "apibuddy-webview")
print(f"WebView executable: {target / 'release/apibuddy-webview'}")
