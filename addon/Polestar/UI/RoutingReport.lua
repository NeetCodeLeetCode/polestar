-- The routing report: one button in the guide window's menu (and /polestar routing)
-- that gathers why the arrow and the window chose what they did, as text to copy and
-- hand to whoever looks into it. The game gives addons no clipboard and no files, so
-- the report opens in the copy window (UI/Copy.lua), selected: Ctrl+C copies it.
--
-- It is the bug report's details (UI/Report.lua, R.Details: the guide and the step, the
-- goals, the arrow and its distance, where the player stands, routes and travel, While
-- you're here, the quest log, the settings changed, the errors caught) and, before them,
-- what the routing choices weighed (Engine.RoutingState): the step reorder's window,
-- best order and saving, the arrow that takes nearby objectives with each line's value,
-- score and seconds, and the Recalibrate count. The routing settings follow with their
-- values, changed or not.
--
-- No identifying text goes out. The lines hold race, class, level, zone and the ids of
-- quests, never a name. As a second guard RR.Scrub swaps whatever still names the
-- player for a placeholder: the character's name, the realm, the guild, the party's and
-- the raid's names, the account's other characters, links to players, a Battle.net tag,
-- an e-mail address and the folders in a path (user, account). Built on use only.

local ADDON, ns = ...
local UI = ns.UI
local L = ns.L
local floor, ipairs, pairs, type, tostring, tonumber = math.floor, ipairs, pairs, type, tostring, tonumber

local RR = {}
UI.RoutingReport = RR

-- The most lines the copy window shows at once, and its width.
RR.VIEW, RR.WIDTH, RR.LINE_MAX = 16, 520, 100

-- The settings that steer the route, with their values whether or not they changed: a
-- setting a Polestar does not have (no value) is left out.
RR.SETTINGS = {
	"behavior.whileHere", "behavior.whileHereRadius", "behavior.whileHereTurnins", "behavior.whileHereTurninSeconds",
	"behavior.whileHereAim", "behavior.whileHereAimSeconds", "behavior.whileHereDungeons", "behavior.whileHereFlights",
	"behavior.whileHereGather", "behavior.whileHereVendors", "behavior.stepReorder", "behavior.skipSlow", "behavior.skipGrey",
	"behavior.dungeons", "behavior.groupQuests", "behavior.rareDrops", "behavior.lostQuests", "behavior.routeChoice",
	"nav.routeStyle", "nav.navRouting", "nav.travel",
}

---------------------------------------------------------------------------
-- Keeping names out
---------------------------------------------------------------------------

-- Every name that identifies the player or whoever is with them, longest first so a
-- "Name-Realm" goes before its parts: { text, placeholder }. Names of one character
-- are not words of the guides, so a name under two letters is left alone.
function RR.Names()
	local list, seen = {}, {}
	local function add(name, label)
		if type(name) ~= "string" or #name < 2 or seen[name:lower()] then return end
		seen[name:lower()] = true
		list[#list + 1] = { name, label }
	end
	if UnitName then
		add(UnitName("player"), "<character>")
		add(UnitName("pet"), "<pet>")
		for i = 1, 4 do add(UnitName("party" .. i), "<party member>") end
		for i = 1, 40 do add(UnitName("raid" .. i), "<party member>") end
	end
	local C = ns.Compat
	local realm = (C and C.RealmName and C.RealmName()) or (GetRealmName and GetRealmName())
	if type(realm) == "string" then
		add(realm, "<realm>")
		add((realm:gsub("[%s%-']", "")), "<realm>")
	end
	if GetNormalizedRealmName then add(GetNormalizedRealmName(), "<realm>") end
	if GetGuildInfo then add((GetGuildInfo("player")), "<guild>") end
	-- The account's other characters (the Characters page): "Name-Realm" keys, names and realms.
	local db = ns.DB and ns.DB()
	local alts = type(db) == "table" and type(db.alts) == "table" and type(db.alts.chars) == "table" and db.alts.chars or {}
	for key, e in pairs(alts) do
		if type(key) == "string" then
			add(key, "<character>")
			for part in key:gmatch("[^%-]+") do add(part, "<character>") end
		end
		if type(e) == "table" then
			add(e.name, "<character>")
			add(e.realm, "<realm>")
		end
	end
	table.sort(list, function(a, b)
		if #a[1] ~= #b[1] then return #a[1] > #b[1] end
		return a[1] < b[1]
	end)
	return list
end

-- A byte that belongs to a word: a letter, a digit, an underscore or part of a UTF-8 letter.
local function wordByte(c)
	return c ~= "" and (c:find("^[%w_]") ~= nil or c:byte() >= 128)
end

-- `text` with each name of `names` (RR.Names) swapped for its placeholder wherever it
-- stands as a word, in any case; then the patterns that name a player or a person's
-- folders. Names go through a marker first, so a placeholder is never read as a name.
function RR.Scrub(text, names)
	text = tostring(text or "")
	text = text:gsub("|Hplayer:[^|]*|h[^|]*|h", "<player>")
	text = text:gsub("[%w_]+#%d%d%d%d+", "<battletag>")
	text = text:gsub("[%w%._%%%+%-]+@[%w%.%-]+%.%a%a+", "<email>")
	-- A path from the drive to the AddOns folder, a user's folder, the account's saved-variables folders.
	text = text:gsub("%a:[\\/][^\n]-Interface[\\/]AddOns", "Interface/AddOns")
	text = text:gsub("%a:[\\/][Uu]sers[\\/][^\\/%s]+", "<user folder>")
	text = text:gsub("/[Uu]sers/[^/%s]+", "<user folder>")
	text = text:gsub("/home/[^/%s]+", "<user folder>")
	text = text:gsub("Account[\\/][^%s]+", "Account/<account>")
	local labels = {}
	for i, e in ipairs(names or RR.Names()) do
		local name, low = e[1]:lower(), text:lower()
		local out, from = {}, 1
		while true do
			local a, b = low:find(name, from, true)
			if not a then break end
			if wordByte(text:sub(a - 1, a - 1)) or wordByte(text:sub(b + 1, b + 1)) then
				out[#out + 1] = text:sub(from, b)
			else
				labels[i] = e[2]
				out[#out + 1] = text:sub(from, a - 1) .. "\1" .. i .. "\2"
			end
			from = b + 1
		end
		out[#out + 1] = text:sub(from)
		text = table.concat(out)
	end
	return (text:gsub("\1(%d+)\2", function(i) return labels[tonumber(i)] end))
end

---------------------------------------------------------------------------
-- The report
---------------------------------------------------------------------------

local function fixed(n) return ("%.1f"):format(n or 0) end

-- "step 4 (guide step 7)": the step as the player counts it, then the guide's own index.
local function stepName(s)
	local E = ns.Engine
	local number = E and E.StepNumber and E.StepNumber(s)
	if number and number ~= s then return ("step %s (guide step %d)"):format(tostring(number), s) end
	return ("step %d"):format(s)
end

local function stepList(steps)
	local out = {}
	for i, s in ipairs(steps or {}) do out[i] = tostring((ns.Engine.StepNumber and ns.Engine.StepNumber(s)) or s) end
	return table.concat(out, " ")
end

local function onOff(v) return v and "on" or "off" end

-- A setting as the report writes it: key=on, key=180, key=balanced. The route style reads as the router reads it
-- (a style it does not know is Balanced) and real paths are on unless turned off, as Report's Routes line has them.
local function settingValue(path)
	local v = ns.Get(path)
	if path == "nav.routeStyle" then return (v == "fastest" or v == "roads") and v or "balanced" end
	if path == "nav.navRouting" then return onOff(v ~= false) end
	if type(v) == "boolean" then return onOff(v) end
	if type(v) == "number" then return tostring(floor(v * 100 + 0.5) / 100) end
	if type(v) == "string" then return (v:gsub("[^%w%._%-%+/#:,]", "_")) end
	return nil
end

-- The step reorder's lines: whether it is on, what it chose and why, the walking it
-- measured and what it needs to move a step.
local function reorderLines(r, add)
	local head = "Step reorder: " .. onOff(r.on)
	if r.on then
		head = head .. (" · live %s"):format(r.live and stepName(r.live) or "none")
		head = head .. (" · on screen %s"):format(r.shown and stepName(r.shown) or "the live step")
		if r.why then head = head .. " · " .. r.why end
		if (r.rest or 0) > 0 then head = head .. (" · rests %d s"):format(r.rest) end
	end
	add(head)
	if not r.on then return end
	local last = r.last
	if last and last.xp then
		-- The XP order (behavior.stepReorderXp): the seconds an XP point waits on average, not yards.
		local per = last.per or 1
		add(("Reorder XP: best order %s pays %s s sooner · an XP point waits %d s against the guide's %d s · measured %d s ago"):format(
			stepList(last.order), fixed(last.saved), floor((last.cost or 0) * per + 0.5), floor((last.base or 0) * per + 0.5), last.ago))
	elseif last then
		add(("Reorder walk: best order %s saves %s s · %d yd against the guide's %d yd · measured %d s ago"):format(
			stepList(last.order), fixed(last.saved), floor((last.cost or 0) + 0.5), floor((last.base or 0) + 0.5), last.ago))
	end
	add(("Reorder needs: a saving of %d s to show a step, %d s more to switch, under %d s to go back"):format(r.take, r.switch, r.release))
	if r.plan then add("Reorder plan: " .. stepList(r.plan)) end
	local win = {}
	for i, w in ipairs(r.window or {}) do
		win[i] = ("%s%s, %d quest%s"):format(stepName(w.step), w.ready and "" or " (waits for a hand-in or a pick-up)", w.quests, w.quests == 1 and "" or "s")
	end
	if #win > 0 then add("Reorder window: " .. table.concat(win, "; ")) end
end

-- The arrow that takes nearby objectives: its setting, what it weighed in its last poll
-- (each line's value in percent of a level, its score for each minute of extra walking,
-- the seconds it adds) and what it did.
local function aimLines(a, from, add)
	local head = "Take nearby objectives: " .. onOff(a.on)
	if a.on then
		head = head .. (" · up to %d s of extra walking"):format(a.limit)
		if a.held then head = head .. (" · holding the line of quest %s"):format(tostring(a.held.quest)) end
		if a.why then head = head .. " · " .. a.why .. (a.ago and (" (%d s ago)"):format(a.ago) or "") end
	end
	add(head)
	add("Arrow follows: " .. (from == "auto" and "an objective it took by itself" or from == "picked" and "a goal picked by a row, a click or a waypoint key" or "the guide's own next goal"))
	local last = a.on and a.last
	if not last then return end
	for i, line in ipairs(last.lines) do
		add(("Objective %d: quest %s · worth %s%% of a level · %s a minute, %s s extra%s"):format(i, tostring(line.quest), fixed(line.value),
			line.score and fixed(line.score) or "no score", line.secs and fixed(line.secs) or "?", last.held == i and " · held" or ""))
	end
	if last.first then
		add(("Best path: starts with objective %d · scores %s a minute, %s s extra%s · takes at %s, lets go under %s, a swap needs %s times as much"):format(
			last.first, fixed(last.score), fixed(last.secs), last.later and (" · %s after a later stop"):format(fixed(last.later)) or "",
			fixed(a.trigger), fixed(a.release), fixed(a.margin)))
	end
end

-- Every line of the report, in order: what was weighed, then the bug report's details.
function RR.Lines()
	local lines = {}
	local function add(line) lines[#lines + 1] = UI.Report.Cut(line, 400) end
	add(L["Polestar routing report"])
	local E = ns.Engine
	local st = E and E.RoutingState and E.RoutingState()
	if st then
		reorderLines(st.reorder, add)
		aimLines(st.aim, st.arrowFrom, add)
		if st.recalibrated then
			add(("Recalibrate: pressed %d time%s this session · last %d s ago"):format(st.recalibrated.count, st.recalibrated.count == 1 and "" or "s", st.recalibrated.ago))
		end
	end
	local settings = {}
	for _, path in ipairs(RR.SETTINGS) do
		local v = settingValue(path)
		if v then settings[#settings + 1] = path .. "=" .. v end
	end
	-- A line of at most LINE_MAX characters each, the first headed.
	local line = "Routing settings:"
	for _, text in ipairs(settings) do
		if #line + 1 + #text > RR.LINE_MAX then
			add(line)
			line = "  " .. text
		else
			line = line .. " " .. text
		end
	end
	add(line)
	local d = UI.Report.Details()
	-- The packed settings line is for the Discord bot; the list above it reads as it is.
	for i, line in ipairs(d.lines) do
		if d.kinds[i] ~= "settingcode" then lines[#lines + 1] = line end
	end
	if #(d.tail or {}) > 0 then
		lines[#lines + 1] = L["Log:"]
		for _, line in ipairs(d.tail) do lines[#lines + 1] = "> " .. line end
	end
	return lines
end

-- The report as text, with the names kept out.
function RR.Text()
	return RR.Scrub(table.concat(RR.Lines(), "\n"))
end

-- Opens the copy window on the report, selected for Ctrl+C. The game gives no clipboard and no file to addons.
function UI.ShowRoutingReport()
	UI.ShowCopy(L["Routing report"], RR.Text(), {
		icon = "bug", view = RR.VIEW, width = RR.WIDTH,
		hint = L["Press Ctrl+C (Cmd+C on a Mac) to copy the report, then paste it where you want to send it. It holds no character name, realm, guild, account, party names or chat."],
	})
end

ns.On("SHOW_ROUTING_REPORT", function() UI.ShowRoutingReport() end)
