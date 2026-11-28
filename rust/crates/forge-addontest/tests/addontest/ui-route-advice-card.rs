//! The route advice card (addon/Polestar/UI/RouteAdviceCard.lua, behavior.routeAdvice) on the stub world, with the
//! synthetic Codex tests/fixtures/codex/free-mode.json (the same one as ui-next-stop-card.rs): a Human Warrior at
//! level 20 in Hale's Reach (map 9001) at (10, 90), a guide whose next stop is next door, and Lurkers in the Marsh
//! (7003) in the log, with a detour of seven minutes stood in. The card tells a quest off the route to be abandoned, a quest of the hand-written list, and a missed pickup. It never abandons,
//! takes or leaves a quest by itself.

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

const GUIDES: &str = r#"
Polestar.RegisterGuide({ id = "test/near", pack = "test", title = "Near", minLevel = 1, maxLevel = 60, steps = {
	{ title = "Next door", goals = { { t = "goto", map = 9001, x = 12, y = 88, text = "Near camp" } } },
} })
Polestar.RegisterGuide({ id = "test/dungeon", pack = "test", title = "Dungeon", category = "Dungeons", minLevel = 1, maxLevel = 60, steps = {
	{ title = "Next door", goals = { { t = "goto", map = 9001, x = 12, y = 88, text = "Near camp" } } },
} })
"#;

const PRELUDE: &str = r#"
local E, CT, RAC, FX = Polestar.Engine, Polestar.UI.ClassToast, Polestar.UI.RouteAdviceCard, Polestar.FastestXP
function card()
	local m = CT.current
	if not m then return false end
	return { kind = m.kind, head = m.head, title = m.title, lines = m.giver, route = m.route, act = m.act and m.act.label or false,
		off = m.off and m.off.label or false }
end
-- The advice on, in the guide next door. The pack has no quest database, so the quest's detour is stood in (the real
-- one: engine-route-advice-reads.rs): `detour` seconds, or none when false.
function ready(detour)
	Polestar.Set("behavior.routeAdvice", true)
	E.SetGuide("test/near"); stub.advance(1)
	local seconds = detour == nil and 420 or detour
	E.QuestDetour = function() return seconds or nil end
end
function look() RAC.Look(); stub.advance(0.1) end
-- Ten minutes and a bit later: the gap between two cards, and a quest no longer fresh.
function later() stub.now = stub.now + 700 end
"#;

fn run(before: &str, script: &str) -> Value {
	let pack = PACK.with(|p| p.stub_lua.clone());
	let before = format!(
		"{pack}\nlocal w = stub.world\n{}\nw.level, w.race, w.raceName, w.faction = 20, \"Human\", \"Human\", \"Alliance\"\nw.class, w.className = \"WARRIOR\", \"Warrior\"\nw.moveTo(9001, 10, 90)\nw.titles[7003] = \"Lurkers in the Marsh\"\nw.titles[701] = \"Guile of the Raptor\"\n{before}",
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

const FAR_QUEST: &str = "w.onQuest(7003, { 6 })";

#[test]
fn it_is_off_by_default_and_fastest_xp_turns_it_on_and_off() {
	let out = run(
		FAR_QUEST,
		r#"
		local r = { default = Polestar.Get("behavior.routeAdvice") }
		ready(); Polestar.Set("behavior.routeAdvice", false)
		look()
		r.card = card()
		Polestar.Set("behavior.fastestXp", true); stub.advance(1)
		r.on = Polestar.Get("behavior.routeAdvice")
		Polestar.Set("behavior.fastestXp", false); stub.advance(1)
		r.off = Polestar.Get("behavior.routeAdvice")
		emit(r)"#,
	);
	assert_eq!(out, json!({ "default": false, "card": false, "on": true, "off": false }));
}

#[test]
fn a_log_quest_far_off_the_route_is_told_to_be_abandoned_with_its_detour() {
	let out = run(FAR_QUEST, "ready(); look(); emit({ card = card() })");
	assert_eq!(out["card"]["kind"], "routeadvice");
	assert_eq!(out["card"]["title"], "We recommend abandoning Lurkers in the Marsh");
	let lines = out["card"]["lines"].as_str().unwrap();
	assert!(lines.starts_with("Finishing it adds about "), "{lines}");
	assert_eq!(out["card"]["act"], "Abandon");
	assert_eq!(out["card"]["off"], "Keep it");
}

#[test]
fn abandon_asks_the_game_and_never_abandons_by_itself() {
	let out = run(
		FAR_QUEST,
		r#"
		local asked
		Polestar.Compat.AskAbandonQuest = function(q) asked = q; return true end
		ready(); look()
		local before = asked
		CT.current.act.run()
		emit({ before = before or false, asked = asked, stillInLog = stub.world.log[7003] ~= nil })"#,
	);
	assert_eq!(out, json!({ "before": false, "asked": 7003, "stillInLog": true }));
}

#[test]
fn keep_it_is_remembered_for_the_character() {
	let out = run(
		FAR_QUEST,
		r#"
		ready(); look()
		CT.current.off.run()
		local r = { kept = Polestar.CharState().routeAdvice[7003] }
		later(); RAC.Reset(); look()
		r.again = card()
		emit(r)"#,
	);
	assert_eq!(out, json!({ "kept": true, "again": false }));
}

#[test]
fn a_quest_is_told_once_for_a_level_and_cards_wait_ten_minutes() {
	let out = run(
		FAR_QUEST,
		r#"
		local r = {}
		ready(); look()
		r.first = card() and card().kind
		CT.Hide()
		look()
		r.sameLevel = card()
		later(); look()
		r.laterSameLevel = card()
		stub.world.setLevel(21); later(); look()
		r.nextLevel = card() and card().kind
		emit(r)"#,
	);
	assert_eq!(out, json!({ "first": "routeadvice", "sameLevel": false, "laterSameLevel": false, "nextLevel": "routeadvice" }));
}

#[test]
fn a_quest_taken_in_the_last_ten_minutes_is_left_alone() {
	let out = run(
		"",
		r#"
		ready()
		stub.world.accept(7003, { 6 }); stub.advance(1)
		look()
		local r = { fresh = card() }
		later(); look()
		r.old = card() and card().kind
		emit(r)"#,
	);
	assert_eq!(out, json!({ "fresh": false, "old": "routeadvice" }));
}

#[test]
fn a_finished_quest_is_never_told_to_be_abandoned() {
	let out = run(
		FAR_QUEST,
		r#"
		ready()
		stub.world.finish(7003); stub.advance(1)
		look()
		emit({ card = card() })"#,
	);
	assert_eq!(out["card"], false);
}

#[test]
fn no_card_in_a_dungeon_guide() {
	let out = run(
		FAR_QUEST,
		r#"
		ready()
		E.SetGuide("test/dungeon"); stub.advance(1)
		look()
		emit({ card = card() })"#,
	);
	assert_eq!(out["card"], false);
}

#[test]
fn a_quest_of_the_hand_written_list_in_the_log_is_told_with_its_reason() {
	let out = run("w.onQuest(701, { 12 })", "ready(false); look(); emit({ card = card() })");
	assert_eq!(out["card"]["kind"], "routeadvice");
	assert_eq!(out["card"]["title"], "Guile of the Raptor is a quest players skip");
	assert!(out["card"]["lines"].as_str().unwrap().starts_with("Its "), "{out}");
	assert_eq!(out["card"]["act"], "Abandon");
	assert_eq!(out["card"]["off"], "Keep it");
}

// A quest placement passed as skipped earlier: the route's own list, with the giver's detour and the dependents stood in.
const MISSED: &str = r#"
local taken, dismissed
E.SkippedEarlier = function() return { { quest = 800, title = "Lost Pickup" } } end
E.TakeEarlier = function(q) taken = q; return true end
E.DismissEarlier = function(q) dismissed = q; return true end
function setPickup(seconds, needed)
	E.PickupDetour = function() return seconds end
	E.AdviceNeeded = function() return needed end
end
function done() return { taken = taken or false, dismissed = dismissed or false } end
"#;

#[test]
fn a_missed_pickup_near_the_route_is_taken_and_one_far_from_it_is_left() {
	let out = run(
		"",
		&format!(
			r#"{MISSED}
		local r = {{}}
		ready()
		setPickup(30, false); look()
		r.near = card()
		CT.current.act.run()
		r.nearDone = done()
		taken, dismissed = nil, nil
		RAC.Reset(); setPickup(900, false); look()
		r.far = card()
		CT.current.act.run()
		r.farDone = done()
		emit(r)"#
		),
	);
	assert_eq!(out["near"]["title"], "You did not pick up Lost Pickup");
	assert_eq!(out["near"]["act"], "Take it");
	assert_eq!(out["near"]["off"], "Leave it");
	assert_eq!(out["near"]["route"], "That is close to your route. We recommend taking it.");
	assert_eq!(out["nearDone"], json!({ "taken": 800, "dismissed": false }));
	assert_eq!(out["far"]["act"], "Leave it");
	assert_eq!(out["far"]["route"], "That is far from your route. We recommend leaving it.");
	assert_eq!(out["farDone"], json!({ "taken": false, "dismissed": 800 }));
}

#[test]
fn a_far_pickup_that_a_later_accept_needs_is_not_told_to_be_left() {
	let out = run(
		"",
		&format!(
			r#"{MISSED}
		ready()
		setPickup(900, true); look()
		emit({{ card = card() }})"#
		),
	);
	assert_eq!(out["card"], false);
}
