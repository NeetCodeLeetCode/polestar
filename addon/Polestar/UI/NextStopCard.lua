-- The next-stop heads-up (behavior.nextStop; the ranking: Core/NextStop.lua, the places: Core/NextStopPlaces.lua).
-- A card on the guide window, as the Liminal Passage card's, that names the best place for XP for each minute
-- when there is nothing left to do here, with "Go": the arrow takes the trip (Nav.StartTrip, so "Back to the
-- guide" ends it). It never changes the guide or the player's ticks, and never moves the arrow by itself.
--
-- Shown only while Fastest XP is on, in a level-range guide, with the guide's own arrow at least
-- NSC.NEAR_SECS away (nearer, the guide has work here), never in combat or while Polestar is hidden, and once
-- for each place and level. A Hardcore character gets the filter of Core/NextStop.lua.
--
--   UI.NextStopCard.Look()     work out the ranking now and show the card when it is due
--   UI.NextStopCard.Enabled() -> bool   the setting, Fastest XP on and a level-range guide
--   UI.NextStopCard.Model(entry) -> card model   entry: an entry of NextStop.Rank's ranked list

local ADDON, ns = ...
local UI = ns.UI
local L = ns.L
local format = string.format

local NSC = {}
UI.NextStopCard = NSC

NSC.KIND = "nextstop"
NSC.ICON = "Interface\\AddOns\\" .. ADDON .. "\\Media\\Map\\icon-flightmaster"
NSC.EVERY = 20 -- seconds between looks
NSC.NEAR_SECS = 300 -- the guide's arrow this near: the guide has work here, so no card

ns.RegisterDefaults("behavior", { nextStop = true })

local told = {} -- "level:place" -> true: the places already shown

-- Forget the places told (a new session of the card; the tests).
function NSC.Reset() told = {} end

local function CT() return UI.ClassToast end

local function duration(seconds)
	local Q = ns.QoL
	return Q and Q.Duration and Q.Duration(seconds) or format(L["%ds"], seconds)
end

local function turnOff()
	ns.Set("behavior.nextStop", false)
	ns.Print(L["Next-stop tips are off. Turn them back on under Settings > Guide > Automation > Fastest XP."])
end

local function go(place)
	local Nav = ns.Nav
	if not (Nav and Nav.SetTarget and Nav.StartTrip) then return false end
	local target = Nav.SetTarget(place.map, place.x, place.y, place.name, place.f)
	if not target then return false end
	Nav.StartTrip(target, place.name, L["Next stop"])
	local T = CT()
	if T and T.Hide then T.Hide() end
	return true
end

function NSC.Model(e)
	local place, NS = e.place, ns.NextStop
	local m = { kind = NSC.KIND, icon = NSC.ICON, iconColor = "accent", titleColor = "text", head = L["NEXT STOP"] }
	if place.kind == "dungeon" then
		m.title = format(L["Go do %s now"], place.name)
		m.giver = format(L["%d quests inside are ready for you. About %.0f%% of a level in %s, not counting kills."], place.count or e.quests, e.xp, duration(e.minutes * 60))
	else
		m.title = format(L["Go to %s"], place.name)
		local where = place.kind == "zone" and place.name or format(L["%s, in %s"], place.name, place.zone)
		m.giver = format(L["%d quests for your level at %s. About %.0f%% of a level in %s."], e.quests, where, e.xp, duration(e.minutes * 60))
	end
	if e.reason == NS.LEAVE then
		m.route = L["This pays much better than staying here."]
	else
		m.route = L["Nothing near you pays more for the time."]
	end
	m.act = { label = L["Go"], run = function() return go(place) end }
	m.off = { label = L["Turn these off"], run = turnOff }
	return m
end

function NSC.Enabled()
	local FX = ns.FastestXP
	local E = ns.Engine
	return ns.Get("behavior.nextStop") == true and FX ~= nil and FX.On() and E ~= nil and E.IsLeveling ~= nil and E.IsLeveling() == true
end

-- The guide's own arrow is near: its work is here.
local function guideHasWork()
	local E = ns.Engine
	local wp = E and E.GetWaypoint and E.GetWaypoint()
	if not (wp and wp.map and wp.x and wp.y) then return false end
	local secs = ns.TravelCost and ns.TravelCost(nil, { map = wp.map, x = wp.x, y = wp.y, f = wp.f })
	return secs ~= nil and secs < NSC.NEAR_SECS
end

local function hideOurs()
	local T = CT()
	if T and T.current and T.current.kind == NSC.KIND then T.Hide() end
end

function NSC.Look()
	if not NSC.Enabled() then return hideOurs() end
	local T = CT()
	if not (T and T.ShowCard) or ns.Hidden() or InCombatLockdown() then return end
	if guideHasWork() then return hideOurs() end
	local Places, NS = ns.NextStop.Places, ns.NextStop
	if not Places.Ready() then return Places.Scan(NSC.Look) end
	local places, here = Places.Build()
	if not places then return end
	local p = ns.Lookup.PlayerInfo()
	local r = NS.Rank({ level = p.level, hardcore = ns.FastestXP.Hardcore(), group = GetNumGroupMembers and GetNumGroupMembers() or 1,
		dungeons = ns.Get("behavior.dungeons"), here = here }, places)
	local top = r.top
	if not top or top.reason == NS.STAY then return hideOurs() end
	local key = format("%d:%s", p.level, tostring(top.place.id))
	if told[key] then return end
	local cur = T.current
	if cur and cur.kind ~= NSC.KIND then return end
	if T.ShowCard(NSC.Model(top)) then told[key] = true end
end

-- The next look: on a level up (the scan is for a level), a Recalibrate, a new zone and every NSC.EVERY seconds.
ns.On("RECALIBRATED", NSC.Look)
ns.RegisterEvent("PLAYER_LEVEL_UP", function() ns.After(1, NSC.Look) end)
ns.RegisterEvent("ZONE_CHANGED_NEW_AREA", function() ns.After(1, NSC.Look) end)
ns.OnInit(function() ns.NewTicker(NSC.EVERY, NSC.Look) end)
