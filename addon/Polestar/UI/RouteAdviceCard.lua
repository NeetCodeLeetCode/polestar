-- Route advice (behavior.routeAdvice; the judge: Core/RouteAdvice.lua, docs/ROUTE-ADVICE.md).
-- A card on the guide window, as the next-stop card's, for three cases, all from route distance:
--   off course  a quest in the log that no guide of the route uses, and that adds more than RA.LIMIT seconds to the
--               guide's next stops: "We recommend abandoning it". Abandon asks through the game's own confirmation.
--   missed      a quest placement passed as skipped earlier: "You did not pick up X", take it when its giver is
--               near the route, leave it when far (never leave one a later accept needs).
--   listed      a quest in the log that the hand-written list of slow quests names (Core/SlowQuestList.lua).
-- It never abandons, takes or skips by itself. Shown only while the setting is on (Fastest XP turns it on), in a
-- level-range guide, on events (a level up, a new zone, Recalibrate, a log of 36 quests, once after login), never in
-- combat or while Polestar is hidden, one card at a time, at most RA.PER_HOUR an hour and RA.GAP seconds apart.
-- A quest is told once for each level (one card for it, whatever the verdict). Keep it is remembered for the character; closing the card is no answer.
--
--   UI.RouteAdviceCard.Look()           gather, judge and show the card when one is due
--   UI.RouteAdviceCard.Enabled() -> bool
--   UI.RouteAdviceCard.Entries() -> { entry, ... }   what the judge reads (Core/RouteAdvice.lua)
--   UI.RouteAdviceCard.Model(advice) -> card model

local ADDON, ns = ...
local UI = ns.UI
local L = ns.L
local format = string.format

local RAC = {}
UI.RouteAdviceCard = RAC

RAC.KIND = "routeadvice"
RAC.ICON_STOP = "Interface\\AddOns\\" .. ADDON .. "\\Media\\Map\\icon-stop"
RAC.ICON_QUEST = "Interface\\AddOns\\" .. ADDON .. "\\Media\\Map\\icon-quest-available"
RAC.LOGIN_WAIT = 30 -- seconds after login to the first look

ns.RegisterDefaults("behavior", { routeAdvice = false })

local told = {}  -- "quest:level" -> true: the quest was told this level, whatever the verdict
local taken = {} -- quest -> seconds (GetTime) it was accepted this session
local pace = { times = {} }

-- Forget what was told and the pacing (a new session of the card; the tests).
function RAC.Reset() told, taken, pace = {}, {}, { times = {} } end

-- The pacing state and the places told, for the routing report: { last, times }, and "quest:level" keys.
function RAC.Pacing() return pace, told end

local function CT() return UI.ClassToast end
local function now() return GetTime and GetTime() or 0 end

local function duration(seconds)
	local Q = ns.QoL
	return Q and Q.Duration and Q.Duration(seconds) or format(L["%ds"], seconds)
end

-- The quests the character said to keep (per character).
local function keepSet()
	local st = ns.CharState()
	if type(st.routeAdvice) ~= "table" then st.routeAdvice = {} end
	return st.routeAdvice
end

local function keep(q)
	keepSet()[q] = true
	ns.Print(L["Kept. Route advice will not mention that quest again."])
end

local function progressOf(q)
	local have, need = 0, 0
	for _, o in ipairs(ns.Compat.GetQuestObjectives(q) or {}) do
		have, need = have + (o.numFulfilled or 0), need + (o.numRequired or 0)
	end
	return need > 0 and have / need or 0
end

local function ageOf(q)
	return taken[q] and now() - taken[q] or nil
end

function RAC.Enabled()
	local E = ns.Engine
	return ns.Get("behavior.routeAdvice") == true and E ~= nil and E.IsLeveling ~= nil and E.IsLeveling() == true
end

-- What the judge reads: the log quests off the route, the log quests on the hand-written list, and the quests
-- placement passed as skipped earlier. Detours are measured only for a quest the rules do not already quiet.
function RAC.Entries()
	local E, C, P, List = ns.Engine, ns.Compat, ns.RoutePlan, ns.SlowQuestList
	local mine, out = keepSet(), {}
	for _, r in ipairs(P and P.OffRoute and P.OffRoute() or {}) do
		local q = r.quest
		local e = { kind = "off", quest = q, title = r.title or C.GetQuestTitle(q) or "", ready = r.ready, kept = mine[q] == true,
			progress = progressOf(q), age = ageOf(q), hold = E.AdviceHold(q) }
		if not (e.ready or e.kept or e.hold) then e.seconds = E.QuestDetour(q) end
		out[#out + 1] = e
	end
	for _, q in ipairs(List and List.Slow and C.GetLogQuests() or {}) do
		local code = List.Slow(q)
		if code then
			out[#out + 1] = { kind = "listed", quest = q, title = C.GetQuestTitle(q) or "", code = code, ready = C.IsQuestReadyForTurnIn(q) or false,
				kept = mine[q] == true, progress = progressOf(q), age = ageOf(q), hold = E.AdviceHold(q) }
		end
	end
	for _, r in ipairs(E.SkippedEarlier()) do
		local e = { kind = "missed", quest = r.quest, title = r.title, hold = E.AdviceHold(r.quest) }
		if not e.hold then e.seconds, e.needed = E.PickupDetour(r.quest), E.AdviceNeeded(r.quest) end
		out[#out + 1] = e
	end
	return out
end

local function done(fn)
	return function(...)
		local r = fn(...)
		local T = CT()
		if T and T.Hide then T.Hide() end
		return r
	end
end

function RAC.Model(a)
	local e, E, C = a.entry, ns.Engine, ns.Compat
	local m = { kind = RAC.KIND, icon = RAC.ICON_QUEST, iconColor = "accent", titleColor = "text" }
	local abandon = { label = L["Abandon"], run = done(function() return C.AskAbandonQuest(e.quest) end) }
	local stay = { label = L["Keep it"], run = done(function() return keep(e.quest) end) }
	local take = { label = L["Take it"], run = done(function() return E.TakeEarlier(e.quest) end) }
	local leave = { label = L["Leave it"], run = done(function() return E.DismissEarlier(e.quest) end) }
	if a.verdict == "abandon" then
		m.icon, m.head, m.title = RAC.ICON_STOP, L["OFF COURSE"], format(L["We recommend abandoning %s"], e.title)
		m.giver = format(L["Finishing it adds about %s to the stops ahead on your guide."], duration(e.seconds))
		m.route = L["It frees a log slot. You can also finish it when you pass by."]
		m.act, m.off = abandon, stay
	elseif a.verdict == "listed" then
		m.icon, m.head, m.title = RAC.ICON_STOP, L["SLOW QUEST"], format(L["%s is a quest players skip"], e.title)
		m.giver = E.SlowText(e.code)
		m.route = L["It costs more time than it pays. We recommend abandoning it."]
		m.act, m.off = abandon, stay
	elseif a.verdict == "take" then
		m.head, m.title = L["MISSED PICKUP"], format(L["You did not pick up %s"], e.title)
		m.giver = format(L["Its giver adds about %s to the stops ahead on your guide."], duration(e.seconds))
		m.route = L["That is close to your route. We recommend taking it."]
		m.act, m.off = take, leave
	else
		m.head, m.title = L["MISSED PICKUP"], format(L["You did not pick up %s"], e.title)
		m.giver = format(L["Its giver adds about %s to the stops ahead on your guide."], duration(e.seconds))
		m.route = L["That is far from your route. We recommend leaving it."]
		m.act, m.off = leave, take
	end
	return m
end

local function hideOurs()
	local T = CT()
	if T and T.current and T.current.kind == RAC.KIND then T.Hide() end
end

function RAC.Look()
	if not RAC.Enabled() then return hideOurs() end
	local T = CT()
	if not (T and T.ShowCard) or ns.Hidden() or InCombatLockdown() or T.current then return end
	local t, RA = now(), ns.RouteAdvice
	if not RA.Allowed(pace, t) then return end
	local p = ns.Lookup.PlayerInfo()
	local log = #(ns.Compat.GetLogQuests() or {})
	for _, a in ipairs(RA.Judge(RAC.Entries(), log)) do
		local key = format("%d:%d", a.entry.quest, p.level)
		if not told[key] then
			if T.ShowCard(RAC.Model(a)) then
				told[key] = true
				RA.Record(pace, t)
			end
			return
		end
	end
end

-- The next look: a level up, a new zone, a Recalibrate, a full log, and once after login. No ticker.
local function later() ns.After(2, RAC.Look) end
ns.On("RECALIBRATED", RAC.Look)
ns.RegisterEvent("PLAYER_LEVEL_UP", later)
ns.RegisterEvent("ZONE_CHANGED_NEW_AREA", later)
ns.RegisterEvent("QUEST_ACCEPTED", function(_, a, b)
	local q = type(b) == "number" and b or a
	if type(q) ~= "number" then return end
	taken[q] = now()
	if #(ns.Compat.GetLogQuests() or {}) >= ns.RouteAdvice.CROWDED_LOG then later() end
end)
ns.OnInit(function() ns.After(RAC.LOGIN_WAIT, RAC.Look) end)
