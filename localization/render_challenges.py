"""Render the gameplay challenge cards with the approved Chinese type system."""

from __future__ import annotations

import json
from pathlib import Path

from PIL import Image, ImageDraw

from render_preview import PROJECT, center_text, font, image_for_hash


OUT = PROJECT / "output" / "assets" / "challenges"

# hash, Chinese title, Chinese body.  Hashes are also the stable source IDs
# recorded in the original TTS JSON URLs.
CARDS = {
    "635B0FB7C43FB3B5A931399B1A0E0D5D9411F861": (
        "万能钥匙",
        "将一张“万能”牌加入牌库！\n\n此牌对你的手牌而言是最理想的牌。\n（可能产生重复牌）",
    ),
    "29DF1920381479E5D207DA5C914F42D2EDD1322E": (
        "替身小丑",
        "将两张“替身小丑”加入牌库。\n\n若玩家抽到其中一张作为底牌：抽两张，保留一张，弃掉另一张。\n\n若抽到公共牌中：抽两张，每名玩家只能使用其中一张。",
    ),
    "9326C73EE45C2B0071FE7E66F4D1A874B240E1C3": (
        "均衡",
        "第 1 轮：每位玩家少发一张底牌。\n\n第 2 轮：每位玩家额外发一张底牌。",
    ),
    "DC231252E6DE911796A5705C697D2A81F14826CD": (
        "精密锁",
        "本局剩余时间内，获胜牌型（高牌除外）禁用。\n包含加入此挑战的回合。\n\n这些牌型将依序跳过，任何人不得使用。",
    ),
    "4F412532BB634B0AA76EC90A7C01C7558BB08DB4": (
        "倒序推进",
        "第 2、3、4 轮的公共牌数量互换。",
    ),
    "12087C1A0A0C20D3C68C882E7798AE411E551D7F": (
        "花色过多",
        "将两张“全花色 0”加入牌库！\n\n它们视为拥有全部花色，但牌点为 0。\n\n* 0 可以彼此组成对子，但不能参与顺子；既不能作高端也不能作低端。*",
    ),
    "AE74A8BF50CA639B1B61ACC4E43BF41B0B5DC470": (
        "漫长收尾",
        "第 4 轮额外加入一张公共牌。\n\n若至少两项挑战会增加公共牌，启用“小奥马哈”规则。\n\n* 至少必须使用一张手牌。*",
    ),
    "0E40C63D4C38BC2139E7B66D4A61CCC5C16CE63B": (
        "加班时刻",
        "每种花色各加入三张额外牌：B < C < D。\n这些牌点高于 A，延伸每种花色的高端。\n\n皇家同花顺改为 K < A < B < C < D。\nA 仍可用于低端顺子。",
    ),
    "FF6080A7C537C060FB99B32CC9A52168E96888FC": (
        "杂乱工具箱",
        "劫案开始时，随机一个牌点被“抹除”。\n\n该牌点的牌点视为 0，且不属于任何花色。\n\n* 0 可以彼此组成对子，但不能参与顺子；既不能作高端也不能作低端。*",
    ),
    "AC2D39C728F21C140F3512EE4CBB6BBC53A0D0D4": (
        "不速之客",
        "第 3 轮额外加入一张公共牌。\n\n若至少两项挑战会增加公共牌，启用“小奥马哈”规则。\n\n* 至少必须使用一张手牌。*",
    ),
    "3AFD0012A4B536AF63D74F6066AD9EC1AA860AF3": (
        "粗暴开局",
        "第 2 轮额外加入一张公共牌。\n\n若至少两项挑战会增加公共牌，启用“小奥马哈”规则。\n\n* 至少必须使用一张手牌。*",
    ),
    "94014B47929A5D42AA95FDEEB9209FC64AD055EA": (
        "门缝一脚",
        "第 1 轮额外加入一张公共牌。\n\n若至少两项挑战会增加公共牌，启用“小奥马哈”规则。\n\n* 至少必须使用一张手牌。*",
    ),
    "25472E7AC9A0C23EA9798917DA4C9F243AB6AF63": (
        "齿轮磨损",
        "将第五种花色“齿轮”加入牌库！\n\n它与其他花色完全相同，含完整的 2 至 A 牌组。\n\n* 新增牌型：“彩虹”“彩虹顺子”。\n* 彩虹 > 两对；彩虹顺子 > 四条。",
    ),
    "A06CA057EB3782513A8372BDF3722B75D1111845": (
        "全力突击",
        "你必须打出手中点数最高的一张牌，\n即使会使牌型变弱。\n\n此限制只针对一张牌；可以多出，但至少要出一张。",
    ),
    "3AF738F0F51FCF19F9240058067081DDA5C69D55": (
        "量子混沌",
        "牌点顺序颠倒！\n2 > 3 > 4 > 5 > 6 > 7 > 8 > 9 > 10 > J > Q > K > A\n\n仍要组成尽可能强的牌型。顺子按其中最小牌决定；同花由五张最小牌组成。\n\n皇家同花顺为“5 + 4 + 3 + 2 + A”。",
    ),
    "6E0FF7FFE40633237FC21800571A827C6408A38E": (
        "沉睡守卫",
        "你必须打出手中点数最低的一张牌，\n即使会使牌型变弱。\n\n此限制只针对一张牌；可以多出，但至少要出一张。",
    ),
    "5401CBC94D08F4430F21C0BD45950DC72DC801B89": (
        "通风井",
        "第 1、2、3 轮：将数值最高的筹码翻至锁定（黑色）面。\n\n锁定筹码不能从其他玩家处取走，也不能放回中央。",
    ),
    "B1D76F9A5DDB58586EA31524F8A98C3E77C39E07": (
        "指纹扫描",
        "第 4 轮摊牌时，在最高筹码玩家亮出手牌前：\n其他玩家必须共同猜测该玩家的牌型。\n\n* 猜错即失败。*",
    ),
    "B8EC08CCA12A4ABB7F6661C54A8B68F7A68E4F4A": (
        "激光绊线",
        "第 2 轮：若新翻开的公共牌中没有 J、Q 或 K，\n白色筹码数值最高的玩家必须弃掉手牌，\n并从牌库重新抽取一组底牌。",
    ),
    "958A17D60C292CC9977AB5B585773A3EEB9B4907": (
        "运动探测器",
        "第 2 轮：若新翻开的公共牌中至少有一张 J、Q 或 K，\n白色筹码数值最低的玩家必须弃掉手牌，\n并从牌库重新抽取一组底牌。",
    ),
    "231615D69ED11F9F816F8E5859F821F286D4DDC6": (
        "视网膜扫描",
        "第 4 轮摊牌时，在最高筹码玩家亮出手牌前：\n其他玩家必须共同猜测该玩家底牌中\n一张牌的牌点。\n\n* 猜错即失败。*",
    ),
    "40CB2D6EE0F7FC2BF4733404DEA4049378B286A0": (
        "监控摄像头",
        "第 1 轮：每位玩家额外获得一张底牌。",
    ),
    "F8349B875D8392AFF37CAA1AB43B42C8A1144AAD": (
        "停电",
        "每个第 2、3、4 轮开始时：\n所有玩家弃掉自己的筹码。",
    ),
    "8F6BFB2FA5D9864D2764B59D2E8E48F4CE0833CC": (
        "仓促撤离",
        "第 3 轮不发放橙色筹码！\n第 2 轮后，翻开河牌公共牌，直接进入第 4 轮。",
    ),
    "9F998ACA82CF05E80C703E9BCC6830B23BB9FB69": (
        "快速通道",
        "第 1 轮不发放白色筹码！\n发完底牌后，翻开翻牌公共牌，直接进入第 2 轮。",
    ),
    "57FBD682AB3906C7DE8E555202F295AB3C84CD541": (
        "不看花色",
        "同花牌型禁用！\n包括普通同花、同花顺和皇家同花顺。\n\n这些牌型不存在，结算时直接跳过。",
    ),
    "B9937B44DBCA0CE11D11FD0433A0C3F776121887": (
        "快速执行",
        "第 2 轮不发放黄色筹码！\n第 1 轮后，翻开转牌公共牌，直接进入第 3 轮。",
    ),
    "AC7CD3AB3FF454C779A80B2AC396418BC8F420EA": (
        "噪声传感器",
        "第 1、2、3 轮：将数值最低的筹码翻至锁定（黑色）面。\n\n锁定筹码不能从其他玩家处取走，也不能放回中央。\n\n* 9 或 10 人游戏时，0 号筹码也需翻面。*",
    ),
}


def body_font(text: str):
    # Chinese needs more horizontal room than the original condensed Latin font.
    for size in range(32, 17, -1):
        candidate = font(size)
        probe = ImageDraw.Draw(Image.new("RGBA", (998, 1000)))
        box = probe.multiline_textbbox((0, 0), text, font=candidate, spacing=size // 4, align="center")
        if box[2] - box[0] <= 815 and box[3] - box[1] <= 290:
            return candidate, size // 4
    return font(17), 4


def render(asset_hash: str, title: str, body: str) -> Image.Image:
    image = Image.open(image_for_hash(asset_hash)).convert("RGBA")
    draw = ImageDraw.Draw(image)
    panel = [(85, 535), (916, 535), (954, 573), (954, 1000), (44, 1000), (44, 573)]
    draw.polygon(panel, fill=(250, 249, 247, 255), outline=(15, 15, 15, 255), width=4)
    center_text(draw, (75, 548, 925, 635), title, font(52, display=True), (12, 12, 12, 255))
    draw.line((110, 645, 888, 645), fill=(218, 160, 55, 255), width=4)
    selected_font, spacing = body_font(body)
    center_text(draw, (74, 660, 924, 972), body, selected_font, (18, 18, 18, 255), spacing=spacing)
    return image


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    url_map = {}
    for asset_hash, (title, body) in CARDS.items():
        filename = f"challenge_{asset_hash.lower()}_zh.png"
        output = OUT / filename
        render(asset_hash, title, body).save(output, "PNG", optimize=True)
        url_map[asset_hash] = output.as_posix()
        print(output)
    (OUT.parent / "challenge_asset_map.json").write_text(
        json.dumps(url_map, ensure_ascii=False, indent=2), encoding="utf-8"
    )


if __name__ == "__main__":
    main()
