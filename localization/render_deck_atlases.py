"""Render the two TTS card atlases without changing their grid geometry.

Every replacement stays in the original cell, so the save's CardID values and
NumWidth/NumHeight definitions remain valid.
"""

from __future__ import annotations

import json
from pathlib import Path

from PIL import Image, ImageDraw

from render_preview import PROJECT, center_text, font, image_for_hash


OUT = PROJECT / "output" / "assets"

TOOLS = [
    ("润滑剂", "获得此牌后立刻：", "将此牌放在你面前。\n本局剩余时间内（非劫案期间），\n你可以翻看被遮挡的筹码。", "* 仅可翻至普通面；筹码数值不受影响。*"),
    ("干扰器", "摊牌前任意时刻：", "亮出此牌。若本次劫案失败，\n不要触发警报。", "* 你可弃掉此牌以抽取一张新工具卡，\n但也要抽取一张挑战卡。*"),
    ("撬棍", "摊牌前任意时刻：", "亮出此牌。若本次劫案成功，\n洗劫保险库两次。若失败，照常触发警报，\n并额外抽取一张挑战卡。", "* 你可弃掉此牌以抽取一张新工具卡，\n但也要抽取一张挑战卡。*"),
    ("手电筒", "摊牌前任意时刻：", "将此牌放在你面前。从公共牌堆抽一张牌，\n放到牌堆顶。它算作公共牌，\n但只有你可以使用。", "* 任何涉及底牌的规则都不适用于此牌。*"),
    ("一次性手机", "获得此牌后立刻：", "弃掉此牌。抽两张专家卡作为替代。\n你可以在正常可使用时机使用其中一张。", "* 使用时弃掉另一张。*"),
    ("开锁器", "亮出底牌时：", "亮出此牌。将你一张底牌的牌点\n增加或减少 1。", "* 可循环变化：最低牌点可变为最高牌点，反之亦然。\n可组成“五条”。*"),
    ("无线耳机", "摊牌前任意时刻：", "亮出此牌。将你的一张底牌交给另一名玩家，\n然后该玩家交还给你一张底牌。", "* 可以交还刚刚交换得到的那张牌。*"),
    ("后门钥匙", "第 4 轮、摊牌前：", "亮出此牌。你的筹码视为“撤离筹码”。", "* 摊牌顺序中跳过你不会产生影响。*"),
    ("夜视镜", "摊牌前任意时刻：", "将此牌放在你面前。将你的一张底牌\n正面朝上放在此牌上。", "* 亮出的牌仍算作底牌。*"),
    ("烟雾弹", "亮出底牌时：", "亮出此牌。将至多两张底牌的花色\n改为任意可用花色。", "* 两张牌必须改为同一种花色。*"),
    ("魔法 8 号球", "摊牌前任意时刻：", "亮出此牌。获知你的手牌当前所在位置\n对应的正确数字筹码。", "* 不计入其他工具卡（如开锁器、烟雾弹）的影响；\n仅限数字版。*"),
]

SPECIALISTS = [
    ("协调员", "发放底牌后立刻：", "每位玩家同时将自己的一张底牌\n传给左手边的玩家。"),
    ("线人", "", "其中一名玩家向另一名玩家展示\n自己的一张底牌。"),
    ("主谋", "", "选择一个牌点（2 至 A）。其中一名玩家告诉所有人：\n大家各自拥有多少张该牌点的底牌。"),
    ("黑客", "", "其中一名玩家从牌库额外抽取一张底牌，\n然后将自己的一张底牌背面朝上放入弃牌堆。"),
    ("逃脱车手", "", "其中一名玩家告诉所有人：\n自己当前手牌属于哪一种牌型\n（从高牌至皇家同花顺）。"),
    ("打手", "", "其中一名玩家获得以下能力：\n你的手牌胜过所有与其牌型相同的手牌。"),
    ("杰克", "", "其中一名玩家将此牌加入自己的底牌，\n再将自己另一张底牌背面朝上放入弃牌堆。\n此杰克仍视为 J，但不属于任何花色。"),
    ("数学奇才", "发放底牌后立刻：", "每位玩家告诉所有人自己底牌牌点之和。\n2 至 10 的数值为 2 至 10；J、Q、K 为 10；A 为 11。"),
    ("骗子", "发放底牌后立刻：", "所有人看过底牌后，将每位玩家的底牌\n背面朝上混洗，再重新发放。"),
    ("投资人", "发放底牌后立刻：", "每位玩家告诉所有人：\n自己底牌中有多少张“人头牌”（J、Q、K）。"),
]


def cell_bounds(image: Image.Image, col: int, row: int, cols: int, rows: int) -> tuple[int, int, int, int]:
    return (
        round(col * image.width / cols),
        round(row * image.height / rows),
        round((col + 1) * image.width / cols),
        round((row + 1) * image.height / rows),
    )


def render_tools() -> Path:
    image = Image.open(image_for_hash("3A1EA5DABD6C920A96ABA3218ED968E2B3932BCB")).convert("RGBA")
    for index, (title, lead, body, note) in enumerate(TOOLS):
        left, top, right, bottom = cell_bounds(image, index % 5, index // 5, 5, 3)
        width, height = right - left, bottom - top
        draw = ImageDraw.Draw(image)
        draw.rectangle((left + 24, top + round(height * .465), right - 25, bottom - 20), fill=(222, 222, 221, 255), outline=(35, 35, 35, 255), width=3)
        center_text(draw, (left + 55, top + 610, right - 55, top + 700), title, font(53, display=True), (13, 13, 13, 255))
        draw.line((left + 92, top + 712, right - 92, top + 712), fill=(205, 147, 37, 255), width=5)
        center_text(draw, (left + 55, top + 730, right - 55, top + 842), lead, font(29), (18, 18, 18, 255))
        center_text(draw, (left + 52, top + 842, right - 52, bottom - 166), body, font(27), (20, 20, 20, 255), spacing=6)
        center_text(draw, (left + 52, bottom - 166, right - 52, bottom - 28), note, font(20), (40, 40, 40, 255), spacing=4)
    output = OUT / "tools_zh.png"
    image.save(output, "PNG", optimize=True)
    return output


def render_specialists() -> Path:
    image = Image.open(image_for_hash("0CA7B287BEDA6CF82B8DC0C083762EC6F2486DCE")).convert("RGBA")
    for index, (title, lead, body) in enumerate(SPECIALISTS):
        left, top, right, bottom = cell_bounds(image, index % 5, index // 5, 5, 2)
        draw = ImageDraw.Draw(image)
        draw.rectangle((left + 20, top + 1265, right - 22, bottom - 18), fill=(250, 250, 248, 255), outline=(25, 25, 25, 255), width=4)
        center_text(draw, (left + 75, top + 1282, right - 80, top + 1392), title, font(72, display=True), (15, 15, 15, 255))
        draw.line((left + 150, top + 1410, right - 160, top + 1410), fill=(144, 29, 35, 255), width=4)
        center_text(draw, (left + 90, top + 1425, right - 90, top + 1522), lead, font(36), (20, 20, 20, 255))
        center_text(draw, (left + 78, top + 1520, right - 78, bottom - 48), body, font(36), (20, 20, 20, 255), spacing=10)
    output = OUT / "specialists_zh.png"
    image.save(output, "PNG", optimize=True)
    return output


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    outputs = [render_tools(), render_specialists()]
    (OUT / "deck_atlas_translations.json").write_text(
        json.dumps({"tools": TOOLS, "specialists": SPECIALISTS}, ensure_ascii=False, indent=2), encoding="utf-8"
    )
    print("\n".join(str(path) for path in outputs))


if __name__ == "__main__":
    main()
