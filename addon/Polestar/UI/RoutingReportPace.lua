-- The routing report's pace section: where the character stands (level, XP rate, time to level), the guide's own
-- pace against its level targets, the Fastest XP switch and the levers that were changed by hand, and the route
-- advice card: pacing, and each quest it judged with the reason it drew advice or none.
-- Read only; quest ids, never titles.

local ADDON, ns = ...
local UI = ns.UI
local RR = UI.RoutingReport
local F = UI.ReportFormat
local floor = math.floor

local SEP = " \194\183 "
local ADVICE_ROWS = 8

local function duration(seconds)
	local Q = ns.QoL
	if Q and Q.Duration then return Q.Duration(seconds) end
	return F.clock(seconds)
end

-- The window's pace line, without its colour codes.
local function paceText(stats)
	local GF = UI.GuideFrame
	local ok, text = pcall(function() return GF and GF.PaceText and GF.PaceText(stats) end)
	if not (ok and type(text) == "string") then return nil end
	return (text:gsub("|c%x%x%x%x%x%x%x%x", ""):gsub("|r", ""))
end

local function xpLines(add)
	local Q = ns.QoL
	local stats = Q and Q.GetXPStats and Q.GetXPStats()
	if type(stats) ~= "table" then return end
	local parts = { ("level %d, %s into it"):format(stats.level or 0, F.percent(stats.percent or 0)) }
	if stats.maxed then
		add("Level pace: " .. parts[1] .. SEP .. "at the top")
		return
	end
	parts[#parts + 1] = ("%d XP/h now, %d for the session"):format(floor((stats.rollingRate or 0) + 0.5), floor((stats.sessionRate or 0) + 0.5))
	if stats.timeToLevel then parts[#parts + 1] = duration(stats.timeToLevel) .. " to level at that rate" end
	parts[#parts + 1] = (stats.restedPercent or 0) > 0 and (F.percent(stats.restedPercent) .. " rested") or nil
	add("Level pace: " .. table.concat(parts, SEP))
	add(("Session: %s long%s%d XP%s%d quest%s%s%d kill%s%s%s a level so far"):format(F.clock(stats.sessionSeconds or 0), SEP, stats.sessionXP or 0,
		SEP, stats.sessionQuests or 0, stats.sessionQuests == 1 and "" or "s", SEP, stats.sessionKills or 0, stats.sessionKills == 1 and "" or "s",
		SEP, F.clock(stats.levelSeconds or 0)))
	add(("Averages: a kill pays %s XP%sa quest pays %s XP"):format(stats.avgKillXP and floor(stats.avgKillXP + 0.5) or "?", SEP,
		stats.avgQuestXP and floor(stats.avgQuestXP + 0.5) or "?"))
	local text = paceText(stats)
	if text then add("Guide pace: " .. text) end
	local L = ns.Levels
	local pace = L and L.Pace and L.Pace()
	if pace then
		add(("Guide pace numbers: target level %s at %s%sthe steps up to it leave you at level %s%s%s levels short (%d bars, %d XP)%skind %s"):format(
			F.fixed(pace.target, pace.target % 1 == 0 and 0 or 1), RR.StepName(pace.step), SEP, F.fixed(pace.reach, 2), SEP, F.fixed(pace.short, 2),
			pace.bars or 0, floor((pace.xp or 0) + 0.5), SEP, tostring(pace.kind)))
	end
end

local function fastestLines(add)
	local FX = ns.FastestXP
	if not FX then return end
	local state = FX.State()
	local line = "Fastest XP: " .. tostring(state)
	local changed = state ~= "off" and FX.Changed and FX.Changed() or {}
	if #changed > 0 then
		local keys = {}
		for i, lever in ipairs(changed) do keys[i] = tostring(lever.key or lever) end
		line = line .. SEP .. "changed by hand since the mode wrote them: " .. table.concat(keys, ", ")
	end
	if FX.Hardcore and FX.Hardcore() then line = line .. SEP .. "Hardcore: the arrow's own pick and deathskips keep your values" end
	add(line)
end

local function adviceLines(add)
	local RAC, RA, E = UI.RouteAdviceCard, ns.RouteAdvice, ns.Engine
	if not (RAC and RA) then return end
	local enabled = RAC.Enabled()
	add(("Route advice: %s%scard %s"):format(F.onOff(ns.Get("behavior.routeAdvice") == true), SEP, enabled and "can show" or "is off (needs the setting on and a level-range guide)"))
	if not enabled then return end
	local pace, told = RAC.Pacing()
	local now = GetTime and GetTime() or 0
	local lastAt = pace.last
	local hour = 0
	for _, t in ipairs(pace.times or {}) do
		if now - t < 3600 then hour = hour + 1 end
	end
	local C = ns.Compat
	local log = #(C and C.GetLogQuests and C.GetLogQuests() or {})
	local told_n = 0
	for _ in pairs(told) do told_n = told_n + 1 end
	add(("Route advice pacing: %s%s%d card%s in the last hour (at most %d)%sat least %d s apart%s%d quest%s told this session%squest log %d, so a detour over %d s is off course"):format(
		lastAt and ("last card " .. F.ago(lastAt)) or "no card yet", SEP, hour, hour == 1 and "" or "s", RA.PER_HOUR, SEP, RA.GAP, SEP, told_n,
		told_n == 1 and "" or "s", SEP, log, RA.Limit(log)))
	local ok, entries = pcall(RAC.Entries)
	if not ok then
		add("Route advice entries: could not be read: " .. tostring(entries))
		return
	end
	if #entries == 0 then
		add("Route advice entries: no quest is off course, listed as slow, or skipped earlier")
		return
	end
	local verdict = {}
	for _, a in ipairs(RA.Judge(entries, log)) do verdict[a.entry] = a.verdict end
	for i, e in ipairs(entries) do
		if i > ADVICE_ROWS then
			add(("  and %d more"):format(#entries - ADVICE_ROWS))
			break
		end
		local detour = e.seconds and ("adds %d s to the stops ahead"):format(floor(e.seconds + 0.5)) or "detour not measured"
		local result = verdict[e] and ("advice: " .. verdict[e]) or ("no advice: " .. tostring(RA.Why(e, log) or "waiting for pacing or a card slot"))
		add(("  quest %s (%s)%s%s%s%s"):format(tostring(e.quest), tostring(e.kind), SEP, detour, SEP, result))
	end
end

RR.AddSection("pace", function(add)
	xpLines(add)
	fastestLines(add)
	adviceLines(add)
end, "after")
