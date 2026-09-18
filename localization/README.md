# THE GANG 简体中文本地化（首轮校对包）

本目录是对 Workshop 存档 `3689375010.json` 的非破坏性本地化工作区。

- `translation_glossary.md`：首轮术语与代表卡译文，供校对确认。
- `manifest.json`：素材、对象 GUID、源 URL、交付文件和处理状态的可编辑清单。
- `render_preview.py`：从本机 TTS 缓存的原图确定性生成四张中文预览图；不会改写原始缓存或 Workshop JSON。
- `fonts/`：标题使用 OFL 开源的 ZCOOL QingKe HuangYou；正文使用系统的思源黑体（Noto Sans SC）Medium 字重。
- `previews/`：运行脚本后生成的预览图。

本轮仅制作术语与预览，确认后才渲染全量卡牌、规则书和最终存档。

## 运行

```powershell
& 'C:\Users\Administrator\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe' .\localization\render_preview.py
```

源图从 `D:\Documents\My Games\Tabletop Simulator\Mods\Images` 读取，输出只写入本项目的 `localization\previews`。
