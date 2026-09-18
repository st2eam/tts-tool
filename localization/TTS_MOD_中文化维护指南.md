# THE GANG TTS 本地中文化维护指南

本文记录当前 `THE GANG` 中文化副本的制作、安装和维护方式。目标是只在本机使用中文素材，不覆盖 Steam Workshop 原模组。

## 1. 当前文件与目录

| 用途 | 路径 |
| --- | --- |
| Workshop 原版（只读保留） | `D:\Documents\My Games\Tabletop Simulator\Mods\Workshop\3689375010.json` |
| 中文 Workshop 副本 | `D:\Documents\My Games\Tabletop Simulator\Mods\Workshop\3689375010_zh.json` |
| 中文本地存档副本 | `D:\Documents\My Games\Tabletop Simulator\Saves\THE_GANG_中文版.json` |
| 中文图片素材 | `D:\Documents\My Games\Tabletop Simulator\Mods\Images\TheGangZH\` |
| 可维护源文件与脚本 | `D:\projects\tts-tool\localization\` |

中文图片目录包含 31 张 PNG：28 张挑战卡、工具卡合图、专家卡合图和选项面板各一张。

## 2. 修改原则

1. **绝不覆盖原版 Workshop JSON。** 所有改动只写入 `3689375010_zh.json` 和中文存档。
2. **保持 TTS 结构不变。** GUID、`CardID`、牌组 `NumWidth` / `NumHeight`、Lua 函数、变量与状态键均不可改名。
3. **卡牌合图不可改变尺寸或网格。** 工具卡为 5 x 3，专家卡为 5 x 2；只重绘每格中的文字区域。
4. **动态界面优先使用脚本既有的 `zh-CN` 文案。** 当前模组 Lua 已包含简体中文按钮和广播文本，无须改动游戏逻辑。
5. **音乐、音频与纯装饰素材不翻译。** 原链接保持原样；中文化只处理游玩相关卡牌、标签与面板。

## 3. TTS 本地资源路径：最重要的注意事项

TTS 的本地资源不应使用 `file:///D:/...` URI。该形式在当前 TTS 环境中会造成加载失败。

应使用 Windows 原生路径，例如：

```text
D:\Documents\My Games\Tabletop Simulator\Mods\Images\TheGangZH\options_panel_zh.png
```

也就是说，JSON 中的 `ImageURL`、`FaceURL`、`BackURL` 等字段应保存为双反斜杠转义后的 Windows 路径：

```json
"ImageURL":"D:\\Documents\\My Games\\Tabletop Simulator\\Mods\\Images\\TheGangZH\\options_panel_zh.png"
```

在 TTS 内通过“Browse / 浏览”导入文件并选择 **Local / 本地** 时，TTS 也会生成这种原生路径格式。

## 4. 图片制作流程

### 字体规范

- 标题：`ZCOOL QingKe HuangYou`（清刻黄油体）
- 正文：`Noto Sans SC Medium`
- 标题不要额外加粗；正文使用 Medium，不使用 Regular 或 Bold。

这样能接近原卡窄而有冲击力的标题感，同时避免正文过细或过黑。

### 渲染脚本

| 脚本 | 作用 |
| --- | --- |
| `render_preview.py` | 生成代表性预览与通用排版函数 |
| `render_challenges.py` | 生成 28 张独立挑战卡 |
| `render_deck_atlases.py` | 生成工具卡与专家卡合图 |
| `build_localized_save.py` | 生成最终中文 JSON，并替换本地资源引用 |

输出路径：`localization\output\assets\`。

重绘文字时，必须保持：原分辨率、透明通道、插画、边框、颜色、卡片顺序和合图网格不变。仅覆盖原英文文字面板并进行中文排版。

## 5. 更新 JSON 的流程

1. 修改翻译或渲染脚本。
2. 重新生成 PNG：

```powershell
& '<Python 路径>' localization\render_challenges.py
& '<Python 路径>' localization\render_deck_atlases.py
```

3. 将生成的 PNG 复制到 `Mods\Images\TheGangZH\`，保持文件名不变。
4. 执行：

```powershell
& '<Python 路径>' localization\build_localized_save.py
```

5. 用输出 JSON 覆盖两份**中文副本**：

```text
Mods\Workshop\3689375010_zh.json
Saves\THE_GANG_中文版.json
```

6. 完全退出并重新打开 TTS，再加载中文副本。不要复用已经加载的旧房间。

## 6. 已知专项修复

“通风井”在桌面上是一个独立挑战对象（GUID：`ef03e2`），它不会自动沿用牌堆中的卡面引用。因此在构建脚本中为它显式指定：

```text
TheGangZH\challenges\challenge_5401cbc94d08f4430f21c0bd45950dc72dc801b89_zh.png
```

若未来发现“对象名称已经中文、卡面仍英文”，应检查该对象的 `CustomImage.ImageURL`，而不只是检查牌堆的 `CustomDeck`。

## 7. 验证清单

- JSON 可解析。
- 顶层对象数与原版一致：266。
- GUID、CardID、牌组网格和 Lua 逻辑未变。
- 31 张中文 PNG 均存在、可正常解码。
- 中文资源路径不含 `file:///`。
- 加载后抽取挑战、工具和专家卡，确认中文卡面显示；重点检查“通风井”。
- 若原 Steam 资源仍报错，确认 TTS 的 Mod Caching 已启用，并检查 `Mods\Images` 中是否存在对应缓存文件。

## 8. 当前范围与后续工作

当前已完成卡牌、选项面板、可见对象名称和本地资源路径的中文化。原版 Lua 已含 `zh-CN` 动态文本。

Deluxe 规则书的中文 PDF 尚未替换；在完成逐页翻译、排版和视觉校验之前，保留原规则书链接，避免把未经校对的文件装入模组。规则书封面直接复用原版，不重绘或汉化品牌封面；仅处理内页规则与说明文字。

## 9. 回滚与卸载

删除以下新增文件即可回到原版：

```text
Mods\Workshop\3689375010_zh.json
Saves\THE_GANG_中文版.json
Mods\Images\TheGangZH\
```

不要删除 `Mods\Workshop\3689375010.json`，它是原始 Workshop 缓存。
