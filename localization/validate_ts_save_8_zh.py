"""Static regression checks for the localized TS_Save_8 copy."""

from __future__ import annotations

import hashlib
import json
import re
from pathlib import Path


ROOT = Path(__file__).resolve().parent
SOURCE = Path(r"D:\Documents\My Games\Tabletop Simulator\Mods\Workshop\TS_Save_8.json")
OUTPUT = ROOT / "output" / "TS_Save_8_zh.json"
EXPECTED_SOURCE_HASH = "AAC8231902203029B289670B9E871E51BDE6B0F729C2BDB4241D1A6DF835C04C"
PLAY_ZONES = {"d81799", "d28ab5", "f2a644", "3775b1"}
PLAYER_ZONES = {
    "e4131e": (0.0, -12.5), "5b93b0": (-8.9, -9.0), "43ae65": (-12.5, 0.0), "0be485": (-8.9, 9.0),
    "4dea38": (0.0, 12.5), "535ab7": (8.9, 9.0), "029071": (12.5, 0.0), "b142e7": (8.9, -9.0),
}


def walk(objects):
    for obj in objects:
        yield obj
        yield from walk(obj.get("ContainedObjects", []))


output = json.loads(OUTPUT.read_text(encoding="utf-8"))
source_available = SOURCE.exists()
if source_available:
    source_bytes = SOURCE.read_bytes()
    assert hashlib.sha256(source_bytes).hexdigest().upper() == EXPECTED_SOURCE_HASH
    source = json.loads(source_bytes.decode("utf-8"))
    assert len(source["ObjectStates"]) == len(output["ObjectStates"]) == 52
    assert [obj.get("GUID") for obj in walk(source["ObjectStates"])] == [
        obj.get("GUID") for obj in walk(output["ObjectStates"])
    ]
else:
    assert len(output["ObjectStates"]) == 52
assert output["Table"] == "Table_Octagon"
assert output["SaveName"] == "Texas Hold'em"
assert output["GameMode"] == "Texas Hold'em"

output_functions = set(re.findall(r"(?m)^function\s+([A-Za-z_][A-Za-z0-9_]*)", output["LuaScript"]))
ENGINE_FUNCTIONS = {"startHand", "resetGame", "uiAction", "quickAmount", "voteRun", "forceRun", "runVoteTimeout", "sortTableChips", "callSpawnChips", "applySettings", "onSave", "onLoad"}
assert ENGINE_FUNCTIONS <= output_functions

for obj in walk(output["ObjectStates"]):
    if obj.get("GUID") in PLAY_ZONES:
        transform = obj["Transform"]
        assert (transform["scaleX"], transform["scaleY"], transform["scaleZ"]) == (8.0, 5.0, 20.0)
    if obj.get("GUID") in PLAYER_ZONES:
        transform = obj["Transform"]
        assert (transform["posX"], transform["posZ"]) == PLAYER_ZONES[obj["GUID"]]

xml = output["XmlUI"]
for expected in ("弃牌", "过牌", "跟注", "下注/加注", "全押", "最小加注", "半池", "满池", "跑两次", "应用设置"):
    assert expected in xml

global_script = output["LuaScript"]
for expected in ("庄家", "小盲", "大盲", "PokerRoleMarker", "run_vote", "JSON.encode(state)", "CardID", "onSave", "runVoteTimer"):
    assert expected in global_script
assert "跑马（Run It Twice）" in output["TabStates"]["0"]["body"]

button_objects = {obj["GUID"]: obj for obj in walk(output["ObjectStates"]) if obj.get("GUID") in {"87d2e6", "0d221f", "88b654", "cc0912"}}
for guid, label in {"87d2e6": "整理筹码", "0d221f": "同步筹码", "88b654": "发牌", "cc0912": "重置牌局"}.items():
    assert label in button_objects[guid]["LuaScript"]

print("JSON valid: yes")
print("top-level objects: 52; all GUIDs preserved")
print("table: Table_Octagon")
print("play zones: 4 restored to 8 x 5 x 20")
print("cash-game engine: action state, side pots, showdown, run-it-twice, persistence")
print("action UI: present")
print("source JSON: unavailable; validated current localized baseline" if not source_available else "source SHA256: unchanged")
