-- env: world
-- The Guide page's Dynamic Questing > Reorder the next steps (Core/Engine.lua, Own.orderTick): the guide's first
-- step is the wolves 600 yd west of the player and its second the kobold vermin 140 yd south, both quests in the log
-- and nothing between them. Each probe walks in from far off and reads the step on screen: the wolves (1), or the
-- vermin first (2), the order that walks less.
local w, E
local function at(x, y) w.moveTo(1429, x, y); stub.advance(1.2) end
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
		Polestar.RegisterGuide({ id = "test/order", pack = "test", title = "Order", minLevel = 1, maxLevel = 10, steps = {
			{ title = "Wolves", goals = { { t = "kill", target = "Young Wolf", npc = 299, count = 8, quest = 33, oi = 1, map = 1429, x = 20.0, y = 35.1 } } },
			{ title = "Vermin", goals = { { t = "kill", target = "Kobold Vermin", npc = 6, count = 10, quest = 7, oi = 1, map = 1429, x = 40.0, y = 42.1 } } },
		} })
		E.SetGuide("test/order")
		stub.fire("QUEST_LOG_UPDATE")
		stub.advance(0.6)
	end,
	settle = 0.3,
	probes = {
		["behavior.stepReorder"] = { probe = function()
			at(75.0, 80.0)
			at(40.0, 35.1)
			-- A step handed back for the walk away from it rests 30 s: the reload run took it before the walk.
			stub.advance(35)
			return E.GetStepIndex()
		end },
	},
}
