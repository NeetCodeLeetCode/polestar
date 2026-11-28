//! What the route advice reads of the route (addon/Polestar/Core/Engine.lua: QuestDetour, PickupDetour, AdviceHold),
//! and the card built on them (UI/RouteAdviceCard.lua), on the stub world with the fixture quest database. Kobold
//! Camp Cleanup (7) asks for kobold vermin, which stand at about (47-51, 35-36) and at (70, 70) in Elwynn Forest, a map
//! 3000 yd across (30 yd per map percent on x) and 2000 yd high (20 yd per percent on y). The guide's next stop is
//! beside the player: the detour of the quest is the walk to the vermin and back.

use forge_addontest::questdb::{build_db, fixture_dir, QuestDb};
use forge_addontest::{emitted, Addon, RunAddon, TEST_PACK, TEST_SOURCE};
use serde_json::{json, Value};

const WORLD: &str = r#"
local w = stub.world
w.addMap(1429, { name = "Elwynn Forest", c = 0, left = 1500, right = -1500, top = 500, bottom = -1500 })
w.titles[7] = "Kobold Camp Cleanup"
w.titles[9] = "Orcish Errand"
w.level = 5
w.onQuest(7, { 10 })
"#;

// A leveling guide: step 1 goes to a point (the player's), step 2 takes Orcish Errand, far from the vermin.
const GUIDE: &str = r#"
Polestar.RegisterGuide({
	id = "test/advice", pack = "test", title = "Advice", minLevel = 1, maxLevel = 10, steps = {
		{ title = "Next door", goals = { { t = "goto", map = 1429, x = POSX, y = POSY, text = "Camp" } } },
		{ title = "Errand", goals = { { t = "accept", quest = 9, title = "Orcish Errand", npc = 3100, npcName = "Grimtak", map = 1429, x = 22, y = 78 } } },
	},
})
"#;

const HELPERS: &str = r#"
E = Polestar.Engine
function flush() stub.advance(0.25) end
"#;

fn db() -> QuestDb {
	build_db(&fixture_dir())
}

// The player stands at (x, y) in Elwynn Forest, and the guide's first stop is a little off it.
fn run(db: &QuestDb, x: f64, y: f64, script: &str) -> Value {
	let guide = GUIDE.replace("POSX", &format!("{}", x + 1.0)).replace("POSY", &format!("{}", y - 1.0));
	emitted(&RunAddon {
		stubs: vec!["stub_world.lua", "stub_quests.lua"],
		before: format!("stub.quests.dbFiles = {}\n{WORLD}\nstub.world.moveTo(1429, {x}, {y})", db.lua_files()),
		addons: vec![Addon::name("Polestar")],
		saved: vec![("Polestar".into(), r#"PolestarCharDB = { schema = 1, guide = "test/advice" }"#.into())],
		script: [TEST_PACK, TEST_SOURCE, &guide, HELPERS, "stub.login()", r#"stub.fire("QUEST_LOG_UPDATE"); stub.advance(0.6)"#, script].join("\n"),
		..Default::default()
	})
	.remove(0)
}

#[test]
fn a_quest_far_from_the_route_has_a_detour_of_the_walk_there_and_back() {
	let out = run(&db(), 20.0, 80.0, "emit({ far = E.QuestDetour(7) })");
	let far = out["far"].as_f64().unwrap_or_else(|| panic!("a number of seconds: {out}"));
	// About 1200 yd each way at 7 yd/s, less the step the route takes anyway.
	assert!(far > 250.0 && far < 450.0, "{far}");
}

#[test]
fn a_quest_whose_work_is_on_the_way_has_a_small_detour() {
	let out = run(&db(), 48.0, 38.0, "emit({ near = E.QuestDetour(7) })");
	let near = out["near"].as_f64().unwrap_or_else(|| panic!("a number of seconds: {out}"));
	assert!(near < 60.0, "{near}");
}

#[test]
fn a_quest_with_no_place_in_the_database_has_no_detour() {
	let out = run(&db(), 20.0, 80.0, r#"stub.world.onQuest(4242, { 3 }); emit({ unknown = E.QuestDetour(4242) or "none", pickup = E.PickupDetour(4242) or "none" })"#);
	assert_eq!(out, json!({ "unknown": "none", "pickup": "none" }));
}

#[test]
fn the_giver_of_a_quest_the_guide_takes_has_a_detour_and_an_ordinary_quest_is_not_held() {
	let out = run(&db(), 20.0, 80.0, "emit({ pickup = E.PickupDetour(9), hold = E.AdviceHold(7) or \"none\", needed = E.AdviceNeeded(7) })");
	assert!(out["pickup"].is_number(), "{out}");
	assert_eq!(out["hold"], "none");
	assert_eq!(out["needed"], false);
}

#[test]
fn the_card_tells_a_far_quest_off_the_route_to_be_abandoned_and_a_near_one_nothing() {
	let far = run(&db(), 20.0, 80.0, r#"
		Polestar.Set("behavior.routeAdvice", true); stub.advance(0.3)
		local RAC, CT = Polestar.UI.RouteAdviceCard, Polestar.UI.ClassToast
		RAC.Look(); stub.advance(0.1)
		local m = CT.current
		emit({ title = m and m.title or false, lines = m and m.giver or false })"#);
	assert_eq!(far["title"], "We recommend abandoning Kobold Camp Cleanup");
	assert!(far["lines"].as_str().unwrap().starts_with("Finishing it adds about "), "{far}");
	let near = run(&db(), 48.0, 38.0, r#"
		Polestar.Set("behavior.routeAdvice", true); stub.advance(0.3)
		local RAC, CT = Polestar.UI.RouteAdviceCard, Polestar.UI.ClassToast
		RAC.Look(); stub.advance(0.1)
		emit({ card = CT.current and CT.current.title or false })"#);
	assert_eq!(near["card"], false);
}
