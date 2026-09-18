"""Create the standalone Chinese TTS save without touching Workshop files."""

from __future__ import annotations

import json
import re
from pathlib import Path

from render_challenges import CARDS


PROJECT = Path(__file__).resolve().parent
SOURCE = PROJECT / "source" / "3689375010.original.json"
OUTPUT = PROJECT / "output" / "THE_GANG_中文版.json"
LOCAL_ROOT = Path(r"D:\Documents\My Games\Tabletop Simulator\Mods\Images\TheGangZH")
MODS_ROOT = LOCAL_ROOT.parents[1]


NAMES = {
    "Rulebook": "规则书", "Wild Joker": "万能小丑", "Wild Joker Challenge": "万能小丑挑战",
    "Master Key": "万能钥匙", "Replace Joker": "替换小丑", "Replacement Joker": "替身小丑",
    "Gear Deck": "齿轮牌库", "Gear Extra": "额外齿轮", "Standard EXTRA": "标准额外牌",
    "Standard ZERO": "标准 0", "Gear ZERO": "齿轮 0", "Vaults": "保险库",
    "Never Played Challenges": "未使用挑战", "Acknowledged Challenges": "已确认挑战",
    "Balance": "均衡", "Intricate Lock": "精密锁", "Reverse Run": "倒序推进",
    "Too Many Colors": "花色过多", "Lengthy Finish": "漫长收尾", "Extra Hours": "加班时刻",
    "Cluttered Toolbox": "杂乱工具箱", "Uninvited Guest": "不速之客", "Rough Kickoff": "粗暴开局",
    "A Foot In The Door": "门缝一脚", "Grinding Gears": "齿轮磨损", "Special Occasion Challenges": "特殊场合挑战",
    "All Out Attack": "全力突击", "Quantum Chaos": "量子混沌", "Sleeping Guard": "沉睡守卫",
    "Hand Rank Reference": "牌型速查", "Info Reference": "信息速查", "Challenges": "挑战",
    "Ventilation Shaft": "通风井", "Noise Sensor": "噪声传感器", "Fingerprint Scan": "指纹扫描",
    "Laser Tripwires": "激光绊线", "Motion Detector": "运动探测器", "Retina Scan": "视网膜扫描",
    "Security Cameras": "监控摄像头", "Airpods": "无线耳机", "Burner Phone": "一次性手机",
    "Jamming Device": "干扰器", "Crowbar": "撬棍", "Smoke Grenade": "烟雾弹", "Flashlight": "手电筒",
    "Night Vision Goggles": "夜视镜", "Magic 8 Ball": "魔法 8 号球", "Backdoor Key": "后门钥匙",
    "Lock Pick": "开锁器", "All Suit 0s": "全花色 0", "Con Artiist": "骗子", "Blackout": "停电",
    "Hasty Getaway": "仓促撤离", "Quick Access": "快速通道", "We  Don't See Color": "不看花色",
    "Quick Execution": "快速执行", "Lubricant": "润滑剂", "Math Wiz": "数学奇才",
    "Mastermind": "主谋", "Investor": "投资人", "Getaway  Driver": "逃脱车手", "Hacker": "黑客",
    "Informant": "线人", "Jack": "杰克", "Coordinator": "协调员", "Muscle": "打手",
    "ValueChart": "牌点表", "Dealer": "庄家", "CommunityTilesFlop": "翻牌公共牌",
    "CommunityTilesRiver": "河牌公共牌", "CommunityTilesTurn": "转牌公共牌",
    "communityFlopTile": "翻牌公共牌", "communityTurnTile": "转牌公共牌", "communityRiverTile": "河牌公共牌",
    "VaultButton": "保险库按钮", "OrangeHR": "橙色牌型", "WhiteHR": "白色牌型", "RedHR": "红色牌型",
    "YellowHRMain": "黄色牌型", "PurpleHR": "紫色牌型", "BlueHR": "蓝色牌型", "GreenHR": "绿色牌型",
    "PinkHR": "粉色牌型", "Sound Manager": "声音管理器", "Portable Platform": "便携平台",
}


def local_path(relative: str) -> str:
    # TTS writes a Windows path (not file:///) when the user chooses the
    # official "Local" import option.  This form is what its loader accepts.
    return str(LOCAL_ROOT / relative)


CHALLENGE_URIS = {
    source_hash.lower(): local_path(f"challenges/challenge_{source_hash.lower()}_zh.png")
    for source_hash in CARDS
}

CACHE_FILES = [
    item for folder in ("Images", "Models", "Assetbundles", "Audio", "PDF")
    for item in (MODS_ROOT / folder).glob("*") if item.is_file()
]


def cached_path(url: str) -> str | None:
    normalized = re.sub(r"[^a-z0-9]", "", url.lower())
    for item in CACHE_FILES:
        if item.name.lower().startswith(normalized):
            return str(item)
    return None


def replace_value(value):
    if isinstance(value, dict):
        return {key: replace_value(item) for key, item in value.items()}
    if isinstance(value, list):
        return [replace_value(item) for item in value]
    if not isinstance(value, str):
        return value
    lower = value.lower()
    for source_hash, uri in CHALLENGE_URIS.items():
        if source_hash in lower:
            return uri
    if "3a1ea5dabd6c920a96aba3218ed968e2b3932bcb" in lower:
        return local_path("tools_zh.png")
    if "0ca7b287beda6cf82b8dc0c083762ec6f2486dce" in lower:
        return local_path("specialists_zh.png")
    if "2ee0627e7e4aa2160c49198036536ae7233f46dc" in lower:
        return local_path("options_panel_zh.png")
    if value.startswith(("http://", "https://")):
        return cached_path(value) or value
    return value


def translate_names(value):
    if isinstance(value, dict):
        result = {key: translate_names(item) for key, item in value.items()}
        if result.get("Nickname") in NAMES:
            result["Nickname"] = NAMES[result["Nickname"]]
        return result
    if isinstance(value, list):
        return [translate_names(item) for item in value]
    return value


def main() -> None:
    data = json.loads(SOURCE.read_text(encoding="utf-8"))
    data = translate_names(replace_value(data))

    # The physical Hand Rank Reference normally polls Global every two seconds.
    # Expose a public redraw method and call it directly from the Rainbow toggle
    # so the new RB/SR rows appear immediately instead of waiting for polling.
    board_marker = "function checkForChanges()"
    board_refresh = (
        "function refreshRainbowDisplayNow()\n"
        "  lastRainbowState = Global.getVar(\"rainbowActive\") or false\n"
        "  lastLockedSnapshot = buildLockedSnapshot()\n"
        "  lastDeckSnapshot = buildDeckSnapshot()\n"
        "  buildButtons()\n"
        "end\n\n"
    )
    def add_board_refresh(objects):
        for obj in objects:
            if obj.get("GUID") == "b554dc" and board_marker in obj.get("LuaScript", ""):
                obj["LuaScript"] = obj["LuaScript"].replace(board_marker, board_refresh + board_marker, 1)
            add_board_refresh(obj.get("ContainedObjects", []))
    add_board_refresh(data["ObjectStates"])

    toggle_marker = "  updatehandRankObjects() -- recalc immediately so rainbow ranks update now, not on the next 2s tick"
    toggle_replacement = (
        "  updatehandRankObjects() -- recalc immediately so rainbow ranks update now, not on the next 2s tick\n"
        "  local handRankReference = getObjectFromGUID(\"b554dc\")\n"
        "  if handRankReference then handRankReference.call(\"refreshRainbowDisplayNow\") end"
    )
    if toggle_marker in data.get("LuaScript", ""):
        data["LuaScript"] = data["LuaScript"].replace(toggle_marker, toggle_replacement, 1)
    # Loose challenge tiles are separately serialized table objects.  Apply a
    # final pass to every matching face so none can fall back to an English
    # cache image even if its source URL was already converted to a local path.
    def fix_object(objects):
        for obj in objects:
            image = obj.get("CustomImage")
            if image and isinstance(image.get("ImageURL"), str):
                face_url = image["ImageURL"].lower()
                for source_hash, replacement in CHALLENGE_URIS.items():
                    if source_hash in face_url:
                        image["ImageURL"] = replacement
                        break
            fix_object(obj.get("ContainedObjects", []))
    fix_object(data["ObjectStates"])
    data["SaveName"] = "THE GANG 中文版 - 豪华版"
    data["GameMode"] = "[FF8c00]THE GANG [FFFFFF]- [00FF00]10 人 [FFFFFF]- [DDDDDD]合作扑克（中文）"
    data["VersionNumber"] = f"{data.get('VersionNumber', '')} ZH"
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(json.dumps(data, ensure_ascii=False, separators=(",", ":")), encoding="utf-8")
    print(OUTPUT)


if __name__ == "__main__":
    main()
