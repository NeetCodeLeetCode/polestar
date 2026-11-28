# Fastest XP

Fastest XP is one switch, `behavior.fastestXp`, that sets the leveling settings to their fastest values.
The code is `addon/Polestar/Core/FastestXP.lua`. The engine does not know the mode exists.

## What it does

- If you turn it on, it saves the value of each lever setting, then writes the lever's value with `ns.Set`.
- The Options rows show the values in use. The engine reads the same keys as before.
- If you change a lever setting by hand, the mode stays on and the state reads "customized".
- If you turn it off, it puts back the saved value of each setting that still holds the mode's value.
  A setting you changed by hand stays as you set it.
- The saved values are in `fastest.undo` in the profile. A profile string never carries them.

## The levers

`FX.LEVERS` lists them: key, value, group and label.
A lever with `hardcore = "leave"` is not written on a Hardcore character.
A key that no module registers is passed over.
`tests/fastest-xp.test.js` names a lever whose key no module registers.

The mode never writes these settings: group quests, dungeons, gathering spots, missable warnings,
the guide end, the zone choice, class teleports and the Shortest Path Forever hand-off.

## Where it shows

- Options, Guide, Automation, section "Fastest XP", and first among the Guide Essentials.
- Quick settings, first row.
- `/polestar fast` (state), `/polestar fast on`, `off` and `restore`.

## Safety that does not depend on this file

- Class quests and chains are never skipped (Skip slow quests, Own.slowSkipped).
- A quest in the log stays.
- Deathskips stop at level 10 and never run on Hardcore (Own.deathskips).
- Nearby lines offer only mobs a few levels over the player (HERE_MOBS_ABOVE).

## What the speed test shows

`rust/crates/forge-addontest/tests/addontest/engine-live-ablation.rs` measures the mode in the simulated walk of
`engine-live-speed.rs`: six kill quests in one zone, a player that follows only the arrow at 7 yards a second.
The measure is yards walked until the last quest is done. It is not a measure of XP an hour in the game.

Result, 9 scenarios for each row (zones 1 to 3 and zones 4 to 6), with the mode at its levers after PR 35 (order by XP):

| Arm | Random guide order | Guide that walks nearest first |
|---|---|---|
| Fastest XP against the defaults | 36 to 38 percent fewer yards | 17 to 19 percent fewer yards |
| Mode without the step reorder | 20 to 31 percent fewer yards (the reorder gives the most) | 9 to 10 percent fewer yards |
| Mode without nearby objectives (aim) | 0 to 3 percent fewer yards than the mode | 1 to 5 percent fewer yards than the mode |
| Mode without order by XP | no change | no change |

- With the step reorder on, the arrow's own pick of nearby objectives can walk up to 5 percent more in this walk.
  Each quest here is a step the reorder already sees. The walk has no hand-ins or later-step objectives, which is what aim is for.
  Aim stays in the mode until the in-game test shows it costs time. The test holds it at 5 percent at most.
- Order by XP changes when the XP arrives, not the yards, so this walk cannot show its gain. It never takes longer in all than the guide's order (PR 35).
- Skips, automation, travel and deathskips cannot change a yard in this walk. The table lists them as "not measured here".
- The detour budget (5 to 60 seconds) changes nothing with the reorder on. The radius at 180 yards walks at most 3 percent more than the best of 60 to 400.
- Not measured yet: Forge hours to level 60 and the alternate-level test in the game.
