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
