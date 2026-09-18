"""Render first-review Chinese previews without changing TTS source assets.

The production renderer will reuse the same approach for every card: preserve
artwork/card geometry, rebuild only the text panel, then typeset CJK text with
the approved glossary.
"""

from __future__ import annotations

from pathlib import Path
from typing import Iterable

from PIL import Image, ImageDraw, ImageFont


PROJECT = Path(__file__).resolve().parent
OUT = PROJECT / "previews"
TTS_IMAGES = Path(r"D:\Documents\My Games\Tabletop Simulator\Mods\Images")
TITLE_FONT = PROJECT / "fonts" / "ZCOOLQingKeHuangYou-Regular.ttf"
BODY_FONT = Path(r"C:\Windows\Fonts\NotoSansSC-VF.ttf")


def image_for_hash(asset_hash: str) -> Path:
    matches = list(TTS_IMAGES.glob(f"*{asset_hash}*"))
    if not matches:
        raise FileNotFoundError(f"TTS image cache does not contain {asset_hash}")
    return matches[0]


def font(size: int, *, display: bool = False) -> ImageFont.FreeTypeFont:
    selected = TITLE_FONT if display else BODY_FONT
    result = ImageFont.truetype(selected, size=size, index=0)
    if not display:
        result.set_variation_by_name(b"Medium")
    return result


def center_text(
    draw: ImageDraw.ImageDraw,
    box: tuple[int, int, int, int],
    text: str,
    fnt,
    fill,
    *,
    spacing: int = 4,
    stroke_width: int = 0,
) -> None:
    left, top, right, bottom = box
    bounds = draw.multiline_textbbox(
        (0, 0), text, font=fnt, spacing=spacing, align="center", stroke_width=stroke_width
    )
    x = left + (right - left - (bounds[2] - bounds[0])) / 2
    y = top + (bottom - top - (bounds[3] - bounds[1])) / 2
    draw.multiline_text(
        (x, y),
        text,
        font=fnt,
        fill=fill,
        spacing=spacing,
        align="center",
        stroke_width=stroke_width,
        stroke_fill=fill,
    )


def noise_sensor() -> Image.Image:
    image = Image.open(image_for_hash("AC7CD3AB3FF454C779A80B2AC396418BC8F420EA")).convert("RGBA")
    draw = ImageDraw.Draw(image)
    panel = [(85, 535), (916, 535), (954, 573), (954, 1000), (44, 1000), (44, 573)]
    draw.polygon(panel, fill=(250, 249, 247, 255), outline=(15, 15, 15, 255), width=4)
    center_text(draw, (75, 548, 925, 635), "噪声传感器", font(48, display=True), (12, 12, 12, 255))
    draw.line((110, 645, 888, 645), fill=(218, 160, 55, 255), width=4)
    center_text(
        draw,
        (82, 660, 918, 892),
        "第 1、2、3 轮：将数值最低的筹码\n翻至锁定（黑色）面。\n\n锁定筹码不能从其他玩家处取走，\n也不能放回中央。",
        font(30),
        (16, 16, 16, 255),
        spacing=6,
    )
    center_text(draw, (82, 904, 918, 980), "* 9 或 10 人游戏时，0 号筹码也需翻面。*", font(24), (35, 35, 35, 255))
    return image


def lubricant() -> Image.Image:
    atlas = Image.open(image_for_hash("3A1EA5DABD6C920A96ABA3218ED968E2B3932BCB")).convert("RGBA")
    image = atlas.crop((0, 0, 925, 1281))
    draw = ImageDraw.Draw(image)
    draw.rectangle((24, 595, 900, 1260), fill=(222, 222, 221, 255), outline=(35, 35, 35, 255), width=3)
    center_text(draw, (55, 612, 870, 700), "润滑剂", font(53, display=True), (13, 13, 13, 255))
    draw.line((92, 712, 833, 712), fill=(205, 147, 37, 255), width=5)
    center_text(draw, (62, 730, 862, 850), "获得此牌后立刻：", font(30), (18, 18, 18, 255))
    center_text(
        draw,
        (55, 848, 870, 1104),
        "将此牌放在你面前。\n本局剩余时间内（非劫案期间），\n你可以翻看被遮挡的筹码。",
        font(29),
        (20, 20, 20, 255),
        spacing=7,
    )
    center_text(draw, (55, 1110, 870, 1245), "* 仅可翻至普通面；筹码数值不受影响。*", font(24), (40, 40, 40, 255))
    return image


def coordinator() -> Image.Image:
    atlas = Image.open(image_for_hash("0CA7B287BEDA6CF82B8DC0C083762EC6F2486DCE")).convert("RGBA")
    image = atlas.crop((0, 0, 1350, 1853))
    draw = ImageDraw.Draw(image)
    draw.rectangle((20, 1265, 1328, 1835), fill=(250, 250, 248, 255), outline=(25, 25, 25, 255), width=4)
    center_text(draw, (75, 1282, 1270, 1392), "协调员", font(72, display=True), (15, 15, 15, 255))
    draw.line((150, 1410, 1190, 1410), fill=(144, 29, 35, 255), width=4)
    center_text(draw, (95, 1432, 1250, 1522), "发放底牌后立刻：", font(40), (20, 20, 20, 255))
    center_text(draw, (85, 1530, 1260, 1795), "每位玩家同时将自己的一张底牌\n传给左手边的玩家。", font(44), (20, 20, 20, 255), spacing=12)
    return image


def options_panel() -> Image.Image:
    image = Image.open(image_for_hash("2EE0627E7E4AA2160C49198036536AE7233F46DC")).convert("RGBA")
    draw = ImageDraw.Draw(image)
    draw.rectangle((288, 184, 720, 242), fill=(11, 18, 53, 255))
    center_text(draw, (295, 184, 713, 242), "选项面板", font(42, display=True), (200, 37, 43, 255))
    return image


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    previews: Iterable[tuple[str, Image.Image]] = (
        ("challenge_noise_sensors_zh.png", noise_sensor()),
        ("tool_lubricant_zh.png", lubricant()),
        ("specialist_coordinator_zh.png", coordinator()),
        ("options_panel_zh.png", options_panel()),
    )
    for filename, image in previews:
        image.save(OUT / filename, "PNG", optimize=True)
        print(OUT / filename)


if __name__ == "__main__":
    main()
