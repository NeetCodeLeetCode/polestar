-- The arrow's log: who owns the arrow now, and the last few times it changed hands. The routing report
-- (UI/RoutingReportArrow.lua) reads it; nothing in the arrow reads it back.
--
--   Nav.ArrowLog.Describe(target) -> { source, text, label, map, x, y, kind, quest, step, goal, trip? }
--       source: "none" | "held" | "trip" | "waypoint" | "point" | "objective" | "picked" | "guide"
--         held       the way back to a corpse (Nav.Hold)
--         trip       a place the player asked to be taken to (Nav.StartTrip): a Next stop, a trainer, a dungeon
--         waypoint   a point the player picked on a map or with /way
--         point      any other point a card or a button set by hand (Nav.SetTarget) that is no trip
--         objective  an objective line the arrow took by itself (Take nearby objectives)
--         picked     a goal the player picked by a row, a click or a waypoint key
--         guide      the guide's own next goal
--   Nav.ArrowLog.Entries() -> { entry, ... }   oldest first; an entry is a Describe table plus
--       at (GetTime), clock ("14:05:09" or nil), why (the event just before it, or nil), count (the same
--       arrow announced again), last (the time of the latest announcement)
--   Nav.ArrowLog.TripAge(trip) -> seconds | nil   how long a trip has run
--   Nav.ArrowLog.Because(text)   what the next change is for: set by the events below, and by a caller that
--       knows more (the Next stop card)
--   Nav.ArrowLog.Clear()         forget the log (the tests)
--
-- Plain tables only; a missing module (no Engine, no Nav) leaves its part out.

local ADDON, ns = ...
local Nav = ns.Nav
if not Nav then return end

local AL = {}
Nav.ArrowLog = AL

AL.MAX = 16 -- entries kept
AL.REASON_SECS = 2 -- a reason older than this belongs to an earlier change

local log, count = {}, 0
local because = { at = nil, text = nil }
local tripAt = setmetatable({}, { __mode = "k" }) -- trip -> GetTime when it started

local function now() return GetTime and GetTime() or 0 end

local function engine() return ns.Engine end

-- What the Engine says of whose goal the arrow leads to: "auto", "picked" or "guide".
local function engineFrom()
	local E = engine()
	local st = E and E.RoutingState and E.RoutingState()
	return st and st.arrowFrom or "guide"
end

local TEXT = {
	none = "no arrow",
	held = "held on a point (the way back to a corpse)",
	waypoint = "your own waypoint (a point you picked)",
	point = "a point set by a card or a button, not a trip",
	objective = "an objective the arrow took by itself (Take nearby objectives)",
	picked = "a goal picked by a row, a click or a waypoint key",
	guide = "the guide's own next goal",
}

function AL.Describe(target)
	local d = { source = "none" }
	if not target then
		if Nav.IsHeld and Nav.IsHeld() then d.source = "held" end
		d.text = TEXT[d.source]
		return d
	end
	d.label, d.map, d.x, d.y, d.kind, d.quest = target.label, target.map, target.x, target.y, target.kind, target.quest
	d.step, d.goal = target.stepIndex, target.goalIndex
	if Nav.IsHeld and Nav.IsHeld() and not (Nav.IsOverride and Nav.IsOverride()) and not (engine() and engine().GetWaypoint and engine().GetWaypoint() == target) then
		d.source = "held"
	elseif Nav.IsOverride and Nav.IsOverride() then
		local trip = Nav.Trip and Nav.Trip()
		if trip then
			d.source, d.trip = "trip", trip
			d.text = ("a trip: %s to %s"):format(tostring(trip.label or "?"), tostring(trip.name or "?"))
			return d
		end
		d.source = target.mine and "waypoint" or "point"
	else
		d.source = engineFrom()
		if d.source == "auto" then d.source = "objective" end
		if not TEXT[d.source] then d.source = "guide" end
	end
	d.text = TEXT[d.source]
	return d
end

local function key(d)
	return table.concat({ d.source, tostring(d.map), tostring(d.x), tostring(d.y), tostring(d.step), tostring(d.goal), tostring(d.label) }, "|")
end

local function reasonNow(t, source)
	if because.text and because.at and t - because.at <= AL.REASON_SECS then return because.text end
	if source == "objective" then
		local E = engine()
		local st = E and E.RoutingState and E.RoutingState()
		local a = st and st.aim
		if a and a.why and (a.ago or 0) <= AL.REASON_SECS then return a.why end
	end
	return nil
end

-- Writes the entry of a change of target that happened at time `t`.
local function record(target, t)
	local d = AL.Describe(target)
	local k = key(d)
	local last = log[count]
	if last and last.key == k then
		last.count, last.last = last.count + 1, t
		return
	end
	d.key, d.at, d.last, d.count = k, t, t, 1
	d.why = reasonNow(t, d.source)
	d.clock = date and date("%H:%M:%S") or nil
	if count >= AL.MAX then
		table.remove(log, 1)
		count = count - 1
	end
	count = count + 1
	log[count] = d
end

-- A change of target: one entry, or one more count on the entry it repeats. Written a frame later: describing the
-- arrow asks Nav.Trip, which drops a trip another target replaced without saying so, and Nav's own check for that
-- (it fires NAV_TRIP) comes right after this call. A trip is set as a point first, so by then it is a trip.
function AL.OnTarget(target)
	local t = now()
	ns.After(0, function() record(target, t) end)
end

function AL.Because(text)
	because.at, because.text = now(), text
end

function AL.Entries()
	local out = {}
	for i = 1, count do out[i] = log[i] end
	return out
end

function AL.TripAge(trip)
	local at = trip and tripAt[trip]
	return at and now() - at or nil
end

function AL.Clear()
	for i = 1, count do log[i] = nil end
	count = 0
	because.at, because.text = nil, nil
end

-- A trip began or ended (Nav.StartTrip, Nav.EndTrip, another target): the reason of the entry written next.
ns.On("NAV_TRIP", function(trip)
	if trip then
		tripAt[trip] = now()
		AL.Because(("a trip started: %s to %s"):format(tostring(trip.label or "?"), tostring(trip.name or "?")))
	else
		AL.Because("the trip ended (arrived, or Back to the guide)")
	end
end)
ns.On("STEP_CHANGED", function(step) AL.Because("the guide moved to step " .. tostring(step)) end)
ns.On("RECALIBRATED", function() AL.Because("Recalibrate") end)
ns.On("PLAYER_SKIPPED_STEP", function() AL.Because("a step was skipped") end)

Nav.RegisterModule({ OnTarget = function(_, target) AL.OnTarget(target) end })
