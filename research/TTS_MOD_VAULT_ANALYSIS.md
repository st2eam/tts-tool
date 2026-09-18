# TTS Mod Vault 深度剖析

## 1. 分析范围与版本

- 仓库：<https://github.com/markomijic/TTS-Mod-Vault>
- 本地副本：`D:\code\tts-tool\research\TTS-Mod-Vault`
- 分析提交：`ba4f677e471bfb23ccc236be1717b4d53dcf73d3`
- 提交时间：2026-06-30
- 技术栈：Flutter / Dart，版本 `2.1.0+1`
- 目标平台：Windows、Linux、macOS

本文重点分析实现逻辑，不复制或修改上游源码。

## 2. 项目定位

TTS Mod Vault 不是单纯的 ZIP 解压器，而是一个围绕 Tabletop Simulator 本地 Mods/Saves 目录构建的资源管理器。它将一个 TTS 图包建模为：

```text
TTS JSON / Save
        │
        ├── 资源 URL 与资源类型
        ├── 本地资源存在性
        ├── 备份状态
        ├── URL 有效性
        └── 更新时间与缓存元数据
```

核心能力包括：

- Mods、Saves、Saved Objects 的扫描和索引；
- 图片、模型、AssetBundle、音频、PDF、视频、文本等资源统计；
- `.ttsmod` 备份创建、导入、恢复；
- Steam Workshop ID 下载和更新；
- 缺失资源下载与缓存；
- 旧 Steam CDN URL 替换；
- 失效 URL 检查；
- 孤儿资源清理；
- 批量备份、下载、更新、删除和导入；
- 搜索、过滤、排序、图片/PDF/音频查看；
- Windows、Linux、macOS 桌面打包。

## 3. 目录与模块结构

```text
lib/
├── main.dart                         启动、Hive、窗口、PDFium 初始化
└── src/
    ├── app.dart                      MaterialApp、路由、主题
    ├── splash/                       启动扫描、设置目录
    ├── mods/                         图包列表、详情、批量交互
    ├── backups/                      备份列表、筛选、网格/列表视图
    ├── state/
    │   ├── asset/                    本地资源索引
    │   ├── backup/                   备份、恢复、备份缓存
    │   ├── bulk_actions/             批量操作状态机
    │   ├── cleanup/                  孤儿资源清理
    │   ├── delete_assets/            指定资源删除
    │   ├── directories/              Mods/Saves/资源目录解析
    │   ├── download/                 下载、重试、取消、URL 检查
    │   ├── mods/                     JSON 扫描、资源提取、修改
    │   ├── settings/                 设置与音频策略
    │   ├── storage/                  Hive 持久化
    │   └── sort_and_filter/          搜索、筛选、排序
    ├── models/                       URL 替换预设等模型
    ├── utils.dart                    URL、平台文件操作、Steam 工具
    └── utils/
        ├── zip_central_directory_reader.dart
        └── zip_asset_counter.dart
```

项目把 UI、Riverpod 状态、领域操作和持久化分开，但 `utils.dart`、`provider.dart` 和若干大型 Notifier 仍承载较多横切职责。

## 4. 启动与状态管理

### 4.1 启动流程

`lib/main.dart` 的启动顺序大致为：

1. `WidgetsFlutterBinding.ensureInitialized()`；
2. 初始化 `window_manager`；
3. 初始化 Hive；
4. macOS 配置 PDFium 动态库路径；
5. 初始化 `pdfrx_engine`；
6. 设置桌面窗口最小尺寸并显示；
7. 使用 `ProviderScope` 启动 Flutter 应用。

`SplashPage` 随后负责：

1. 读取设置；
2. 初始化目录；
3. 初始化备份缓存；
4. 可选执行 GitHub 版本检查；
5. 扫描 Mods、Saves 和 Saved Objects；
6. 如果目录缺失，显示目录选择界面；
7. 加载完成后进入 Vault。

### 4.2 Riverpod Provider 分层

关键 Provider 位于 `lib/src/state/provider.dart`：

- `modsProvider`：主图包索引；
- `downloadProvider`：下载与 URL 检查；
- `backupProvider`：备份创建；
- `cleanupProvider`：孤儿资源清理；
- `bulkActionsProvider`：批量任务；
- `deleteAssetsProvider`：资源删除；
- `directoriesProvider`：目录设置；
- `settingsProvider`：应用设置；
- `storageProvider`：Hive 存储服务；
- `existingAssetListsProvider`：本地文件存在性索引；
- `filteredModsProvider` / `filteredBackupsProvider`：派生筛选结果；
- `actionInProgressProvider`：聚合所有进行中的任务，统一限制 UI 操作。

这种设计的优点是操作状态与 UI 解耦，下载、备份、清理都能显示进度和取消状态。缺点是全局 Provider 数量较多，跨模块操作容易集中到大型 Notifier 中。

## 5. 图包扫描与资源模型

### 5.1 Mod 模型

`lib/src/state/mods/mod_model.dart` 中的 `Mod` 是主要领域对象，包含：

- Mod 类型：普通 Mod、Save、Saved Object；
- JSON 路径与父目录；
- 显示名称；
- 保存时间；
- `AssetLists` 类型资源集合；
- 总资源数和已存在资源数；
- 音频显示策略；
- 备份状态和备份元数据；
- 无效 URL 状态。

### 5.2 资源类型

资源类型由 `AssetTypeEnum` 及其子类型集中定义。资源字段不是散落在 UI 中，而是通过类型枚举统一管理，这使得：

- URL 提取；
- 本地文件扫描；
- 资源统计；
- 下载；
- 备份；
- 清理；

可以共享同一套类型边界。

### 5.3 扫描性能

大型目录扫描使用 isolate：

- `mods.dart` 根据 CPU 数量拆分批次；
- `mods_isolates.dart` 在后台解析 JSON、构造 Mod；
- 本地资源先建立文件名到路径的索引；
- 资源存在性判断使用 O(1) Map 查找，而不是对每个 URL 重新递归扫描；
- Hive 数据使用批量读取和批量写入。

这是 TTS Mod Vault 相比简单“每次点击都扫描 Mods”实现的核心性能优势。

## 6. JSON 资源引用解析

### 6.1 为什么没有完全依赖 JSON 反序列化

TTS 文件可能很大，且部分资源引用会出现在 Lua 或转义后的嵌套 JSON 中。因此项目采用面向 URL 的正则提取，而不是将任意 TTS JSON 映射为完整 Dart 类型。

关键实现位于：

- `lib/src/state/mods/mods_isolates.dart`
- `extractUrlsFromJson()`
- `extractUrlsFromJsonString()`
- `_extractUrlsWithRegex()`
- `buildAssetListsFromUrls()`

流程为：

```text
文件内容
  ↓
URL 正则提取
  ↓
过滤 file:/ 与换行转义
  ↓
旧 CDN 前缀归一化
  ↓
按 AssetTypeEnum 分类
  ↓
与本地文件名索引比对
  ↓
AssetLists + 总数 + 已存在数
```

### 6.2 URL 兼容处理

项目集中处理旧版与新版 Steam CDN：

- 旧：`http://cloud-3.steamusercontent.com/`
- 新：`https://steamusercontent-a.akamaihd.net/`

备份、清理、本地存在性判断会兼容两个前缀对应的文件名，避免旧图包因 URL 迁移而被误判为缺失或孤儿。

### 6.3 优点

- 支持标准 JSON 和 Lua 中的转义 JSON；
- 支持多种 URL scheme；
- 资源类型通过统一枚举扩展；
- 不需要为完整 TTS JSON 建立脆弱的类型模型；
- 通过 isolate 和本地索引适合大型 Mods 库。

### 6.4 局限

- 正则不是结构化解析，可能漏掉新字段或特殊转义；
- malformed JSON 可能产生部分成功结果；
- 新增 TTS 资源字段需要更新枚举；
- 部分异常在批次处理中被记录后跳过，用户可能只看到“扫描完成”而不知道存在不完整结果。

## 7. `.ttsmod` 备份格式

### 7.1 创建备份

`lib/src/state/backup/backup.dart` 的 `BackupNotifier.createBackup()`：

1. 选择或读取备份目录；
2. 解析当前 Mod 的所有资源 URL；
3. 为每种资源类型定位本地文件；
4. 添加 Mod JSON；
5. 添加可选 PNG 预览图；
6. 使用 isolate 创建 ZIP；
7. 将路径保存为相对 Mods/Saves 父目录的路径。

典型 ZIP 结构类似：

```text
Mods/Workshop/example.json
Mods/Images/...
Mods/Models/...
Mods/Audio/...
Saves/Workshop/...
```

这样备份不仅保存 JSON，还保留 TTS 标准目录层级，便于跨机器恢复。

### 7.2 文件匹配策略

备份会根据当前 Steam CDN 文件名和旧 `cloud-3` 文件名查找本地资源。找不到的资源不会让整个备份失败，而是跳过该文件。

优点是部分资源损坏时仍能得到可用备份；缺点是最终 ZIP 可能不完整，成功状态没有强制要求用户确认“缺失多少资源”。

### 7.3 备份导入

`lib/src/state/backup/import_backup.dart` 的导入流程：

1. 解码 ZIP；
2. 查找 `Workshop` 路径下的 JSON；
3. 解析 TTS 保存时间；
4. 根据显式目标、已有 Mod 或 ZIP 内路径决定恢复位置；
5. 推断 Mod、Save 或 Saved Object 类型；
6. 检查本地 JSON 是否更新；
7. 如有冲突，通过 `JsonConflictChoice` 请求用户选择；
8. 恢复 JSON 和资源；
9. 更新本地资源索引；
10. 刷新共享资源关联的其他 Mod。

导入器还支持直接导入 JSON/PNG。

## 8. 备份缓存与 ZIP 中央目录

项目没有每次都完整解压或读取所有备份内容，而是：

- 使用 `backup_cache.dart` 保存备份元数据；
- 使用 `existing_backups.dart` 发现已有备份；
- 使用 `zip_central_directory_reader.dart` 仅读取 ZIP 中央目录；
- 使用 `zip_asset_counter.dart` 根据路径统计资源数量。

这对大量备份文件非常重要：列表页可以先展示名称、时间和资源统计，而不必读取每个 ZIP 的全部内容。

## 9. 下载、缓存与 URL 修复

### 9.1 下载状态机

`lib/src/state/download/download.dart` 的 `DownloadNotifier` 管理：

- 当前下载任务；
- 每种资源类型的进度；
- Dio `CancelToken`；
- 已存在文件跳过；
- 下载后的 Mod 刷新；
- 共享资源依赖刷新。

### 9.2 重试策略

`_downloadUrlWithRetry()` 对以下错误重试：

- 连接超时；
- 接收超时；
- 网络连接错误；
- HTTP 5xx。

重试间隔为 500ms、1000ms、2000ms。取消和 HTTP 4xx 不重试。

这比直接 `GET` 一次的实现更适合 Workshop CDN，但仍属于固定策略，没有按响应头或服务端限流动态调整。

### 9.3 下载后的校验

下载逻辑还包含：

- 临时文件；
- 响应内容检查；
- 图片解码；
- BSON/Steam 内容处理；
- 扩展名与资源类型处理；
- 下载完成后移动到目标目录。

### 9.4 URL 替换

项目提供：

- 单个资源 URL 修改；
- 批量 URL 替换；
- 前缀升级；
- URL 替换预设；
- 无效 URL 检查；
- Steam Workshop 更新。

关键入口包括：

- `updateModAsset()`
- `_replaceUrlInJsonFile()`
- `updateUrlPrefixes()`
- `checkUrlsAllMods()`
- `updateModsAll()`

## 10. Workshop 更新与应用更新

TTS Mod Vault 有两类更新：

### 10.1 Workshop 内容更新

通过 Steam `GetPublishedFileDetails` API 查询 Workshop 信息，构造 Workshop 链接，并在 UI 中提供普通更新和强制更新选项。

### 10.2 TTS Mod Vault 自身更新

启动时可查询 GitHub Releases API：

1. 读取当前应用版本；
2. 查询 GitHub 最新 Release；
3. 比较版本；
4. 显示下载对话框。

它是“通知并下载”，不是静默自更新器。

## 11. 清理与删除

### 11.1 孤儿资源清理

`lib/src/state/cleanup/cleanup.dart`：

1. 加载所有已知 Mod；
2. 读取 Hive 中缓存的 URL 映射；
3. 生成大小写不敏感的引用文件名集合；
4. 并行扫描标准资源目录和 Raw 目录；
5. 标记未被引用的文件；
6. 将旧 CDN 前缀重复文件纳入候选；
7. 用户确认后执行删除。

### 11.2 风险

清理依赖缓存 URL 映射。如果缓存过期、丢失或扫描不完整，文件可能被错误标记为孤儿。删除阶段会继续处理其他文件，但失败文件没有形成结构化报告。

## 12. 批量操作与筛选

`bulk_actions.dart` 统一编排：

- 全部下载；
- 全部备份；
- 下载并备份；
- 删除资源；
- 检查 URL；
- 更新 URL 前缀；
- 更新 Mods；
- 导入备份；
- 取消任务。

多选状态通过 `multiModsProvider` 保存，批量操作状态包含进度、结果和取消动作。

筛选维度包括：

- 名称；
- 文件夹；
- 备份状态；
- 资源是否完整；
- 是否包含音频、图片、模型、PDF、AssetBundle。

当前全局搜索主要匹配 `saveName`，资源 URL 搜索更多是选中 Mod 后的局部功能。

## 13. 持久化

Hive 使用三个主要 Box：

- `ModUrls`：每个 Mod 的 URL 映射；
- `ModMetadata`：时间戳、音频偏好等；
- `AppData`：目录和应用设置。

项目还实现批量读写：

- `getModUrlsBulk()`
- `getAllModUrls()`
- `getAllModDateTimeStamps()`
- `getAllModAudioPreferences()`
- `saveAllModUrlsData()`
- `saveAllModMetadata()`

当 JSON 被删除后，`pruneOrphanedModData()` 会清理对应缓存。

优点是避免把所有派生数据都重新计算；缺点是 Hive URL 数据使用较弱的动态类型，缓存损坏或结构漂移时编译期保护不足。

## 14. UI 与平台实现

### 14.1 UI

主要页面：

- `SplashPage`：启动和目录选择；
- `Vault`：主容器与侧边导航；
- `ModsPage`：图包列表；
- `SelectedModView`：单个图包详情与操作；
- `MultiSelectView`：批量选择；
- `BackupsPage`：备份浏览；
- `ImagesViewerPage`：图片查看。

图包支持网格/列表展示，备份支持筛选和排序，资源可直接打开图片、音频和 PDF。

### 14.2 平台

目录默认值：

- Windows：`Documents/My Games/Tabletop Simulator`；
- macOS：`Library/Tabletop Simulator`；
- Linux：优先 Steam Snap 路径，否则 `~/.local/share/Tabletop Simulator`。

文件选择使用 `file_picker`，文件打开使用 `open_filex`，链接使用 `url_launcher`。桌面窗口使用 `window_manager`。

macOS 构建额外处理 PDFium 路径；发布文档说明 macOS 包未签名，可能需要移除 quarantine 属性。

## 15. 错误处理与取消

### 做得较好的部分

- 下载可取消；
- 批量任务有取消入口；
- 长任务放到 isolate；
- 操作状态有加载、确认、完成和错误阶段；
- 备份导入有 JSON 新旧冲突选择；
- UI 有进度与结果对话框。

### 需要改进的部分

- 若干模块使用宽泛的 `catch (Object)` 或 `catch (Exception)`；
- 部分错误只 `debugPrint`，没有结构化传递到 UI；
- 导入 JSON/备份失败时不总能形成清晰错误状态；
- 备份添加文件失败时跳过，但成功结果可能没有突出“不完整”；
- 清理失败文件没有完整错误清单；
- URL 正则部分失败可能呈现为正常但不完整的扫描结果；
- 没有持久化错误日志或遥测。

## 16. 测试与验证现状

仓库没有发现针对核心业务的 `test/` 测试套件。虽然 `pubspec.yaml` 声明了 `flutter_test`，但当前仓库没有配套的业务单元测试。

尝试执行：

```powershell
Set-Location 'D:\code\tts-tool\research\TTS-Mod-Vault'
flutter analyze
```

结果：当前环境没有安装或没有配置 Flutter SDK，命令不可用。因此本文的实现结论来自源码分析，未能完成 Flutter 静态检查和运行时测试。

## 17. 与当前 `tts-tool` 的对比

| 能力 | TTS Mod Vault | 当前 `tts-tool` |
|---|---|---|
| 技术栈 | Flutter/Dart，跨平台 | Rust/egui，Windows 单文件 |
| 主要定位 | Mods/Saves 资源管理器 | ZIP/RAR 安全导入工具 |
| JSON 资源解析 | URL 正则 + 类型枚举 + 本地索引 | 当前已加入资源引用索引与 manifest |
| 导入 | `.ttsmod` / ZIP 备份恢复、JSON 导入 | ZIP/RAR 导入，事务与回滚 |
| 导出 | `.ttsmod` 备份，保留标准目录 | ZIP + `tts-tool-manifest.json` |
| 远程资源 | 下载、缓存、重试、失效检查 | 尚未下载远程资源 |
| Workshop | ID 下载、更新 | 尚未实现 |
| 清理 | 孤儿资源扫描和删除 | 尚未实现 |
| 批量操作 | 完整 | 尚未实现 |
| UI | 功能丰富、跨平台、暗色主题 | Windows 原生感、轻量、详情和封面 |
| 安全导入 | 重点不在不可信归档安全边界 | ZIP/RAR 路径遍历、链接检查、事务回滚 |
| 测试 | 核心业务测试较少 | Rust 核心已有单元测试 |

## 18. 对 `tts-tool` 的可复用设计

### P0：资源模型从“文件名集合”升级为“类型化引用图”

TTS Mod Vault 最值得吸收的是 `AssetTypeEnum → AssetLists → 本地存在性 Map` 链路。当前 `tts-tool` 已经从全目录同名匹配升级为资源 manifest，但下一步应：

1. 为每类资源定义字段/扩展名集合；
2. 记录 JSON 字段路径和原始引用；
3. 区分本地路径、远程 URL、GUID/Workshop 引用；
4. 对同名冲突保留候选列表；
5. 将未解析项作为用户可见结果。

### P1：完善 `.ttsmod` 互操作

当前 `tts-tool` 的 ZIP 导出已保留相对路径并附带 manifest，可以进一步兼容 TTS Mod Vault 的 `.ttsmod` 约定：

- 导出时使用标准 `Mods/Workshop`、`Mods/Images` 等路径；
- 导入时支持识别带 `Mods/` 或 `Saves/` 前缀的备份；
- 对 JSON 新旧时间戳提供覆盖/跳过/另存选择；
- 导入后刷新共享资源索引。

### P1：引入后台资源索引

TTS Mod Vault 通过 isolate 和缓存避免 UI 卡顿。Rust 版本可以采用：

- 后台线程执行目录扫描；
- `Arc` 快照替换 UI 数据；
- 按目录修改时间或文件大小缓存；
- 只在 Mods 目录变化后重建索引；
- 封面和详情使用独立缓存。

### P1：引入批量操作框架

可将导出、重新扫描、校验、清理设计为统一任务：

```text
任务计划 → 后台执行 → 进度事件 → 可取消 → 结果报告
```

不要把长任务直接放进 egui 绘制函数。

### P2：下载和缓存

远程下载功能应排在本地导入/导出稳定性之后。若实现，建议复用：

- 临时文件下载；
- 内容类型和文件大小检查；
- 超时与 5xx 重试；
- 取消；
- 原子移动；
- SHA-256 校验；
- 失败资源清单。

### P2：孤儿资源清理

清理必须基于明确资源引用图，而不是只依赖文件名。执行删除前要展示：

- 被判定为孤儿的文件；
- 最近修改时间；
- 文件大小；
- 是否存在同名引用；
- 可恢复备份位置。

## 19. 最终评价

TTS Mod Vault 的核心价值不只是功能多，而是它把 TTS 图包从“一个 JSON 文件”提升为“带资源依赖、缓存状态、备份状态和更新状态的实体”。它最值得当前项目借鉴的部分是：

1. 类型化资源索引；
2. 本地文件存在性缓存；
3. 标准目录结构的备份互操作；
4. 下载、校验、重试、取消的任务模型；
5. 批量操作与结果反馈；
6. ZIP 中央目录与缓存带来的大库性能。

同时不应直接照搬其不足：

- 正则解析缺少结构化保证；
- 核心业务测试不足；
- 某些异常被静默跳过；
- 备份完整性没有始终显式呈现；
- 清理依赖缓存，存在误判风险。

对 `tts-tool` 的推荐路线是：先继续强化本地资源闭包、`.ttsmod` 互操作和后台索引，再考虑远程下载与 Workshop 更新。这样可以保持当前 Rust Windows 工具在安全导入、单文件分发和可回滚方面的优势，同时吸收 TTS Mod Vault 的资源管理能力。

