//! One test per class: today's routing changes against that class's own content (addon/Polestar/Core/Engine.lua:
//! the route-aware trainer pick, objectives-first, placement, Take nearby objectives, the live step reorder).
//! The guide is a small one in the shape of the class's real chain in the leveling guides (real quest ids): a common
//! step, two class-only steps (the chain: the second quest waits for the first), the class training step and a
//! common step. Every test checks the same four things for its class, and that another class never meets them.
//!
//! Elwynn here is 3000 yd across (30 yd per map percent on x) and 2000 yd high (20 yd per percent on y).

use forge_addontest::{emitted, Addon, RunAddon, TEST_PACK, TEST_SOURCE};
use serde_json::{json, Value};

const CLASSES: [&str; 9] = ["WARRIOR", "PALADIN", "HUNTER", "ROGUE", "PRIEST", "SHAMAN", "MAGE", "WARLOCK", "DRUID"];

const CLASS_IDS: [u32; 9] = [1, 2, 3, 4, 5, 7, 8, 9, 11];

fn world(class: &str) -> String {
	let i = CLASSES.iter().position(|c| *c == class).unwrap();
	let id = CLASS_IDS[i];
	let name = format!("{}{}", &class[..1], class[1..].to_lowercase());
	format!(
		r#"
local w = stub.world
w.addMap(1429, {{ name = "Elwynn Forest", c = 0, left = 1500, right = -1500, top = 500, bottom = -1500 }})
w.race, w.raceName, w.faction = "Human", "Human", "Alliance"
w.class, w.className, w.classID = "{class}", "{name}", {id}
w.level = 10
w.zone, w.subzone = "Elwynn Forest", ""
w.moveTo(1429, 47.6, 45)
"#
	)
}

// One trainer of each class, all in the step's zone; the others' stand nearer to the player than the player's own.
fn trainers(own: &str) -> String {
	CLASSES
		.iter()
		.enumerate()
		.map(|(i, c)| {
			let y = if *c == own { 90.0 } else { 40.0 + i as f64 };
			format!(r#"{{ npc = {}, npcName = "Trainer {c}", map = 1429, x = 47.6, y = {y}, faction = "A", classes = {{ "{c}" }} }}"#, 9000 + i)
		})
		.collect::<Vec<_>>()
		.join(", ")
}

// q1, q2: the chain's quests (q2 follows q1). Steps: 1 common north, 2 class (q1: accept, kill far south, hand in),
// 3 class (q2 after q1), 4 class training, 5 common south.
fn guide(class: &str, q1: u32, q2: u32) -> String {
	format!(
		r#"
Polestar.RegisterGuide({{ id = "test/class", pack = "test", title = "Class", minLevel = 1, maxLevel = 20, steps = {{
	{{ title = "A", goals = {{ {{ t = "kill", target = "A", npc = 299, count = 8, quest = 33, oi = 1, map = 1429, x = 47.6, y = 20 }} }} }},
	{{ title = "Chain 1", only = {{ classes = {{ "{class}" }} }}, goals = {{
		{{ t = "accept", quest = {q1}, title = "Chain one", npc = 197, npcName = "Giver", map = 1429, x = 47.6, y = 45 }},
		{{ t = "kill", target = "Chain one", npc = 6, count = 8, quest = {q1}, oi = 1, map = 1429, x = 47.6, y = 85 }},
		{{ t = "turnin", quest = {q1}, title = "Chain one", npc = 197, npcName = "Giver", map = 1429, x = 47.6, y = 45 }} }} }},
	{{ title = "Chain 2", only = {{ classes = {{ "{class}" }} }}, goals = {{
		{{ t = "accept", quest = {q2}, title = "Chain two", npc = 197, npcName = "Giver", map = 1429, x = 47.6, y = 45, pre = {{ {q1} }} }},
		{{ t = "kill", target = "Chain two", npc = 6, count = 8, quest = {q2}, oi = 1, map = 1429, x = 47.6, y = 85 }} }} }},
	{{ title = "Train", goals = {{ {{ t = "train", class = true, zone = 1429, candidates = {{ {trainers} }} }} }} }},
	{{ title = "B", goals = {{ {{ t = "kill", target = "B", npc = 6, count = 8, quest = 7, oi = 1, map = 1429, x = 47.6, y = 60 }} }} }},
}} }})
"#,
		trainers = trainers(class)
	)
}

const HELPERS: &str = r#"
E = Polestar.Engine
w = stub.world
function flush() stub.advance(0.6) end
-- The stops the arrow leads to for a player who follows only the arrow, 3.5 yd every 0.5 s; each stop's work is done there.
function follow(limit, finish)
	local secs, seen, last = 0, {}, nil
	while secs < limit do
		local x = E.GetWaypoint()
		if not x then break end
		local key = x.stepIndex .. ":" .. x.goalIndex
		if key ~= last then seen[#seen + 1] = key end
		last = key
		local dx, dy = (x.x - w.x) * 30, (x.y - w.y) * 20
		local d = math.sqrt(dx * dx + dy * dy)
		if d <= 5 then
			if finish[key] then w.finish(finish[key]) end
			stub.advance(0.6)
			secs = secs + 0.6
		else
			local step = math.min(d, 3.5)
			w.moveTo(1429, w.x + dx / d * step / 30, w.y + dy / d * step / 20)
			stub.advance(0.5)
			secs = secs + 0.5
		end
	end
	return seen
end
"#;

fn run(class: &str, q1: u32, q2: u32, setup: &str, script: &str) -> Vec<Value> {
	run_as(class, class, q1, q2, setup, script)
}

// `player`: who plays; `class`: whose steps the guide holds.
fn run_as(player: &str, class: &str, q1: u32, q2: u32, setup: &str, script: &str) -> Vec<Value> {
	emitted(&RunAddon {
		stubs: vec!["stub_world.lua"],
		before: world(player),
		addons: vec![Addon::files("Polestar", &["Core/Core.xml"])],
		saved: vec![("Polestar".into(), r#"PolestarCharDB = { schema = 1, guide = "test/class" }"#.into())],
		script: [TEST_PACK, TEST_SOURCE, &guide(class, q1, q2), HELPERS, "stub.login()", r#"stub.fire("QUEST_LOG_UPDATE"); stub.advance(0.6)"#, setup, script].join("\n"),
		..Default::default()
	})
}

const ALL_ON: &str = r#"Polestar.Set("behavior.stepReorder", true); Polestar.Set("behavior.whileHereAim", true); flush(); flush()"#;

fn other(class: &str) -> &'static str {
	let i = CLASSES.iter().position(|c| *c == class).unwrap();
	CLASSES[(i + 1) % 9]
}

fn check(class: &str, q1: u32, q2: u32) {
	// 1. Its own chain: with the reorder and the arrow's detours on, the first quest's work is done before the second
	//    quest's step, the second quest's accept is never offered ahead of its prerequisite, and the class steps stay.
	let setup = format!(r#"w.onQuest(33, {{ 8 }}, "A"); w.onQuest(7, {{ 8 }}, "B"); w.onQuest({q1}, {{ 8 }}, "Chain one"); {ALL_ON}"#);
	let own = run(class, q1, q2, &setup, r#"emit(follow(400, { ["1:1"] = 33, ["2:2"] = QQ1, ["5:1"] = 7 }))"#.replace("QQ1", &q1.to_string()).as_str());
	let seen: Vec<String> = own[0].as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect();
	let first = |p: &str| seen.iter().position(|k| k.starts_with(p));
	assert!(first("2:").is_some(), "{class}: its own chain step was never led to: {seen:?}");
	assert!(first("3:").is_none() || first("2:") < first("3:"), "{class}: the second quest's step came before the first's: {seen:?}");
	// 2. Another class, with the same quests in its log by mistake, is never led into the class steps (2, 3) nor to the
	//    class's trainer.
	let o = other(class);
	let out = run_as(o, class, q1, q2, &setup, r#"emit(follow(400, { ["1:1"] = 33, ["5:1"] = 7 }))"#);
	let seen: Vec<String> = out[0].as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect();
	assert!(seen.iter().all(|k| !k.starts_with("2:") && !k.starts_with("3:")), "{o} was led into {class}'s steps: {seen:?}");
	// 3. The class training step goes to the class's own trainer, not to another class's nearer one (the route-aware pick).
	let t = run(class, q1, q2, "", r#"emit(E.GetStepView(4).goals[1].npcName)"#);
	assert_eq!(t[0], json!(format!("Trainer {class}")), "{class}: wrong trainer");
	// 4. Objectives first: the hand-in step with the objective still open leads to the objective, then to the hand-in.
	let g = format!(r#"w.onQuest({q1}, {{ {{ required = 8, fulfilled = 3 }} }}, "Chain one"); stub.fire("QUEST_LOG_UPDATE"); flush()"#);
	let hand = run(
		class,
		q1,
		q2,
		&g,
		r#"
		if E.SkipStep then E.SkipStep(1) end; flush()
		local x = E.GetWaypoint()
		emit(x and { x.stepIndex, x.goalIndex } or "none")
		w.finish(QQ1); stub.fire("QUEST_LOG_UPDATE"); flush()
		local y = E.GetWaypoint()
		emit(y and { y.stepIndex, y.goalIndex } or "none")"#
			.replace("QQ1", &q1.to_string())
			.as_str(),
	);
	assert_eq!(hand, vec![json!([2, 2]), json!([2, 3])], "{class}: objectives first");
}

macro_rules! class_test {
	($name:ident, $class:expr, $q1:expr, $q2:expr) => {
		#[test]
		fn $name() {
			check($class, $q1, $q2);
		}
	};
}

class_test!(warrior_defensive_stance_chain, "WARRIOR", 1638, 1639);
class_test!(paladin_tome_of_divinity_chain, "PALADIN", 1642, 1643);
class_test!(hunter_taming_the_beast_chain, "HUNTER", 94863, 94864);
class_test!(rogue_touch_of_zanzil_chain, "ROGUE", 2607, 2608);
class_test!(priest_chain, "PRIEST", 8255, 8256);
class_test!(shaman_call_of_earth_chain, "SHAMAN", 94374, 94465);
class_test!(mage_chain, "MAGE", 1861, 1882);
class_test!(warlock_the_binding_chain, "WARLOCK", 1689, 1738);
class_test!(druid_chain, "DRUID", 6122, 29);

// ---------------------------------------------------------------------------------------------------------------------
// Gaps the audit found, each fixed with its test turned on.
// ---------------------------------------------------------------------------------------------------------------------

// A class training step first, then a quest step. The player takes the later quest early (a pick-up on the way), then
// enters a new zone: placement runs again (Own.placeIfMoved). It used to pass the training step because later quest work
// is done (Own.place, `leadsToDone`), so the class trainer was never visited. A placing in play keeps the step open.
#[test]
fn every_class_keeps_its_training_step_when_a_later_quest_is_taken_early_and_a_zone_changes() {
	for class in CLASSES {
		let g = format!(
			r#"Polestar.RegisterGuide({{ id = "test/class", pack = "test", title = "Class", minLevel = 1, maxLevel = 20, steps = {{
	{{ title = "Train", goals = {{ {{ t = "train", class = true, zone = 1429, candidates = {{ {} }} }} }} }},
	{{ title = "Take", goals = {{ {{ t = "accept", quest = 5, title = "Quest Five", npc = 197, npcName = "Giver", map = 1429, x = 50, y = 45 }},
		{{ t = "kill", target = "Wolf", npc = 299, count = 8, quest = 5, oi = 1, map = 1429, x = 50, y = 60 }} }} }},
}} }})"#,
			trainers(class)
		);
		let out = emitted(&RunAddon {
			stubs: vec!["stub_world.lua"],
			before: world(class),
			addons: vec![Addon::files("Polestar", &["Core/Core.xml"])],
			saved: vec![("Polestar".into(), r#"PolestarCharDB = { schema = 1, guide = "test/class" }"#.into())],
			script: [
				TEST_PACK,
				TEST_SOURCE,
				&g,
				HELPERS,
				"stub.login()",
				r#"stub.fire("QUEST_LOG_UPDATE"); stub.advance(0.6)"#,
				r#"w.onQuest(5, { { required = 8, fulfilled = 2 } }, "Quest Five"); stub.fire("QUEST_ACCEPTED"); stub.fire("QUEST_LOG_UPDATE"); flush()
				stub.fire("ZONE_CHANGED_NEW_AREA"); flush(); flush()
				emit(E.GetStepIndex())"#,
			]
			.join("\n"),
			..Default::default()
		});
		assert_eq!(out, vec![json!(1)], "{class}: the class training step was passed");
	}
}

// The live reorder measures the walk on to the step after its window. A class training step has no coordinates (the
// trainer is picked at run time), so that last leg is left out and the reorder can take an order that ends far from the
// trainer.
#[test]
fn the_reorder_counts_the_walk_to_the_class_trainer_after_its_window() {
	for class in CLASSES {
		let trainer = format!(r#"{{ npc = 198, npcName = "Trainer", map = 1429, x = 47.6, y = 98, faction = "A", classes = {{ "{class}" }} }}"#);
		let g = format!(
			r#"Polestar.RegisterGuide({{ id = "test/class", pack = "test", title = "Class", minLevel = 1, maxLevel = 20, steps = {{
	{{ title = "North", goals = {{ {{ t = "kill", target = "N", npc = 299, count = 8, quest = 33, oi = 1, map = 1429, x = 47.6, y = 20 }} }} }},
	{{ title = "South", goals = {{ {{ t = "kill", target = "S", npc = 6, count = 8, quest = 7, oi = 1, map = 1429, x = 47.6, y = 80 }} }} }},
	{{ title = "Train", goals = {{ {{ t = "train", class = true, zone = 1429, candidates = {{ {trainer} }} }} }} }},
}} }})"#
		);
		let out = emitted(&RunAddon {
			stubs: vec!["stub_world.lua"],
			before: world(class).replace("w.moveTo(1429, 47.6, 45)", "w.moveTo(1429, 47.6, 56)"),
			addons: vec![Addon::files("Polestar", &["Core/Core.xml"])],
			saved: vec![("Polestar".into(), r#"PolestarCharDB = { schema = 1, guide = "test/class" }"#.into())],
			script: [
				TEST_PACK,
				TEST_SOURCE,
				&g,
				HELPERS,
				"stub.login()",
				r#"stub.fire("QUEST_LOG_UPDATE"); stub.advance(0.6)"#,
				r#"w.onQuest(33, { 8 }, "N"); w.onQuest(7, { 8 }, "S"); stub.fire("QUEST_LOG_UPDATE"); flush(); "#,
				r#"Polestar.Set("behavior.stepReorder", true); flush(); flush(); emit(E.GetStepIndex())"#,
			]
			.join("\n"),
			..Default::default()
		});
		// North first, then south, then the trainer in the south is the shortest walk in all (2280 yd, not 3240).
		assert_eq!(out, vec![json!(1)], "{class}: the reorder sent the player north-last, away from the trainer");
	}
}

// The route-aware vendor pick reads the guide's next located stops. A class training step in between has no point of
// its own (Own.goalSpot gives the trainer picked for the character), so a vendor on the way to the trainer must win.
#[test]
fn the_vendor_pick_counts_the_class_trainer_stop_between_two_steps() {
	let class = "MAGE";
	let g = format!(
		r#"Polestar.RegisterGuide({{ id = "test/class", pack = "test", title = "Class", minLevel = 1, maxLevel = 20, steps = {{
	{{ title = "Sell", goals = {{ {{ t = "vendor", candidates = {{
		{{ npc = 1, npcName = "NORTH vendor", map = 1429, x = 47.6, y = 2, faction = "A" }},
		{{ npc = 2, npcName = "SOUTH vendor", map = 1429, x = 47.6, y = 38, faction = "A" }} }} }} }} }},
	{{ title = "Train", goals = {{ {{ t = "train", class = true, zone = 1429, candidates = {{ {{ npc = 198, npcName = "Trainer", map = 1429, x = 47.6, y = 0, faction = "A", classes = {{ "{class}" }} }} }} }} }} }},
	{{ title = "Next", goals = {{ {{ t = "goto", map = 1429, x = 47.6, y = 50 }} }} }},
}} }})"#
	);
	let out = emitted(&RunAddon {
		stubs: vec!["stub_world.lua"],
		before: world(class).replace("w.moveTo(1429, 47.6, 45)", "w.moveTo(1429, 47.6, 20)"),
		addons: vec![Addon::files("Polestar", &["Core/Core.xml"])],
		saved: vec![("Polestar".into(), r#"PolestarCharDB = { schema = 1, guide = "test/class" }"#.into())],
		script: [TEST_PACK, TEST_SOURCE, &g, HELPERS, "stub.login()", r#"stub.fire("QUEST_LOG_UPDATE"); stub.advance(0.6)"#, r#"emit(E.GetStepView(1).goals[1].npcName)"#].join("\n"),
		..Default::default()
	});
	assert_eq!(out, vec![json!("NORTH vendor")]);
}
