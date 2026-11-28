-- Next stop's log: what each look of the card (UI/NextStopCard.lua) saw and decided, for the routing report
-- (UI/RoutingReportNextStop.lua). The card writes it; nothing in Next stop reads it back.
--
--   NextStopLog.Look(why, ctx, shown)   one look ended: `why` says how, `ctx` holds what it weighed
--   NextStopLog.Last() -> look | nil    the latest look
--   NextStopLog.Shown() -> look | nil   the look that showed the card last
--   NextStopLog.Clear()                 forget both (the tests)
--
-- look = { at (GetTime), why, ctx... } where ctx is the card's own table, kept as it is:
--   level, here (the place the player stands in, or nil), group, dungeons, hardcore, guideSecs (seconds to the
--   guide's own arrow, nil without one), places (what NextStop.Rank was given), ranking (what it returned),
--   key (the card's "level:place" key), told (that key was shown before)
-- A look that stopped early holds only the fields it had by then.

local ADDON, ns = ...

local NL = {}
ns.NextStopLog = NL

local last, shown

function NL.Look(why, ctx, wasShown)
	local look = {}
	for k, v in pairs(ctx or {}) do look[k] = v end
	look.why, look.at = why, GetTime and GetTime() or 0
	last = look
	if wasShown then shown = look end
	return look
end

function NL.Last() return last end
function NL.Shown() return shown end
function NL.Clear() last, shown = nil, nil end
