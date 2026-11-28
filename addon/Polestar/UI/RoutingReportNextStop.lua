-- The routing report's Next stop section: whether the card is on, what its latest look decided and why, the trip
-- it started, and every place it weighed with its numbers and the reason each one won or lost
-- (Core/NextStopLog.lua holds what the looks saw; Core/NextStop.lua ranks).
-- Read only. Place names are quest givers' and zones' names, never the player's.

local ADDON, ns = ...
local UI = ns.UI
local RR = UI.RoutingReport
local F = UI.ReportFormat
local floor = math.floor

local SEP = " \194\183 "
local ROWS_SHOWN, ROWS_LATEST, EXCLUDED_NAMES = 10, 6, 6

local function place(e)
	local p = e.place
	local kind = p.kind == "zone" and "zone" or p.kind == "dungeon" and "dungeon" or "hub"
	local where = (p.zone and p.kind ~= "zone") and (", in " .. tostring(p.zone)) or ""
	return ("%s (%s%s)"):format(tostring(p.name), kind, where)
end

-- Why entry e is where it is in the ranking, from the numbers the ranking holds.
local function why(e, r, NS)
	local top, stay = r.top, r.stay
	if e == top then
		if e.reason == NS.LEAVE then
			return ("chosen: %s a minute is %s times the %s of staying here, and leaving needs %s times"):format(F.fixed(e.rate, 2),
				stay and stay.rate > 0 and F.fixed(e.rate / stay.rate) or "many", stay and F.fixed(stay.rate, 2) .. " a minute" or "rate", F.fixed(NS.LEAVE_MARGIN))
		elseif e.reason == NS.STAY then
			local best = r.ranked[2]
			return "chosen: staying here; " .. (best and ("the best other place, %s, pays %s a minute, under %s times the %s of staying"):format(
				tostring(best.place.name), F.fixed(best.rate, 2), F.fixed(NS.LEAVE_MARGIN), F.fixed(e.rate, 2)) or "no other place has a quest")
		end
		return "chosen: nothing is left to do here, so the best rate wins"
	end
	if e == stay then
		return ("stay: pays %s a minute; the best other place needs %s times that (%s a minute) to pull you away"):format(
			F.fixed(e.rate, 2), F.fixed(NS.LEAVE_MARGIN), F.fixed(e.rate * NS.LEAVE_MARGIN, 2))
	end
	if top == stay and stay then
		return ("lost: %s a minute, and leaving needs %s times the %s of staying, %s"):format(F.fixed(e.rate, 2), F.fixed(NS.LEAVE_MARGIN),
			F.fixed(stay.rate, 2), F.fixed(stay.rate * NS.LEAVE_MARGIN, 2))
	end
	return ("lost: %s a minute against the best's %s (%s%%)"):format(F.fixed(e.rate, 2), top and F.fixed(top.rate, 2) or "?",
		top and top.rate > 0 and floor(e.rate / top.rate * 100 + 0.5) or 0)
end

local function rowLine(i, e, r, NS)
	local trip = e == r.stay and 0 or (e.place.tripSecs or 0)
	local work = e.minutes * 60 - trip
	return ("  %d. %s%strip %d s + work %d s = %s min%s%d quest%s%s%s of a level%s%s a minute%s%s"):format(
		i, place(e), SEP, floor(trip + 0.5), floor(work + 0.5), F.fixed(e.minutes), SEP, e.quests, e.quests == 1 and "" or "s", SEP,
		F.percent(e.xp), SEP, F.fixed(e.rate, 2) .. "%", SEP, why(e, r, NS))
end

-- The places the ranking left out, and the rule that did it.
local function leftOut(look, r, NS, add)
	local ranked = {}
	for _, e in ipairs(r.ranked) do ranked[e.place] = true end
	local dungeons, empty = {}, {}
	for _, p in ipairs(look.places or {}) do
		if not ranked[p] and p ~= look.here then
			if p.kind == "dungeon" and (look.group or 1) < NS.GROUP_MIN and look.dungeons == "skip" then
				dungeons[#dungeons + 1] = tostring(p.name)
			else
				empty[#empty + 1] = tostring(p.name)
			end
		end
	end
	local function names(list)
		local shown = {}
		for i = 1, math.min(#list, EXCLUDED_NAMES) do shown[i] = list[i] end
		return table.concat(shown, ", ") .. (#list > EXCLUDED_NAMES and (" and %d more"):format(#list - EXCLUDED_NAMES) or "")
	end
	if #dungeons > 0 then
		add(("  Left out, dungeons: %s%sneeds a group of %d (you are %d) or Dungeons in my route not on Skip (it is %s)"):format(
			names(dungeons), SEP, NS.GROUP_MIN, look.group or 1, tostring(look.dungeons)))
	end
	if #empty > 0 then
		add(("  Left out, no quest pays XP at your level%s: %d place%s%s%s"):format(look.hardcore and " (Hardcore filter on)" or "", #empty, #empty == 1 and "" or "s", SEP, names(empty)))
	end
end

local function weighed(label, look, rows, add)
	local NS = ns.NextStop
	local r = look and look.ranking
	if not (r and NS) then return end
	add(("%s (%s)%slevel %s%s%s%s%s player%s%s%sdungeons %s%s%d place%s built, %d ranked"):format(label, F.ago(look.at), SEP, tostring(look.level),
		SEP, look.here and ("standing at " .. tostring(look.here.name)) or "standing at no hub with quests", SEP, tostring(look.group or 1),
		(look.group or 1) == 1 and "" or "s", look.hardcore and (SEP .. "Hardcore filter on") or "", SEP, tostring(look.dungeons), SEP,
		#(look.places or {}), #(look.places or {}) == 1 and "" or "s", #r.ranked))
	for i, e in ipairs(r.ranked) do
		if i > rows then
			add(("  and %d more ranked below"):format(#r.ranked - rows))
			break
		end
		add(rowLine(i, e, r, NS))
	end
	leftOut(look, r, NS, add)
end

local function section(add)
	local NSC, Log, NS = UI.NextStopCard, ns.NextStopLog, ns.NextStop
	if not (NSC and Log and NS) then return end
	local FX, E = ns.FastestXP, ns.Engine
	add(("Next stop: %s%sFastest XP %s%slevel-range guide %s%splaces shown this session: %s"):format(
		F.onOff(ns.Get("behavior.nextStop") == true), SEP, F.onOff(FX and FX.On()), SEP, (E and E.IsLeveling and E.IsLeveling()) and "yes" or "no", SEP,
		#NSC.ToldKeys()))
	local last, shown = Log.Last(), Log.Shown()
	if not last then
		add(("Next stop looks: none yet (it looks every %d s once the card is on)"):format(NSC.EVERY))
	else
		add(("Next stop latest look: %s%s%s"):format(F.ago(last.at), SEP, tostring(last.why)))
		if last.guideSecs then
			add(("Next stop guide check: the guide's own arrow is %d s away; the card stays quiet when it is under %d s"):format(floor(last.guideSecs + 0.5), NSC.NEAR_SECS))
		elseif last.ranking or last.why:find("guide") then
			add("Next stop guide check: the guide has no arrow to measure")
		end
	end
	local N = ns.Nav
	local trip = N and N.Trip and N.Trip()
	if trip and trip.label == ns.L["Next stop"] then
		local age = N.ArrowLog and N.ArrowLog.TripAge(trip)
		add(("Next stop trip: to %s%s%s%s"):format(tostring(trip.name), SEP, age and ("started " .. F.clock(age) .. " ago") or "start unknown",
			shown and (SEP .. "from the card shown " .. F.ago(shown.at)) or ""))
	end
	if shown and shown.ranking then weighed("Next stop weighed when the card showed", shown, ROWS_SHOWN, add) end
	if last and last ~= shown and last.ranking then weighed("Next stop weighed in the latest look", last, ROWS_LATEST, add) end
end

RR.AddSection("next stop", section, "before")
