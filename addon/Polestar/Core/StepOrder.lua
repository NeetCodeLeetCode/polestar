-- The best order to do the guide's next few steps in, for the live step reorder
-- (Engine Own.orderTick). World yards and seconds only: no game calls and no writes. It tries
-- every order of at most MAX steps that keeps each step after the steps it needs
-- (`need`), by an exact search over the sets of steps done (2^n sets, not n! orders), and
-- measures two things:
--   walk  the walking between the steps: from the player to the first step's entry, from each
--         step's exit to the next one's entry, and from the last exit on to the guide's step
--         after the window (`tail`, when there is one). The walking inside a step is the same
--         in every order.
--   xp    when the caller gives each step its XP and its work time (Order.byXp): the seconds an
--         XP point waits on average, XP-weighted: the sum over steps of XP times the time the
--         step is done at (walking, the walk inside the step and its work), over the XP in all.
--         An order that does the steps that pay most for their time first scores low. It is
--         taken only when it takes no longer in all than the guide's own order does and than the
--         order the caller already went by (`keep`); else the best of the orders that do.
local ADDON, ns = ...
local Order = {}
ns.StepOrder = Order

local MAX = 8 -- steps tried: more are left out (the Engine passes the live step and the next ones)
Order.MAX = MAX

-- Filled by the caller before Order.Best (reused, so a plan allocates nothing); step 1 is the live
-- step, the others follow in the guide's order:
--   ex[i], ey[i]  where step i is entered (world yards, one continent)
--   ox[i], oy[i]  where it is left
--   need[i][j]    true when step i cannot be done before step j (j < i)
--   tx, ty        where the guide goes after the window, or nil
--   per           seconds a yard takes (nil: 1, so the costs are yards)
--   byXp          score by XP (below), else by walking
--   xp[i], work[i]  the XP step i pays and the seconds its work takes (only with byXp)
--   keep          nil, or an order (a list of the steps 1..n) the player is already going by: the best
--                 never takes longer in all than it (only with byXp)
Order.ex, Order.ey, Order.ox, Order.oy, Order.need = {}, {}, {}, {}, {}
Order.xp, Order.work = {}, {}
Order.tx, Order.ty, Order.per, Order.byXp, Order.keep = nil, nil, nil, false, nil
-- The result:
--   order[k]      the step done k-th in the best order (1 = the live step first)
--   Order.cost    its score and Order.base the score of the guide's own order: walking in yards, or
--                 with byXp (while Order.mode is "xp") the weighted seconds an XP point waits, as yards
--                 (seconds over `per`), so (base - cost) * per is the seconds saved either way
--   first[i]      the score of the best order that starts with step i (nil: none starts with it)
--   Order.mode    "walk" or "xp": what the scores are
--   Order.forced  the guide's own order takes longer in all than the order gone by (`keep`): the best is
--                 to be shown whatever it saves
Order.order, Order.first = {}, {}
Order.cost, Order.base, Order.mode, Order.forced = 0, 0, "walk", false

local EX, EY, OX, OY, NEED, XP, WORK, ORDER, FIRST = Order.ex, Order.ey, Order.ox, Order.oy, Order.need, Order.xp, Order.work, Order.order, Order.first

local SETS = 2 ^ MAX
local COLS = MAX + 2 -- a state is the set done and the step it ended on (column n + 1: the player, no step)
local F = {} -- F[set * COLS + last]: the least score still to come from that state
local WSET = {} -- WSET[set]: the XP of the steps in the set
local POW, NEEDM = {}, {} -- 2^(i-1); the sets step i needs
local LEG = {} -- LEG[(i - 1) * COLS + j]: seconds from where step i is left to where step j is entered
local FROM = {} -- seconds from the player to where step j is entered
local INNER = {} -- seconds the step takes itself: the walk inside it and its work
local TAIL = {} -- seconds from where step i is left to the guide's step after the window
local N, FULL, XPMODE, TOTALW = 0, 0, false, 0
local INF = math.huge

local floor = math.floor

local function leg(ax, ay, bx, by)
	return ((ax - bx) ^ 2 + (ay - by) ^ 2) ^ 0.5
end

local function has(set, i)
	return floor(set / POW[i]) % 2 == 1
end

-- Whether step j may come next with `set` done.
local function ready(j, set)
	local need = NEED[j]
	if not need then return true end
	for i = 1, N do
		if need[i] and not has(set, i) then return false end
	end
	return true
end

-- What step j adds to the score after `set` and the step `last` (N + 1: the player): its walk in, and with
-- byXp the time it takes (walk in, the step itself) as seconds every XP point still to come waits.
local function add(last, j, set)
	local walk = last > N and FROM[j] or LEG[(last - 1) * COLS + j]
	if not XPMODE then return walk end
	return (walk + INNER[j]) * (TOTALW - WSET[set])
end

-- What the walk on after the last step adds: only the walk, where nothing is left to wait.
local function finish(last)
	if XPMODE then return 0 end
	return TAIL[last] or 0
end

-- The least score to come for every state, from the full set back to nothing done. Of equal scores the
-- step first in the guide's order stays.
local function solve()
	for set = FULL, 0, -1 do
		local last0 = set == 0 and N + 1 or 1
		local last1 = set == 0 and N + 1 or N
		for last = last0, last1 do
			if set == 0 or has(set, last) then
				local best = INF
				if set == FULL then
					best = finish(last)
				else
					for j = 1, N do
						if not has(set, j) and ready(j, set) then
							local c = add(last, j, set) + F[(set + POW[j]) * COLS + j]
							if c < best then best = c end
						end
					end
				end
				F[set * COLS + last] = best
			end
		end
	end
end

-- The order the states give from the start (fills ORDER), and the first steps' scores (FIRST).
local function follow()
	local set, last = 0, N + 1
	for k = 1, N do
		local pick, best = nil, INF
		for j = 1, N do
			if not has(set, j) and ready(j, set) then
				local c = add(last, j, set) + F[(set + POW[j]) * COLS + j]
				if c < best - 1e-9 then pick, best = j, c end
			end
		end
		ORDER[k] = pick
		set, last = set + POW[pick], pick
	end
	for j = 1, N do
		FIRST[j] = ready(j, 0) and add(N + 1, j, 0) + F[POW[j] * COLS + j] or nil
	end
end

-- The seconds it takes, in all, to do the steps in the order of `list` (the walk on after the last one
-- too), and the XP-weighted score of that order.
local function measure(list)
	local t, score, set, last = 0, 0, 0, N + 1
	for k = 1, N do
		local j = list[k]
		local walk = last > N and FROM[j] or LEG[(last - 1) * COLS + j]
		t = t + walk + INNER[j]
		if XPMODE then
			score = score + add(last, j, set)
		else
			score = score + walk
		end
		set, last = set + POW[j], j
	end
	if not XPMODE then score = score + (TAIL[last] or 0) end
	return t + (TAIL[last] or 0), score
end

local IDENTITY, WALKED = {}, {}

-- Fills the tables of seconds for steps 1..n from the player at px, py.
local function prepare(n, px, py)
	local per = Order.per or 1
	N, FULL = n, 2 ^ n - 1
	for i = 1, n do
		POW[i] = 2 ^ (i - 1)
		IDENTITY[i] = i
		INNER[i] = leg(EX[i], EY[i], OX[i], OY[i]) * per + (XPMODE and WORK[i] or 0)
		FROM[i] = leg(px, py, EX[i], EY[i]) * per
		TAIL[i] = Order.tx and leg(OX[i], OY[i], Order.tx, Order.ty) * per or nil
		for j = 1, n do
			LEG[(i - 1) * COLS + j] = leg(OX[i], OY[i], EX[j], EY[j]) * per
		end
	end
	TOTALW = 0
	for i = 1, n do TOTALW = TOTALW + (XP[i] or 0) end
	WSET[0] = 0
	for set = 1, FULL do
		local w = 0
		for i = 1, n do
			if has(set, i) then w = w + (XP[i] or 0) end
		end
		WSET[set] = w
	end
end

-- The best order of steps 1..n (n at most MAX) from the player at px, py. -> the best order's first
-- step, and fills Order.order, Order.cost, Order.base, Order.mode and Order.first. The guide's own order wins
-- every tie. The first is nil for n under 2.
function Order.Best(n, px, py)
	if n > MAX then n = MAX end
	for i = 1, MAX do FIRST[i] = nil end
	Order.mode, Order.forced = "walk", false
	if n < 2 then
		Order.cost, Order.base = 0, 0
		return nil
	end
	local per = Order.per or 1
	XPMODE = false
	prepare(n, px, py)
	-- The weights: none to weigh by (no XP anywhere) leaves the walk to decide.
	if Order.byXp and TOTALW > 0 then
		XPMODE = true
		prepare(n, px, py)
		solve()
		follow()
		local took, score = measure(ORDER)
		local ownTook, ownScore = measure(IDENTITY)
		local unit = TOTALW * per
		-- It takes no longer in all than the guide's own order or the order gone by.
		local cap = ownTook
		if Order.keep then
			local keepTook = measure(Order.keep)
			if keepTook < cap then cap = keepTook end
		end
		Order.mode = "xp"
		Order.forced = ownTook > cap + 1e-9
		if took <= cap + 1e-9 then
			for j = 1, n do
				if FIRST[j] then FIRST[j] = FIRST[j] / unit end
			end
			Order.cost, Order.base = score / unit, ownScore / unit
			-- Ties go to the guide's order: the best never counts as better than it by a rounding.
			if ownTook <= cap + 1e-9 and score > ownScore - 1e-9 then
				for i = 1, n do ORDER[i] = i end
				Order.cost = Order.base
			end
			return ORDER[1]
		end
		-- Longer: of the orders that take no longer, the one that pays soonest. The walk order takes the least of
		-- all; the guide's order and the order gone by count when they are within the cap, and win a tie.
		XPMODE = false
		prepare(n, px, py)
		solve()
		follow()
		for i = 1, n do WALKED[i] = ORDER[i] end
		XPMODE = true
		prepare(n, px, py)
		local best, bestScore = nil, INF
		local function consider(list)
			local t, sc = measure(list)
			if t <= cap + 1e-9 and sc < bestScore - 1e-9 then best, bestScore = list, sc end
		end
		consider(IDENTITY)
		if Order.keep then consider(Order.keep) end
		consider(WALKED)
		for i = 1, n do ORDER[i] = best[i] end
		for j = 1, n do FIRST[j] = nil end
		Order.cost, Order.base = bestScore / unit, ownScore / unit
		FIRST[ORDER[1]] = Order.cost
		return ORDER[1]
	end
	solve()
	follow()
	local _, score = measure(ORDER)
	local _, own = measure(IDENTITY)
	for j = 1, n do
		if FIRST[j] then FIRST[j] = FIRST[j] / per end
	end
	Order.cost, Order.base = score / per, own / per
	if Order.cost > Order.base - 1e-9 then
		for i = 1, n do ORDER[i] = i end
		Order.cost = Order.base
	end
	return ORDER[1]
end
