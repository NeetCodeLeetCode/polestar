//! The next-stop heads-up (addon/Polestar/UI/NextStopCard.lua, behavior.nextStop) and its places (Core/NextStopPlaces.lua):
//! with Fastest XP on in a level-range guide, and the guide's own arrow far off, a card names the best place for XP for
//! each minute, told once for each place and level, with Go (a trip) and Turn these off. The whole addon on the stub
//! world, with the synthetic Codex tests/fixtures/codex/free-mode.json (the same one as free-mode.rs): a Human Warrior at
//! level 20 in Hale's Reach (map 9001), whose quests are done, so Merrow Fields (map 9002) has four to take (three at
//! Scout Merrow's hub, one across the fields).

use forge_addontest::codex::{fixture, write_codex_pack, CodexOpts, CodexPack};
use forge_addontest::{emitted, Addon, RunAddon, TEST_PACK};
use serde_json::{json, Value};

thread_local! {
	static PACK: CodexPack = write_codex_pack(&fixture("free-mode.json"), &CodexOpts::default());
}

fn maps() -> String {
	let codex = fixture("free-mode.json");
	let mut frames: Vec<(&String, &Value)> = codex["frames"].as_object().unwrap().iter().collect();
	frames.sort_by_key(|(id, _)| id.parse::<u64>().unwrap_or(u64::MAX));
	frames
		.into_iter()
		.map(|(id, f)| {
			format!(
				"w.addMap({id}, {{ name = {}, c = {}, left = {}, right = {}, top = {}, bottom = {} }})",
				serde_json::to_string(&codex["zones"][id.as_str()]["name"]).unwrap(),
				f["c"],
				f["left"],
				f["right"],
				f["top"],
				f["bottom"]
			)
		})
		.collect::<Vec<_>>()
		.join("\n")
}

// A level-range guide whose arrow is far off (over five minutes), and one that is not a level-range guide.
const GUIDES: &str = r#"
Polestar.RegisterGuide({ id = "test/far", pack = "test", title = "Far", minLevel = 1, maxLevel = 60, steps = {
	{ title = "Far away", goals = { { t = "goto", map = 9001, x = 95, y = 10, text = "Far camp" } } },
} })
Polestar.RegisterGuide({ id = "test/near", pack = "test", title = "Near", minLevel = 1, maxLevel = 60, steps = {
	{ title = "Next door", goals = { { t = "goto", map = 9001, x = 12, y = 88, text = "Near camp" } } },
} })
Polestar.RegisterGuide({ id = "test/dungeon", pack = "test", title = "Dungeon", category = "Dungeons", minLevel = 1, maxLevel = 60, steps = {
	{ title = "Far away", goals = { { t = "goto", map = 9001, x = 95, y = 10, text = "Far camp" } } },
} })
"#;

const PRELUDE: &str = r#"
local E, CT, NSC, FX = Polestar.Engine, Polestar.UI.ClassToast, Polestar.UI.NextStopCard, Polestar.FastestXP
function card()
	local m = CT.current
	if not m then return false end
	return { kind = m.kind, title = m.title, lines = m.giver, route = m.route, act = m.act and m.act.label or false,
		off = m.off and m.off.label or false }
end
-- Look until the scan has ended (a slice a frame).
function look()
	NSC.Look()
	stub.advance(1)
	NSC.Look()
	stub.advance(1)
end
"#;

fn run(script: &str) -> Value {
	let pack = PACK.with(|p| p.stub_lua.clone());
	let before = format!(
		"{pack}\nlocal w = stub.world\n{}\nw.level, w.race, w.raceName, w.faction = 20, \"Human\", \"Human\", \"Alliance\"\nw.class, w.className = \"WARRIOR\", \"Warrior\"\nw.moveTo(9001, 10, 90)\nw.complete({{ 7001, 7002, 7003, 7004, 7005, 7106, 7110 }})",
		maps()
	);
	emitted(&RunAddon {
		stubs: vec!["stub_world.lua"],
		before,
		addons: vec![Addon::name("Polestar")],
		login: true,
		script: format!("{TEST_PACK}\n{GUIDES}\nstub.advance(0.5)\n{PRELUDE}\n{script}"),
		..Default::default()
	})
	.remove(0)
}

#[test]
fn with_fastest_xp_on_and_nothing_near_the_card_names_the_zone_with_the_most_xp_for_the_trip() {
	let out = run(r#"
		E.SetGuide("test/far"); stub.advance(1)
		Polestar.Set("behavior.fastestXp", true); stub.advance(1)
		look()
		emit({ card = card(), leveling = E.IsLeveling() })"#);
	assert_eq!(out["leveling"], true);
	assert_eq!(out["card"]["kind"], "nextstop");
	assert_eq!(out["card"]["title"], "Go to Merrow Fields");
	let lines = out["card"]["lines"].as_str().unwrap();
	assert!(lines.starts_with("4 quests for your level at Merrow Fields. About "), "{lines}");
	assert_eq!(out["card"]["act"], "Go");
	assert_eq!(out["card"]["off"], "Turn these off");
}

#[test]
fn it_is_told_once_for_a_place_and_level_and_not_at_all_when_the_mode_the_setting_or_the_guide_says_no() {
	let out = run(r#"
		local r = {}
		E.SetGuide("test/far"); stub.advance(1)
		look()
		r.modeOff = card()
		Polestar.Set("behavior.fastestXp", true); stub.advance(1)
		look()
		r.first = card() and card().kind
		CT.Hide()
		look()
		r.again = card()
		Polestar.Set("behavior.nextStop", false); stub.advance(1)
		w.level = 20
		Polestar.Set("behavior.nextStop", true)
		E.SetGuide("test/dungeon"); stub.advance(1)
		look()
		r.dungeonGuide = card()
		emit(r)"#);
	assert_eq!(out["modeOff"], false);
	assert_eq!(out["first"], "nextstop");
	assert_eq!(out["again"], false);
	assert_eq!(out["dungeonGuide"], false);
}

#[test]
fn no_card_while_the_guides_own_arrow_is_near() {
	let out = run(r#"
		E.SetGuide("test/near"); stub.advance(1)
		Polestar.Set("behavior.fastestXp", true); stub.advance(1)
		look()
		emit({ card = card() })"#);
	assert_eq!(out["card"], false);
}

#[test]
fn off_hides_the_card_and_sets_the_setting_off() {
	let out = run(r#"
		E.SetGuide("test/far"); stub.advance(1)
		Polestar.Set("behavior.fastestXp", true); stub.advance(1)
		look()
		CT.current.off.run()
		emit({ on = Polestar.Get("behavior.nextStop") })"#);
	assert_eq!(out, json!({ "on": false }));
}

#[test]
fn go_takes_a_trip_to_the_hub_of_the_place() {
	let out = run(r#"
		E.SetGuide("test/far"); stub.advance(1)
		Polestar.Set("behavior.fastestXp", true); stub.advance(1)
		look()
		local ok = CT.current.act.run()
		local t = Polestar.Nav.GetTarget and Polestar.Nav.GetTarget()
		emit({ ok = ok, map = t and t.map, hidden = CT.current == nil or not CT.current })"#);
	assert_eq!(out["ok"], true);
	assert_eq!(out["map"], 9002);
}

// A dungeon the character can run for its quests: 120 percent of a level in quests ready, at an entrance in Merrow
// Fields. The dungeon list is the maintainer's: only its three calls are stood in for.
const DUNGEON: &str = r#"
local D = Polestar.Dungeons
D.List = function() return { { key = "tdun", name = "Test Dungeon", kind = "dungeon", minLevel = 18, maxLevel = 26 } } end
D.QuestXP = function(key) return { ready = { from = 20, to = 21.2, levels = 1.2 }, groups = { ready = { n = 5, xp = 99999 } } } end
D.Entrance = function(key) return 9002, 30, 30, "e" end
"#;

#[test]
fn a_dungeon_is_offered_only_to_a_group_of_three_or_when_the_route_takes_dungeons() {
	let out = run(&format!(
		r#"
		{DUNGEON}
		local r = {{}}
		E.SetGuide("test/far"); stub.advance(1)
		Polestar.Set("behavior.fastestXp", true); stub.advance(1)
		look()
		r.alone = card() and card().title
		CT.Hide()
		NSC.Reset()
		GetNumGroupMembers = function() return 3 end
		look()
		r.group = card() and {{ title = card().title, lines = card().lines }}
		CT.Hide()
		NSC.Reset()
		GetNumGroupMembers = function() return 1 end
		Polestar.Set("behavior.dungeons", "run")
		look()
		r.route = card() and card().title
		emit(r)"#
	));
	assert_eq!(out["alone"], "Go to Merrow Fields");
	assert_eq!(out["group"]["title"], "Go do Test Dungeon now");
	let lines = out["group"]["lines"].as_str().unwrap();
	assert!(lines.starts_with("5 quests inside are ready for you. About 120% of a level in "), "{lines}");
	assert!(lines.ends_with(", not counting kills."), "{lines}");
	assert_eq!(out["route"], "Go do Test Dungeon now");
}
