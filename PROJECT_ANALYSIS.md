# TTS 小工具项目分析

## 一、项目定位

这是一个用于导入 Tabletop Simulator（TTS）图包的 Windows 桌面工具。它的目标非常明确：

- 允许用户将 `.zip` 或 `.rar` 图包拖到窗口中；
- 自动发现本机 TTS 的 `Mods` 目录，或让用户手动选择；
- 只处理 TTS 标准目录，例如 `Images`、`Models`、`Workshop` 等；
- 在覆盖已有文件前，先做备份，保证可撤销；
- 输出一个能直接分发的单文件 EXE。

它不是一个通用型游戏工具，而是一个“安全、单纯、可迁移”的 TTS 资源导入辅助工具。

---

## 二、项目结构

```text
tts-tool/
├─ .cargo/
├─ src/
│  ├─ lib.rs          # 核心导入逻辑：ZIP/RAR 解析、解压、覆盖、回滚
│  └─ main.rs         # UI 入口：egui 窗口、拖放交互、状态与进度展示
├─ Cargo.toml         # Rust 项目配置和依赖声明
├─ build.rs           # 编译期资源嵌入
├─ README.md          # 项目说明
├─ app.manifest       # Windows DPI / 高分辨率兼容配置
├─ package.ps1        # 打包脚本
├─ tts-tool.rc        # Windows 资源脚本
├─ dist/              # 构建输出目录
├─ target/            # Cargo 编译缓存
└─ .gitignore
```

核心分工非常清晰：

- `src/lib.rs`：负责“真实业务逻辑”，例如归档识别、安全路径检查、导入事务、回滚。
- `src/main.rs`：负责“用户界面”，包括窗口渲染、状态展示、拖放操作和进度反馈。

---

## 三、技术栈与依赖

### 1. 运行时栈

- Rust 2024 edition
- `eframe`：桌面 GUI 框架，基于 egui
- `zip`：ZIP 解压与读取
- `unrar-rs`：RAR 解压支持
- `winreg`：读取 Windows 注册表中的 Steam 安装信息
- `dirs-next`：定位 Documents / AppData 目录
- `tempfile`：临时目录与事务工作区
- `serde` / `serde_json`：JSON 备份清单和设置存储
- `rfd`：文件选择对话框（虽然在当前代码里看起来不是主路径）

### 2. 设计目的

这个项目显然是“把 Windows 原生桌面工具做成可分发、零依赖、单文件”的 Rust 实现，适合用于：

- 轻量部署
- 纯本地操作
- 低依赖，不依赖复杂运行环境

---

## 四、核心功能模块

### 1. `destination_candidates()`：自动寻找 TTS Mods 路径

`lib.rs` 中的 `destination_candidates()` 会：

- 向 Documents 目录查找：
  - `My Games/Tabletop Simulator/Mods`
- 读取 Windows 注册表中的 Steam 路径
- 查找 Steam 库中 `Tabletop Simulator` 的典型安装位置
- 生成多个可候选目录列表

这样用户几乎无须手动设置，程序可直接推荐最常见的 TTS 路径。

---

### 2. `inspect_archive()`：识别压缩包结构

这个函数会解析 ZIP 并寻找已知的 TTS 目录锚点，例如：

- `Images`
- `Models`
- `Workshop`
- `Audio`
- `Text`
- `Video`
- `PDF`
- `Assetbundles`

它的关键逻辑是：

- 只接受包含标准 TTS 内容目录的压缩包；
- 识别最可能的“根目录层”；
- 如果有多个模糊根目录，拒绝导入，避免误导用户；
- 对 `../`、绝对路径、非法路径组件做安全防护。

这部分非常关键，因为它直接决定了导入是否安全可靠。

---

### 3. `safe_components()` 与路径审查

`lib.rs` 中有 `safe_components()` 和 `is_tts_folder()`，用于：

- 拒绝路径穿越（path traversal）
- 拒绝绝对路径 / 根路径 / 前缀路径
- 只允许 TTS 标准目录路径结构进入导入流程

这是该项目最重要的安全设计之一。

---

### 4. `import_zip_with_progress()`：导入事务与回滚

这是核心业务流程：

1. 先检查压缩包是否包含可识别 TTS 内容；
2. 解压到临时 staging 目录；
3. 遍历导入的每个文件；
4. 如目标存在，先备份旧文件；
5. 将新文件写入目标目录；
6. 保存 `manifest.json`，记录：
   - `destination`
   - `created`
   - `replaced`
7. 生成最新备份，便于 `undo_last_import()` 回滚。

也就是说，这里不是简单“复制文件”，而是一个事务式导入系统。

---

### 5. `undo_last_import()`：可撤销恢复

程序会保存最新一次成功导入的备份，撤销逻辑：

- 读取 `manifest.json`
- 删除新增文件
- 用备份恢复被覆盖的文件
- 删除回滚目录

这使得导入操作具备“可撤销、可逆”的用户体验。

---

### 6. RAR 支持

`rar_as_safe_zip()` 和 `import_archive_with_progress()` 提供了 RAR 支持：

- 对 `.rar` 文件，不直接把它交给不安全的处理方式；
- 先读取 RAR 元数据
- 识别是否加密：如果是加密的文件列表，则直接拒绝
- 把 RAR 先提取到临时目录，再转换成“安全的 ZIP 结构”进行统一处理

这样它能用统一的 ZIP 导入逻辑处理两种压缩包格式。

---

## 五、UI 架构

### 1. 主窗口入口：`main.rs`

`main.rs` 里定义了：

- `UiMode`：Ready / Working / Success / Error
- `State`：保存当前路径、候选目录、正在导入的压缩包、进度等状态
- `TtsApp`：egui 应用主结构

它使用 `eframe` 来渲染主窗口，并以“暖色、中性风格”的 UI 设计覆盖一层。

### 2. 交互方式

用户可以：

- 拖动压缩包到窗口
- 手动选择 `Mods` 目录
- 选择已发现的候选目录
- 观看导入进度和阶段描述
- 执行回滚或确认导入成功

### 3. 进度系统

`ImportStage` 定义了多个阶段：

- `InspectingArchive`
- `ExtractingRar`
- `PreparingRarFiles`
- `ExtractingArchive`
- `InstallingFiles`
- `Finishing`

UI 通过这些阶段把真实导入过程，转换成平滑的进度条体验，而不是只显示“卡住”的状态。这样更容易让用户理解当前工作正在做什么。

---

## 六、关键设计特点

### 1. 安全优先

这是最明显的特点：

- 路径穿越检查
- 禁止非法绝对路径
- 只接受标准 TTS 目录
- 对 RAR 的加密 / 符号链接 / 硬链接做检查
- 导入前备份，成功后可回滚

这不是泛泛而谈的安全处理，而是导入“外部压缩包”时最关键的防御策略。

### 2. 事务式导入

导入不是“盲目覆盖”，而是：

- 先备份
- 再覆盖
- 记录清单
- 支持撤销

这点很重要，因为 TTS 资源经常是模型、图像和 Workshop 内容，误覆盖会导致难以恢复。

### 3. 轻量部署

从 `Cargo.toml` 和 `package.ps1` 可以看到，项目设计了：

- 静态链接
- LTO
- 优化压缩
- 移除符号
- 单文件输出

最终目标是 "dist\TTS小工具.exe" 这种最简单分发方式。

---

## 七、构建与打包

`README.md` 中明确说明：

```powershell
cargo test
.\package.ps1
```

构建过程的关键点：

- `build.rs` 使用 `embed-resource` 把 `tts-tool.rc` 编译成资源；
- `package.ps1` 调用 `mt.exe` 把 `app.manifest` 注入 PE；
- `app.manifest` 启用 Common Controls 6.0 与 PerMonitorV2 DPI 感知；
- `.cargo/config.toml` 开启 `crt-static`，使 EXE 不依赖运行时 DLL；
- 发布模式中开启 LTO、Z 级优化、panic abort、符号剥离。

这说明项目非常重视“单文件分发”的体验，尤其适合给普通用户使用。

---

## 八、测试情况

已实际执行：

```powershell
cargo test --quiet
```

结果：

- 6 个单元测试通过
- 3 个测试通过
- 0 失败

测试覆盖了：

- 包装目录识别
- 路径穿越拒绝
- 大文件解压进度更新
- 回滚记录有效性检查
- 撤销备份消费
- 清理应用状态

这说明这是一个“已形成稳定核心机制”的项目，而不是启动阶段原型。

---

## 九、项目价值总结

这个项目最核心的价值在于：

1. 解决了 TTS 图包导入的实际操作问题；
2. 把“脆弱的文件解压与覆盖”做成了事务式流程；
3. 通过安全路径检查和回滚机制显著降低误操作风险；
4. 用 Rust + egui 做成一个轻量桌面工具，且能做成单文件发布；
5. 代码组织清晰，控制器和逻辑分离明确，便于后续扩展。

---

## 十、总体判断

这个项目并不是一个大型系统，而是一个“精巧的单功能桌面工具”。它的设计重点不是广泛抽象，而是：

- 安全
- 清晰
- 可回滚
- 单文件分发
- 低门槛使用

如果你要继续开发它，最值得扩展的方向是：

- 支持更多的压缩包类型
- 增加拖放校验提示
- 加强自动发现 TTS 安装目录的覆盖逻辑
- 增加“批量导入”或“选择多个包”的支持
- 把状态与日志做成更完整的诊断界面

---

## 十一、关键结论

如果用一句话概括：

“这是一个用 Rust 编写、专门处理 TTS 图包导入与回滚的 Windows 桌面工具，设计风格偏安全型、轻量型、单文件分发型。”
