//! A simulated player that follows only the arrow, with a clock, XP and events.

use crate::playtest::SIMULATE;

/// Lua defining `ARROW.run(options)` (tests/lua/arrow_player.lua): it plays the open guide on stub_world.lua by
/// walking to the place the arrow leads to and doing the goal there, and returns the time, XP, yards, reversals,
/// stuck state and CPU and memory it measured. It does each goal the way the simulated player of the guide tests
/// does ([`SIMULATE`]'s act, loaded here without its loop), apart from the quest work, which carries the clock and
/// the XP. Run it in `script` after the guide is set. Read from the checkout the tests run in
/// ([`crate::run::root`]), not baked in when they are built.
pub static ARROW_PLAYER: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
	let file = crate::run::root().join("tests/lua/arrow_player.lua");
	let player = std::fs::read_to_string(&file).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
	format!("SIM_ACT_ONLY = true\n(function()\n{}\nend)()\nSIM_ACT_ONLY = nil\n{player}", SIMULATE.as_str())
});
