-- A small window holding text to copy (a position from /polestar where, a
-- link): the band with a title, a read-only copy box with the text selected
-- and focused, and how to copy it. The ✕ and Escape close it.
-- UI.ShowCopy(title, text, opts), or SHOW_COPY(title, text) from Core.
-- opts: { icon = media name shown before the title, iconColor = palette
-- name (default text), hint = the line above the box, view = the most
-- lines the box shows at once, width = the window's width }; the Polestar
-- Discord's invite comes with its mark (UI/Report.lua). A text of more
-- lines than `view` scrolls in a well of that height (the routing report,
-- UI/RoutingReport.lua); a shorter one is as tall as it is.
--
-- Built on first use; colours are Theme tokens only.

local ADDON, ns = ...
local UI, Theme, W, Anim = ns.UI, ns.Theme, ns.Widgets, ns.Anim
local L = ns.L

local WIDTH = 360
local PAD = 24
local HEAD_H = 56

local VIEW_LINE = 12 -- the scrolling box's small font: points of one line
local F

local function build()
	F = CreateFrame("Frame", "PolestarCopy", ns.UIParent)
	F:SetFrameStrata("DIALOG")
	F:SetToplevel(true)
	F:SetClampedToScreen(true)
	F:EnableMouse(true)
	F:SetMovable(true)
	F:SetWidth(WIDTH)
	F:SetPoint("CENTER", UIParent, "CENTER", 0, 120)
	F:Hide()
	Theme.Skin(F, "window")

	local head = CreateFrame("Frame", nil, F)
	head:SetPoint("TOPLEFT", F, "TOPLEFT", 1, -1)
	head:SetPoint("TOPRIGHT", F, "TOPRIGHT", -1, -1)
	head:SetHeight(HEAD_H)
	ns.DragWindow(head, F)
	F.Head = head
	F.Band = Theme.Band(head, "BACKGROUND", -5)
	F.Band:SetAllPoints(head)
	F.Band:SetTexCoord(Theme.BandCoords(WIDTH - 2, HEAD_H))
	-- The house window's light: the gold rim over a faint haze in the band.
	W.Haze(head, { width = WIDTH * 0.9, height = HEAD_H })
	W.Rim(F, { width = WIDTH * 0.7 })
	F.Close = W.IconButton(head, "close", 28, L["Close"], function() F:Hide() end, 14)
	F.Close:SetPoint("TOPRIGHT", head, "TOPRIGHT", -12, -14)
	W.FrameClose(F, F.Close)
	F.Icon = head:CreateTexture(nil, "ARTWORK")
	F.Icon:SetSize(28, 28)
	F.Icon:SetPoint("LEFT", head, "LEFT", PAD - 2, 0)
	F.Icon:Hide()
	F.Title = W.Text(head, "title", "text")
	F.Title:SetPoint("RIGHT", F.Close, "LEFT", -8, 0)
	F.Title:SetJustifyH("LEFT")
	F.Title:SetWordWrap(false)

	F.Hint = W.Text(F, "note", "textDim")
	F.Hint:SetPoint("TOPLEFT", F, "TOPLEFT", PAD, -(HEAD_H + 12))
	F.Hint:SetWidth(WIDTH - PAD * 2)
	F.Hint:SetJustifyH("LEFT")
	F.Hint:SetWordWrap(true)

	F:SetScript("OnShow", function(self) Anim.To(self, "alpha", 0, 1, Anim.FAST, "out") end)
	ns.CloseOnEscape("PolestarCopy")
	if ns.Gamepad and ns.Gamepad.Register then ns.Gamepad.Register(F, {}) end
end

-- The box is made for the number of lines it holds.
local function box(lines)
	local cur = rawget(F, "Box")
	if cur and cur.psLines == lines then
		cur:Show()
		return cur
	end
	if cur then cur:Hide() end
	F.boxes = F.boxes or {}
	local b = F.boxes[lines]
	if not b then
		b = W.CopyBox(F, { lines = lines })
		b.psLines = lines
		b:SetWidth(WIDTH - PAD * 2)
		-- Escape in the box closes the window (the text is copied by then).
		b:SetScript("OnEscapePressed", function() F:Hide() end)
		F.boxes[lines] = b
	end
	F.Box = b
	b:Show()
	return b
end

-- The scrolling box for a long text: a well, its scroll view and the copy box in it (the report window's
-- pattern, UI/Report.lua), built once; `width` is the window's.
local function scrollBox(width)
	if not rawget(F, "ScrollWell") then
		local well = CreateFrame("Frame", nil, F)
		local sf = W.Scroll(well)
		sf:SetAllPoints(well)
		local b = W.ScrollChild(sf, W.CopyBox(sf, { lines = 2, well = well }))
		Theme.Font(b, "small")
		b:SetScript("OnEscapePressed", function() F:Hide() end)
		well:EnableMouse(true)
		well:SetScript("OnMouseUp", function() W.SelectCopy(b) end)
		-- The text in the box's font at its inner width: what its height is measured on. Hidden, never alpha 0.
		local m = W.Text(F, "small", "text")
		m:SetJustifyH("LEFT")
		m:SetWordWrap(true)
		m:SetNonSpaceWrap(true)
		m:Hide()
		F.ScrollWell, F.ScrollView, F.ScrollBox, F.Measure = well, sf, b, m
	end
	F.ScrollWell:SetWidth(width - PAD * 2)
	F.ScrollBox:SetWidth(width - PAD * 2)
	F.Measure:SetWidth(width - PAD * 2 - 20)
	return F.ScrollBox
end

function UI.ShowCopy(title, text, opts)
	if not F then build() end
	opts = opts or {}
	text = text or ""
	local lines = 1
	for _ in text:gmatch("\n") do lines = lines + 1 end
	local width = opts.width or WIDTH
	F:SetWidth(width)
	F.Band:SetTexCoord(Theme.BandCoords(width - 2, HEAD_H))
	F.Hint:SetWidth(width - PAD * 2)
	F.Title:SetText(title or "")
	F.Title:ClearAllPoints()
	F.Title:SetPoint("RIGHT", F.Close, "LEFT", -8, 0)
	if opts.icon then
		F.Icon:SetTexture(Theme.Media(opts.icon))
		W.Color(F.Icon, opts.iconColor or "text")
		F.Icon:Show()
		F.Title:SetPoint("LEFT", F.Icon, "RIGHT", 10, 0)
	else
		F.Icon:Hide()
		F.Title:SetPoint("LEFT", F.Head, "LEFT", PAD, 0)
	end
	F.Hint:SetText(opts.hint or L["Selected for you: copy it with Ctrl+C (Cmd+C on a Mac), then press Escape."])
	local scrolls = opts.view and lines > opts.view
	local cur = rawget(F, "Box")
	if cur then cur:Hide() end
	if rawget(F, "ScrollWell") then F.ScrollWell:Hide() end
	local b, height
	if scrolls then
		b = scrollBox(width)
		F.ScrollWell:ClearAllPoints()
		F.ScrollWell:SetPoint("TOPLEFT", F.Hint, "BOTTOMLEFT", 0, -10)
		F.ScrollWell:Show()
		b:Show()
		W.SetCopyText(b, text)
		-- As tall as the wrapped text (never under a line per line of it), the well a view of at most opts.view lines.
		F.Measure:SetText(text)
		local content = math.floor(math.max(F.Measure:GetStringHeight(), lines * VIEW_LINE) + 16 + 0.5)
		b:SetHeight(content)
		height = math.min(content, opts.view * VIEW_LINE + 16)
		F.ScrollWell:SetHeight(height)
		W.SetScrollContent(F.ScrollView, content, height)
		W.ScrollTo(F.ScrollView, 0)
	else
		b = box(lines)
		b:ClearAllPoints()
		b:SetPoint("TOPLEFT", F.Hint, "BOTTOMLEFT", 0, -10)
		b:SetWidth(width - PAD * 2)
		W.SetCopyText(b, text)
		height = b:GetHeight()
	end
	F:SetHeight(HEAD_H + 12 + F.Hint:GetStringHeight() + 10 + height + PAD)
	F:Show()
	F:Raise()
	W.SelectCopy(b)
end

ns.On("SHOW_COPY", function(title, text) UI.ShowCopy(title, text) end)
