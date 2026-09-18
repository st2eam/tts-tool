# Tabletop Simulator 本地模组汉化通用指南

适用于已经下载到本机的 TTS Workshop 模组。目标是在不破坏原模组、不上传素材的前提下，制作仅供本机游玩的中文副本。

## 一、先建立副本，不改原文件

典型目录：

```text
Documents\My Games\Tabletop Simulator\Mods\Workshop\        # Workshop JSON
Documents\My Games\Tabletop Simulator\Mods\Images\          # 图片缓存
Documents\My Games\Tabletop Simulator\Saves\                # 本地存档
```

操作原则：

1. 保留原 Workshop JSON，作为随时可回滚的基线。
2. 新建一个独立 JSON 副本，例如 `模组ID_zh.json`。
3. 新建独立中文素材目录，例如 `Mods\Images\MyModZH\`。
4. 需要在 TTS 中快速加载时，也可在 `Saves` 下保留同一份中文 JSON。

不要修改原始 Workshop JSON，也不要把新素材混入原图片缓存文件名。

## 二、先盘点，再翻译

从 JSON 中收集以下字段：

- `CustomImage.ImageURL` / `ImageSecondaryURL`
- `CustomDeck.FaceURL` / `BackURL`
- `CustomPDF.PDFUrl`
- `CustomMesh.MeshURL` / `ColliderURL`
- `Nickname`、`Description`
- `LuaScript`、`XmlUI`

建立翻译清单，至少记录：源 URL、对象 GUID、类型、英文原文、中文译文、输出文件名、JSON 字段位置。

通常优先处理玩家实际会看到的内容：卡牌、玩家提示板、规则速查、物件名、按钮、广播文字。音乐封面、音频和纯装饰素材可按项目范围决定是否保留原样。

规则书的封面通常可直接复用原版，特别是在其主要承担品牌识别而非规则阅读时；只翻译内页规则、图注和说明即可。这样可减少重绘工作，也能避免改变品牌标识。

## 三、图片汉化规则

### 独立卡面

1. 保持原始宽高、透明通道、插画、边框和色彩。
2. 只清理英文文字区域，再排入中文。
3. 中文需自动换行、缩放，并保留安全边距。
4. 输出 PNG；不要随意改变文件格式。

### 卡牌合图（Deck Atlas）

合图最容易损坏 TTS 的卡牌映射。必须保持：

- 原图像素尺寸不变；
- `NumWidth` 与 `NumHeight` 不变；
- 每张卡仍位于原网格位置；
- `CardID` 不变。

只能在对应格子内改文字。移动、裁切、重排或重新导出为不同尺寸，都会使 TTS 发出错误卡面。

### 字体建议

为保证中文易读，标题和正文宜分别选字重：

- 标题：较窄的展示中文字体；
- 正文：常规或 Medium 字重的无衬线中文字体。

先制作一张卡、一张合图卡和一个面板作为预览。确认字重、字号、行距、语气后，再批量渲染。

## 四、JSON 修改安全边界

通常可以修改：

- 图片、PDF 的资源路径；
- 玩家可见的 `Nickname`、`Description`；
- Lua 中仅用于显示的文字；
- XML UI 中的标签。

通常不要修改：

- GUID；
- `CardID`、牌组网格；
- Lua 函数名、变量名、状态键；
- 脚本用作内部识别的字符串；
- 对象层级、位置和物理参数（除非任务明确要求）。

若不确定某个英文字符串是否被脚本逻辑使用，先搜索它在 Lua 中的引用；不要直接批量替换。

## 五、本地图片路径的正确格式

TTS 在 Windows 的本地资源应使用**原生 Windows 路径**，不要写成 `file:///` URI。

正确示例：

```json
"ImageURL":"D:\\Documents\\My Games\\Tabletop Simulator\\Mods\\Images\\MyModZH\\card_01_zh.png"
```

错误示例：

```text
file:///D:/Documents/My%20Games/Tabletop%20Simulator/Mods/Images/MyModZH/card_01_zh.png
```

最可靠的验证方式：在 TTS 中用“Browse / 浏览”选择本地文件并选择 **Local / 本地**，观察 TTS 写入的路径格式。

本地资源只适合单机或同机热座；其他玩家无法访问你的磁盘路径。多人联机应改用 Steam Cloud 或经授权的公共资源托管。

## 六、构建与安装流程

1. 备份原 JSON。
2. 渲染中文 PNG 到工作目录。
3. 用图片库验证所有 PNG 可打开、尺寸与源图一致。
4. 复制中文 PNG 到独立目录 `Mods\Images\MyModZH\`。
5. 生成中文 JSON；将资源引用改为原生 Windows 路径。
6. 写入新的 Workshop 副本和可选的本地 Save 副本。
7. 完全退出 TTS 后重新打开，再加载中文 JSON。

不要在已经打开的旧房间中测试刚改过的路径；资源可能仍被会话缓存。

## 七、验证清单

- JSON 能解析。
- 输出对象数量与原版一致。
- GUID、CardID、牌组网格与 Lua 逻辑未改变。
- 每个本地路径指向存在的文件。
- 所有 PNG 都能解码，且尺寸、透明度符合源图。
- TTS 可加载和重载存档。
- 测试洗牌、发牌、抽卡、翻面、脚本按钮、广播和规则书。
- 检查中文没有截断、重叠、错字或字号过小。

## 八、常见故障排查

| 现象 | 优先检查 |
| --- | --- |
| 图片加载失败 | 路径是否为 Windows 原生路径；文件是否存在；TTS 是否已完全重启 |
| 卡面错位 | 合图尺寸、网格和 `CardID` 是否被改动 |
| 部分卡仍为英文 | 是否存在桌面独立对象；检查该对象的 `CustomImage.ImageURL`，不只检查牌堆 |
| 原 Steam 图片失败 | Mod Caching 是否启用；本机缓存是否仍保留该资源；网络是否可访问 |
| 脚本报错 | 是否改动了内部字符串、GUID、函数名或状态字段 |
| 其他玩家看不到中文图 | 本地路径只在你的电脑有效；需使用共享资源托管 |

## 九、回滚

删除中文副本 JSON 和独立中文素材目录即可：

```text
Mods\Workshop\<模组ID>_zh.json
Saves\<模组名>_中文版.json
Mods\Images\MyModZH\
```

原 Workshop JSON 和原始缓存素材不要删除。
