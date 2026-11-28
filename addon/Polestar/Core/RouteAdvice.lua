-- Route advice: what to say about the quests the player picked up or passed by (docs/ROUTE-ADVICE.md).
-- Plain numbers and tables: no game calls, no settings read, no writes. UI/RouteAdviceCard.lua gathers the
-- entries (Core/Engine.lua's QuestDetour, PickupDetour, AdviceHold) and shows the verdict.
--
-- The judge reads one thing, route distance: the seconds a stop adds to the guide's next stops. It never guesses
-- kill counts, drop chances or the time a quest takes. "Slow" comes from the hand-written list alone
-- (data/codex/slow-quests.json, Core/SlowQuestList.lua).
--
--   RA.Limit(logCount) -> seconds       the detour that makes a quest off course
--   RA.Judge(entries, logCount) -> { advice, ... }   the advice due, most useful first
--   RA.Allowed(state, now) -> bool      whether pacing lets a card show now
--   RA.Record(state, now)               note a card shown
--
-- An entry: { kind = "off" | "missed" | "listed", quest, title, seconds (a detour, nil when unknown), hold (a reason
-- no advice may be given, or nil), ready (finished), progress (0 to 1), age (seconds since taken, nil unknown),
-- kept (the player said to keep it), needed (a later accept of the guide needs it), code (the list's reason, kind
-- "listed") }.
-- An advice: { verdict = "abandon" | "take" | "leave" | "listed", entry }.

local ADDON, ns = ...

local RA = {}
ns.RouteAdvice = RA

RA.ABANDON, RA.TAKE, RA.LEAVE, RA.LISTED = "abandon", "take", "leave", "listed"

RA.LIMIT = 240 -- seconds a quest adds to the next stops before it is off course (a first guess, tuned from play notes)
RA.CROWDED_LOG = 36 -- a log this full halves the limit: every slot is worth more
RA.PROGRESS = 0.5 -- a quest more than this done is kept: the work is mostly paid
RA.MIN_AGE = 600 -- seconds: a quest taken less than this long ago is left alone
RA.GAP = 600 -- seconds between two cards
RA.PER_HOUR = 2 -- cards in any hour

function RA.Limit(logCount)
	if (logCount or 0) >= RA.CROWDED_LOG then return RA.LIMIT / 2 end
	return RA.LIMIT
end

-- Whether entry e may be told at all.
local function quiet(e)
	return e.hold ~= nil or e.kept == true or e.seconds == nil
end

local function judgeOne(e, limit)
	if e.kind == "off" then
		-- A finished quest is handed in, never abandoned: it is no advice here.
		if quiet(e) or e.ready or (e.progress or 0) > RA.PROGRESS or (e.age or math.huge) < RA.MIN_AGE then return nil end
		if e.seconds > limit then return { verdict = RA.ABANDON, entry = e } end
	elseif e.kind == "missed" then
		if quiet(e) then return nil end
		if e.seconds <= limit then return { verdict = RA.TAKE, entry = e } end
		-- A quest a later accept needs is never told to be left.
		if e.needed then return nil end
		return { verdict = RA.LEAVE, entry = e }
	elseif e.kind == "listed" then
		if e.hold ~= nil or e.kept == true or e.ready or (e.progress or 0) > RA.PROGRESS or (e.age or math.huge) < RA.MIN_AGE then return nil end
		return { verdict = RA.LISTED, entry = e }
	end
	return nil
end

-- Abandon first (the dearest detour first), then the list, then a pickup to take (the nearest first), then one to leave.
local RANK = { [RA.ABANDON] = 1, [RA.LISTED] = 2, [RA.TAKE] = 3, [RA.LEAVE] = 4 }

function RA.Judge(entries, logCount)
	local limit, out = RA.Limit(logCount), {}
	for _, e in ipairs(entries or {}) do
		local a = judgeOne(e, limit)
		if a then out[#out + 1] = a end
	end
	table.sort(out, function(a, b)
		local ra, rb = RANK[a.verdict], RANK[b.verdict]
		if ra ~= rb then return ra < rb end
		local sa, sb = a.entry.seconds or 0, b.entry.seconds or 0
		if sa ~= sb then
			if a.verdict == RA.ABANDON then return sa > sb end
			return sa < sb
		end
		return a.entry.quest < b.entry.quest
	end)
	return out
end

-- Pacing: `state` = { last = seconds | nil, times = { seconds, ... } }.
function RA.Allowed(state, now)
	if state.last and now - state.last < RA.GAP then return false end
	local n = 0
	for _, t in ipairs(state.times or {}) do
		if now - t < 3600 then n = n + 1 end
	end
	return n < RA.PER_HOUR
end

function RA.Record(state, now)
	local kept = {}
	for _, t in ipairs(state.times or {}) do
		if now - t < 3600 then kept[#kept + 1] = t end
	end
	kept[#kept + 1] = now
	state.times, state.last = kept, now
end
