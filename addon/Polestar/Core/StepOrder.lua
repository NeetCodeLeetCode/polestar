-- The shortest order to do the guide's next few steps in, for the live step reorder
-- (Engine Own.orderTick). World yards only: no game calls and no writes. It tries
-- every order of at most MAX steps that keeps each step after the steps it needs
-- (`need`) and measures only the walking between the steps: from the player to the
-- first step's entry, from each step's exit to the next one's entry, and from the
-- last exit on to the guide's step after the window (`tail`, when there is one).
-- The walking inside a step is the same in every order.
local ADDON, ns = ...
local Order = {}
ns.StepOrder = Order

local MAX = 5 -- steps tried: more are left out (the Engine passes the live step and the next ones)
Order.MAX = MAX

-- Filled by the caller before Order.Best (reused, so a plan allocates nothing); step 1 is the live
-- step, the others follow in the guide's order:
--   ex[i], ey[i]  where step i is entered (world yards, one continent)
--   ox[i], oy[i]  where it is left
--   need[i][j]    true when step i cannot be done before step j (j < i)
--   tx, ty        where the guide goes after the window, or nil
Order.ex, Order.ey, Order.ox, Order.oy, Order.need = {}, {}, {}, {}, {}
Order.tx, Order.ty = nil, nil
-- The result:
--   order[k]      the step done k-th in the best order (1 = the live step first)
--   Order.cost    its walking in yards, Order.base the walking of the guide's own order
--   first[i]      the walking of the best order that starts with step i (nil: none starts with it)
Order.order, Order.first = {}, {}
Order.cost, Order.base = 0, 0

local EX, EY, OX, OY, NEED, ORDER, FIRST = Order.ex, Order.ey, Order.ox, Order.oy, Order.need, Order.order, Order.first
local USED, CUR, N, PX, PY, BEST, BESTFIRST = {}, {}, 0, 0, 0, nil, nil

local function leg(ax, ay, bx, by)
	return ((ax - bx) ^ 2 + (ay - by) ^ 2) ^ 0.5
end

local function ready(i)
	local need = NEED[i]
	if not need then return true end
	for j = 1, N do
		if need[j] and not USED[j] then return false end
	end
	return true
end

-- The best strictly: of equal orders the one found first stays, and the guide's own order is tried first.
local function walk(k, lx, ly, cost)
	if k > N then
		if Order.tx then cost = cost + leg(lx, ly, Order.tx, Order.ty) end
		local f = CUR[1]
		if not FIRST[f] or cost < FIRST[f] - 1e-9 then FIRST[f] = cost end
		if not BEST or cost < BEST - 1e-9 then
			BEST = cost
			for i = 1, N do ORDER[i] = CUR[i] end
		end
		return
	end
	for i = 1, N do
		if not USED[i] and ready(i) then
			local c = cost + leg(lx, ly, EX[i], EY[i])
			-- The best order (any first step) bounds it; a first step has its own best to find.
			if k > 1 and BESTFIRST[CUR[1]] and c >= BESTFIRST[CUR[1]] then
				-- Cannot beat the best order that starts with this step.
			else
				USED[i], CUR[k] = true, i
				walk(k + 1, OX[i], OY[i], c)
				USED[i] = false
			end
		end
	end
end

-- The guide's own order: its walking, in yards.
local function written()
	local cost, lx, ly = 0, PX, PY
	for i = 1, N do
		cost = cost + leg(lx, ly, EX[i], EY[i])
		lx, ly = OX[i], OY[i]
	end
	if Order.tx then cost = cost + leg(lx, ly, Order.tx, Order.ty) end
	return cost
end

-- The best order of steps 1..n (n at most MAX) from the player at px, py. -> the best order's first
-- step, and fills Order.order, Order.cost, Order.base and Order.first. The guide's own order wins
-- every tie. The first is nil for n under 2.
function Order.Best(n, px, py)
	if n > MAX then n = MAX end
	N, PX, PY = n, px, py
	for i = 1, MAX do USED[i], FIRST[i] = false, nil end
	if n < 2 then
		Order.cost, Order.base = 0, 0
		return nil
	end
	BESTFIRST = FIRST
	BEST = nil
	Order.base = written()
	walk(1, px, py, 0)
	Order.cost = BEST
	-- Ties go to the guide's order: the best never counts as better than it by a rounding.
	if BEST > Order.base - 1e-9 then
		for i = 1, N do ORDER[i] = i end
		Order.cost = Order.base
	end
	return ORDER[1]
end
