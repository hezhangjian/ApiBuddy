"""Bundle the locally built preview; do not sign or publish it."""
import os
import plistlib
import shutil
import sys
from pathlib import Path

root = Path(__file__).resolve().parent.parent
profile = sys.argv[1] if len(sys.argv) > 1 else "release"
variant = sys.argv[2] if len(sys.argv) > 2 else "native"
if variant not in ("native", "webview"):
    raise SystemExit("variant must be native or webview")
name = "ApiBuddy" if variant == "native" else "ApiBuddy WebView"
exe = "apibuddy-app" if variant == "native" else "apibuddy-webview"
identifier = "io.github.hezhangjian.apibuddy" + (".webview" if variant == "webview" else "")
target = Path(os.environ.get("CARGO_TARGET_DIR", str(root / "target"))).expanduser()
binary = target / profile / exe
if not binary.exists():
    flag = " --release" if profile == "release" else "" if profile == "debug" else f" --profile {profile}"
    raise SystemExit(f"missing binary: {binary}; run cargo build{flag} -p {exe} first")

bundle = root / "dist" / f"{name}.app"
if variant == "webview" and not (root / "dist/web/index.html").is_file():
    raise SystemExit("missing Web UI; run python3 scripts/build-webview.py first")
if bundle.exists():
    shutil.rmtree(bundle)
(bundle / "Contents/MacOS").mkdir(parents=True)
(bundle / "Contents/Resources").mkdir()
shutil.copy2(binary, bundle / "Contents/MacOS" / exe)
if variant == "webview":
    shutil.copytree(root / "dist/web", bundle / "Contents/Resources/web")

with (bundle / "Contents/Info.plist").open("wb") as stream:
    plistlib.dump(
        {
            "CFBundleName": name,
            "CFBundleDisplayName": name,
            "CFBundleIdentifier": identifier,
            "CFBundleExecutable": exe,
            "CFBundlePackageType": "APPL",
            "CFBundleShortVersionString": "0.0.1",
            "CFBundleVersion": "1",
            "LSMinimumSystemVersion": "14.0",
            "NSHighResolutionCapable": True,
        },
        stream,
    )
print(bundle)
