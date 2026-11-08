"""Regenerate the committed app icons from the original Buddy SVG on macOS."""

import shutil
import subprocess
import tempfile
from pathlib import Path


def main():
    renderer = shutil.which("rsvg-convert")
    iconutil = shutil.which("iconutil")
    if not renderer or not iconutil:
        raise SystemExit("requires macOS iconutil and rsvg-convert (brew install librsvg)")

    brand = Path(__file__).resolve().parent.parent / "crates/apibuddy-app/assets/brand"
    source = brand / "ApiBuddy-app-icon.svg"
    with tempfile.TemporaryDirectory(prefix="apibuddy-icons-") as directory:
        temporary = Path(directory)
        iconset = temporary / "ApiBuddy.iconset"
        iconset.mkdir()
        for size in (16, 32, 128, 256, 512):
            for scale, suffix in ((1, ""), (2, "@2x")):
                pixels = str(size * scale)
                subprocess.run(
                    [
                        renderer, "--width", pixels, "--height", pixels,
                        "--output", str(iconset / f"icon_{size}x{size}{suffix}.png"),
                        str(source),
                    ],
                    check=True,
                )
        icon = temporary / "ApiBuddy.icns"
        subprocess.run([iconutil, "-c", "icns", str(iconset), "-o", str(icon)], check=True)
        # Keep normal builds independent of the SVG conversion tools.
        shutil.copy2(iconset / "icon_512x512@2x.png", brand / "ApiBuddy-app-icon.png")
        shutil.copy2(icon, brand / icon.name)
    print(f"Updated app icons in {brand}")


if __name__ == "__main__":
    main()
