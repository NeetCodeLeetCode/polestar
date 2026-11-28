-- Next stop, the places: what ns.NextStop.Rank ranks, built from the Codex for the character.
--
--   Places.Scan(done?)       judge the Codex quests for the character's level, a slice a frame (done() when it ends)
--   Places.Ready() -> bool   the scan for the character's level has ended
--   Places.Reset()           forget it (a level up, or another character)
--   Places.Build(from?) -> places, here   the places for ns.NextStop.Rank: a zone each, and in the zone the player is
--                            in a hub each; `here`: the hub the player stands at (an entry of `places`), or nil
--
-- A quest counts when the character may take it now (not grey, up to Places.ABOVE levels above the character,
-- its prerequisites done, not in the log, not blocked by race, class or faction) and its giver stands on a map.
-- Its XP is the Codex reward for the character's level, as a percent of the level's bar. Its work is a flat
-- Places.WORK_SECS: the Codex has no times. The trip is the planner's own estimate (ns.TravelCost).
-- Rules of the maintainer's Free mode (Polestar_FreeMode/Free.lua, a zone hint) are read, not changed.

local ADDON, ns = ...
local ipairs, pairs, min = ipairs, pairs, math.min
local sort = table.sort

local Places = {}
ns.NextStop.Places = Places

Places.ABOVE = 3 -- levels above the character a quest counts
Places.HUB_YARDS = 150 -- givers this close to one another make one hub
Places.WORK_SECS = 240 -- seconds of work for one quest: a first guess
Places.SLICE = 300 -- Codex quests judged a frame
Places.NEAREST = 12 -- the zones whose trip is worked out in full: the nearest by the straight walk

local RUN = 7 -- yards a second, as Guides' travel estimate runs

local state = { level = nil, busy = false, zones = nil }

local function Codex() return ns.Codex end

local function located(p)
	return p and p.map and p.x and p.y
end

local function direct(a, b)
	local cost = ns.DirectTravelCost
	return located(a) and located(b) and cost and cost(a, b) or nil
end

local function greyLevel(level)
	local Q = ns.QuestDB
	return Q and Q.GreyLevel and Q.GreyLevel(level) or 0
end

local function prereqsDone(q)
	local K = ns.Compat
	for _, id in ipairs(q.pre or {}) do
		if not K.IsQuestCompleted(id) then return false end
	end
	local any = q.preAny or {}
	if #any == 0 then return true end
	for _, id in ipairs(any) do
		if K.IsQuestCompleted(id) then return true end
	end
	return false
end

local function takeable(q, p, grey)
	if type(q) ~= "table" or not q.level then return false end
	if q.level <= grey or q.level > p.level + Places.ABOVE then return false end
	if ns.Lookup.QuestBlock(q, p) then return false end
	if ns.Compat.IsOnQuest(q.id) then return false end
	return prereqsDone(q)
end

-- Quest q's giver for the character, at its spawn on the quest's zone (else its first), or nil.
local function giverOf(q, p)
	local C = Codex()
	for _, id in ipairs(q.start and q.start.npc or {}) do
		local npc = C.Npc(id)
		if npc and npc.spawns and #npc.spawns > 0 and ns.Lookup.SideOK(npc.faction, p) then
			local at = npc.spawns[1]
			for _, s in ipairs(npc.spawns) do
				if s.map == q.zone then
					at = s
					break
				end
			end
			if located(at) then return { npc = id, name = npc.name, map = at.map, x = at.x, y = at.y, f = at.f == "e" and "e" or nil } end
		end
	end
	return nil
end

-- A quest's XP as a percent of the character's level bar, or nil when the Codex gives none.
local function percent(rec, level)
	if not (rec and type(rec.xp) == "number" and rec.xp > 0) then return nil end
	local lv = ns.Levels
	local bar = lv and lv.LEVEL_XP and lv.LEVEL_XP[level]
	if not bar or bar <= 0 then return nil end
	local share = lv.Share and lv.Share(level, rec.level) or 1
	return rec.xp * share / bar * 100
end

---------------------------------------------------------------------------
-- The scan
---------------------------------------------------------------------------

function Places.Reset()
	state.level, state.zones = nil, nil
end

function Places.Ready()
	local p = ns.Lookup.PlayerInfo()
	return state.zones ~= nil and state.level == p.level
end

function Places.Scan(done)
	local C = Codex()
	local p = ns.Lookup.PlayerInfo()
	if state.busy or Places.Ready() or not (C and C.IsLoaded()) then return false end
	state.busy = true
	local ids = {}
	C.Each("quest", function(id) ids[#ids + 1] = id end)
	local grey, zones, k = greyLevel(p.level), {}, 1
	local function slice()
		local stop = min(#ids, k + Places.SLICE - 1)
		for i = k, stop do
			local q = C.Quest(ids[i])
			if takeable(q, p, grey) then
				local xp = percent(q, p.level)
				local giver = xp and giverOf(q, p)
				if giver then
					local z = zones[giver.map]
					if not z then
						z = { map = giver.map, givers = {}, quests = {} }
						zones[giver.map] = z
					end
					z.givers[#z.givers + 1] = giver
					z.quests[#z.quests + 1] = { xp = xp, secs = Places.WORK_SECS, level = q.level, id = q.id, giver = giver }
				end
			end
		end
		k = stop + 1
		if k <= #ids then return ns.After(0, slice) end
		state.zones, state.level, state.busy = zones, p.level, false
		if done then done() end
	end
	-- Never inside the call that asked.
	ns.After(0, slice)
	return true
end

---------------------------------------------------------------------------
-- The places
---------------------------------------------------------------------------

-- Hubs of one zone: the giver with the most others within HUB_YARDS and everyone near it make a hub, the rest
-- go on. -> { { at = giver, quests = { ... } }, ... }, richest first
local function hubsOf(z)
	local left = {}
	for _, q in ipairs(z.quests) do left[#left + 1] = q end
	local hubs = {}
	while #left > 0 do
		local best, bestN
		for _, a in ipairs(left) do
			local n = 0
			for _, b in ipairs(left) do
				local d = direct(a.giver, b.giver)
				if d and d * RUN <= Places.HUB_YARDS then n = n + 1 end
			end
			if not bestN or n > bestN then best, bestN = a, n end
		end
		local hub, rest = { at = best.giver, quests = {} }, {}
		for _, q in ipairs(left) do
			local d = direct(best.giver, q.giver)
			if d and d * RUN <= Places.HUB_YARDS then
				hub.quests[#hub.quests + 1] = q
			else
				rest[#rest + 1] = q
			end
		end
		if #hub.quests == 0 then hub.quests, rest = { best }, {} end
		hubs[#hubs + 1] = hub
		left = rest
	end
	sort(hubs, function(a, b) return #a.quests > #b.quests end)
	return hubs
end

local function trip(from, to)
	if ns.TravelCost then
		local s = ns.TravelCost(nil, to)
		if s then return s end
	end
	return direct(from, to)
end

local function mapName(map)
	return ns.Compat.GetMapName(map) or "?"
end

function Places.Build(from)
	if not Places.Ready() then return nil end
	from = from or ns.PlayerPoint()
	local out, here = {}, nil
	local mine = from and from.map
	local ranked = {}
	for _, z in pairs(state.zones) do
		local hubs = hubsOf(z)
		ranked[#ranked + 1] = { z = z, hubs = hubs, d = from and direct(from, hubs[1].at) or 0 }
	end
	sort(ranked, function(a, b)
		if a.d ~= b.d then return (a.d or 0) < (b.d or 0) end
		return a.z.map < b.z.map
	end)
	for i, e in ipairs(ranked) do
		local z, hubs = e.z, e.hubs
		if z.map == mine then
			-- The zone the player is in: a place for each hub, so the next hub is a candidate.
			for h, hub in ipairs(hubs) do
				local d = from and direct(from, hub.at)
				local place = {
					id = "hub:" .. z.map .. ":" .. h, kind = "hub", name = hub.at.name, map = z.map, x = hub.at.x, y = hub.at.y,
					f = hub.at.f, tripSecs = d or 0, quests = hub.quests, zone = mapName(z.map),
				}
				if d and d * RUN <= Places.HUB_YARDS then
					place.tripSecs = 0
					here = place
				end
				out[#out + 1] = place
			end
		else
			local at = hubs[1].at
			local secs = i <= Places.NEAREST and trip(from, at) or e.d
			if secs then
				out[#out + 1] = {
					id = "zone:" .. z.map, kind = "zone", name = mapName(z.map), map = z.map, x = at.x, y = at.y, f = at.f,
					tripSecs = secs, quests = z.quests, zone = mapName(z.map),
				}
			end
		end
	end
	return out, here
end
