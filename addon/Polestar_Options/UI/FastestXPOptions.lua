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
		{ kind = "button", wide = true, button = L["Restore"], label = L["Put Fastest XP's settings back"],
			help = L["Writes its values again into the settings you changed by hand since you turned it on."],
			disabled = function() return FX.State() ~= "customized" end,
			onClick = function() FX.Reapply() end },
	},
})
if not ok and geterrorhandler then geterrorhandler()(err) end
