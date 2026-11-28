-- env: world
-- The Guide page's route rows (Polestar_Options/UI/Options.lua; Core/Engine.lua) on the whole addon, each on a
-- guide shaped like its engine test's: a quest the route passed abandoned (engine-lost-quest.rs), a group quest
-- (engine-group-default.rs), a dungeon's pick-up in the route (engine-dungeon-route.rs), a grey quest at level 20
-- (engine-smart.rs), a deathskip (engine-die.rs), and a guide with a step skipped by hand for Reset guide progress.
-- A level 1 Human Warrior in Elwynn Forest.
local w, E
local function flush() stub.advance(0.6) end
local function open(id)
	E.SetGuide(id)
	stub.fire("QUEST_LOG_UPDATE")
	flush()
end
local level
return {
	setup = function()
		w, E = stub.world, Polestar.Engine
		-- Dungeons in the route are held back while Core/Init.lua's switch is off (dungeon-route-held.rs): on here,
		-- so "Dungeons in my route" shows and is checked.
		Polestar.dungeonRoute = true
		w.addMap(1429, { name = "Elwynn Forest", c = 0, left = 1500, right = -1500, top = 500, bottom = -1500 })
		w.addMap(1411, { name = "Durotar", c = 1, left = -1962.5, right = -7250, top = 1808.333, bottom = -1716.667 })
		w.moveTo(1429, 48, 42)
		w.titles[783] = "A Threat Within"
		w.titles[7] = "Kobold Camp Cleanup"
		UnitIsDeadOrGhost = UnitIsDeadOrGhost or function() return false end
		Polestar.RegisterPack({ addon = "Polestar_Test", id = "test", format = 1, title = "Test", author = "Test", version = "1", license = "MIT" })
		Polestar.RegisterGuide({ id = "test/lost", pack = "test", title = "Lost", minLevel = 1, maxLevel = 10, steps = {
			{ title = "Deputy Willem", goals = { { t = "accept", quest = 783, title = "A Threat Within", npc = 823, npcName = "Deputy Willem", map = 1429, x = 48.2, y = 42.0 } } },
			{ title = "Marshal McBride", goals = { { t = "accept", quest = 7, title = "Kobold Camp Cleanup", npc = 197, npcName = "Marshal McBride", map = 1429, x = 48.9, y = 41.6 } } },
			{ title = "Trainer", goals = { { t = "train", npc = 911, npcName = "Llane Beshere" } } },
		} })
		Polestar.RegisterGuide({ id = "test/group", pack = "test", title = "Group", minLevel = 1, maxLevel = 10, steps = {
			{ title = "Group quest", when = { { k = "group", q = 7 } }, goals = {
				{ t = "accept", quest = 7, title = "Kobold Camp Cleanup", npc = 197, npcName = "Marshal McBride", map = 1429, x = 48.9, y = 41.6 },
			} },
			{ title = "The solo way", when = { { k = "group", q = 7, neg = true } }, goals = { { t = "task", text = "Pull one at a time" } } },
			{ title = "Marshal McBride", goals = { { t = "turnin", quest = 783, title = "A Threat Within", npc = 197, npcName = "Marshal McBride", map = 1429, x = 48.9, y = 41.6 } } },
		} })
		Polestar.RegisterGuide({ id = "test/grey", pack = "test", title = "Grey", minLevel = 15, maxLevel = 25, steps = {
			{ goals = { { t = "accept", quest = 101, title = "Grey alone", qlvl = 2 } } },
			{ goals = { { t = "accept", quest = 104, title = "Current", qlvl = 19 } } },
			{ goals = { { t = "level", level = 30 } } },
		} })
		Polestar.RegisterGuide({ id = "test/die", pack = "test", title = "Die", minLevel = 1, maxLevel = 10, steps = {
			{ title = "Deathskip to Sen'jin Village", goals = { { t = "die", location = "Sen'jin Village", map = 1411, x = 57.2, y = 73.3, f = "e" } } },
			{ title = "Razor Hill", goals = { { t = "accept", quest = 840, title = "Conscript of the Horde", npc = 3336, npcName = "Takrin Pathseeker", map = 1411, x = 50.8, y = 43.6 } } },
		} })
		-- A quest an item starts (engine-rare-drops.rs): the book's spot is nearer than the kobolds.
		Polestar.RegisterGuide({ id = "test/drops", pack = "test", title = "Drops", minLevel = 1, maxLevel = 30, steps = {
			{ title = "Woods", goals = {
				{ t = "kill", target = "Kobold Vermin", npc = 6, count = 10, map = 1429, x = 47.6, y = 35.1 },
				{ t = "collect", item = 2794, itemName = "An Old History Book", count = 1, optional = true, map = 1429, x = 48.5, y = 40.5 },
				{ t = "accept", quest = 337, title = "An Old History Book", item = 2794, itemName = "An Old History Book", optional = true },
			} },
		} })
		-- A quest of Core/SlowQuestList.lua (engine-slow-quests.rs): Guile of the Raptor, left out or offered.
		Polestar.RegisterGuide({ id = "test/slow", pack = "test", title = "Slow", minLevel = 30, maxLevel = 40, steps = {
			{ title = "Tor'gan", goals = { { t = "accept", quest = 701, title = "Guile of the Raptor", npc = 2561, npcName = "Tor'gan", map = 1429, x = 48.9, y = 41.6 } } },
			{ title = "Marshal McBride", goals = { { t = "accept", quest = 7, title = "Kobold Camp Cleanup", npc = 197, npcName = "Marshal McBride", map = 1429, x = 48.9, y = 41.6 } } },
		} })
		Polestar.RegisterGuide({ id = "test/reset", pack = "test", title = "Reset", minLevel = 1, maxLevel = 10, steps = {
			{ goals = { { t = "accept", quest = 201, title = "First" } } },
			{ goals = { { t = "accept", quest = 202, title = "Second" } } },
			{ goals = { { t = "accept", quest = 203, title = "Third" } } },
		} })
		-- A dungeon in the route (engine-dungeon-route.rs): its pick-up ahead of the live step, behind `when dungeon`.
		Polestar.RegisterGuide({ id = "test/dungeon", pack = "test", title = "Dungeon", minLevel = 1, maxLevel = 30, steps = {
			{ title = "Marshal McBride", goals = { { t = "accept", quest = 7, title = "Kobold Camp Cleanup", npc = 197, npcName = "Marshal McBride", map = 1429, x = 48.9, y = 41.6 } } },
			{ title = "Scout Riell", when = { { k = "dungeon", name = "The Deadmines", map = 36 } }, goals = {
				{ t = "accept", quest = 214, title = "Red Silk Bandanas", npc = 6087, npcName = "Scout Riell", map = 1429, x = 49.5, y = 41.0 },
			} },
			{ title = "Deputy Willem", goals = { { t = "accept", quest = 783, title = "A Threat Within", npc = 823, npcName = "Deputy Willem", map = 1429, x = 48.2, y = 42.0 } } },
		} })
		open("test/reset")
	end,
	probes = {
		-- Kobold Camp Cleanup, taken (the route at the trainer), abandoned: the guide holds and asks, or goes back
		-- for it at once. The quest taken again afterwards.
		["behavior.lostQuests"] = { prepare = function()
			w.onQuest(783, {})
			w.onQuest(7, {})
			open("test/lost")
		end, probe = function()
			w.abandon(7); flush()
			local q = E.LostQuestion()
			local out = { live = E.GetLiveStep(), asks = q or false }
			if q then E.AnswerLost(q, "back"); flush() end
			w.onQuest(7, {}); stub.fire("QUEST_LOG_UPDATE"); flush()
			return out
		end },
		-- The book's start: optional as written, the arrow leads to the book, or the start is left out.
		["behavior.rareDrops"] = { prepare = function() open("test/drops") end, probe = function()
			local v = E.GetStepView(1)
			return { current = v.current, book = v.goals[2].status, accept = v.goals[3].status }
		end },
		-- Kobold Camp Cleanup marked for a group: the guide waits and asks, or takes it with a group, or the solo way.
		["behavior.groupQuests"] = { prepare = function()
			if w.log[7] then w.abandon(7) end
			w.onQuest(783, {})
			open("test/group")
		end, probe = function() return { at = E.GetLiveStep(), asks = E.GroupQuestion() or false } end },
		-- The Deadmines' pick-up ahead: no number for a character who skips it (the solo route's numbers), its own
		-- number when asked about or run.
		["behavior.dungeons"] = { prepare = function() open("test/dungeon") end, probe = function()
			return { total = E.StepTotal(), third = (E.StepNumber(3)), pickup = select(2, E.StepNumber(2)) }
		end },
		-- At level 20, a level 2 quest: left out (the route on the next), or the guide's first step.
		["behavior.skipGrey"] = { prepare = function()
			level = w.level
			w.setLevel(20)
			open("test/grey")
		end, probe = function() return E.GetLiveStep() end, cleanup = function() w.setLevel(level) end },
		-- Guile of the Raptor, a quest of the slow list: offered, or left out.
		["behavior.skipSlow"] = { prepare = function() open("test/slow") end, probe = function()
			return { slow = E.GetStepView(1).goals[1].status, other = E.GetStepView(2).goals[1].status }
		end },
		-- Fastest XP (fastest-xp.rs) writes Skip slow quests: Guile of the Raptor is left out while it is on. It writes
		-- other settings, which a profile with only itself at the default, or a reload of only itself, keeps as they were.
		["behavior.fastestXp"] = { once = true, reload = false, prepare = function() open("test/slow") end, probe = function()
			return { slow = E.GetStepView(1).goals[1].status, other = E.GetStepView(2).goals[1].status }
		end, cleanup = function() Polestar.Set("behavior.skipSlow", false) end },
		-- Suggest the next stop: the card is due only while Fastest XP is on in a level-range guide.
		["behavior.nextStop"] = { once = true, reload = false, prepare = function()
			open("test/slow")
			Polestar.Set("behavior.fastestXp", true); flush()
		end, probe = function() return Polestar.UI.NextStopCard.Enabled() end,
			cleanup = function() Polestar.Set("behavior.fastestXp", false) end },
		-- Route advice: the card is due only while the setting is on, in a level-range guide.
		["behavior.routeAdvice"] = { prepare = function() open("test/slow") end, probe = function() return Polestar.UI.RouteAdviceCard.Enabled() end },
		-- Skip slow quests turned off by hand while Fastest XP is on: the button writes its value again.
		["Guide:Put Fastest XP's settings back"] = { press = true, prepare = function()
			open("test/slow")
			Polestar.Set("behavior.fastestXp", true); flush()
			Polestar.Set("behavior.skipSlow", false); flush()
		end, probe = function() return E.GetStepView(1).goals[1].status end,
			cleanup = function() Polestar.Set("behavior.fastestXp", false) end },
		-- A deathskip to Sen'jin Village: the guide's step, or passed.
		["behavior.deathskips"] = { prepare = function() open("test/die") end, probe = function()
			return { at = E.GetLiveStep(), status = E.GetStepView(1).goals[1].status }
		end },
		-- The first step skipped by hand: back once reset.
		["Guide:Reset guide progress"] = { press = true, prepare = function()
			open("test/reset")
			E.Skip(); flush()
		end, probe = function() return E.GetLiveStep() end },
	},
}
