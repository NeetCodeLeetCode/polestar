-- The routing report's arrow section: who owns the arrow, what each owner holds, how far the guide's own goal
-- is, and the last times the arrow changed hands (Nav/ArrowLog.lua). It answers "why does the arrow point there".
-- Read only: it takes the numbers from Nav, the Engine and the log, never from a guess.

local ADDON, ns = ...
local UI = ns.UI
local RR = UI.RoutingReport
local F = UI.ReportFormat
local floor = math.floor

local SEP = " \194\183 "

local function nav() return ns.Nav end
local function log() return nav() and nav().ArrowLog end

-- "label · place · distance" for a target table of the arrow or the Engine.
local function targetText(t)
	local parts = { tostring(t.label or "?") }
	parts[#parts + 1] = F.point(t.map, t.x, t.y, t.f)
	parts[#parts + 1] = F.away(t.map, t.x, t.y, t.f)
	return table.concat(parts, SEP)
end

-- "step 58 goal 2 (turnin, quest 1234)"
local function goalText(t)
	local parts = {}
	if t.stepIndex then
		parts[#parts + 1] = RR.StepName(t.stepIndex) .. (t.goalIndex and (" goal " .. t.goalIndex) or "")
	end
	local what = {}
	if t.kind then what[#what + 1] = tostring(t.kind) end
	if t.quest then what[#what + 1] = "quest " .. tostring(t.quest) end
	if #what > 0 then parts[#parts + 1] = "(" .. table.concat(what, ", ") .. ")" end
	return table.concat(parts, " ")
end

local function chainLine(add, n, name, state, leads)
	add(("Arrow chain %d, %s: %s%s"):format(n, name, state, leads and (SEP .. "THE ARROW FOLLOWS THIS") or ""))
end

local function sourceLines(add)
	local N, E, AL = nav(), ns.Engine, log()
	if not (N and AL) then return end
	local target = N.GetTarget and N.GetTarget()
	local d = AL.Describe(target)
	local line = "Arrow source: " .. d.text
	if d.source == "trip" and d.trip then
		local age = AL.TripAge(d.trip)
		line = line .. (age and (SEP .. "started " .. F.clock(age) .. " ago") or "")
	end
	add(line)
	if target then
		add("Arrow target: " .. targetText(target) .. (d.source == "guide" and "" or (SEP .. goalText(target))))
		local state, dist = N.Sample and N.Sample()
		if state then add(("Arrow state: %s%s"):format(tostring(state), type(dist) == "number" and (SEP .. F.yards(dist) .. " to the next point of its route") or "")) end
	else
		local why = "no target"
		local paused = ns.Paused and ns.Paused()
		if paused then why = "paused (" .. tostring(paused) .. ")" end
		if N.HandedOff and N.HandedOff() then why = "handed to Shortest Path Forever" end
		add("Arrow target: none" .. SEP .. why)
	end
	-- The guide's own goal, and where the arrow is against it.
	local wp = E and E.GetWaypoint and E.GetWaypoint()
	if wp then
		add("Guide goal: " .. goalText(wp) .. SEP .. targetText(wp))
		if target and target ~= wp then
			local a, b = F.travel(target.map, target.x, target.y, target.f), F.travel(wp.map, wp.x, wp.y, wp.f)
			local diff = a and b and (a - b)
			add("The arrow is NOT on the guide's goal" .. (diff and (SEP .. ("the arrow's target is %d s %s than the guide's goal by the travel planner"):format(floor(math.abs(diff) + 0.5), diff >= 0 and "farther" or "nearer")) or ""))
		elseif target then
			add("The arrow is on the guide's goal")
		end
	else
		add("Guide goal: none (the guide has no point to lead to now)")
	end
end

-- The links of the chain in the order Nav takes them (Nav.apply: a hold, then a point set by hand, then the
-- Engine's goal), each with whether it holds the arrow now.
local function chainLines(add)
	local N, E, AL = nav(), ns.Engine, log()
	if not (N and AL) then return end
	local target = N.GetTarget and N.GetTarget()
	local d = AL.Describe(target)
	local st = E and E.RoutingState and E.RoutingState()
	local held = N.IsHeld and N.IsHeld()
	chainLine(add, 1, "held on a point (the way back to a corpse)", held and "set" or "none", d.source == "held")
	local trip = N.Trip and N.Trip()
	local point = N.IsOverride and N.IsOverride()
	local text = "none"
	if trip then
		local age = AL.TripAge(trip)
		text = ("a trip: %s to %s%s"):format(tostring(trip.label or "?"), tostring(trip.name or "?"), age and (", started " .. F.clock(age) .. " ago") or "")
	elseif point then
		text = d.source == "waypoint" and "your own waypoint" or "a point set by a card or a button"
	end
	chainLine(add, 2, "a trip or a point set by hand", text, d.source == "trip" or d.source == "waypoint" or d.source == "point")
	local from = st and st.arrowFrom or "guide"
	chainLine(add, 3, "a goal picked by a row, a click or a waypoint key", from == "picked" and "set" or "none", d.source == "picked")
	local aim = st and st.aim
	local auto = from == "auto"
	local autoText = auto and (aim and aim.held and ("holding the line of quest " .. tostring(aim.held.quest)) or "set") or ((aim and aim.on) and "none" or "none (Take nearby objectives is off)")
	chainLine(add, 4, "an objective the arrow took", autoText, d.source == "objective")
	local wp = E and E.GetWaypoint and E.GetWaypoint()
	chainLine(add, 5, "the guide's own goal", wp and goalText(wp) or "none", d.source == "guide")
	if target == nil then
		local paused = ns.Paused and ns.Paused()
		local why = paused and ("paused " .. tostring(paused)) or (wp and "a goal was dismissed with Clear target, or the arrow is hidden" or "nothing to lead to")
		add("Arrow chain: no link holds the arrow" .. SEP .. why)
	end
end

local function tripLines(add)
	local N, AL = nav(), log()
	local trip = N and N.Trip and N.Trip()
	if not (trip and AL) then return end
	local t = trip.target or {}
	add(("Trip: %s to %s%sends on %s%s"):format(tostring(trip.label or "?"), tostring(trip.name or "?"), SEP,
		trip.ends == "instance" and "going through an instance portal" or "arrival", SEP .. "also on Back to the guide, or any other target the arrow takes"))
	local age = AL.TripAge(trip)
	if age then add("Trip started: " .. F.clock(age) .. " ago") end
	if t.map then add("Trip target: " .. targetText(t)) end
end

local function bannerLine(add)
	local GF = UI.GuideFrame
	local fn = GF and GF.BannerNotice
	if not fn then return end
	local ok, kind = pcall(fn)
	add("Guide window banner: " .. ((ok and kind) and tostring(kind) or "none"))
end

local function logLines(add)
	local AL = log()
	if not AL then return end
	local entries = AL.Entries()
	if #entries == 0 then
		add("Arrow changes: none since the game loaded")
		return
	end
	add(("Arrow changes, oldest first (the last %d kept):"):format(AL.MAX))
	for _, e in ipairs(entries) do
		local parts = { e.clock and (e.clock .. " (" .. F.ago(e.at) .. ")") or F.ago(e.at), e.text or e.source }
		if e.source ~= "none" and e.label then parts[#parts + 1] = tostring(e.label) end
		if e.map then parts[#parts + 1] = F.point(e.map, e.x, e.y) end
		if e.step then parts[#parts + 1] = RR.StepName(e.step) .. (e.goal and (" goal " .. e.goal) or "") end
		if e.why then parts[#parts + 1] = "because " .. e.why end
		if (e.count or 1) > 1 then parts[#parts + 1] = ("announced %d times, last %s"):format(e.count, F.ago(e.last)) end
		add("  " .. table.concat(parts, SEP))
	end
end

RR.AddSection("arrow", function(add)
	if not log() then return end
	sourceLines(add)
	chainLines(add)
	tripLines(add)
	bannerLine(add)
	logLines(add)
end, "before")
