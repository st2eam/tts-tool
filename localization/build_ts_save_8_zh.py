"""Build a standalone Chinese TS_Save_8 copy without modifying the source."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path


SOURCE = Path(r"D:\Documents\My Games\Tabletop Simulator\Mods\Workshop\TS_Save_8.json")
OUTPUT = Path(__file__).resolve().parent / "output" / "TS_Save_8_zh.json"
BASELINE = Path(__file__).resolve().parent / "baseline" / "TS_Save_8_zh_pre_engine.json"
ENGINE = Path(__file__).resolve().parent / "texas_holdem_engine.lua"
ENGINE_UI = Path(__file__).resolve().parent / "texas_holdem_ui.xml"

PLAY_ZONE_GUIDS = {"d81799", "d28ab5", "f2a644", "3775b1"}

PLAYER_ZONE_GUIDS = ("e4131e", "5b93b0", "43ae65", "0be485", "4dea38", "535ab7", "029071", "b142e7")
ORIGINAL_PLAYER_CENTERS = (
    (0.0, -12.5), (-8.9, -9.0), (-12.5, 0.0), (-8.9, 9.0),
    (0.0, 12.5), (8.9, 9.0), (12.5, 0.0), (8.9, -9.0),
)
LARGE_TABLE_PLAYER_CENTERS = (
    (0.0, -15.0), (-10.6, -10.6), (-15.0, 0.0), (-10.6, 10.6),
    (0.0, 15.0), (10.6, 10.6), (15.0, 0.0), (10.6, -10.6),
)


GLOBAL_REPLACEMENTS = {
    "label='Deal Flop'": "label='发翻牌'",
    "label='Deal Turn'": "label='发转牌'",
    "label='Deal River'": "label='发河牌'",
    "label='Deal Cards'": "label='发牌'",
    "tooltip = 'Burn one card and flip three onto the table'": "tooltip = '烧一张牌，然后在桌面翻开三张公共牌'",
    "tooltip = 'Burn one card and flip the fourth card onto the table'": "tooltip = '烧一张牌，然后翻开第四张公共牌'",
    "tooltip = 'Burn one card and flip the last card onto the table'": "tooltip = '烧一张牌，然后翻开最后一张公共牌'",
    "tooltip = 'Button disabled until next round'": "tooltip = '下一局开始前按钮不可用'",
    "tooltip = 'Deal to cards to all seated players'": "tooltip = '为所有已入座玩家发两张底牌'",
    'counterText = "Sitting Out\\r\\n"': 'counterText = "暂离中\\r\\n"',
    'UI.setValue("sitOutButton", "Rejoin")': 'UI.setValue("sitOutButton", "返回游戏")',
    'UI.setValue("sitOutButton", "Sit Out")': 'UI.setValue("sitOutButton", "暂离")',
    "local int value = 0": "local value = 0",
}


OBJECT_SCRIPT_REPLACEMENTS = {
    "87d2e6": {
        'self.setName("Gather")': 'self.setName("整理筹码")',
        'self.setDescription("Gather Chips")': 'self.setDescription("按账本重新整理桌面筹码")',
        "bParam.label = 'Gather Chips'": "bParam.label = '整理筹码'",
        "bParam.tooltip = 'Collect and group all chips in the play area'": "bParam.tooltip = '按账本重新整理个人筹码与底池'",
    },
    "0d221f": {
        'self.setName("Chips")': 'self.setName("同步筹码")',
        'self.setDescription("Spawn 5000 chips")': 'self.setDescription("由牌局账本同步实体筹码")',
        "bParam.label = 'Spawn Chips'": "bParam.label = '同步筹码'",
        "bParam.tooltip = 'Spawn 5000 chips in the play area'": "bParam.tooltip = '由牌局账本重建实体筹码'",
    },
    "88b654": {
        'self.setName("Play")': 'self.setName("发牌")',
        'self.setDescription("Progress game to the next stage")': 'self.setDescription("推进到下一个发牌阶段")',
        "bParam.label = 'Deal Cards'": "bParam.label = '发牌'",
        "bParam.tooltip = 'Deal to cards to all seated players'": "bParam.tooltip = '为所有已入座玩家发两张底牌'",
    },
    "cc0912": {
        'self.setName("Reset")': 'self.setName("重置牌局")',
        'self.setDescription("Reset game for next hand")': 'self.setDescription("重置牌局并准备下一手牌")',
        "bParam.label = 'Reset Cards'": "bParam.label = '重置牌局'",
        "bParam.tooltip = 'Reset all cards to start a new hand'": "bParam.tooltip = '收回所有牌并开始新一手牌'",
    },
}


XML_REPLACEMENTS = {
    "\t\tFold\r\n": "\t\t弃牌\r\n",
    "\t\tSort Chips\r\n": "\t\t整理筹码\r\n",
    ">Sit Out / Return</Button>": ">暂离 / 返回</Button>",
    ">Pot Total:</Text>": ">底池总额：</Text>",
}


CHIP_FILTER_HELPER = '''-- Build the chip list used by counters and sorting. For the enlarged
-- central area, exclude chips that are simultaneously inside a player's
-- personal chip zone so they are not counted in or gathered into the pot.
function getFilteredChipTable(zoneGUIDs)
    local chipTable = buildObjTable(zoneGUIDs, {"Chip"})
    if zoneGUIDs ~= playZoneGUIDs then
        return chipTable
    end

    local playerChipTable = buildObjTable(colorZoneArray, {"Chip"})
    local playerChipGUIDs = {}
    for _, obj in pairs(playerChipTable) do
        playerChipGUIDs[obj.getGUID()] = true
    end

    local filtered = {}
    for _, obj in pairs(chipTable) do
        if not playerChipGUIDs[obj.getGUID()] then
            table.insert(filtered, obj)
        end
    end
    return filtered
end

'''


BETTING_AREA_BORDER_HELPER = '''-- Create a visible, non-interactive border inside the overlapping central chip zones.
-- 3D text has no physical collision, so it does not interfere with cards or chips.
function createBettingAreaBorder()
    local function borderSegment(text, position, rotation)
        local segment = spawnObject({
            type = "3DText",
            position = position,
            rotation = rotation,
            scale = {x=1.25, y=1.25, z=1.25},
        })
        segment.TextTool.setFontColor({r=1, g=0.82, b=0.25})
        segment.TextTool.setFontSize(48)
        segment.TextTool.setValue(text)
        segment.setName("BettingAreaBorder")
        segment.setLock(true)
    end

    borderSegment("━━━━━━━━ 投注区 ━━━━━━━━", {0, 1.03, -7.5}, {90, 180, 0})
    borderSegment("━━━━━━━━ 投注区 ━━━━━━━━", {0, 1.03, 7.5}, {90, 0, 0})
    borderSegment("━━━━━━━━━━", {-8, 1.03, 0}, {90, 90, 0})
    borderSegment("━━━━━━━━━━", {8, 1.03, 0}, {90, 270, 0})
end

'''


ROLE_MARKER_HELPER = '''-- Visible poker-role markers. They are labels only: chip values are never changed.
dealerSeatIndex = 0

function getActiveSeatIndexes()
    local active = {}
    for index = 1, 8 do
        if colorStatusArray[index] == 1 then
            table.insert(active, index)
        end
    end
    return active
end

function nextActiveSeat(startIndex)
    for offset = 1, 8 do
        local candidate = ((startIndex - 1 + offset) % 8) + 1
        if colorStatusArray[candidate] == 1 then
            return candidate
        end
    end
    return 0
end

function spawnRoleMarker(colorIndex, label, fontColor)
    local center = colorCenterArray[colorIndex]
    local position = {center[1] * 0.86, 1.06, center[3] * 0.86}
    local rotation = {90, (colorIndex + 1) * 45, 90}
    local marker = spawnObject({
        type = "3DText",
        position = position,
        rotation = rotation,
        scale = {x=1.25, y=1.25, z=1.25},
    })
    marker.TextTool.setFontColor(fontColor)
    marker.TextTool.setFontSize(58)
    marker.TextTool.setValue(label)
    marker.setName("PokerRoleMarker")
    marker.setLock(true)
end

function refreshRoleMarkers()
    for _, obj in pairs(getAllObjects()) do
        if obj.name == "PokerRoleMarker" then
            destroyObject(obj)
        end
    end

    local active = getActiveSeatIndexes()
    if #active < 2 then
        return
    end
    if dealerSeatIndex == 0 or colorStatusArray[dealerSeatIndex] ~= 1 then
        dealerSeatIndex = nextActiveSeat(0)
    end

    local smallBlindSeat = 0
    local bigBlindSeat = 0
    if #active == 2 then
        smallBlindSeat = dealerSeatIndex
        bigBlindSeat = nextActiveSeat(dealerSeatIndex)
    else
        smallBlindSeat = nextActiveSeat(dealerSeatIndex)
        bigBlindSeat = nextActiveSeat(smallBlindSeat)
    end

    spawnRoleMarker(dealerSeatIndex, "庄家", {r=1, g=0.82, b=0.25})
    spawnRoleMarker(smallBlindSeat, "小盲", {r=0.35, g=0.85, b=1})
    spawnRoleMarker(bigBlindSeat, "大盲", {r=1, g=0.4, b=0.4})
end

function initializeRoleMarkers()
    if dealerSeatIndex == 0 or colorStatusArray[dealerSeatIndex] ~= 1 then
        dealerSeatIndex = nextActiveSeat(0)
    end
    refreshRoleMarkers()
end

function advanceDealerAndRoles()
    dealerSeatIndex = nextActiveSeat(dealerSeatIndex)
    refreshRoleMarkers()
end

'''


TEST_BOT_HELPER = '''-- Virtual seats exist only to test poker-role rotation without extra Steam players.
testBotSeats = {}

function isRoleActiveSeat(index)
    return colorStatusArray[index] == 1 or testBotSeats[index] == true
end

function setTestBotStatus()
    local names = {}
    for index = 1, 8 do
        if testBotSeats[index] == true then
            table.insert(names, colorNameArray[index])
        end
    end
    local suffix = "无"
    if #names > 0 then
        suffix = table.concat(names, "、")
    end
    UI.setValue("testBotStatus", "测试座位：" .. suffix)
end

function addTestBot(player, value, id)
    for index = 1, 8 do
        if colorStatusArray[index] ~= 1 and testBotSeats[index] ~= true then
            testBotSeats[index] = true
            setTestBotStatus()
            initializeRoleMarkers()
            return
        end
    end
end

function clearTestBots(player, value, id)
    testBotSeats = {}
    dealerSeatIndex = 0
    setTestBotStatus()
    initializeRoleMarkers()
end

'''


TEST_BOT_UI = '''

<!-- [[Role rotation test controls]] -->
<HorizontalLayout width="900" height="80" cellSpacing="5" rectAlignment="LowerLeft" padding="20 20 14 8">
    <Button id="addTestBotButton" fontSize="24" fontStyle="Bold" onClick="Global/addTestBot">添加电脑占位</Button>
    <Button id="clearTestBotButton" fontSize="24" fontStyle="Bold" onClick="Global/clearTestBots">清空电脑占位</Button>
</HorizontalLayout>
<HorizontalLayout width="650" height="80" rectAlignment="LowerLeft" padding="20 20 92 8">
    <Text id="testBotStatus" fontSize="24" color="White" alignment="MiddleLeft">电脑占位：无</Text>
</HorizontalLayout>
'''


RULES_TAB_BODY = '''德州扑克（Texas Hold'em）

每位玩家持有 2 张底牌，配合 5 张公共牌组成最佳 5 张牌型。

流程
1. 庄家顺时针轮换；庄家左手为小盲，再左手为大盲。
2. 发两张底牌后进行翻牌前下注。
3. 依次发翻牌（3 张）、转牌（1 张）、河牌（1 张）；每轮均可下注。
4. 剩余玩家摊牌，最佳五张牌赢得底池；同牌型平分。

两人局
庄家同时为小盲；另一位为大盲。

跑马（Run It Twice）
仅在所有仍在牌局中的玩家均已全下、且没有后续行动时使用，并应在发出剩余公共牌前由相关玩家一致同意。
- 底池平均拆为两份；同一组底牌分别跑两次剩余公共牌。
- 若翻牌后全下：先发第一跑的转牌、河牌，再回到翻牌面发第二跑的转牌、河牌。
- 若转牌后全下：从同一转牌面分别发两张河牌。
- 每一跑独立判定赢家并分配该半池；奇数筹码的归属应在开局前约定。
- 未一致同意时，默认只跑一次。

自动牌局引擎
- 默认买入 10,000；小盲/大盲为 50/100，所有下注以 50 为步长。
- 仅当前行动玩家可弃牌、过牌、下注、跟注、加注或全押。界面会显示需跟注、最小加注、个人下注、剩余筹码及底池。
- 系统自动扣除盲注、记录账本、计算主池/边池、亮牌、比较最佳五张牌并按庄家左手起处理平分余数。
- 可以把实体筹码拖进有边框的投注区；当前行动玩家确认操作后，系统将其作为待确认下注处理。超过本次操作的筹码须先取回。
- 全员全押时，仍在竞争的玩家可投票跑一次或跑两次；20 秒未决默认跑一次，任意已入座玩家可强制决定。
- “整理筹码”与“同步筹码”会按账本重建个人筹码与中央底池；请勿在牌局进行中用它们改变余额。
'''


def replace_required(text: str, replacements: dict[str, str], context: str) -> str:
    for old, new in replacements.items():
        if old not in text:
            raise ValueError(f"Missing expected text in {context}: {old!r}")
        text = text.replace(old, new)
    return text


def replace_or_confirm(text: str, replacements: dict[str, str], context: str) -> str:
    """Apply source replacements, accepting an already-localized copy as input."""
    for old, new in replacements.items():
        if old in text:
            text = text.replace(old, new)
        elif new not in text:
            raise ValueError(f"Missing expected text in {context}: {old!r}")
    return text


def walk_objects(objects: list[dict]):
    for obj in objects:
        yield obj
        yield from walk_objects(obj.get("ContainedObjects", []))


def patch_chip_logic(script: str) -> str:
    if "function getFilteredChipTable(zoneGUIDs)" in script:
        return script
    marker = "-- This function groups the chips in a passed in zone and returns the count of chips it grouped\n"
    if marker not in script:
        raise ValueError("Chip helper insertion point was not found")
    required = {
        'local chipTable = buildObjTable(zoneGUIDs, {"Chip"})':
            "local chipTable = getFilteredChipTable(zoneGUIDs)",
        'Wait.time(function() chipTable = buildObjTable(zoneGUIDs, {"Chip"}) end, moveDelay)':
            "Wait.time(function() chipTable = getFilteredChipTable(zoneGUIDs) end, moveDelay)",
        'local chipList = buildObjTable(zoneGUIDs, {"Chip"})':
            "local chipList = getFilteredChipTable(zoneGUIDs)",
    }
    script = replace_required(script, required, "chip filtering")
    return script.replace(marker, CHIP_FILTER_HELPER + marker, 1)


def patch_betting_area_border(script: str) -> str:
    if "function createBettingAreaBorder()" in script:
        return script

    # Migrate the first localized revision, which used a single text marker.
    old_helper_start = "-- Create a visible, non-interactive marker for the central chip zones.\n"
    old_helper_end = "--[[ The onLoad event is called after the game save finishes loading. --]]\n"
    if old_helper_start in script:
        helper_start = script.index(old_helper_start)
        helper_end = script.index(old_helper_end, helper_start)
        script = script[:helper_start] + BETTING_AREA_BORDER_HELPER + script[helper_end:]
        script = script.replace(
            'if v.name == "3DText" or v.name == "BettingAreaMarker" then',
            'if v.name == "3DText" or v.name == "BettingAreaBorder" then',
            1,
        )
        return script.replace("    createBettingAreaMarker()\n", "    createBettingAreaBorder()\n", 1)

    cleanup = '''        if v.name == "3DText" then
            destroyObject(v)
        end
'''
    replacement = '''        if v.name == "3DText" or v.name == "BettingAreaBorder" then
            destroyObject(v)
        end
'''
    script = replace_required(script, {cleanup: replacement}, "3D text cleanup")
    insertion = "    -- Update  status for any players already seated\n"
    if insertion not in script:
        raise ValueError("Betting border insertion point was not found")
    script = script.replace(insertion, "    createBettingAreaBorder()\n\n" + insertion, 1)
    marker = "--[[ The onLoad event is called after the game save finishes loading. --]]\n"
    if marker not in script:
        raise ValueError("Betting border helper insertion point was not found")
    return script.replace(marker, BETTING_AREA_BORDER_HELPER + marker, 1)


def patch_seating_and_roles(data: dict) -> None:
    old_centers = '''colorCenterArray =
{
    {0, 3, -12.5}
    , {-8.9,3,-9}
    , {-12.5,3,0}
    , {-8.9,3,9}
    , {0,3,12.5}
    , {8.9,3,9}
    , {12.5,3,0}
    , {8.9,3,-9}
    , {0,0,0}
    , {0,0,0}
}
'''
    large_table_centers = '''colorCenterArray =
{
    {0, 3, -15}
    , {-10.6,3,-10.6}
    , {-15,3,0}
    , {-10.6,3,10.6}
    , {0,3,15}
    , {10.6,3,10.6}
    , {15,3,0}
    , {10.6,3,-10.6}
    , {0,0,0}
    , {0,0,0}
}
'''
    script = data["LuaScript"]
    if large_table_centers in script:
        script = script.replace(large_table_centers, old_centers, 1)
    elif old_centers not in script:
        raise ValueError("Player-center array was not found")

    if "function initializeRoleMarkers()" not in script:
        cleanup = 'if v.name == "3DText" or v.name == "BettingAreaBorder" then'
        if cleanup not in script:
            raise ValueError("Role marker cleanup insertion point was not found")
        script = script.replace(
            cleanup,
            'if v.name == "3DText" or v.name == "BettingAreaBorder" or v.name == "PokerRoleMarker" then',
            1,
        )
        onload_marker = "--[[ The onLoad event is called after the game save finishes loading. --]]\n"
        if onload_marker not in script:
            raise ValueError("Role marker helper insertion point was not found")
        script = script.replace(onload_marker, ROLE_MARKER_HELPER + onload_marker, 1)
        script = script.replace(
            "    managePlayerStatus()\n\n    -- Timer that controls the automatic chip counters",
            "    managePlayerStatus()\n    initializeRoleMarkers()\n\n    -- Timer that controls the automatic chip counters",
            1,
        )
        script = script.replace(
            "    getObjectFromGUID(playButtonGUID).editButton({index=0, label='发牌', tooltip = '为所有已入座玩家发两张底牌', color = {111/255, 255/255, 125/255}, click_function =\"playGame\"})\n",
            "    getObjectFromGUID(playButtonGUID).editButton({index=0, label='发牌', tooltip = '为所有已入座玩家发两张底牌', color = {111/255, 255/255, 125/255}, click_function =\"playGame\"})\n    advanceDealerAndRoles()\n",
            1,
        )
        script = script.replace(
            "function onPlayerConnect(person)\n    managePlayerStatus()\nend",
            "function onPlayerConnect(person)\n    managePlayerStatus()\n    initializeRoleMarkers()\nend",
            1,
        )
        script = script.replace(
            "function onPlayerChangeColor(newColor)\n    managePlayerStatus()\nend",
            "function onPlayerChangeColor(newColor)\n    managePlayerStatus()\n    initializeRoleMarkers()\nend",
            1,
        )
    data["LuaScript"] = script

    objects = {obj.get("GUID"): obj for obj in walk_objects(data.get("ObjectStates", []))}
    move_initial_chips = False
    for guid, target in zip(PLAYER_ZONE_GUIDS, ORIGINAL_PLAYER_CENTERS):
        transform = objects[guid]["Transform"]
        large_table = LARGE_TABLE_PLAYER_CENTERS[PLAYER_ZONE_GUIDS.index(guid)]
        if abs(transform["posX"] - large_table[0]) < 0.01 and abs(transform["posZ"] - large_table[1]) < 0.01:
            move_initial_chips = True
        transform["posX"], transform["posZ"] = target

    if move_initial_chips:
        for obj in objects.values():
            if obj.get("Name") != "ChipStack":
                continue
            transform = obj["Transform"]
            distances = [
                (transform["posX"] - x) ** 2 + (transform["posZ"] - z) ** 2
                for x, z in LARGE_TABLE_PLAYER_CENTERS
            ]
            nearest = min(range(len(distances)), key=distances.__getitem__)
            if distances[nearest] <= 49:
                old_x, old_z = LARGE_TABLE_PLAYER_CENTERS[nearest]
                new_x, new_z = ORIGINAL_PLAYER_CENTERS[nearest]
                transform["posX"] += new_x - old_x
                transform["posZ"] += new_z - old_z


def patch_test_bots(data: dict) -> None:
    xml = data["XmlUI"]
    if 'id="addTestBotButton"' not in xml:
        data["XmlUI"] = xml + TEST_BOT_UI

    script = data["LuaScript"]
    if "function addTestBot(player, value, id)" not in script:
        marker = "function getActiveSeatIndexes()\n"
        if marker not in script:
            raise ValueError("Test-bot helper insertion point was not found")
        script = script.replace(marker, TEST_BOT_HELPER + marker, 1)
        script = script.replace(
            "if colorStatusArray[index] == 1 then\n            table.insert(active, index)",
            "if isRoleActiveSeat(index) then\n            table.insert(active, index)",
            1,
        )
        script = script.replace(
            "if colorStatusArray[candidate] == 1 then\n            return candidate",
            "if isRoleActiveSeat(candidate) then\n            return candidate",
            1,
        )
        script = script.replace(
            "colorStatusArray[dealerSeatIndex] ~= 1",
            "not isRoleActiveSeat(dealerSeatIndex)",
        )
        script = script.replace(
            "    initializeRoleMarkers()\n\n    -- Timer that controls the automatic chip counters",
            "    initializeRoleMarkers()\n    setTestBotStatus()\n\n    -- Timer that controls the automatic chip counters",
            1,
        )
    data["LuaScript"] = script


def patch_role_marker_cleanup(data: dict) -> None:
    """Use the TTS object-name API so old role text is reliably removed each hand."""
    script = data["LuaScript"]
    script = script.replace(
        'if obj.name == "PokerRoleMarker" then',
        'if obj.getName() == "PokerRoleMarker" then',
    )
    script = script.replace(
        'if v.name == "3DText" or v.name == "BettingAreaBorder" or v.name == "PokerRoleMarker" then',
        'if v.getName() == "3DText" or v.getName() == "BettingAreaBorder" or v.getName() == "PokerRoleMarker" then',
    )
    data["LuaScript"] = script


def patch_bot_hand_dealing(data: dict) -> None:
    """Deal two real hole cards to each computer placeholder's active hand zone."""
    script = data["LuaScript"]
    script = script.replace('"测试座位：" .. suffix', '"电脑占位：" .. suffix')
    script = script.replace(
        "if playerStatus == 1 then\n                deckOfCards.deal(1, colorNameArray[colorIndex])",
        "if playerStatus == 1 or testBotSeats[colorIndex] == true then\n                deckOfCards.deal(1, colorNameArray[colorIndex])",
    )
    data["LuaScript"] = script
    data["XmlUI"] = data["XmlUI"].replace("添加测试玩家", "添加电脑占位").replace("清空测试玩家", "清空电脑占位").replace("测试座位：无", "电脑占位：无")


def remove_test_bots(data: dict) -> None:
    """Remove temporary computer-placeholder controls and restore real-player-only dealing."""
    data["XmlUI"] = data["XmlUI"].replace(TEST_BOT_UI, "")
    script = data["LuaScript"]
    helper_start = "-- Virtual seats exist only to test poker-role rotation without extra Steam players.\n"
    helper_end = "function getActiveSeatIndexes()\n"
    if helper_start in script:
        start = script.index(helper_start)
        end = script.index(helper_end, start)
        script = script[:start] + script[end:]
    script = script.replace("if isRoleActiveSeat(index) then", "if colorStatusArray[index] == 1 then")
    script = script.replace("if isRoleActiveSeat(candidate) then", "if colorStatusArray[candidate] == 1 then")
    script = script.replace("not isRoleActiveSeat(dealerSeatIndex)", "colorStatusArray[dealerSeatIndex] ~= 1")
    script = script.replace("    setTestBotStatus()\n", "")
    script = script.replace(
        "if playerStatus == 1 or testBotSeats[colorIndex] == true then",
        "if playerStatus == 1 then",
    )
    data["LuaScript"] = script


def patch_rules_tab(data: dict) -> None:
    data["TabStates"]["0"]["title"] = "规则"
    data["TabStates"]["0"]["body"] = RULES_TAB_BODY


def patch_heads_up_role_marker(data: dict) -> None:
    """In heads-up play, the dealer is also the small blind, so show one combined marker."""
    script = data["LuaScript"]
    old = '''    spawnRoleMarker(dealerSeatIndex, "庄家", {r=1, g=0.82, b=0.25})
    spawnRoleMarker(smallBlindSeat, "小盲", {r=0.35, g=0.85, b=1})
    spawnRoleMarker(bigBlindSeat, "大盲", {r=1, g=0.4, b=0.4})
'''
    new = '''    if #active == 2 then
        spawnRoleMarker(dealerSeatIndex, "庄家 / 小盲", {r=1, g=0.82, b=0.25})
    else
        spawnRoleMarker(dealerSeatIndex, "庄家", {r=1, g=0.82, b=0.25})
        spawnRoleMarker(smallBlindSeat, "小盲", {r=0.35, g=0.85, b=1})
    end
    spawnRoleMarker(bigBlindSeat, "大盲", {r=1, g=0.4, b=0.4})
'''
    if old in script:
        script = script.replace(old, new, 1)
    elif new not in script:
        raise ValueError("Heads-up role marker insertion point was not found")
    data["LuaScript"] = script


def main() -> None:
    input_path = SOURCE if SOURCE.exists() else (BASELINE if BASELINE.exists() else OUTPUT)
    if not input_path.exists():
        raise FileNotFoundError(f"No source, baseline, or existing localized copy exists: {SOURCE}, {BASELINE}, {OUTPUT}")
    source_bytes = input_path.read_bytes()
    source_hash = hashlib.sha256(source_bytes).hexdigest().upper()
    data = json.loads(source_bytes.decode("utf-8"))

    data["SaveName"] = "Texas Hold'em"
    data["GameMode"] = "Texas Hold'em"
    data["Table"] = "Table_Octagon"

    # The engine and UI are maintained as standalone source files, rather than
    # incrementally patching the legacy workshop Global script.
    data["LuaScript"] = ENGINE.read_text(encoding="utf-8")
    data["XmlUI"] = ENGINE_UI.read_text(encoding="utf-8")
    patch_rules_tab(data)

    patched_objects = set()
    resized_zones = set()
    for obj in walk_objects(data.get("ObjectStates", [])):
        guid = obj.get("GUID")
        if guid in OBJECT_SCRIPT_REPLACEMENTS:
            # The original English Workshop file is no longer available.  The
            # existing localized output may carry these earlier button labels.
            # Normalize them before applying the current, idempotent mapping.
            if guid == "87d2e6":
                obj["LuaScript"] = obj.get("LuaScript", "").replace('self.setName("收拢筹码")', 'self.setName("Gather")')
                obj["LuaScript"] = obj["LuaScript"].replace("bParam.label = '收拢筹码'", "bParam.label = 'Gather Chips'")
                obj["LuaScript"] = obj["LuaScript"].replace("收拢并整理中央投注区的筹码", "Gather Chips")
                obj["LuaScript"] = obj["LuaScript"].replace("收拢并按面额整理中央投注区的所有筹码", "Collect and group all chips in the play area")
            elif guid == "0d221f":
                obj["LuaScript"] = obj.get("LuaScript", "").replace('self.setName("补充筹码")', 'self.setName("Chips")')
                obj["LuaScript"] = obj["LuaScript"].replace("bParam.label = '补充筹码'", "bParam.label = 'Spawn Chips'")
                obj["LuaScript"] = obj["LuaScript"].replace("在中央投注区补充 5000 筹码", "Spawn 5000 chips")
                obj["LuaScript"] = obj["LuaScript"].replace("在中央投注区生成总值 5000 的筹码", "Spawn 5000 chips in the play area")
            obj["LuaScript"] = replace_or_confirm(
                obj.get("LuaScript", ""), OBJECT_SCRIPT_REPLACEMENTS[guid], f"object {guid}"
            )
            if guid == "cc0912":
                obj["LuaScript"] = obj["LuaScript"].replace(
                    'Global.call("resetGame")',
                    'Global.call("resetGame", {playerColor = playerColorClicked})',
                )
            patched_objects.add(guid)
        if guid in PLAY_ZONE_GUIDS:
            transform = obj["Transform"]
            transform["scaleX"] = 8.0
            transform["scaleY"] = 5.0
            transform["scaleZ"] = 20.0
            resized_zones.add(guid)

    if patched_objects != set(OBJECT_SCRIPT_REPLACEMENTS):
        raise ValueError(f"Missing button objects: {sorted(set(OBJECT_SCRIPT_REPLACEMENTS) - patched_objects)}")
    if resized_zones != PLAY_ZONE_GUIDS:
        raise ValueError(f"Missing play zones: {sorted(PLAY_ZONE_GUIDS - resized_zones)}")

    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(json.dumps(data, ensure_ascii=False, separators=(",", ":")), encoding="utf-8")
    print(f"input={input_path}")
    print(f"input_sha256={source_hash}")
    print(OUTPUT)


if __name__ == "__main__":
    main()
