-- The best order of a few nearby stops for the arrow's take-an-objective mode
-- (Engine Own.hereAutoAim). World yards only: no game calls and no writes. It
-- tries every order of every subset of at most MAX stops (at most 64 paths
-- for four stops), each one done now or after any of the guide's next stops
-- (the lookahead), and scores a path by what it is worth for each minute of
-- extra walking it adds to the way the guide leads.
local ADDON, ns = ...
local Plan = {}
ns.StopPlanner = Plan

local MAX = 4 -- stops tried: more are left out (the Engine passes the nearest)
local AHEAD = 4 -- the guide's stops the way is measured through: more are left out
Plan.MAX, Plan.AHEAD = MAX, AHEAD

-- Filled by the caller before Plan.Best (reused, so a plan allocates nothing):
--   x[i], y[i]    where stop i is (world yards, one continent)
--   value[i]      what visiting it is worth (any one unit: the Engine uses percent of a level)
--   gx[k], gy[k]  where the guide's k-th next stop is, in order (the first is where the arrow leads now)
--   gown[k]       the stop i whose work the guide's k-th stop is (0: none): a path that visits i
--                 leaves the guide's stop out, as the guide then reads it done
Plan.x, Plan.y, Plan.value = {}, {}, {}
Plan.gx, Plan.gy, Plan.gown = {}, {}, {}
-- The result for each first stop i of a path done now: the best such path, or nil when none fits the limit.
--   score[i]  its value for each minute of extra walking
--   secs[i]   the extra walking, in seconds
Plan.score, Plan.secs = {}, {}

local X, Y, V, GX, GY, GOWN, SCORE, SECS = Plan.x, Plan.y, Plan.value, Plan.gx, Plan.gy, Plan.gown, Plan.score, Plan.secs
local USED, FX, FY, PL, SL = {}, {}, {}, {}, {}
local N, M, PX, PY, PER, FLOOR, LIMIT, BASE
local bestFirst, bestScore, bestSecs, laterScore

local function leg(ax, ay, bx, by)
	return ((ax - bx) ^ 2 + (ay - by) ^ 2) ^ 0.5
end

-- A path of stops first ... last (`inner` yards between them, `value` in all), done after the guide's
-- stop k (k = 0: now, from the player), then on to the guide's stop k + 1: score it. The guide's stops
-- the path visits (Plan.gown) are left out of the way on, which is walked as it stands without the path:
-- doing a stop of the guide early saves going back for it. Extra walking under 0 counts as none.
-- Headed nowhere (M = 0) the path comes back to the player, as a hand-in's detour counts it.
local function close(first, last, inner, value)
	local f = 0
	for k = 1, M do
		local own = GOWN[k]
		if own == 0 or not USED[own] then
			f = f + 1
			FX[f], FY[f] = GX[k], GY[k]
		end
	end
	PL[0] = 0
	for k = 1, f do
		PL[k] = PL[k - 1] + (k == 1 and leg(PX, PY, FX[1], FY[1]) or leg(FX[k - 1], FY[k - 1], FX[k], FY[k]))
	end
	SL[f] = 0
	for k = f - 1, 1, -1 do SL[k] = SL[k + 1] + leg(FX[k], FY[k], FX[k + 1], FY[k + 1]) end
	for k = 0, f do
		local sx, sy = PX, PY
		if k > 0 then sx, sy = FX[k], FY[k] end
		local cost = PL[k] + leg(sx, sy, X[first], Y[first]) + inner
		if k < f then
			cost = cost + leg(X[last], Y[last], FX[k + 1], FY[k + 1]) + SL[k + 1]
		elseif M == 0 then
			cost = cost + leg(X[last], Y[last], PX, PY)
		end
		local extra = cost - BASE
		if extra < 0 then extra = 0 end
		local secs = extra * PER
		if secs <= LIMIT then
			local score = value / ((secs + FLOOR) / 60)
			if k == 0 then
				-- Strictly better only: of equal paths the one found first (fewer stops, then lower index) stays.
				if not SCORE[first] or score > SCORE[first] then SCORE[first], SECS[first] = score, secs end
				if not bestScore or score > bestScore then bestFirst, bestScore, bestSecs = first, score, secs end
			elseif not laterScore or score > laterScore then
				laterScore = score
			end
		end
	end
end

local function extend(first, last, inner, value)
	close(first, last, inner, value)
	for j = 1, N do
		if not USED[j] then
			USED[j] = true
			extend(first, j, inner + leg(X[last], Y[last], X[j], Y[j]), value + V[j])
			USED[j] = false
		end
	end
end

-- The best path over stops 1..n (n at most MAX) from the player at px, py, with the guide's m next
-- stops (gx, gy, gown; m at most AHEAD) ahead of it, where `perYard` turns yards into seconds and `floor`
-- seconds keep a stop on the way from scoring without end. Only paths whose extra walking, over
-- walking the guide's stops straight, is at most `limit` seconds count. -> the first stop of the
-- best path done now, its score, its extra seconds, and the best score of any path done later (after
-- one of the guide's stops; nil when none fits). The first is nil when no path done now fits.
-- Plan.score and Plan.secs hold the best path done now of each first stop.
function Plan.Best(n, m, px, py, perYard, floor, limit)
	if n > MAX then n = MAX end
	if m > AHEAD then m = AHEAD end
	N, M, PX, PY, PER, FLOOR, LIMIT = n, m, px, py, perYard, floor, limit
	bestFirst, bestScore, bestSecs, laterScore = nil, nil, nil, nil
	BASE = 0
	for k = 1, m do BASE = BASE + (k == 1 and leg(px, py, GX[1], GY[1]) or leg(GX[k - 1], GY[k - 1], GX[k], GY[k])) end
	for i = 1, MAX do SCORE[i], SECS[i], USED[i] = nil, nil, false end
	for i = 1, n do
		USED[i] = true
		extend(i, i, 0, V[i])
		USED[i] = false
	end
	return bestFirst, bestScore, bestSecs, laterScore
end
