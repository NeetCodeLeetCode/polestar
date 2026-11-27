-- env: world
-- The Guide page's Dynamic Questing > Arrow takes nearby objectives (Core/Engine.lua, Own.hereAutoAim), as
-- engine-live-detour.rs plays it: the arrow leading west to the wolves 600 yd off, and a suggested objective (the
-- kobold vermin of quest 7, in the log) 140 yd south of the player, a detour of about 28 s. Each probe walks in from
-- far off (a line shown stays a little past its limit) and reads the step the arrow leads to.
local w, E
local function at(x, y) w.moveTo(1429, x, y); stub.advance(1.2) end
local function led()
	at(75.0, 80.0)
	at(40.0, 35.1)
	local wp = E.GetWaypoint()
	return wp and wp.stepIndex or false
end
return {
	setup = function()
		w, E = stub.world, Polestar.Engine
		w.addMap(1429, { name = "Elwynn Forest", c = 0, left = 1500, right = -1500, top = 500, bottom = -1500 })
		w.setLevel(5)
		w.setZone("Elwynn Forest", "")
		w.moveTo(1429, 40.0, 35.1)
		w.onQuest(33, { 8 }, "Wolves Across the Border")
		w.onQuest(7, { { required = 10, fulfilled = 8 } }, "Kobold Camp Cleanup")
		Polestar.RegisterPack({ addon = "Polestar_Test", id = "test", format = 1, title = "Test", author = "Test", version = "1", license = "MIT" })
		Polestar.RegisterGuide({ id = "test/aim", pack = "test", title = "Aim", minLevel = 1, maxLevel = 10, steps = {
			{ title = "Wolves", goals = { { t = "kill", target = "Young Wolf", npc = 299, count = 8, quest = 33, oi = 1, map = 1429, x = 20.0, y = 35.1 } } },
			{ title = "Vermin", goals = { { t = "kill", target = "Kobold Vermin", npc = 6, count = 10, quest = 7, oi = 1, map = 1429, x = 40.0, y = 42.1 } } },
		} })
		E.SetGuide("test/aim")
		stub.fire("QUEST_LOG_UPDATE")
		stub.advance(0.6)
	end,
	settle = 0.3,
	probes = {
		-- Within the 60 s the objective costs: the arrow leads to it (step 2), or stays with the wolves (step 1).
		["behavior.whileHereAim"] = { with = { ["behavior.whileHereAimSeconds"] = 60 }, probe = led },
		-- Switched on: 28 s is over the default 15 s and within 60.
		["behavior.whileHereAimSeconds"] = { with = { ["behavior.whileHereAim"] = true }, probe = led },
	},
}
