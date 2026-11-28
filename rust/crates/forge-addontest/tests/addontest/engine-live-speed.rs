//! The arrow that takes nearby objectives (behavior.whileHereAim) and the step reorder (behavior.stepReorder) against the
//! guide's own order: the same quests, the same starting point, a simulated player that follows only the arrow at 7 yd/s,
//! with the setting off and on. Each scenario is a guide of six kill quests at random places of one zone, in a random order
//! (or, for the "sensible" guides, nearest-first from the start with one pair swapped). A random order is a poor guide, so
//! the gains here are larger than a real guide's; the sensible guides are the check that the reorder is not slower. The measures are the yards walked until the last quest is done, and the reversals:
//! the arrow leads to X, then to Y, leaves Y open and goes back to X with X still open.
//! The constants (Own.AIM_* in Core/Engine.lua) were set before this test existed. Zones 1 to 3 are the zones a later change
//! is tuned on; zones 4 to 6 are held out: nothing may be set from them, they only report. The pass bars must hold on both: a
//! median of 5 percent shorter, shorter in 70 percent of the scenarios, none more than 10 percent longer, under 1 reversal
//! for each hour of walking. Real Core manifest, stub world.

use forge_addontest::{emitted, Addon, RunAddon, TEST_PACK, TEST_SOURCE};

const WORLD: &str = r#"
local w = stub.world
w.addMap(1429, { name = "Elwynn Forest", c = 0, left = 1500, right = -1500, top = 500, bottom = -1500 })
w.level = 5
w.zone, w.subzone = "Elwynn Forest", ""
"#;

// Walks to what the arrow leads to, 3.5 yd every 0.5 s; at its place the quest's work is done. -> { yards, seconds,
// quests done, reversals }.
const HELPERS: &str = r#"
E = Polestar.Engine
w = stub.world
function walk(limit)
	local secs, yards, done, reversals = 0, 0, 0, 0
	local before, last, finished = nil, nil, {}
	while secs < limit do
		local x = E.GetWaypoint()
		if not x then break end
		local key = x.stepIndex .. ":" .. x.goalIndex
		if key ~= last then
			if before == key and not finished[key] and not finished[last] then reversals = reversals + 1 end
			before, last = last, key
		end
		local dx, dy = (x.x - w.x) * 30, (x.y - w.y) * 20
		local d = math.sqrt(dx * dx + dy * dy)
		if d <= 5 then
			if QUEST[key] then w.finish(QUEST[key]); finished[key] = true; done = done + 1 end
			stub.advance(0.6)
			secs = secs + 0.6
		else
			local step = math.min(d, 3.5)
			w.moveTo(1429, w.x + dx / d * step / 30, w.y + dy / d * step / 20)
			stub.advance(0.5)
			yards, secs = yards + step, secs + 0.5
		end
	end
	return { math.floor(yards), math.floor(secs), done, reversals }
end
"#;

// A tiny generator, so a scenario is the same on every machine.
struct Rng(u64);
impl Rng {
	fn next(&mut self) -> f64 {
		self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
		((self.0 >> 33) as f64) / ((1u64 << 31) as f64)
	}
	fn between(&mut self, lo: f64, hi: f64) -> f64 {
		lo + (hi - lo) * self.next()
	}
}

// The six zones: (left, top, right, bottom) in map percent, each 800 by 600 yd.
const ZONES: [(f64, f64, f64, f64); 6] = [(10.0, 10.0, 36.7, 40.0), (40.0, 10.0, 66.7, 40.0), (70.0, 10.0, 96.7, 40.0), (10.0, 55.0, 36.7, 85.0), (40.0, 55.0, 66.7, 85.0), (70.0, 55.0, 96.7, 85.0)];
const QUESTS: usize = 6;

// How the guide orders its six quests.
#[derive(Clone, Copy)]
enum Guide {
	Random,
	// Nearest-first from where the player starts, the way a guide author walks it, then two neighbours swapped.
	Sensible,
}

// The guide, the quest log and where the player stands, for scenario `seed` in `zone`.
fn scene(seed: u64, zone: usize, style: Guide) -> (String, String) {
	let (l, t, r, b) = ZONES[zone];
	let mut rng = Rng(seed * 7919 + zone as u64 * 104729 + 17);
	let (sx, sy) = (rng.between(l, r), rng.between(t, b));
	let mut steps = String::new();
	let mut quest = String::new();
	let mut log = String::new();
	let mut xp = String::new();
	let mut places: Vec<(f64, f64, f64)> = Vec::new();
	for _ in 1..=QUESTS {
		let (x, y) = (rng.between(l, r), rng.between(t, b));
		places.push((x, y, (rng.between(8.0, 40.0)).round()));
	}
	if let Guide::Sensible = style {
		let (mut at, mut rest, mut ordered) = ((sx, sy), places.clone(), Vec::new());
		while !rest.is_empty() {
			let near = (0..rest.len()).min_by(|&a, &b| {
				let d = |p: &(f64, f64, f64)| ((p.0 - at.0) * 30.0).powi(2) + ((p.1 - at.1) * 20.0).powi(2);
				d(&rest[a]).partial_cmp(&d(&rest[b])).unwrap()
			});
			let p = rest.remove(near.unwrap());
			at = (p.0, p.1);
			ordered.push(p);
		}
		let k = (rng.next() * (QUESTS - 1) as f64) as usize;
		ordered.swap(k, k + 1);
		places = ordered;
	}
	for (n, (x, y, xp_of)) in places.iter().enumerate() {
		let i = n + 1;
		steps += &format!(
			"{{ title = \"Quest {i}\", goals = {{ {{ t = \"kill\", target = \"Mob {i}\", npc = {n}, count = 10, quest = {i}, oi = 1, map = 1429, x = {x:.1}, y = {y:.1} }} }} }},\n",
			n = 100 + i
		);
		quest += &format!("[\"{i}:1\"] = {i}, ");
		log += &format!("w.onQuest({i}, {{ {{ required = 10, fulfilled = 8 }} }}, \"Quest {i}\")\n");
		xp += &format!("[{i}] = {xp_of}, ");
	}
	let guide = format!("Polestar.RegisterGuide({{ id = \"test/speed\", pack = \"test\", title = \"Speed\", minLevel = 1, maxLevel = 10, steps = {{\n{steps}}} }})\nQUEST = {{ {quest} }}\n");
	let setup = format!("{log}w.moveTo(1429, {sx:.1}, {sy:.1})\nXP = {{ {xp} }}; GetQuestLogRewardXP = function(id) return XP[id] end\n");
	(guide, setup)
}

// The settings that are switched on: a list of keys.
fn walk(guide: &str, setup: &str, on: &[&str]) -> Vec<f64> {
	let flip: String = on.iter().map(|k| format!("Polestar.Set(\"{k}\", true); stub.advance(0.6)\n")).collect();
	let out = emitted(&RunAddon {
		stubs: vec!["stub_world.lua"],
		before: WORLD.into(),
		addons: vec![Addon::files("Polestar", &["Core/Core.xml"])],
		saved: vec![("Polestar".into(), r#"PolestarCharDB = { schema = 1, guide = "test/speed" }"#.into())],
		script: [TEST_PACK, TEST_SOURCE, guide, HELPERS, setup, "stub.login()", r#"stub.fire("QUEST_LOG_UPDATE"); stub.advance(0.6)"#, &flip, "emit(walk(4000))"].join("\n"),
		..Default::default()
	});
	out[0].as_array().unwrap().iter().map(|v| v.as_f64().unwrap()).collect()
}

struct Result {
	seed: u64,
	zone: usize,
	off: f64,
	on: f64,
	reversals: f64,
	secs: f64,
}

fn play(seeds: std::ops::Range<u64>, zones: std::ops::Range<usize>, style: Guide, on: &[&str]) -> Vec<Result> {
	let mut all = Vec::new();
	for zone in zones {
		for seed in seeds.clone() {
			let (guide, setup) = scene(seed, zone, style);
			let off = walk(&guide, &setup, &[]);
			let on = walk(&guide, &setup, on);
			assert_eq!(off[2], QUESTS as f64, "every quest done with the setting off (seed {seed}, zone {zone}): {off:?}");
			assert_eq!(on[2], QUESTS as f64, "every quest done with the setting on (seed {seed}, zone {zone}): {on:?}");
			all.push(Result { seed, zone, off: off[0], on: on[0], reversals: on[3], secs: on[1] });
		}
	}
	all
}

// `median_bar`: the median change in yards must be at most this many percent (-5: five percent shorter).
fn report(name: &str, rs: &[Result], median_bar: f64) {
	let mut change: Vec<f64> = rs.iter().map(|r| (r.on - r.off) / r.off * 100.0).collect();
	change.sort_by(|a, b| a.partial_cmp(b).unwrap());
	let median = change[change.len() / 2];
	let wins = rs.iter().filter(|r| r.on < r.off * 0.995).count();
	let worst = change[change.len() - 1];
	let hours: f64 = rs.iter().map(|r| r.secs).sum::<f64>() / 3600.0;
	let reversals: f64 = rs.iter().map(|r| r.reversals).sum();
	eprintln!("{name}: {} scenarios, median {median:+.1}% yards, shorter in {wins}, worst {worst:+.1}%, {reversals} reversals in {hours:.2} h ({:.2} an hour)", rs.len(), reversals / hours);
	for r in rs {
		eprintln!("  zone {} seed {:>2}: off {:>5} yd, on {:>5} yd ({:+.1}%), {} reversals", r.zone + 1, r.seed, r.off, r.on, (r.on - r.off) / r.off * 100.0, r.reversals);
	}
	assert!(median <= median_bar, "{name}: the median must be {median_bar:+.0} percent or better, was {median:+.1}%");
	if median_bar < 0.0 {
		assert!(wins as f64 >= rs.len() as f64 * 0.7, "{name}: shorter in 70 percent of the scenarios, was {wins} of {}", rs.len());
	}
	assert!(worst <= 10.0, "{name}: no scenario more than 10 percent longer, the worst was {worst:+.1}%");
	assert!(reversals / hours < 1.0, "{name}: under 1 reversal for each hour of walking, was {:.2}", reversals / hours);
}

const AIM: &[&str] = &["behavior.whileHereAim"];
const REORDER: &[&str] = &["behavior.stepReorder"];

#[test]
fn on_the_tuned_zones_the_arrow_walks_less_than_the_guides_order() {
	report("tuned zones 1 to 3", &play(1..9, 0..3, Guide::Random, AIM), -5.0);
}

#[test]
fn on_the_held_out_zones_it_does_too() {
	report("held-out zones 4 to 6", &play(101..109, 3..6, Guide::Random, AIM), -5.0);
}

#[test]
fn the_step_reorder_walks_less_than_a_random_guide_on_the_tuned_zones() {
	report("reorder, random guides, zones 1 to 3", &play(1..9, 0..3, Guide::Random, REORDER), -5.0);
}

#[test]
fn the_step_reorder_walks_less_than_a_random_guide_on_the_held_out_zones() {
	report("reorder, random guides, zones 4 to 6", &play(101..109, 3..6, Guide::Random, REORDER), -5.0);
}

#[test]
fn the_step_reorder_is_not_slower_on_a_sensible_guide() {
	report("reorder, sensible guides, zones 1 to 3", &play(1..9, 0..3, Guide::Sensible, REORDER), 0.0);
	report("reorder, sensible guides, zones 4 to 6", &play(101..109, 3..6, Guide::Sensible, REORDER), 0.0);
}

#[test]
fn both_on_together_are_not_slower_than_either_alone_on_a_sensible_guide() {
	let both = [AIM, REORDER].concat();
	report("both, sensible guides, zones 1 to 6", &play(201..206, 0..6, Guide::Sensible, &both), 0.0);
}

// ---------------------------------------------------------------------------------------------------------------------
// The XP order (behavior.stepReorderXp). Four quests, each a kill step and a hand-in step, in the order a guide author
// walks them (the quest whose kills are nearest first, each handed in at once): the quest goes where the work is, then
// where its giver is, the hand-in paying 200 to 4,000 XP. The player stands for the kills (8 of them, 15 s each, the
// time Core/SlowQuestList.lua gives) and for the talk at a hand-in (30 s). The measures are the seconds until the last
// hand-in and the seconds an XP point waits on average: each hand-in's XP times the time it is paid at, over all the XP.
// ---------------------------------------------------------------------------------------------------------------------

const HAND_IN: &str = r#"
-- The walk winds as the planner counts it (Own.HERE_WINDING): the straight line is 1.25 times longer on the ground.
Own_WINDING = 1.25
function walkx(limit)
	local secs, yards, handed, weighted, total = 0, 0, 0, 0, 0
	while secs < limit do
		local x = E.GetWaypoint()
		if not x then break end
		local key = x.stepIndex .. ":" .. x.goalIndex
		local dx, dy = (x.x - w.x) * 30, (x.y - w.y) * 20
		local d = math.sqrt(dx * dx + dy * dy)
		if d <= 5 then
			if QUEST[key] then
				w.finish(QUEST[key])
				stub.advance(120); secs = secs + 120
			elseif HAND[key] then
				stub.advance(30); secs = secs + 30
				w.turnIn(HAND[key])
				handed = handed + 1
				weighted, total = weighted + XP[HAND[key]] * secs, total + XP[HAND[key]]
			end
			stub.advance(0.6)
			secs = secs + 0.6
		else
			local step = math.min(d, 3.5 / Own_WINDING)
			w.moveTo(1429, w.x + dx / d * step / 30, w.y + dy / d * step / 20)
			stub.advance(0.5)
			yards, secs = yards + step, secs + 0.5
		end
	end
	return { math.floor(yards), math.floor(secs), handed, total > 0 and weighted / total or 0 }
end
"#;

const HAND_INS: usize = 4;

fn xp_scene(seed: u64, zone: usize) -> (String, String) {
	let (l, t, r, b) = ZONES[zone];
	let mut rng = Rng(seed * 7919 + zone as u64 * 104729 + 31);
	let (sx, sy) = (rng.between(l, r), rng.between(t, b));
	// (kill place, giver place, XP) for each quest, the kills nearest first from the start.
	let mut quests: Vec<((f64, f64), (f64, f64), f64)> = (0..HAND_INS)
		.map(|_| ((rng.between(l, r), rng.between(t, b)), (rng.between(l, r), rng.between(t, b)), (rng.between(200.0, 4000.0)).round()))
		.collect();
	let dist = |p: (f64, f64)| ((p.0 - sx) * 30.0).powi(2) + ((p.1 - sy) * 20.0).powi(2);
	quests.sort_by(|a, c| dist(a.0).partial_cmp(&dist(c.0)).unwrap());
	let (mut steps, mut quest, mut hand, mut log, mut xp) = (String::new(), String::new(), String::new(), String::new(), String::new());
	for (n, ((kx, ky), (gx, gy), pays)) in quests.iter().enumerate() {
		let q = n as u32 + 1;
		let (a, c) = (2 * n + 1, 2 * n + 2);
		steps += &format!(
			"{{ title = \"Kill {q}\", goals = {{ {{ t = \"kill\", target = \"Mob {q}\", npc = {npc}, count = 8, quest = {q}, oi = 1, map = 1429, x = {kx:.1}, y = {ky:.1} }} }} }},\n\
			 {{ title = \"Hand in {q}\", goals = {{ {{ t = \"turnin\", quest = {q}, title = \"Quest {q}\", npc = {q}, npcName = \"Giver {q}\", map = 1429, x = {gx:.1}, y = {gy:.1} }} }} }},\n",
			npc = 100 + q
		);
		quest += &format!("[\"{a}:1\"] = {q}, ");
		hand += &format!("[\"{c}:1\"] = {q}, ");
		log += &format!("w.onQuest({q}, {{ {{ required = 8, fulfilled = 0 }} }}, \"Quest {q}\")\n");
		xp += &format!("[{q}] = {pays}, ");
	}
	let guide = format!("Polestar.RegisterGuide({{ id = \"test/speed\", pack = \"test\", title = \"Speed\", minLevel = 1, maxLevel = 10, steps = {{\n{steps}}} }})\nQUEST = {{ {quest} }}\nHAND = {{ {hand} }}\n");
	let setup = format!("{log}w.moveTo(1429, {sx:.1}, {sy:.1})\nXP = {{ {xp} }}; GetQuestLogRewardXP = function(id) return XP[id] end\n");
	(guide, setup)
}

fn walk_xp(guide: &str, setup: &str, on: &[&str]) -> Vec<f64> {
	let flip: String = on.iter().map(|k| format!("Polestar.Set(\"{k}\", true); stub.advance(0.6)\n")).collect();
	let out = emitted(&RunAddon {
		stubs: vec!["stub_world.lua"],
		before: WORLD.into(),
		addons: vec![Addon::files("Polestar", &["Core/Core.xml"])],
		saved: vec![("Polestar".into(), r#"PolestarCharDB = { schema = 1, guide = "test/speed" }"#.into())],
		script: [TEST_PACK, TEST_SOURCE, guide, HELPERS, HAND_IN, setup, "stub.login()", r#"stub.fire("QUEST_LOG_UPDATE"); stub.advance(0.6)"#, &flip, "emit(walkx(6000))"].join("\n"),
		..Default::default()
	});
	out[0].as_array().unwrap().iter().map(|v| v.as_f64().unwrap()).collect()
}

// The XP order against the guide's own, and against the walk order alone, for each scenario of `seeds` in `zones`.
fn play_xp(seeds: std::ops::Range<u64>, zones: std::ops::Range<usize>, name: &str, wait_bar: f64) {
	let (mut waits, mut totals, mut rows) = (Vec::new(), Vec::new(), Vec::new());
	for zone in zones {
		for seed in seeds.clone() {
			let (guide, setup) = xp_scene(seed, zone);
			let off = walk_xp(&guide, &setup, &[]);
			let walk = walk_xp(&guide, &setup, REORDER);
			let xp = walk_xp(&guide, &setup, &["behavior.stepReorder", "behavior.stepReorderXp"]);
			for run in [&off, &walk, &xp] {
				assert_eq!(run[2], HAND_INS as f64, "every hand-in done (seed {seed}, zone {zone}): {run:?}");
			}
			waits.push((xp[3] - off[3]) / off[3] * 100.0);
			totals.push((xp[1] - off[1]) / off[1] * 100.0);
			rows.push(format!("  zone {} seed {:>3}: guide {:>5.0} s wait, {:>5.0} s in all; walk order {:>5.0} / {:>5.0}; XP order {:>5.0} / {:>5.0}", zone + 1, seed, off[3], off[1], walk[3], walk[1], xp[3], xp[1]));
		}
	}
	waits.sort_by(|a, b| a.partial_cmp(b).unwrap());
	totals.sort_by(|a, b| a.partial_cmp(b).unwrap());
	// The mean, not the median: where the guide's own order is already the best one (about half the scenarios) the order is
	// left as it is, so the median sits on the line between the scenarios that gain and those that do not.
	let (median, worst_total) = (waits[waits.len() / 2], totals[totals.len() - 1]);
	let mean = waits.iter().sum::<f64>() / waits.len() as f64;
	let better = waits.iter().filter(|w| **w < -0.5).count();
	eprintln!("{name}: {} scenarios, XP wait mean {mean:+.1}%, median {median:+.1}%, shorter in {better}, time in all worst {worst_total:+.1}%", waits.len());
	for r in rows {
		eprintln!("{r}");
	}
	assert!(mean <= wait_bar, "{name}: the mean wait for XP must be {wait_bar:+.0} percent or better, was {mean:+.1}%");
	assert!(better * 3 >= waits.len(), "{name}: the XP order must pay sooner in a third of the scenarios, it did in {better} of {}", waits.len());
	assert!(worst_total <= 3.0, "{name}: no scenario more than 3 percent longer in all, the worst was {worst_total:+.1}%");
}

#[test]
fn the_xp_order_pays_sooner_on_the_tuned_zones() {
	play_xp(1..7, 0..3, "XP order, zones 1 to 3", -3.0);
}

#[test]
fn the_xp_order_pays_sooner_on_the_held_out_zones() {
	play_xp(101..107, 3..6, "XP order, zones 4 to 6", -3.0);
}
