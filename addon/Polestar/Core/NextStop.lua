-- Next stop: where to go next for the most XP for each minute, at the scale of a hub, a zone or a dungeon.
-- Plain numbers only: no game calls, no writes, no settings read. The caller (the card, UI/NextStopCard.lua) builds the
-- places from the game's data and passes the player's state; this file ranks them.
--
--   NS.Rank(state, places) -> { ranked = { entry, ... }, top = entry | nil, stay = entry | nil }
--
-- state:
--   level          the player's level
--   hardcore       true: leave out quests and places more than NS.HC_ABOVE levels above the player, and escorts
--   group          the players in the party, the player included
--   dungeons       "skip" | anything else: the player's answer to "Dungeons in my route"
--   here           the place the player is in (an entry of `places`), or nil: staying there is the first choice
-- places: { { id, kind = "hub" | "zone" | "dungeon", name, tripSecs, quests = { quest, ... } }, ... }
-- quest:  { xp, secs, level, escort }   xp: percent of a level the quest pays at the player's level (the caller's
--                                       number, so a grey quest is 0); secs: the time its work takes; level: its level
-- entry:  { place, xp, minutes, rate, reason, quests }   rate: percent of a level for each minute, trip and work together
--
-- Rules:
--   Finish the hub before leaving it: the top is another place only when its rate is NS.LEAVE_MARGIN times the rate of
--   staying or more. With nothing left to do here, any place with a rate beats staying.
--   A dungeon is a candidate only with NS.GROUP_MIN players, or when `dungeons` is not "skip".
--   Hardcore: the filter above, forced, whatever the caller passes.
-- The numbers are first guesses. The speed test and the in-game test set them.

local ADDON, ns = ...
local ipairs, tostring, table = ipairs, tostring, table

local NS = {}
ns.NextStop = NS

NS.LEAVE_MARGIN = 1.3 -- the next place must be this many times as good as staying
NS.GROUP_MIN = 3 -- players that make a dungeon a candidate by themselves
NS.HC_ABOVE = 2 -- levels above the player a Hardcore character takes at most

-- reason: why the entry is where it is in the ranking
NS.STAY, NS.LEAVE, NS.BEST, NS.NEXT = "stay", "leave", "best", "next"

-- A place's work: the quests that pass the player's filters, in XP and seconds.
local function work(state, place)
	local xp, secs, n = 0, 0, 0
	for _, q in ipairs(place.quests or {}) do
		local ok = (q.xp or 0) > 0
		if ok and state.hardcore then
			ok = not q.escort and (q.level or 0) <= (state.level or 0) + NS.HC_ABOVE
		end
		if ok then
			xp, secs, n = xp + q.xp, secs + (q.secs or 0), n + 1
		end
	end
	return xp, secs, n
end

local function allowed(state, place)
	if place.kind ~= "dungeon" then return true end
	return (state.group or 1) >= NS.GROUP_MIN or state.dungeons ~= "skip"
end

local function entry(state, place, trip)
	local xp, secs, n = work(state, place)
	local minutes = ((trip or place.tripSecs or 0) + secs) / 60
	return { place = place, xp = xp, minutes = minutes, rate = minutes > 0 and xp / minutes or 0, quests = n }
end

function NS.Rank(state, places)
	local ranked = {}
	for _, place in ipairs(places or {}) do
		if place ~= state.here and allowed(state, place) then
			local e = entry(state, place)
			if e.quests > 0 then ranked[#ranked + 1] = e end
		end
	end
	-- A tie goes to the shorter trip, then to the id, so the answer is the same on every machine.
	table.sort(ranked, function(a, b)
		if a.rate ~= b.rate then return a.rate > b.rate end
		if (a.place.tripSecs or 0) ~= (b.place.tripSecs or 0) then return (a.place.tripSecs or 0) < (b.place.tripSecs or 0) end
		return tostring(a.place.id) < tostring(b.place.id)
	end)
	local stay = state.here and entry(state, state.here, 0) or nil
	if stay and stay.quests == 0 then stay = nil end
	local top
	if stay then
		stay.reason = NS.STAY
		local best = ranked[1]
		if best and best.rate >= stay.rate * NS.LEAVE_MARGIN then
			best.reason = NS.LEAVE
			top = best
		else
			top = stay
		end
	else
		top = ranked[1]
		if top then top.reason = NS.BEST end
	end
	for _, e in ipairs(ranked) do
		if not e.reason then e.reason = NS.NEXT end
	end
	if stay then table.insert(ranked, 1, stay) end
	-- the top first, the rest by rate
	if top and top ~= ranked[1] then
		for i, e in ipairs(ranked) do
			if e == top then table.remove(ranked, i) break end
		end
		table.insert(ranked, 1, top)
	end
	return { ranked = ranked, top = top, stay = stay }
end
