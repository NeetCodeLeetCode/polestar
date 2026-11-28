-- env: world
-- The Guide page's Dynamic Questing > Order the next steps by XP (Core/Engine.lua, Own.orderFill; Core/StepOrder.lua),
-- with Reorder the next steps on: the guide's first step hands in a quest worth 100 XP 600 yd east of the middle and its
-- second one a quest worth 5000 XP 600 yd west of it; both quests are finished and the walk is the same either way.
-- Each probe walks in from far off to the middle and reads the step on screen: the east one (1), or the west one
-- first (2), the one that pays soonest.
local w, E
local function at(x, y) w.moveTo(1429, x, y); stub.advance(1.2) end
return {
	setup = function()
		w, E = stub.world, Polestar.Engine
		w.addMap(1429, { name = "Elwynn Forest", c = 0, left = 1500, right = -1500, top = 500, bottom = -1500 })
		w.setLevel(5)
		w.setZone("Elwynn Forest", "")
		w.moveTo(1429, 47.6, 45.1)
		w.onQuest(33, { 8 }, "Wolves Across the Border")
		w.onQuest(7, { 10 }, "Kobold Camp Cleanup")
		w.finish(33)
		w.finish(7)
		XP = { [33] = 100, [7] = 5000 }
		GetQuestLogRewardXP = function(id) return XP[id] end
		Polestar.RegisterPack({ addon = "Polestar_Test", id = "test", format = 1, title = "Test", author = "Test", version = "1", license = "MIT" })
		Polestar.RegisterGuide({ id = "test/value", pack = "test", title = "Value", minLevel = 1, maxLevel = 10, steps = {
			{ title = "East", goals = { { t = "turnin", quest = 33, title = "Wolves Across the Border", npc = 33, npcName = "Giver", map = 1429, x = 67.6, y = 45.1 } } },
			{ title = "West", goals = { { t = "turnin", quest = 7, title = "Kobold Camp Cleanup", npc = 7, npcName = "Giver", map = 1429, x = 27.6, y = 45.1 } } },
		} })
		E.SetGuide("test/value")
		stub.fire("QUEST_LOG_UPDATE")
		stub.advance(0.6)
		Polestar.Set("behavior.stepReorder", true)
		stub.advance(0.6)
	end,
	settle = 0.3,
	probes = {
		["behavior.stepReorderXp"] = { probe = function()
			at(75.0, 80.0)
			at(47.6, 45.1)
			-- A step handed back for the walk away from it rests 30 s: the reload run took it before the walk.
			stub.advance(35)
			return E.GetStepIndex()
		end },
	},
}
