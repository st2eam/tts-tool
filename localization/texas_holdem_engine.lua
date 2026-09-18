-- Texas Hold'em cash-game engine for Tabletop Simulator.
-- State is intentionally plain Lua data so it can be persisted through onSave.

local COLORS = {"White", "Red", "Orange", "Yellow", "Green", "Blue", "Purple", "Pink"}
local ZONES = {"e4131e", "5b93b0", "43ae65", "0be485", "4dea38", "535ab7", "029071", "b142e7"}
local CENTERS = {{0,3,-12.5},{-8.9,3,-9},{-12.5,3,0},{-8.9,3,9},{0,3,12.5},{8.9,3,9},{12.5,3,0},{8.9,3,-9}}
local PLAY_ZONES = {"d81799", "d28ab5", "f2a644", "3775b1"}
local DECK_GUID, PLAY_GUID, RESET_GUID = "018b71", "88b654", "cc0912"
local UNIT, SB, BB, BUY_IN = 50, 50, 100, 10000
local state = {}
local function smallBlind() return (state.config and state.config.sb) or SB end
local function bigBlind() return (state.config and state.config.bb) or BB end
local function buyIn() return (state.config and state.config.buyin) or BUY_IN end

local function indexOf(color)
  for i,c in ipairs(COLORS) do if c == color then return i end end
  return 0
end
local function seated(color) return Player[color] ~= nil and Player[color].seated end
local function activeIndices()
  local out = {}; for i,c in ipairs(COLORS) do if seated(c) then table.insert(out,i) end end; return out
end
local function nextSeat(i)
  for n=1,8 do local x=((i-1+n)%8)+1; if seated(COLORS[x]) then return x end end
  return 0
end
local function actorColor() return state.actor and COLORS[state.actor] or "" end
local function allowed(color) return seated(color) end
local function p(color) return state.players and state.players[color] end
local function deck() return getObjectFromGUID(DECK_GUID) end
local function zoneObjects(guid) local z=getObjectFromGUID(guid); return z and z.getObjects() or {} end
local function chipValue(obj) return obj.tag == "Chip" and obj.getValue()*math.abs(obj.getQuantity()) or 0 end
local function zoneValue(guid) local t=0; for _,o in ipairs(zoneObjects(guid)) do t=t+chipValue(o) end; return t end
local function personalValue(i) return zoneValue(ZONES[i]) end
local function playValue()
  local seen, total = {}, 0
  for _,z in ipairs(PLAY_ZONES) do for _,o in ipairs(zoneObjects(z)) do
    local id=o.getGUID(); if not seen[id] and o.tag=="Chip" then seen[id]=true; total=total+chipValue(o) end
  end end
  return total
end
local function totalPot() local n=0; for _,v in pairs(state.players or {}) do n=n+(v.total or 0) end; return n end
local function pendingFor(color)
  -- The central zones are shared.  Only the current actor can confirm their
  -- excess, so this is deliberately treated as that actor's pending amount.
  return math.max(0,playValue()-totalPot())
end
local function fmt(n) return "$"..string.format("%d", n or 0) end
local function message(color,text) if color and color~="" then printToColor(text,color,{1,0.85,0.3}) else printToAll(text,{1,0.85,0.3}) end end
local function isLive(v) return v and not v.folded end
local function liveCount() local n=0; for _,v in pairs(state.players or {}) do if isLive(v) then n=n+1 end end; return n end
local function canAct(v) return isLive(v) and not v.allin end

local function destroyChips(guid)
  for _,o in ipairs(zoneObjects(guid)) do if o.tag=="Chip" then o.destruct() end end
end
local function spawnAmount(amount, pos, rotation)
  local specs={{1000,"chip_1000"},{500,"chip_500"},{100,"chip_100"},{50,"chip_50"}}
  local offset=0
  for _,spec in ipairs(specs) do
    local count=math.floor(amount/spec[1]); amount=amount-count*spec[1]
    for _=1,count do
      spawnObject({type=spec[2],position={pos[1],pos[2]+offset,pos[3]},rotation=rotation,scale={x=.75,y=.75,z=.75}})
      offset=offset+.16
    end
  end
end
local function syncChips()
  for i,c in ipairs(COLORS) do
    destroyChips(ZONES[i]); local v=p(c); if v then spawnAmount(v.stack,CENTERS[i],{0,(i+1)*45,0}) end
  end
  for _,g in ipairs(PLAY_ZONES) do destroyChips(g) end
  local pot=totalPot(); if pot>0 then spawnAmount(pot,{-3.2,1.25,-5},{0,90,0}) end
end

local function setUI(id,value) UI.setValue(id,tostring(value or "")) end
local function setAttr(id,key,value) UI.setAttribute(id,key,tostring(value)) end
local function roleText(i)
  if i==state.dealer then return "庄家" end
  if i==state.sbSeat and i==state.dealer then return "庄家 / 小盲" end
  if i==state.sbSeat then return "小盲" end
  if i==state.bbSeat then return "大盲" end
  return ""
end
local function refreshUI()
  local cur=p(actorColor())
  setUI("phaseText", ({idle="等待开局",preflop="翻牌前",flop="翻牌",turn="转牌",river="河牌",run_vote="跑马选择",showdown="摊牌结算",settled="本手结束"})[state.phase] or "")
  setUI("actorText", actorColor()=="" and "当前行动：—" or "当前行动："..actorColor())
  setUI("callText", "需跟注："..fmt(math.max(0,(state.currentBet or 0)-(cur and cur.street or 0))))
  setUI("minRaiseText", "最小总下注："..fmt((state.currentBet or 0)+(state.minRaise or bigBlind())))
  setUI("potText", "底池："..fmt(totalPot()))
  local side=state.pots and #state.pots>1 and ("边池："..tostring(#state.pots-1)) or "边池：0"; setUI("sidePotText",side)
  local pending=pendingFor and pendingFor(actorColor()) or 0
  setUI("myBetText",cur and ("本轮："..fmt(cur.street).."  剩余："..fmt(cur.stack).."  待确认："..fmt(pending)) or "本轮：—")
  for i,c in ipairs(COLORS) do setUI("seat"..c, c.."  "..roleText(i).."  "..(p(c) and fmt(p(c).stack) or "") end
  setUI("smallBlindInput",smallBlind()); setUI("bigBlindInput",bigBlind()); setUI("buyInInput",buyIn())
  -- Keep the Start/Next-hand control available while action callbacks still
  -- enforce that only the current player may make a poker decision.
  setAttr("actionPanel","active","true")
  setAttr("runPanel","active",state.phase=="run_vote" and "true" or "false")
end

local function clearMarkers()
  for _,o in ipairs(getAllObjects()) do if o.getName()=="PokerRoleMarker" then destroyObject(o) end end
end
local function marker(i,label,color)
  local v=CENTERS[i]; local o=spawnObject({type="3DText",position={v[1]*.86,1.06,v[3]*.86},rotation={90,(i+1)*45,90},scale={x=1.25,y=1.25,z=1.25}})
  o.TextTool.setFontColor(color); o.TextTool.setFontSize(58); o.TextTool.setValue(label); o.setName("PokerRoleMarker"); o.setLock(true)
end
local function refreshRoles()
  clearMarkers(); if not state.dealer or state.dealer==0 then return end
  if state.dealer==state.sbSeat then marker(state.dealer,"庄家 / 小盲",{r=1,g=.82,b=.25}) else marker(state.dealer,"庄家",{r=1,g=.82,b=.25}); marker(state.sbSeat,"小盲",{r=.35,g=.85,b=1}) end
  marker(state.bbSeat,"大盲",{r=1,g=.4,b=.4})
end
local function createBettingAreaBorder()
  for _,o in ipairs(getAllObjects()) do if o.getName()=="BettingAreaBorder" then destroyObject(o) end end
  local function edge(pos,scale)
    local o=spawnObject({type="3DText",position=pos,rotation={90,0,0},scale=scale})
    o.TextTool.setValue("━━━━━━━━")
    o.TextTool.setFontColor({r=.15,g=.9,b=1})
    o.TextTool.setFontSize(42)
    o.setName("BettingAreaBorder")
    o.setLock(true)
  end
  edge({0,1.03,-5.25},{x=2.2,y=1,z=1})
  edge({0,1.03,5.25},{x=2.2,y=1,z=1})
  edge({-8.1,1.03,0},{x=2.2,y=1,z=1})
  edge({8.1,1.03,0},{x=2.2,y=1,z=1})
end

local function resetRoundFlags()
  for _,v in pairs(state.players) do v.street=0; v.acted=false end
  state.currentBet=0; state.minRaise=bigBlind()
end
local function makePlayers()
  state.players={}
  for i,c in ipairs(COLORS) do if seated(c) then
    local balance=personalValue(i); if balance<=0 then balance=buyIn() end
    state.players[c]={stack=balance,street=0,total=0,folded=false,allin=false,acted=false,pending=0}
  end end
end
local function commit(color,target)
  local v=p(color); if not v then return false,"无效座位" end
  target=math.floor(target/UNIT)*UNIT; local delta=target-v.street
  if delta<0 then return false,"不能减少本轮下注" end
  if delta>v.stack then return false,"筹码不足" end
  local pending=pendingFor(color)
  if pending>delta then return false,"中央待确认筹码超过本次操作，请先取回多余筹码。" end
  v.stack=v.stack-delta; v.street=target; v.total=v.total+delta; v.pending=0; if v.stack==0 then v.allin=true end
  syncChips(); return true
end
local function currentNeeded(color) local v=p(color); return math.max(0,state.currentBet-v.street) end
local function resetActed(except) for c,v in pairs(state.players) do if c~=except and canAct(v) then v.acted=false end end end
local function roundDone()
  for _,v in pairs(state.players) do if canAct(v) and (not v.acted or v.street~=state.currentBet) then return false end end; return true
end
local function nextActor(after)
  for n=1,8 do local i=((after-1+n)%8)+1; local v=p(COLORS[i]); if canAct(v) then return i end end; return 0
end

local function cardId(o) local d=o and o.getData and o.getData(); return d and d.CardID or -1 end
local function rank(id) return (id%13)+2 end
local function suit(id) return math.floor(id/13) end
local function score5(ids)
  local rs,counts,flush={}, {}, true
  local s=suit(ids[1]); for _,id in ipairs(ids) do local r=rank(id); table.insert(rs,r); counts[r]=(counts[r] or 0)+1; if suit(id)~=s then flush=false end end
  table.sort(rs,function(a,b)return a>b end); local unique={}; for _,r in ipairs(rs) do if #unique==0 or unique[#unique]~=r then table.insert(unique,r) end end
  local high=unique[1]; local straight=false
  if #unique==5 then if unique[1]-unique[5]==4 then straight=true elseif unique[1]==14 and unique[2]==5 and unique[5]==2 then straight=true; high=5 end end
  local groups={}; for r,n in pairs(counts) do table.insert(groups,{n=n,r=r}) end; table.sort(groups,function(a,b) return a.n==b.n and a.r>b.r or a.n>b.n end)
  if straight and flush then return {9,high} end
  if groups[1].n==4 then return {8,groups[1].r,groups[2].r} end
  if groups[1].n==3 and groups[2].n==2 then return {7,groups[1].r,groups[2].r} end
  if flush then return {6,rs[1],rs[2],rs[3],rs[4],rs[5]} end
  if straight then return {5,high} end
  if groups[1].n==3 then local k={}; for _,g in ipairs(groups) do if g.n==1 then table.insert(k,g.r) end end; return {4,groups[1].r,k[1],k[2]} end
  if groups[1].n==2 and groups[2].n==2 then return {3,groups[1].r,groups[2].r,groups[3].r} end
  if groups[1].n==2 then local k={}; for _,g in ipairs(groups) do if g.n==1 then table.insert(k,g.r) end end; return {2,groups[1].r,k[1],k[2],k[3]} end
  return {1,rs[1],rs[2],rs[3],rs[4],rs[5]}
end
local function better(a,b) for i=1,math.max(#a,#b) do if (a[i] or 0)~=(b[i] or 0) then return (a[i] or 0)>(b[i] or 0) end end return false end
local function best7(ids)
  local best=nil; for a=1,#ids-4 do for b=a+1,#ids-3 do for c=b+1,#ids-2 do for d=c+1,#ids-1 do for e=d+1,#ids do
    local s=score5({ids[a],ids[b],ids[c],ids[d],ids[e]}); if not best or better(s,best) then best=s end
  end end end end end; return best
end
local function buildPots()
  local levels={}; for _,v in pairs(state.players) do if v.total>0 then levels[v.total]=true end end
  local sorted={}; for n,_ in pairs(levels) do table.insert(sorted,n) end; table.sort(sorted)
  local pots,prev={},0; for _,level in ipairs(sorted) do
    local contributors,eligible=0,{}; for c,v in pairs(state.players) do if v.total>=level then contributors=contributors+1; if not v.folded then table.insert(eligible,c) end end end
    local amount=(level-prev)*contributors; if amount>0 then table.insert(pots,{amount=amount,eligible=eligible}) end; prev=level
  end; state.pots=pots; return pots
end
local function reveal(color) for _,o in ipairs(Player[color].getHandObjects()) do if o.is_face_down then o.flip() end end end
local function playerCards(color)
  local out={}; for _,o in ipairs(Player[color].getHandObjects()) do table.insert(out,cardId(o)) end; return out
end
local function payout(amount,winners)
  local share=math.floor(amount/#winners/UNIT)*UNIT; local remainder=amount-share*#winners
  for _,c in ipairs(winners) do p(c).stack=p(c).stack+share end
  local i=nextSeat(state.dealer); while remainder>0 do local c=COLORS[i]; for _,w in ipairs(winners) do if c==w then p(w).stack=p(w).stack+UNIT; remainder=remainder-UNIT; break end end; i=nextSeat(i) end
end
local function settle(boards)
  state.phase="showdown"; local pots=buildPots(); for c,v in pairs(state.players) do if not v.folded then reveal(c) end end
  for run,board in ipairs(boards) do for _,pot in ipairs(pots) do
    local best,winners=nil,{}; for _,c in ipairs(pot.eligible) do local ids=playerCards(c); for _,id in ipairs(board) do table.insert(ids,id) end; local score=best7(ids)
      if not best or better(score,best) then best=score; winners={c} elseif not better(best,score) then table.insert(winners,c) end
    end
    local perRun=math.floor(pot.amount/#boards/UNIT)*UNIT
    local remainder=pot.amount-perRun*#boards
    payout(perRun+(run==1 and remainder or 0),winners)
  end end
  for _,v in pairs(state.players) do v.total=0; v.street=0; v.pending=0 end
  state.phase="settled"; state.actor=0; syncChips(); refreshUI(); message("","本手结算完成。点击“下一手”继续。")
end
local function boardIds() local out={}; for _,o in ipairs(state.board or {}) do table.insert(out,cardId(getObjectFromGUID(o))) end; return out end
local function takeBoard(n, offset, done)
  local remaining=n; for k=1,n do
    local o=deck().takeObject({position={state.playAnchor[1]+((#state.board+k-1)*2)+offset,state.playAnchor[2],state.playAnchor[3]},rotation={0,180,180},smooth=false,callback_function=function(card)
      card.setLock(true); card.flip(); table.insert(state.board,card.getGUID()); remaining=remaining-1; if remaining==0 and done then done() end
    end})
  end
end
local function burn() deck().takeObject({position={state.burnAnchor[1],state.burnAnchor[2],state.burnAnchor[3]},rotation={0,180,180},smooth=false}) end
local function takeRunCards(board, amount, offset, done)
  local left=amount
  for k=1,amount do deck().takeObject({position={state.playAnchor[1]+((#board+k-1)*2)+offset,state.playAnchor[2],state.playAnchor[3]+offset*.18},rotation={0,180,180},smooth=false,callback_function=function(card)
    card.setLock(true); card.flip(); table.insert(board,cardId(card)); left=left-1; if left==0 then done() end
  end}) end
end
local function dealRunBoard(base, offset, done)
  local board={}; for _,id in ipairs(base) do table.insert(board,id) end
  local function river() burn(); takeRunCards(board,1,offset,function() done(board) end) end
  local function turn() burn(); takeRunCards(board,1,offset,river) end
  if #board==0 then burn(); takeRunCards(board,3,offset,turn)
  elseif #board==3 then turn()
  elseif #board==4 then river()
  else done(board) end
end
local function runout(count)
  local base, boards=boardIds(), {}
  local function dealNext(run)
    if run>count then settle(boards); return end
    dealRunBoard(base,(run-1)*7,function(board) table.insert(boards,board); dealNext(run+1) end)
  end
  dealNext(1)
end
local function beginRunVote()
  state.phase="run_vote"; state.runVotes={}; refreshUI(); setAttr("runPanel","active","true")
  Timer.destroy("runVoteTimer"); Timer.create({identifier="runVoteTimer",function_name="runVoteTimeout",delay=20,repetitions=1})
end
local function continueAfterRound()
  if liveCount()==1 then
    local winner; for c,v in pairs(state.players) do if not v.folded then winner=c end end; p(winner).stack=p(winner).stack+totalPot(); state.phase="settled"; state.actor=0; syncChips(); refreshUI(); return
  end
  local movable=0; for _,v in pairs(state.players) do if canAct(v) then movable=movable+1 end end
  if movable==0 and #state.board<5 then beginRunVote(); return end
  if state.phase=="preflop" then state.phase="flop"; resetRoundFlags(); burn(); takeBoard(3,0,function() state.actor=nextActor(state.dealer); refreshUI() end)
  elseif state.phase=="flop" then state.phase="turn"; resetRoundFlags(); burn(); takeBoard(1,0,function() state.actor=nextActor(state.dealer); refreshUI() end)
  elseif state.phase=="turn" then state.phase="river"; resetRoundFlags(); burn(); takeBoard(1,0,function() state.actor=nextActor(state.dealer); refreshUI() end)
  else settle({boardIds()}) end
end
local function advance()
  if liveCount()==1 then continueAfterRound(); return end
  if roundDone() then continueAfterRound() else state.actor=nextActor(state.actor); refreshUI() end
end

function startHand(params)
  local color=(type(params)=="table" and (params.playerColor or params.color or params[2])) or params
  if color and color~="" and not allowed(color) then return end
  if state.phase~="idle" and state.phase~="settled" then message(color,"本手尚未结束。"); return end
  local active=activeIndices(); if #active<2 then message(color,"至少需要两名入座玩家。"); return end
  local config=state.config or {sb=SB,bb=BB,buyin=BUY_IN}
  state={phase="preflop",dealer=nextSeat(state.dealer or 0),board={},pots={},runVotes={},players={},playAnchor={-4.2,1,2.5},burnAnchor={-3.2,1,-2.5},config=config}
  makePlayers(); state.sbSeat=nextSeat(state.dealer); state.bbSeat=nextSeat(state.sbSeat); if #active==2 then state.sbSeat=state.dealer; state.bbSeat=nextSeat(state.dealer) end
  state.currentBet=0; state.minRaise=bigBlind(); local function blind(i,amount) local c=COLORS[i]; local v=p(c); commit(c,math.min(amount,v.stack)); v.acted=false end
  blind(state.sbSeat,smallBlind()); blind(state.bbSeat,bigBlind()); state.currentBet=math.min(bigBlind(),p(COLORS[state.bbSeat]).street); state.minRaise=bigBlind()
  deck().shuffle(); deck().setLock(true); for _=1,2 do for _,i in ipairs(active) do deck().deal(1,COLORS[i]) end end
  state.actor=nextActor(state.bbSeat); refreshRoles(); refreshUI()
end
function playGame(params) startHand(params) end
function resetGame(params)
  local color=(type(params)=="table" and (params.playerColor or params.color or params[2])) or params
  if color and color~="" and not allowed(color) then return end
  local d=deck(); if d then d.reset(); d.shuffle(); d.setLock(true) end
  state.phase="idle"; state.actor=0; state.board={}; state.pots={}; state.runVotes={}; refreshUI()
end
function sortTableChips(params)
  syncChips(); refreshUI()
end
function callSpawnChips(params)
  message(params and (params.playerColor or params[2]),"现金局由账本统一管理筹码；请开新局或使用“同步筹码”。")
end
function uiAction(player,value,id)
  local color=player.color; if not p(color) or indexOf(color)~=state.actor then message(color,"尚未轮到你行动。"); return end
  local v=p(color); local action=id; local target=v.street
  if action=="fold" then v.folded=true; v.acted=true
  elseif action=="check" then if currentNeeded(color)~=0 then message(color,"当前不能过牌。"); return else v.acted=true end
  elseif action=="call" then target=math.min(state.currentBet,v.street+v.stack); local ok,err=commit(color,target); if not ok then message(color,err); return end; v.acted=true
  else
    if action=="allin" then target=v.street+v.stack else target=tonumber(UI.getValue("amountInput")) or 0 end
    target=math.floor(target/UNIT)*UNIT
    if target<=state.currentBet then message(color,"下注总额必须高于当前下注；请使用跟注或全押。"); return end
    local min=state.currentBet==0 and bigBlind() or state.currentBet+state.minRaise
    if target<min and target<v.street+v.stack then message(color,"低于最小加注额。"); return end
    local old=state.currentBet; local ok,err=commit(color,math.min(target,v.street+v.stack)); if not ok then message(color,err); return end
    state.currentBet=v.street; local raise=state.currentBet-old; if raise>=state.minRaise then state.minRaise=raise; resetActed(color) end; v.acted=true
  end
  advance()
end
function quickAmount(player,value,id)
  local v=p(player.color); if not v then return end; local amount=state.currentBet+state.minRaise
  if id=="minRaise" then amount=state.currentBet+state.minRaise elseif id=="halfPot" then amount=state.currentBet+math.floor(totalPot()/2/UNIT)*UNIT elseif id=="fullPot" then amount=state.currentBet+totalPot() elseif id=="allinAmount" then amount=v.street+v.stack end
  setUI("amountInput",amount)
end
function voteRun(player,value,id)
  if state.phase~="run_vote" or not isLive(p(player.color)) then return end
  state.runVotes[player.color]=(id=="runTwo") and 2 or 1
  local all=true; for c,v in pairs(state.players) do if isLive(v) and not state.runVotes[c] then all=false end end
  if all then local two=true; for _,x in pairs(state.runVotes) do if x~=2 then two=false end end; forceRun({count=two and 2 or 1}) end
end
function runVoteTimeout() if state.phase=="run_vote" then forceRun({count=1}) end end
function forceRun(player,value,id)
  if state.phase~="run_vote" then return end
  local count=1
  if type(player)=="table" and player.count then count=player.count
  elseif id=="forceTwo" then count=2
  elseif player and player.color and not allowed(player.color) then return end
  Timer.destroy("runVoteTimer"); setAttr("runPanel","active","false"); runout(count)
end
function adminReconcile(player,value,id) if not allowed(player.color) then return end; syncChips(); refreshUI() end
function applySettings(player,value,id)
  if not allowed(player.color) then return end
  if state.phase~="idle" and state.phase~="settled" then message(player.color,"只能在一手牌开始前修改设置。 "); return end
  local sb=tonumber(UI.getValue("smallBlindInput")) or 0
  local bb=tonumber(UI.getValue("bigBlindInput")) or 0
  local buyin=tonumber(UI.getValue("buyInInput")) or 0
  if sb<UNIT or bb<sb*2 or buyin<bb or sb%UNIT~=0 or bb%UNIT~=0 or buyin%UNIT~=0 then message(player.color,"盲注和买入必须为 50 的倍数，且大盲至少为小盲的两倍。 "); return end
  state.config={sb=sb,bb=bb,buyin=buyin}; refreshUI(); message("","牌局设置已更新："..fmt(sb).."/"..fmt(bb).."，买入 "..fmt(buyin))
end
function onSave() return JSON.encode(state) end
function onLoad(saved)
  state={phase="idle",dealer=0,players={},board={},pots={},runVotes={},config={sb=SB,bb=BB,buyin=BUY_IN}}
  if saved and saved~="" then local ok,data=pcall(JSON.decode,saved); if ok and type(data)=="table" then state=data end end
  state.playAnchor=state.playAnchor or {-4.2,1,2.5}; state.burnAnchor=state.burnAnchor or {-3.2,1,-2.5}
  state.config=state.config or {sb=SB,bb=BB,buyin=BUY_IN}
  Wait.time(function()
    createBettingAreaBorder(); refreshRoles(); refreshUI()
    if state.phase=="run_vote" then Timer.destroy("runVoteTimer"); Timer.create({identifier="runVoteTimer",function_name="runVoteTimeout",delay=20,repetitions=1}) end
  end,.5)
end
function onPlayerConnect() refreshUI() end
function onPlayerChangeColor() refreshUI() end
