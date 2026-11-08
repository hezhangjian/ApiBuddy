# ApiBuddy 品牌素材

`ApiBuddy-app-icon.svg` 原样选自用户指定的 `~/ChuQin/Resources/Media/Images/Buddy-Logos/`，为 2026-11-02 的 Buddy 系列 Logo v1。保留青绿花括号、白色双眼、深蓝底与外围透明区域；不依赖字体或远端资源。

- 工作台通过 `include_str!` 嵌入 SVG，由 NSH 原生矢量绘制，运行时不访问原始素材目录。
- `ApiBuddy-app-icon.png` 是 1024 × 1024 透明 PNG，供 `nsh.json` 的原生打包入口使用。
- `ApiBuddy.icns` 包含 macOS 的 16、32、128、256、512 pt 及各自 2× 图层，由 `package-macos.py` 放入应用包。

更新 SVG 后，在仓库根目录运行 `python3 scripts/generate-icons.py`。重建图标需要 macOS 的 `iconutil` 和 `rsvg-convert`（`brew install librsvg`）；中间图层放在系统临时目录并自动清理。PNG 与 ICNS 随源码提交，日常编译、打包不需要图标转换工具。
