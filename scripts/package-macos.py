"""Bundle the locally built preview; do not sign or publish it."""
import os
import plistlib
import shutil
import sys
from pathlib import Path

root = Path(__file__).resolve().parent.parent
profile = sys.argv[1] if len(sys.argv) > 1 else "release"
target = Path(os.environ.get("CARGO_TARGET_DIR", str(root / "target"))).expanduser()
binary = target / profile / "apibuddy-app"
if not binary.exists():
    flag = " --release" if profile == "release" else "" if profile == "debug" else f" --profile {profile}"
    raise SystemExit(f"missing binary: {binary}; run cargo build{flag} -p apibuddy-app first")

bundle = root / "dist" / "ApiBuddy.app"
if bundle.exists():
    shutil.rmtree(bundle)
(bundle / "Contents/MacOS").mkdir(parents=True)
(bundle / "Contents/Resources").mkdir()
shutil.copy2(binary, bundle / "Contents/MacOS/apibuddy-app")

with (bundle / "Contents/Info.plist").open("wb") as stream:
    plistlib.dump(
        {
            "CFBundleName": "ApiBuddy",
            "CFBundleDisplayName": "ApiBuddy",
            "CFBundleIdentifier": "io.github.hezhangjian.apibuddy",
            "CFBundleExecutable": "apibuddy-app",
            "CFBundlePackageType": "APPL",
            "CFBundleShortVersionString": "0.0.1",
            "CFBundleVersion": "1",
            "LSMinimumSystemVersion": "14.0",
            "NSHighResolutionCapable": True,
        },
        stream,
    )
print(bundle)
