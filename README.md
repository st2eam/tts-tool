# TTS 小工具

一个面向 Windows 的桌面工具，用于安全导入、管理和导出
[Tabletop Simulator](https://store.steampowered.com/app/286160/Tabletop_Simulator/)
图包。

它使用 Rust 和 egui 构建，支持 ZIP / RAR 图包导入、TTS Mods 目录识别、
已安装图包浏览、封面预览，以及基于资源引用闭包的完整导出。

## 功能

- **安全导入**
  - 支持 `.zip` 和 `.rar`。
  - 自动发现常见的 TTS `Mods` 目录，也支持手动选择。
  - 仅接受可识别的 TTS 目录，避免误选普通文件夹。
  - 覆盖文件前创建可回滚备份；导入失败时尽量恢复原状。
  - 导入完成后自动刷新图包列表和目录统计。
- **图包管理**
  - 扫描 `Workshop` 目录中的 JSON 图包。
  - 读取 `SaveName` 作为显示名称。
  - 自动查找同名 JPG / JPEG / PNG 封面并缓存缩略图。
  - 点击图包查看详情；右键提供详情、导出、资源管理器定位等操作。
- **资源闭包导出**
  - 解析图包 JSON 中的本地资源引用和远程 URL。
  - 区分已找到、未解析、远程资源和同名冲突。
  - 将 JSON、封面及相关本地资源写入 ZIP。
  - 在导出包中生成 `tts-tool-manifest.json`，记录相对路径、引用来源、
    文件大小和 SHA-256。
  - 可删除已生成的导出 ZIP；不会删除 Mods 目录中的原始图包或共享资源。

## 使用

### 1. 选择 Mods 目录

启动后，程序会尝试发现 TTS Mods 目录。目录必须满足以下条件之一：

- 目录名为 `Mods`；
- 包含一个或多个标准 TTS 子目录，例如 `Workshop`、`Images`、`Models`、
  `Audio`、`PDF` 或 `Video`。

确认目录后，界面会显示已识别目录数量、图包数量和资源文件数量。

### 2. 导入图包

将 ZIP / RAR 文件拖入 **TTS 图包导入** 区域，或点击区域选择文件。
导入过程会显示当前阶段和进度。导入完成后，图包会出现在
**TTS 图包管理** 区域。

### 3. 查看和导出

在管理区域中：

- 单击图包打开详情；
- 右键图包打开操作菜单；
- 选择“导出图包”生成资源闭包 ZIP；
- 选择“在资源管理器中定位”打开原始 JSON；
- 对已经导出的图包选择“删除导出文件”。

导出 ZIP 内的 `tts-tool-manifest.json` 可用于检查导出内容和资源完整性。

## 目录结构

```text
tts-tool/
├─ src/
│  ├─ main.rs              # egui 界面、扫描、导入交互和导出交互
│  └─ lib.rs               # 目录识别、归档导入、资源索引和回滚逻辑
├─ localization/           # TTS 本地化相关工具和资源
├─ research/               # 项目分析和参考实现研究文档
├─ app.manifest            # Windows 应用清单
├─ build.rs                # Windows 资源编译
├─ package.ps1             # Release 构建和 EXE 打包脚本
├─ tts-tool.rc             # Windows 图标资源
├─ icon.ico                # EXE / 资源管理器图标
└─ logo.png                # 运行时窗口图标
```

## 技术栈

- Rust 2024 Edition
- [eframe / egui](https://github.com/emilk/egui)
- `zip`：ZIP 归档读取和写入
- `unrar-rs`：RAR 归档读取
- `serde` / `serde_json`：TTS JSON 解析
- `sha2`：导出 manifest 的 SHA-256 校验
- `rfd`：Windows 文件和目录选择器
- `image`：封面读取和缩略图生成

## 开发环境

| 项目 | 要求 |
| --- | --- |
| 操作系统 | Windows |
| Rust | stable toolchain，支持 Rust 2024 Edition |
| Windows SDK | 构建发布版 EXE 时需要 `mt.exe` |
| Visual C++ 工具链 | 由 Rust MSVC toolchain 提供 |

## 构建

运行测试：

```powershell
cargo test --quiet
```

构建发布版并生成单文件 EXE：

```powershell
.\package.ps1
```

输出文件：

```text
dist\TTS小工具.exe
```

`package.ps1` 会执行以下步骤：

1. 使用 Cargo 构建 Release 版本；
2. 使用 Windows SDK 的 `mt.exe` 注入 `app.manifest`；
3. 清空 `dist\`；
4. 复制最新 EXE 到 `dist\`。

Release 配置已启用 LTO、体积优化、符号剥离和静态 CRT，目标是生成便于分发的
单文件 Windows 应用。`target\` 和 `dist\` 不纳入版本库。

## 许可和第三方声明

项目当前未声明统一的开源许可证。发布或再分发前，请先补充项目许可证。

RAR 支持依赖 `unrar-rs`，其相关 UnRAR 许可要求见
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。
