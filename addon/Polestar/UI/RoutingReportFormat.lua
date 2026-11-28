-- The words and numbers the routing report's lines are written with (UI/RoutingReport.lua and the sections of
-- UI/RoutingReport*.lua). English only: whoever reads the report reads it in the game's own terms.
--
--   F.fixed(n[, digits]) -> "12.3"          F.onOff(v) -> "on" | "off"
--   F.clock(seconds) -> "5:35"              F.ago(at) -> "312 s ago" | "5:12 ago" | "just now" (at: GetTime)
--   F.yards(n) -> "1052 yd"                 F.percent(n) -> "18.2%"
--   F.point(map, x, y, f) -> "The Barrens (map 1413 @52.10,31.40 live)"   the place as a guide writes it, with its name
--   F.straight(map, x, y, f) -> yards | nil   the straight distance from the player to a point, on one continent
--   F.travel(map, x, y, f) -> seconds | nil   the travel planner's estimate to a point
--   F.away(map, x, y, f) -> "45 yd straight, 8 s by the planner" | nil

local ADDON, ns = ...
local UI = ns.UI
local floor, sqrt = math.floor, math.sqrt

local F = {}
UI.ReportFormat = F

function F.fixed(n, digits) return ("%." .. (digits or 1) .. "f"):format(n or 0) end
function F.onOff(v) return v and "on" or "off" end
function F.percent(n) return F.fixed(n, 1) .. "%" end
function F.yards(n) return ("%d yd"):format(floor((n or 0) + 0.5)) end

function F.clock(seconds)
	seconds = floor((seconds or 0) + 0.5)
	return ("%d:%02d"):format(floor(seconds / 60), seconds % 60)
end

function F.ago(at)
	if not at then return "never" end
	local s = floor((GetTime and GetTime() or 0) - at + 0.5)
	if s < 2 then return "just now" end
	if s < 120 then return s .. " s ago" end
	return F.clock(s) .. " ago"
end

function F.point(map, x, y, f)
	local R = UI.Report
	local pos = R and R.Where and R.Where(map, x, y, f)
	if not pos then return "no position" end
	local name = ns.Compat and ns.Compat.GetMapName and ns.Compat.GetMapName(map)
	return (name and (name .. " (" .. pos .. ")")) or pos
end

function F.straight(map, x, y, f)
	local C = ns.Compat
	if not (C and C.GetPlayerWorldPosition and C.MapToWorld and map and x and y) then return nil end
	local pc, px, py = C.GetPlayerWorldPosition()
	local c, wx, wy = C.MapToWorld(map, x, y, f)
	if not (pc and c and pc == c) then return nil end
	return sqrt((wx - px) ^ 2 + (wy - py) ^ 2)
end

function F.travel(map, x, y, f)
	if not (ns.TravelCost and map and x and y) then return nil end
	local ok, secs = pcall(ns.TravelCost, nil, { map = map, x = x, y = y, f = f })
	return ok and secs or nil
end

function F.away(map, x, y, f)
	local d, secs = F.straight(map, x, y, f), F.travel(map, x, y, f)
	local parts = {}
	if d then parts[#parts + 1] = F.yards(d) .. " straight" else parts[#parts + 1] = "another continent or no position" end
	if secs then parts[#parts + 1] = ("%d s (%s) by the travel planner"):format(floor(secs + 0.5), F.clock(secs)) end
	return table.concat(parts, ", ")
end
