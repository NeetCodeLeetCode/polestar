-- Fastest XP: one switch that sets the leveling settings to their fastest values.
--
-- behavior.fastestXp (a toggle, off by default). Turning it on writes each lever's value into the real setting
-- (FX.LEVERS), after saving the value it replaces (fastest.undo), so the Options rows show what is in use and the
-- engine reads the same keys it always read. Nothing in the engine knows the mode exists.
--
--   FX.On() -> bool                       the switch
--   FX.State() -> "off" | "on" | "customized"   "customized": a setting the mode wrote holds another value now
--   FX.Changed() -> { lever, ... }        the levers whose setting the player changed since the mode wrote it
--   FX.Apply(fresh?) -> { { lever, from, to }, ... }   write the values (fresh: forget the saved ones first)
--   FX.Reapply() -> list                  write the values again, keeping what was saved at turn on
--   FX.Release() -> list                  put back the saved value of each setting still at the mode's value
--   FX.Values(skip?) -> { [key] = value } the values the mode writes now, none of a group in `skip` ({ aim = true })
--
-- A hand change leaves the mode on and reads "customized". Turning it off puts back only the settings the player did
-- not change since. A character on Hardcore keeps `hardcore = "leave"` levers as they are (the arrow's own pick of a
-- nearby objective, deathskips). Safety that does not depend on this file stays where it was: class quests and
-- chains are never skipped, a quest in the log stays, deathskips stop at level 10 (Core/Engine.lua).

local ADDON, ns = ...
local L = ns.L
local ipairs, pairs, type, tostring = ipairs, pairs, type, tostring

local FX = {}
ns.FastestXP = FX

ns.RegisterDefaults("behavior", { fastestXp = false })

-- The settings the mode writes, in the order the chat lists them. group: what the speed test takes out together.
-- hardcore = "leave": a Hardcore character keeps its own value. A key no module registered (its addon is not
-- loaded) is passed over.
FX.LEVERS = {
	{ key = "behavior.whileHere", value = true, group = "here", label = L["Dynamic Questing"] },
	{ key = "behavior.whileHereRadius", value = 180, group = "here", label = L["How close"] },
	{ key = "behavior.whileHereTurnins", value = true, group = "here", label = L["Include nearby hand-ins"] },
	{ key = "behavior.whileHereTurninSeconds", value = 20, group = "here", label = L["Hand-ins: how far out of your way"] },
	{ key = "behavior.whileHereFlights", value = true, group = "here", label = L["Flight paths you pass"] },
	{ key = "behavior.whileHereVendors", value = true, group = "here", label = L["Sell and repair reminders"] },
	{ key = "behavior.whileHereSellSlots", value = 4, group = "here", label = L["Sell: fewer free bag slots than"] },
	{ key = "behavior.whileHereRepairPercent", value = 30, group = "here", label = L["Repair: gear worn below"] },
	{ key = "behavior.whileHereAim", value = true, group = "aim", label = L["Take nearby objectives"], hardcore = "leave" },
	{ key = "behavior.whileHereAimSeconds", value = 15, group = "aim", label = L["Objectives: how far out of your way"], hardcore = "leave" },
	{ key = "behavior.stepReorder", value = true, group = "reorder", label = L["Reorder the next steps"] },
	{ key = "behavior.stepReorderXp", value = true, group = "reorderXp", label = L["Order the next steps by XP"] },
	{ key = "behavior.autoAccept", value = true, group = "automation", label = L["Auto-accept"] },
	{ key = "behavior.autoTurnin", value = true, group = "automation", label = L["Auto turn-in"] },
	{ key = "behavior.skipGrey", value = true, group = "skips", label = L["Skip grey quests"] },
	{ key = "behavior.skipSlow", value = true, group = "skips", label = L["Skip slow quests"] },
	{ key = "behavior.rareDrops", value = "hide", group = "skips", label = L["Quests started by a rare drop"],
		names = { optional = L["Optional"], chase = L["Chase them"], hide = L["Hide them"] } },
	{ key = "behavior.routeAdvice", value = true, group = "advice", label = L["Route advice"] },
	{ key = "behavior.deathskips", value = true, group = "deathskips", label = L["Use deathskips"], hardcore = "leave" },
	{ key = "nav.travel", value = true, group = "travel", label = L["Plan travel routes"] },
}

local UNDO = "fastest.undo" -- path -> the value each written setting held before (saved with the settings)
local busy = false -- the mode is writing: its own changes are no turn on or off

local function hardcore()
	local rules = C_GameRules
	return rules and rules.IsHardcoreActive and rules.IsHardcoreActive() and true or false
end

-- The levers in force now: registered, and not left alone on this character.
local function levers(skip)
	local hc, out = hardcore(), {}
	for _, lv in ipairs(FX.LEVERS) do
		if ns.GetDefault(lv.key) ~= nil and not (hc and lv.hardcore == "leave") and not (skip and skip[lv.group]) then
			out[#out + 1] = lv
		end
	end
	return out
end

function FX.Values(skip)
	local out = {}
	for _, lv in ipairs(levers(skip)) do out[lv.key] = lv.value end
	return out
end

-- Whether this character plays Hardcore (the levers marked hardcore = "leave" are not written; Core/NextStop.lua's filter).
FX.Hardcore = hardcore

function FX.On() return ns.Get("behavior.fastestXp") and true or false end

function FX.Changed()
	local out = {}
	for _, lv in ipairs(levers()) do
		if ns.Get(lv.key) ~= lv.value then out[#out + 1] = lv end
	end
	return out
end

function FX.State()
	if not FX.On() then return "off" end
	return #FX.Changed() > 0 and "customized" or "on"
end

local function copyOf(t)
	local out = {}
	if type(t) == "table" then for k, v in pairs(t) do out[k] = v end end
	return out
end

function FX.Apply(fresh)
	local undo = fresh and {} or copyOf(ns.Get(UNDO))
	local done = {}
	busy = true
	for _, lv in ipairs(levers()) do
		local now = ns.Get(lv.key)
		if undo[lv.key] == nil then undo[lv.key] = now end
		if now ~= lv.value then
			ns.Set(lv.key, lv.value)
			done[#done + 1] = { lever = lv, from = now, to = lv.value }
		end
	end
	ns.Set(UNDO, undo)
	busy = false
	return done
end

function FX.Reapply()
	return FX.Apply(false)
end

function FX.Release()
	local undo = ns.Get(UNDO)
	local done = {}
	busy = true
	if type(undo) == "table" then
		for _, lv in ipairs(FX.LEVERS) do
			local before = undo[lv.key]
			if before ~= nil and ns.GetDefault(lv.key) ~= nil and ns.Get(lv.key) == lv.value then
				ns.Set(lv.key, before)
				done[#done + 1] = { lever = lv, from = lv.value, to = before }
			end
		end
	end
	ns.Set(UNDO, nil)
	busy = false
	return done
end

---------------------------------------------------------------------------
-- What the chat says
---------------------------------------------------------------------------

local function valueText(lv, v)
	if lv.names and lv.names[v] then return lv.names[v] end
	if type(v) == "boolean" then return v and L["on"] or L["off"] end
	return tostring(v)
end

local function lines(done)
	for _, d in ipairs(done) do
		ns.Print(("  %s: %s to %s"):format(d.lever.label, valueText(d.lever, d.from), valueText(d.lever, d.to)))
	end
end

local function say(done)
	if #done == 0 then
		ns.Print(L["Fastest XP is on. Every setting it sets already had its value."])
	else
		ns.Print((L["Fastest XP is on. It changed %d settings:"]):format(#done))
		lines(done)
	end
	if hardcore() then
		ns.Print(L["Hardcore character: Fastest XP leaves Take nearby objectives and Use deathskips as they are."])
	end
	FX.SayScope()
end

-- The open guide decides where the mode works: a level-range guide only (Engine Own.fastOut).
function FX.SayScope()
	local E = ns.Engine
	if E and E.IsLeveling and E.GetGuide and E.GetGuide() and not E.IsLeveling() then
		ns.Print(L["The open guide is not a level-range guide, so Fastest XP leaves its arrow and step order as the guide has them."])
	end
end

function FX.Status()
	local state = FX.State()
	if state == "off" then
		ns.Print(L["Fastest XP is off. /polestar fast on sets the leveling settings to their fastest values."])
		return
	end
	local changed = FX.Changed()
	if state == "on" then
		ns.Print(L["Fastest XP is on. Every setting it sets has its value."])
		FX.SayScope()
		return
	end
	ns.Print((L["Fastest XP is on, with %d settings you changed:"]):format(#changed))
	for _, lv in ipairs(changed) do
		ns.Print(("  %s: %s (Fastest XP: %s)"):format(lv.label, valueText(lv, ns.Get(lv.key)), valueText(lv, lv.value)))
	end
	ns.Print(L["/polestar fast restore puts them back."])
	FX.SayScope()
end

---------------------------------------------------------------------------
-- The switch
---------------------------------------------------------------------------

-- The switch changed (the Options row, Quick settings, the command or a profile string that carried it alone):
-- on writes the values, off puts the old ones back. A profile switch or a batch of settings says "" and does neither.
ns.On("SETTING_CHANGED", function(path, value)
	if busy or path ~= "behavior.fastestXp" then return end
	if value then
		say(FX.Apply(true))
	else
		local saved = type(ns.Get(UNDO)) == "table"
		local done = FX.Release()
		if saved then
			ns.Print(L["Fastest XP is off. The settings it changed are back as they were, except the ones you changed since."])
		else
			ns.Print(L["Fastest XP is off. It has no saved values (the switch came with a profile), so the settings stay as they are."])
		end
		if #done > 0 then lines(done) end
	end
end)

ns.RegisterCommand("fast", function(rest)
	local word = (rest or ""):lower()
	if word == "on" then
		if FX.On() then FX.Status() else ns.Set("behavior.fastestXp", true) end
	elseif word == "off" then
		if FX.On() then ns.Set("behavior.fastestXp", false) else FX.Status() end
	elseif word == "restore" then
		if not FX.On() then
			FX.Status()
		else
			local done = FX.Reapply()
			ns.Print((L["Fastest XP put %d settings back."]):format(#done))
			lines(done)
		end
	else
		FX.Status()
	end
end, L["Fastest XP: on, off, restore, or no word for its state"], true)

-- Quick settings (UI/QuickSettings.lua) lists the switch first, and the arrow's Recalibrate under it. Its rows are
-- built when it opens.
ns.OnInit(function()
	local QS = ns.UI and ns.UI.QuickSettings
	if not (QS and QS.ROWS) then return end
	table.insert(QS.ROWS, 1, { kind = "toggle", key = "behavior.fastestXp", label = L["Fastest XP"],
		help = L["Sets the leveling settings to their fastest values. Off puts back what it changed."] })
	table.insert(QS.ROWS, 2, { kind = "button", id = "recalibrate", label = L["Recalibrate the arrow"], onClick = function()
		local E = ns.Engine
		local line = E and E.Recalibrate and E.Recalibrate()
		if line then ns.Print(line) end
	end })
end)
