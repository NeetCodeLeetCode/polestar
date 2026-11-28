-- A simulated player that follows only the arrow, with a clock, XP, levels and events (forge_addontest::arrowplayer).
-- It plays the open guide on the stub world (stub_world.lua): it walks to the place the arrow leads to, does the
-- goal there (accept, work, hand in, vendor, flight, ...), and the quest rewards and kills pay XP, so the character
-- levels as it plays and the guide reacts as it does for a person (grey quests, level gates, new quests). Events
-- (a death, a detour away, an abandoned quest, a setting flipped, a level jump) come at set times.
--
-- ARROW.run(o) -> metrics. Options (o):
--   seconds   the clock limit of the run (default 3600)
--   level     stop at this level (default none)
--   dt        seconds a step of the clock takes (default 1)
--   speed     yards a second on foot (default 7; x1.6 at level 40 and up with o.mount)
--   events    { { at = seconds, kind = "die" | "wander" | "afk" | "abandon" | "set" | "recalibrate" | "level" | "xp" |
--             "call", ... } } (see EVENTS below)
-- The metrics are in `m` below: time, XP, yards, the arrow's reversals, stuck states, rule breaks, CPU and memory.
local E, w, C = Polestar.Engine, stub.world, Polestar.Compat
local LV = Polestar.Levels
local FRAMES = Polestar.MapFrames.era

ARROW = ARROW or {}

local WORK = { kill = 9, collect = 12, use = 5, interact = 5, talk = 5 } -- seconds for each kill or item
local ACT = { accept = 4, turnin = 5, vendor = 25, train = 20, sethome = 8 } -- seconds a stop takes
local DEATHSKIP_SECONDS = 60 -- release, the Spirit Healer's wait and the stone's cast: what a deathskip costs instead of the walk
local KIND = { kill = "monster", collect = "item", interact = "object", talk = "monster" }

-- The full reward of a quest of level `lv`: a share of the level's XP, more for a low level (the client's tables are
-- not here). The same in every run, so a difference between runs is the route's.
local function reward(lv)
	lv = math.max(1, math.min(lv or w.level, LV.MAX_LEVEL - 1))
	return math.floor((LV.LEVEL_XP[lv] or 1000) * math.max(0.06, 0.2 - 0.0066 * lv))
end

local function hypot(a, b) return (a * a + b * b) ^ 0.5 end
local speed0 -- set in run: the yards a second on foot now

function ARROW.run(o)
	local dt, limit = o.dt or 1, o.seconds or 3600
	local now, yards = 0, 0
	local m = {
		xp = 0, kills = 0, level0 = w.level, quests = {}, handins = 0, stuck = nil, breaks = {}, reversals = 0, arrows = 0,
		cpuMax = 0, cpu = 0, ticks = 0, jumps = 0, work = 0, idle = 0, events = {}, trace = {}, left = nil,
	}
	local qlvl, qxp, objs, kinds, preOf = {}, {}, {}, {}, {}
	local G
	local function trace(s)
		local t = m.trace
		t[#t + 1] = ("%d %s"):format(now, s)
		if #t > (TRACE_CAP or 60) then table.remove(t, 1) end
	end

	-- What the open guide tells about its quests: objectives for the log, the level and the prerequisites.
	local function learn(g)
		if G == g then return end
		G = g
		for _, s in ipairs(g.steps) do
			for _, goal in ipairs(s.goals) do
				local q = goal.quest
				if q then
					if goal.qlvl then qlvl[q] = tonumber(goal.qlvl) end
					if goal.t == "accept" then preOf[q] = preOf[q] or goal end
					if goal.oi then
						objs[q] = objs[q] or {}
						objs[q][goal.oi] = math.max(objs[q][goal.oi] or 1, goal.count or 1)
						kinds[q] = kinds[q] or {}
						kinds[q][goal.oi] = kinds[q][goal.oi] or KIND[goal.t]
					end
				end
			end
		end
	end
	local function objectivesFor(q)
		local list, n = {}, 0
		for i in pairs(objs[q] or {}) do if i > n then n = i end end
		for i = 1, n do list[i] = { required = objs[q][i] or 1, type = kinds[q][i] or "event" } end
		return list
	end

	-- XP: it levels the character as the client does, and the clock of the rate counts it.
	local function gain(n)
		n = math.floor(n)
		if n <= 0 then return end
		m.xp = m.xp + n
		local xp = w.xp + n
		while xp >= w.xpMax and w.level < LV.MAX_LEVEL do
			xp = xp - w.xpMax
			w.setLevel(w.level + 1)
			w.xpMax = LV.LEVEL_XP[w.level] or w.xpMax
			trace("level " .. w.level)
		end
		w.setXP(xp)
	end
	local function mobXp(lv, count)
		return LV.MobXP(w.level, lv or w.level, false) * count
	end

	-- Where the player stands, in world yards (c, a, b as Compat.MapToWorld gives them).
	local pc, pa, pb
	local function here()
		local c, a, b = C.GetPlayerWorldPosition()
		if c then pc, pa, pb = c, a, b end
		return pc, pa, pb
	end
	local function put(map, a, b)
		local fr = FRAMES[map]
		if not fr then return end
		local x = (b - fr.left) / (fr.right - fr.left) * 100
		local y = (a - fr.top) / (fr.bottom - fr.top) * 100
		w.moveTo(map, x, y)
		pa, pb = a, b
	end
	local function tick(secs)
		local t0 = os.clock()
		stub.advance(secs)
		local used = os.clock() - t0
		m.cpu, m.ticks = m.cpu + used, m.ticks + 1
		if used > m.cpuMax then m.cpuMax = used end
		now = now + secs
	end
	local function spend(secs, kind)
		m[kind or "work"] = m[kind or "work"] + secs
		local left = secs
		while left > 0 do
			local s = math.min(left, 5)
			tick(s)
			left = left - s
		end
	end

	-- The prerequisites an accept needs, as the guide writes them.
	local function preMet(goal)
		if type(goal.preAll) == "table" then
			for _, p in ipairs(goal.preAll) do if not w.completed[p] then return false end end
		end
		if type(goal.pre) == "table" and #goal.pre > 0 then
			for _, p in ipairs(goal.pre) do if w.completed[p] then return true end end
			return false
		end
		return true
	end

	-- What a goal does where it is. -> seconds of work. Accepting, handing in and the field work are played here (they
	-- carry the clock, the XP and the checks); the rest is the simulated player's own act (tests/lua/simulate.lua).
	-- `view` is the goal as the step's view gives it (the NPC the engine picked), `i` its step.
	local function act(goal, view, i)
		local t, q = goal.t, goal.quest
		local secs = ACT[t] or 2
		if t == "accept" then
			if not preMet(goal) then m.breaks[#m.breaks + 1] = ("t=%d accept %s with its prerequisites not handed in"):format(now, tostring(q)) end
			if not w.log[q] and not w.completed[q] then w.accept(q, objectivesFor(q), goal.subject) end
		elseif t == "turnin" then
			-- A quest that is offered and taken back in one talk was never in the log.
			if not w.completed[q] then
				local xp = math.floor(reward(qlvl[q] or w.level) * LV.Share(w.level, qlvl[q] or w.level))
				if w.log[q] then w.finish(q) end
				w.turnIn(q, xp)
				m.handins = m.handins + 1
				m.quests[#m.quests + 1] = q
				gain(xp)
			end
		elseif t == "level" then
			-- A hold until a level: the player grinds kills at their own level until it is there.
			local want = tonumber((goal.subject or ""):match("%d+")) or (w.level + 1)
			while w.level < want and now < limit do
				spend(10, "work")
				gain(mobXp(w.level, 1))
				m.kills = m.kills + 1
			end
		elseif WORK[t] and (not q or w.log[q]) then
			-- Kills and loot: the work takes time and pays kill XP; the quest's line (or the bags) fill.
			local n = goal.count or 1
			secs = WORK[t] * n
			-- A quest of the slow list pays its price here too: slow respawns and rare drops cost a multiple of the time.
			local List = Polestar.SlowQuestList
			local why = q and List and List.Slow(q)
			if why == "slow-respawn" or why == "low-drop" then secs = secs * (SLOW_FACTOR or 3) end
			local mobLv = qlvl[q] or w.level
			local kills = t == "collect" and math.ceil(n / 0.6) or n
			m.kills = m.kills + kills
			gain(mobXp(mobLv, kills))
			if goal.item and t == "collect" then w.addItem(goal.item, n) end
			if q then
				if goal.oi and w.log[q].objectives[goal.oi] then w.progress(q, goal.oi, w.log[q].objectives[goal.oi].numRequired) else w.finish(q) end
			end
		else
			SIM_ACT(i, view or { index = 0, t = t, quest = q, npc = goal.npc, subject = goal.location or goal.subject }, goal)
		end
		return secs
	end

	-- Flights, boats, the Hearthstone: the arrow leads to another continent, or the guide has a `fly` there. The
	-- player arrives after a time that goes with the distance.
	local function jump(c, a, b, map)
		m.jumps = m.jumps + 1
		spend(60 + math.min(240, (pa and hypot(a - pa, b - pb) or 2000) / 25), "work")
		trace(("jump to map %s (frame %s) zone before %s"):format(tostring(map), tostring(FRAMES[map] ~= nil), tostring(w.zone)))
		put(map, a, b)
		pc = c
	end

	speed0 = function() return (o.speed or 7) * ((o.mount and w.level >= 40) and 1.6 or 1) end

	-- The events.
	local events = o.events or {}
	table.sort(events, function(x, y) return x.at < y.at end)
	local nextEvent = 1
	local EVENTS = {}
	function EVENTS.die(ev)
		-- Killed: a ghost at the graveyard `yards` away, runs back to the corpse, lives again. The arrow is the corpse's.
		local cx, ca, cb = here()
		local map = w.map
		DEAD = true
		stub.fire("PLAYER_DEAD")
		local gy = ev.yards or 700
		local angle = (ev.angle or 0.7)
		put(map, ca + math.sin(angle) * gy, cb + math.cos(angle) * gy)
		tick(2)
		local left = gy
		while left > 0 do
			local s = math.min(left, speed0() * 2)
			left = left - s
			put(map, pa - math.sin(angle) * s, pb - math.cos(angle) * s)
			yards = yards + s
			tick(2)
		end
		put(map, ca, cb)
		DEAD = false
		stub.fire("PLAYER_UNGHOST")
		stub.fire("PLAYER_ALIVE")
		tick(1)
		m.deaths = (m.deaths or 0) + 1
	end
	function EVENTS.wander(ev)
		-- Off the route for a while (the auction house, a fishing spot, a friend): `yards` away and back.
		local map = w.map
		local _, ca, cb = here()
		local gy = ev.yards or 600
		local angle = ev.angle or 2
		local left = gy
		while left > 0 do
			local s = math.min(left, speed0() * 2)
			left = left - s
			put(map, pa + math.sin(angle) * s, pb + math.cos(angle) * s)
			yards = yards + s
			tick(2)
		end
		spend(ev.secs or 60, "idle")
		left = gy
		while left > 0 do
			local s = math.min(left, speed0() * 2)
			left = left - s
			put(map, pa - math.sin(angle) * s, pb - math.cos(angle) * s)
			yards = yards + s
			tick(2)
		end
	end
	function EVENTS.shift(ev)
		-- The player starts somewhere else in the zone: moved, not walked.
		local map = w.map
		here()
		put(map, pa + math.sin(ev.angle or 1) * (ev.yards or 300), pb + math.cos(ev.angle or 1) * (ev.yards or 300))
		tick(1)
	end
	function EVENTS.afk(ev) spend(ev.secs or 300, "idle") end
	function EVENTS.abandon(ev)
		-- The quest in the log that the seed names (the ev.n-th of the sorted ids).
		local ids = {}
		for q in pairs(w.log) do ids[#ids + 1] = q end
		table.sort(ids)
		if #ids > 0 then
			local q = ids[(ev.n or 1) % #ids + 1]
			w.abandon(q)
			m.abandoned = (m.abandoned or 0) + 1
			trace("abandon " .. q)
		end
	end
	function EVENTS.set(ev) Polestar.Set(ev.key, ev.value); tick(1) end
	function EVENTS.recalibrate(ev) for _ = 1, ev.n or 1 do E.Recalibrate(); tick(ev.gap or 0.3) end end
	function EVENTS.level(ev) for _ = 1, ev.n or 1 do gain(w.xpMax - w.xp) end end
	function EVENTS.xp(ev) gain(ev.n or 1000) end
	function EVENTS.call(ev) ARROW.say = trace; ev.fn(); tick(1) end


	-- The run.
	local last, before, finished = nil, nil, {}
	local same, lastProgress = 0, 0
	local lastTarget
	local mem0
	collectgarbage(); collectgarbage()
	mem0 = collectgarbage("count")
	local progressKey = ""
	while now < limit do
		if o.level and w.level >= o.level then m.reachedLevel = true; break end
		while events[nextEvent] and events[nextEvent].at <= now do
			local ev = events[nextEvent]
			nextEvent = nextEvent + 1
			trace("event " .. ev.kind)
			m.events[#m.events + 1] = ev.kind .. "@" .. math.floor(now)
			EVENTS[ev.kind](ev)
		end
		local g = E.GetGuide()
		if not g then m.left = "no guide"; break end
		learn(g)
		local wp = E.GetWaypoint()
		if not wp then
			-- No arrow: a goal without a place (a level to reach, a check-off), or the guide is at its end.
			local i = E.GetStepIndex()
			if not i then m.ended = true; break end
			local view = E.GetStepView(i)
			local open
			for _, goal in ipairs(view.goals) do
				if goal.status ~= "done" and goal.status ~= "na" and goal.status ~= "skipped" and not goal.optional and not goal.side and goal.t ~= "note" then open = goal; break end
			end
			if open and not open.unavailable and open.t ~= "goto" then
				spend(act(g.steps[i].goals[open.index], open, i), "work")
			else
				-- Nothing to do here until a level or a quest turns up: grind.
				m.grinds = (m.grinds or 0) + 1
				if m.grinds == 1 or m.grinds == 30 then
					local why = {}
					for _, goal in ipairs(view.goals) do why[#why + 1] = ("%s %s %s %s"):format(tostring(goal.t), tostring(goal.status), tostring(goal.text), tostring(goal.unavailable or "")) end
					m.grindWhy = (m.grindWhy or "") .. ("[t=%d step %s: %s] "):format(now, tostring(i), table.concat(why, "; "))
				end
				local g0 = m.xp
				gain(mobXp(w.level, 1) * 0.8)
				m.grindXp = (m.grindXp or 0) + (m.xp - g0)
				m.kills = m.kills + 1
				spend(10, "idle")
			end
			same = same + 1
		else
			m.arrows = m.arrows + 1
			local key = wp.stepIndex .. ":" .. wp.goalIndex
			if key ~= last then
				if before == key and not finished[key] and not finished[last] then
					m.reversals = m.reversals + 1
					trace("reversal " .. last .. " -> " .. key)
				end
				before, last = last, key
				trace("arrow " .. key .. " " .. tostring(wp.kind) .. " q" .. tostring(wp.quest))
				if o.explain and E.RoutingState then
					-- Why the engine chose it: the reorder's and the aim's own words (the routing report).
					local r = E.RoutingState()
					trace(("   reorder live=%s shown=%s rest=%s: %s | aim: %s | arrow from %s"):format(tostring(r.reorder.live), tostring(r.reorder.shown), tostring(r.reorder.rest), tostring(r.reorder.why), tostring(r.aim.why), tostring(r.arrowFrom)))
				end
			end
			local c, a, b = C.MapToWorld(wp.map, wp.x, wp.y, wp.f)
			local pc0 = here()
			if c and (not lastTarget or hypot(a - lastTarget[1], b - lastTarget[2]) > 5) then
				if lastTarget and pa then
					m.hops = (m.hops or 0) + 1
					trace(("target %s moved to %.0f,%.0f, %.0f yd from me, r %s"):format(key, a, b, hypot(a - pa, b - pb), tostring(wp.r)))
				end
				lastTarget = { a, b }
			end
			if not c then
				-- A place the map cannot place: go there as it is.
				w.moveTo(wp.map, wp.x, wp.y)
				tick(dt)
			elseif pc0 and pc0 ~= c then
				jump(c, a, b, wp.map)
			elseif wp.kind == "die" and hypot(a - pa, b - pb) > 40 then
				-- A deathskip: the player dies where they stand and wakes at the Spirit Healer, after the corpse-free
				-- release and the healer's wait. Nothing is walked.
				m.deathskips = (m.deathskips or 0) + 1
				spend(DEATHSKIP_SECONDS, "work")
				put(wp.map, a, b)
			else
				local d = hypot(a - pa, b - pb)
				local r = math.max(4, math.min(wp.r or 10, 25))
				if d > r then
					local step = math.min(d - r + 1, speed0() * dt)
					put(wp.map, pa + (a - pa) / d * step, pb + (b - pb) / d * step)
					yards = yards + step
					d = hypot(a - pa, b - pb)
				end
				if d <= r then
					-- Arrived: the work is done at once, before the engine's next look (a point that is only checked
					-- moves the arrow on the moment the player is on it).
					local goal = g.steps[wp.stepIndex].goals[wp.goalIndex]
					local view = E.GetStepView(wp.stepIndex)
					local vg = view and view.goals[wp.goalIndex]
					local secs = act(goal, vg, wp.stepIndex)
					finished[key] = true
					spend(secs, "work")
					-- A check-off the game gives no event for (learn new spells): the player ticks it.
					view = E.GetStepView(wp.stepIndex)
					vg = view and view.goals[wp.goalIndex]
					if vg and vg.manual and vg.status ~= "done" then
						E.ToggleGoal(wp.stepIndex, wp.goalIndex)
						tick(0.6)
						trace(("ticked %s: %s"):format(key, E.GetStepView(wp.stepIndex).goals[wp.goalIndex].status))
					end
					same = same + 1
				else
					tick(dt)
				end
			end
		end
		-- Progress: XP, a hand-in, a step, or 300 yards walked. Nothing of these for a long time is stuck (a long walk is
		-- not; an arrow that sends the player back and forth is the reversals count).
		local pk = m.xp .. ":" .. m.handins .. ":" .. tostring(E.GetStepIndex()) .. ":" .. tostring(E.GetGuide() and E.GetGuide().id) .. ":" .. math.floor(yards / 300)
		if pk ~= progressKey then progressKey, lastProgress = pk, now end
		if now - lastProgress > (o.stuckSecs or 900) then
			local i = E.GetStepIndex()
			local goals = {}
			if i then
				for _, goal in ipairs(E.GetStepView(i).goals) do
					goals[#goals + 1] = ("%s %s %s%s"):format(tostring(goal.t), tostring(goal.status), tostring(goal.text), goal.unavailable and (" [" .. tostring(goal.unavailable) .. "]") or "")
				end
			end
			local log = {}
			for q, e in pairs(w.log) do
				local parts = {}
				for _, ob in ipairs(e.objectives) do parts[#parts + 1] = ob.numFulfilled .. "/" .. ob.numRequired end
				log[#log + 1] = q .. " " .. table.concat(parts, ",")
			end
			local vgoal
			do
				local wp = E.GetWaypoint()
				local vg = wp and E.GetStepView(wp.stepIndex).goals[wp.goalIndex]
				if vg then
					local f = {}
					for k, v in pairs(vg) do if type(v) ~= "table" then f[#f + 1] = k .. "=" .. tostring(v) end end
					table.sort(f)
					vgoal = table.concat(f, " ")
				end
			end
			m.stuck = { zone = w.zone, map = w.map, pos = { here() }, view = vgoal, log = log, bag = w.items, at = math.floor(now), step = i, guide = E.GetGuide() and E.GetGuide().id, goals = goals, wp = E.GetWaypoint() and (("%s kind=%s step=%s goal=%s quest=%s"):format(tostring(E.GetWaypoint().label), tostring(E.GetWaypoint().kind), tostring(E.GetWaypoint().stepIndex), tostring(E.GetWaypoint().goalIndex), tostring(E.GetWaypoint().quest))), goal = (function()
				local wp = E.GetWaypoint()
				local gl = wp and E.GetGuide().steps[wp.stepIndex].goals[wp.goalIndex]
				if not gl then return nil end
				local f = {}
				for k, v in pairs(gl) do if type(v) ~= "table" then f[#f + 1] = k .. "=" .. tostring(v) end end
				table.sort(f)
				return table.concat(f, " ")
			end)(), trace = m.trace }
			break
		end
	end
	-- What the route left out: the steps this character's class owns, and how each ended (done, skipped, still ahead).
	do
		local g = E.GetGuide()
		local cls = tostring(w.className or w.class or ""):upper()
		m.classSteps = { done = 0, skipped = 0, ahead = 0, skippedTitles = {} }
		for i, st in ipairs(g and g.steps or {}) do
			local mine = false
			for _, c in ipairs(st.only and st.only.classes or {}) do
				if tostring(c):upper() == cls then mine = true end
			end
			if mine then
				local view = E.GetStepView(i)
				local done, skipped, open = 0, 0, 0
				for _, goal in ipairs(view and view.goals or {}) do
					if goal.status == "done" then done = done + 1 elseif goal.status == "skipped" then skipped = skipped + 1 elseif goal.status ~= "na" then open = open + 1 end
				end
				if skipped > 0 and done == 0 then
					m.classSteps.skipped = m.classSteps.skipped + 1
					m.classSteps.skippedTitles[#m.classSteps.skippedTitles + 1] = tostring(st.title)
				elseif open > 0 then
					m.classSteps.ahead = m.classSteps.ahead + 1
				else
					m.classSteps.done = m.classSteps.done + 1
				end
			end
		end
	end
	collectgarbage(); collectgarbage()
	m.mem = collectgarbage("count") - mem0
	m.seconds, m.yards, m.level = math.floor(now), math.floor(yards), w.level
	m.xpPerMin = m.xp / math.max(1, now / 60)
	m.guide = E.GetGuide() and E.GetGuide().id
	m.step = E.GetStepIndex()
	return m
end
