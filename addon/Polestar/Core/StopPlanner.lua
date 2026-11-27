-- The best order of a few nearby stops for the arrow's take-an-objective mode
-- (Engine Own.hereAutoAim). World yards only: no game calls and no writes. It
-- tries every order of every subset of at most MAX stops (at most 64 paths
-- for four stops) and scores a path by what it is worth for each minute of
-- extra walking it adds to the way the player was already going.
local ADDON, ns = ...
local Plan = {}
ns.StopPlanner = Plan

local MAX = 4 -- stops tried: more are left out (the Engine passes the nearest)
Plan.MAX = MAX

-- The stops, filled by the caller before Plan.Best (reused, so a plan allocates nothing):
--   x[i], y[i]  where stop i is (world yards, one continent)
--   value[i]    what visiting it is worth (any one unit: the Engine uses percent of a level)
Plan.x, Plan.y, Plan.value = {}, {}, {}
-- The result for each first stop i: the best path starting there, or nil when none fits the limit.
--   score[i]  its value for each minute of extra walking
--   secs[i]   the extra walking, in seconds
Plan.score, Plan.secs = {}, {}

local X, Y, V, SCORE, SECS = Plan.x, Plan.y, Plan.value, Plan.score, Plan.secs
local USED = {}
local N, PX, PY, TX, TY, DIRECT, PER, FLOOR, LIMIT
local bestFirst, bestScore, bestSecs

local function leg(ax, ay, bx, by)
	return ((ax - bx) ^ 2 + (ay - by) ^ 2) ^ 0.5
end

-- Path from the player through k stops (first ... last) of `len` yards and `value`: close it, score it.
local function close(first, last, len, value)
	-- Headed nowhere (TX nil): the path comes back to the player, as a hand-in's detour counts it.
	local back = TX and leg(X[last], Y[last], TX, TY) or leg(X[last], Y[last], PX, PY)
	local extra = len + back - DIRECT
	if extra < 0 then extra = 0 end
	local secs = extra * PER
	if secs > LIMIT then return end
	local score = value / ((secs + FLOOR) / 60)
	-- Strictly better only: of equal paths the one found first (fewer stops, then lower index) stays.
	if not SCORE[first] or score > SCORE[first] then SCORE[first], SECS[first] = score, secs end
	if not bestScore or score > bestScore then bestFirst, bestScore, bestSecs = first, score, secs end
end

local function extend(first, last, len, value)
	close(first, last, len, value)
	for j = 1, N do
		if not USED[j] then
			USED[j] = true
			extend(first, j, len + leg(X[last], Y[last], X[j], Y[j]), value + V[j])
			USED[j] = false
		end
	end
end

-- The best path over stops 1..n (n at most MAX) from the player at px, py, on to the heading tx, ty
-- (nil: headed nowhere, or to another continent), where `perYard` turns yards into seconds and
-- `floor` seconds keep a stop on the way from scoring without end. Only paths whose extra walking
-- is at most `limit` seconds count. -> the first stop of the best path, its score, its extra
-- seconds; nil when no path fits. Plan.score and Plan.secs hold the best path of each first stop.
function Plan.Best(n, px, py, tx, ty, perYard, floor, limit)
	if n > MAX then n = MAX end
	N, PX, PY, TX, TY, PER, FLOOR, LIMIT = n, px, py, tx, ty, perYard, floor, limit
	DIRECT = tx and leg(px, py, tx, ty) or 0
	bestFirst, bestScore, bestSecs = nil, nil, nil
	for i = 1, MAX do SCORE[i], SECS[i], USED[i] = nil, nil, false end
	for i = 1, n do
		USED[i] = true
		extend(i, i, leg(px, py, X[i], Y[i]), V[i])
		USED[i] = false
	end
	return bestFirst, bestScore, bestSecs
end
