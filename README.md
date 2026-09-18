# TTS小工具

Windows 上用于导入 Tabletop Simulator 图包的轻量单文件工具。

将已下载的 `.zip` 或 `.rar` 图包拖到窗口即可。程序会找到或要求选择 TTS 的 `Mods` 目录，并只解压 `Images`、`Models`、`Workshop` 等标准资源目录。覆盖原有文件前会建立可撤销备份。

图包详情支持导出资源闭包：程序会解析 TTS JSON 中的本地资源引用，区分远程 URL、未解析引用和同名冲突，导出的 ZIP 还会包含 `tts-tool-manifest.json`，记录每个文件的相对路径、引用来源、大小和 SHA-256。相比简单的全目录同名扫描，这能避免把无关同名文件静默打进包内。
图包列表支持右键菜单，可直接打开详情、导出图包、定位 JSON 文件，并删除最近生成的导出 ZIP；删除操作不会修改 Mods 目录中的原始资源。
主界面分为两个独立区域：`TTS 图包管理` 用于浏览已安装图包和执行管理操作，`TTS 图包导入` 用于拖放或选择 ZIP / RAR 文件。导入进度和结果只显示在导入区域，不会替换图包列表。

目录选择会校验所选位置是否为 TTS Mods 目录（目录名为 `Mods`，或包含 `Workshop`、`Images`、`Models` 等标准目录），并显示已识别的标准目录、图包数量和资源文件数量。导入成功后会自动重新扫描图包列表。

## 构建

```powershell
cargo test
.\package.ps1
```

**成品位于 `dist\TTS小工具.exe`**，仅此一个文件，可直接分发。

构建说明：
- `build.rs` 在编译期通过 embed-resource 编译 `tts-tool.rc`；`package.ps1` 再用 mt.exe 把 `app.manifest`（Common-Controls 6.0、`asInvoker`、PerMonitorV2 高 DPI 感知）注入 PE。
- `.cargo\config.toml` 开启 `crt-static`，成品不依赖 `vcruntime140.dll` / `ucrtbase.dll`，可单文件分发。
- `Cargo.toml` 的 `[profile.release]` 已启用 LTO、`opt-level="z"`、`panic="abort"`、符号剥离。
- `dist\` 每次构建会被清空，只保留最新版本；`target\` 为 Cargo 构建缓存，不入库。
