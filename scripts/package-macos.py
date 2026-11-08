"""Bundle the locally built preview; do not sign or publish it."""
import os
import plistlib
import shutil
import sys
from pathlib import Path

root = Path(__file__).resolve().parent.parent
profile = sys.argv[1] if len(sys.argv) > 1 else "release"
if len(sys.argv) > 2:
    raise SystemExit("usage: package-macos.py [debug|release]; only the native app is maintained")
name = "ApiBuddy"
exe = "apibuddy-app"
identifier = "io.github.hezhangjian.apibuddy"
target = Path(os.environ.get("CARGO_TARGET_DIR", str(root / "target"))).expanduser()
binary = target / profile / exe
if not binary.exists():
    flag = " --release" if profile == "release" else "" if profile == "debug" else f" --profile {profile}"
    raise SystemExit(f"missing binary: {binary}; run cargo build{flag} -p {exe} first")
icon = root / "crates/apibuddy-app/assets/brand/ApiBuddy.icns"
if not icon.is_file():
    raise SystemExit(f"missing icon: {icon}; run python3 scripts/generate-icons.py first")

bundle = root / "dist" / f"{name}.app"
if bundle.exists():
    shutil.rmtree(bundle)
(bundle / "Contents/MacOS").mkdir(parents=True)
(bundle / "Contents/Resources").mkdir()
shutil.copy2(binary, bundle / "Contents/MacOS" / exe)
shutil.copy2(icon, bundle / "Contents/Resources" / icon.name)

with (bundle / "Contents/Info.plist").open("wb") as stream:
    plistlib.dump(
        {
            "CFBundleName": name,
            "CFBundleDisplayName": name,
            "CFBundleIdentifier": identifier,
            "CFBundleExecutable": exe,
            "CFBundleIconFile": icon.name,
            "CFBundlePackageType": "APPL",
            "CFBundleShortVersionString": "0.0.1",
            "CFBundleVersion": "1",
            "LSMinimumSystemVersion": "14.0",
            "NSHighResolutionCapable": True,
        },
        stream,
    )
print(bundle)
