-- Fastest XP's switch first among the Guide page's Essentials (UI/Essentials.lua builds the list; this loads after it).

local ns = Polestar
local O = ns.UI and ns.UI.Options
local G = O and O.Grouped
if not (G and G.ESSENTIALS and G.ESSENTIALS.guide) then return end

table.insert(G.ESSENTIALS.guide, 1, "behavior.fastestXp")
