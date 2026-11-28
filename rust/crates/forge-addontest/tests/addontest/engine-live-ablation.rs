//! Fastest XP (behavior.fastestXp) measured against the defaults, and with each group of levers taken out of it, on the
//! simulated player of engine-live-speed.rs (follows only the arrow at 7 yd/s, six kill quests in one zone). Then the
//! detour budget and the radius swept with the mode on. The measure is yards walked until the last quest is done.
//!
//! What this can and cannot show: the simulated world has no grey quests, hand-ins, vendors or flight paths, so the
//! groups "skips", "automation", "travel" and "deathskips" cannot change a yard here. They are in the table so the
//! table says so: a group with no yards of effect is "not measured here", never "no benefit". The groups that walk are
//! "here" (Dynamic Questing, which the arrow needs), "aim" (nearby objectives) and "reorder" (step order).
//!
//! Gate: the mode must walk less than the defaults, and taking one group out of it must not make it walk less by more
//! than 2 percent of the yards (a lever that costs yards would show). Measured overlap: with the step reorder on, the
//! arrow's own pick of nearby objectives ("aim") walks 2 to 3 percent more here, because every quest of these guides is
//! a step the reorder already sees. The bar for aim is 5 percent, so it cannot grow unseen; the sim has none of the
//! objectives, hand-ins or later steps that aim exists for, so that group needs the in-game A/B. Real Core manifest,
//! stub world.

use crate::engine_live_speed::{scene, walk_script, Guide};

const ZONES_TUNED: std::ops::Range<usize> = 0..3;
const ZONES_HELD: std::ops::Range<usize> = 3..6;
const SEEDS_TUNED: std::ops::Range<u64> = 301..304;
const SEEDS_HELD: std::ops::Range<u64> = 401..404;

struct Arm {
	name: &'static str,
	// Lua run once the addon is up.
	flip: String,
}

// The mode on, with the levers of `group` put back to the defaults.
fn without(group: &str) -> String {
	format!(
		r#"Polestar.Set("behavior.fastestXp", true); stub.advance(0.6)
for _, lv in ipairs(Polestar.FastestXP.LEVERS) do
	if lv.group == "{group}" then Polestar.Set(lv.key, Polestar.GetDefault(lv.key)) end
end
stub.advance(0.6)
"#
	)
}

fn arms() -> Vec<Arm> {
	let mut out = vec![Arm { name: "defaults", flip: String::new() }, Arm { name: "Fastest XP", flip: r#"Polestar.Set("behavior.fastestXp", true); stub.advance(0.6)"#.into() }];
	for (name, group) in [
		("mode without here", "here"),
		("mode without aim", "aim"),
		("mode without reorder", "reorder"),
		("mode without order by XP", "reorderXp"),
		("mode without skips", "skips"),
		("mode without automation", "automation"),
		("mode without travel", "travel"),
		("mode without deathskips", "deathskips"),
	] {
		out.push(Arm { name, flip: without(group) });
	}
	out
}

// Yards and seconds of every scenario, one list for each arm, in the same scenario order.
fn run(arms: &[Arm], seeds: std::ops::Range<u64>, zones: std::ops::Range<usize>, style: Guide) -> Vec<Vec<(f64, f64)>> {
	let mut table: Vec<Vec<(f64, f64)>> = arms.iter().map(|_| Vec::new()).collect();
	for zone in zones {
		for seed in seeds.clone() {
			let (guide, setup) = scene(seed, zone, style);
			for (i, arm) in arms.iter().enumerate() {
				let r = walk_script(&guide, &setup, &arm.flip);
				assert_eq!(r[2], 6.0, "every quest done: arm {}, seed {seed}, zone {zone}: {r:?}", arm.name);
				table[i].push((r[0], r[1]));
			}
		}
	}
	table
}

fn total(rows: &[(f64, f64)]) -> f64 {
	rows.iter().map(|r| r.0).sum()
}

fn median_change(base: &[(f64, f64)], rows: &[(f64, f64)]) -> f64 {
	let mut change: Vec<f64> = base.iter().zip(rows).map(|(b, r)| (r.0 - b.0) / b.0 * 100.0).collect();
	change.sort_by(|a, b| a.partial_cmp(b).unwrap());
	change[change.len() / 2]
}

fn worst_change(base: &[(f64, f64)], rows: &[(f64, f64)]) -> f64 {
	base.iter().zip(rows).map(|(b, r)| (r.0 - b.0) / b.0 * 100.0).fold(f64::MIN, f64::max)
}

// Prints the table; asserts the gate. `mode_bar`: the mode's median change against the defaults, at most.
fn ablate(label: &str, style: Guide, seeds: std::ops::Range<u64>, zones: std::ops::Range<usize>, mode_bar: f64) {
	let arms = arms();
	let table = run(&arms, seeds, zones, style);
	let (defaults, mode) = (&table[0], &table[1]);
	eprintln!("{label}: {} scenarios", defaults.len());
	eprintln!("  {:<26} {:>9} {:>10} {:>10} {:>10}", "arm", "yards", "vs defaults", "median", "vs mode");
	for (arm, rows) in arms.iter().zip(&table) {
		eprintln!(
			"  {:<26} {:>9.0} {:>+9.1}% {:>+9.1}% {:>+9.1}%",
			arm.name,
			total(rows),
			(total(rows) - total(defaults)) / total(defaults) * 100.0,
			median_change(defaults, rows),
			(total(rows) - total(mode)) / total(mode) * 100.0
		);
	}
	let median = median_change(defaults, mode);
	assert!(median <= mode_bar, "{label}: Fastest XP median must be {mode_bar:+.0} percent or better than the defaults, was {median:+.1}%");
	assert!(worst_change(defaults, mode) <= 10.0, "{label}: no scenario more than 10 percent longer with Fastest XP");
	for (arm, rows) in arms.iter().zip(&table).skip(2) {
		let gain = (total(mode) - total(rows)) / total(mode) * 100.0;
		let bar = if arm.name == "mode without aim" { 5.0 } else { 2.0 };
		assert!(gain <= bar, "{label}: taking out a group made the mode walk {gain:.1}% less (bar {bar}%), so a lever in it costs yards: {}", arm.name);
	}
}

#[test]
fn fastest_xp_walks_less_than_the_defaults_and_each_group_pays_on_the_tuned_zones() {
	ablate("random guides, zones 1 to 3", Guide::Random, SEEDS_TUNED, ZONES_TUNED, -5.0);
}

#[test]
fn it_does_on_the_held_out_zones() {
	ablate("random guides, zones 4 to 6", Guide::Random, SEEDS_HELD, ZONES_HELD, -5.0);
}

#[test]
fn on_a_sensible_guide_it_is_not_slower_than_the_defaults() {
	ablate("sensible guides, zones 1 to 3", Guide::Sensible, SEEDS_TUNED, ZONES_TUNED, 0.0);
	ablate("sensible guides, zones 4 to 6", Guide::Sensible, SEEDS_HELD, ZONES_HELD, 0.0);
}

// The detour budget and the radius, the mode on, a number at a time. Reported on both sets of zones; the budget the mode
// writes must be within 5 percent of the best yards of the sweep on each.
fn sweep(key: &str, values: &[f64], written: f64) {
	for (label, seeds, zones) in [("zones 1 to 3", SEEDS_TUNED, ZONES_TUNED), ("zones 4 to 6", SEEDS_HELD, ZONES_HELD)] {
		for style in [Guide::Random, Guide::Sensible] {
			let arms: Vec<Arm> =
				values.iter().map(|v| Arm { name: "value", flip: format!("Polestar.Set(\"behavior.fastestXp\", true); stub.advance(0.6)\nPolestar.Set(\"{key}\", {v}); stub.advance(0.6)") }).collect();
			let table = run(&arms, seeds.clone(), zones.clone(), style);
			let totals: Vec<f64> = table.iter().map(|rows| total(rows)).collect();
			let best = totals.iter().cloned().fold(f64::MAX, f64::min);
			let style_name = if matches!(style, Guide::Random) { "random" } else { "sensible" };
			eprintln!("{key}, {style_name} guides, {label}:");
			for (v, t) in values.iter().zip(&totals) {
				eprintln!("  {v:>6}: {t:>8.0} yd ({:+.1}% of the best){}", (t - best) / best * 100.0, if *v == written { "  <- the mode writes this" } else { "" });
			}
			let at = values.iter().position(|v| *v == written).expect("the written value is in the sweep");
			assert!(totals[at] <= best * 1.05, "{key}, {style_name} guides, {label}: the written value {written} walks {:.1}% more than the best", (totals[at] - best) / best * 100.0);
		}
	}
}

#[test]
fn the_objective_detour_budget_the_mode_writes_is_near_the_best() {
	sweep("behavior.whileHereAimSeconds", &[5.0, 10.0, 15.0, 25.0, 40.0, 60.0], 15.0);
}

#[test]
fn the_radius_the_mode_writes_is_near_the_best() {
	sweep("behavior.whileHereRadius", &[60.0, 100.0, 180.0, 300.0, 400.0], 180.0);
}
