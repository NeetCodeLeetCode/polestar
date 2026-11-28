-- Fastest XP (Polestar/Core/FastestXP.lua): the switch, in Guide > Automation, and the button that puts its settings
-- back. Part of the settings' addon.

local ns = Polestar
local L = ns.L
local FX = ns.FastestXP

if not (FX and ns.RegisterOptions) then return end

local ok, err = ns.RegisterOptions({
	page = L["Guide"], tab = L["Automation"], section = L["Fastest XP"],
	controls = {
		{ kind = "toggle", key = "behavior.fastestXp", label = L["Fastest XP"], new = "0.4.4",
			help = L["Sets the leveling settings to their fastest values, so you need not know which to turn on. Off puts back what it changed, except what you changed since. Solo or group, dungeons and gathering stay yours. A Hardcore character keeps Take nearby objectives and Use deathskips as they are."],
			search = "fastest xp, fastest leveling, speed, quickest, preset, one click, best settings, recommended, level faster" },
		{ kind = "toggle", key = "behavior.nextStop", label = L["Suggest the next stop"], new = "0.4.4",
			help = L["With Fastest XP on, a card names the best next hub, zone or dungeon for XP for each minute when nothing is left to do here. Go points the arrow there. It never moves the arrow by itself."],
			search = "next stop, where to go next, next zone, next hub, suggest, go do this dungeon" },
		{ kind = "toggle", key = "behavior.routeAdvice", label = L["Route advice"], new = "0.4.4",
			help = L["A card tells you when a quest in your log is far off your route, when you did not pick up a quest on it, or when a quest players skip is in your log. It only advises: Abandon asks the game first, and it never abandons by itself. Fastest XP turns it on."],
			search = "route advice, off course, abandon quest, missed pickup, slow quest, running back and forth, wasted time" },
		{ kind = "button", wide = true, button = L["Restore"], label = L["Put Fastest XP's settings back"],
			help = L["Writes its values again into the settings you changed by hand since you turned it on."],
			disabled = function() return FX.State() ~= "customized" end,
			onClick = function() FX.Reapply() end },
	},
})
if not ok and geterrorhandler then geterrorhandler()(err) end
