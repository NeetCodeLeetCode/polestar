# Route gap test

The route gap test measures how far a route is from the shortest walk that keeps the rules. It counts yards walked,
areas entered again, and there-and-back trips. It runs on the compiled guides and on simulated runs. The tools do not
change the addon.

## What the test compares

The test cuts a route into runs of movable goals: accepts, objectives and hand-ins. Flights, teleports, trainers,
vendors, level goals, timed quests and escorts stay where they are and end a run. For each run the test measures four
orders.

| Order | What it is |
|---|---|
| guide | The goals in the order the guide or the player did them. |
| live | The addon's step reorder (`Core/StepOrder.lua`): the next 8 steps in exact order, the first one taken, then planned again. |
| steps | The best order of whole steps. |
| goals | The best order of single goals. |

Every order keeps these rules. A quest is accepted before its objectives and handed in after them. A follow-up is
accepted after the hand-in that opens it. The quest log stays under its cap. A goal is not done before the level its
quest asks for.

The best orders come from a beam search and a local search. On runs of up to 14 goals the test also computes the exact
best order. The search matched the exact order on every such run.

An area is every goal within 100 yards of the first goal in it. A re-entry is a visit to an area after the route left
it. A there-and-back trip is one or two goals in another area between two visits to one area.

## Running it

On compiled guides (the walk is the flow audit's, see [ROUTE-FEEL.md](ROUTE-FEEL.md)):

```sh
node tools/route-gap.js --addon build --faction Horde --start Orc --class WARRIOR --md gap.md
```

On simulated runs, with the scenario campaign of `forge-addontest` (each run lists where the player did each goal):

```sh
SCENARIO_FAMILY=calm SCENARIO_SEEDS=1-3 SCENARIO_SECONDS=7200 SCENARIO_OUT=runs.jsonl \
  cargo test --release -p forge-addontest --test addontest -- --ignored --exact engine_scenarios::campaign
node tools/route-gap-sim.js runs.jsonl
```

## Numbers on 2026-11-28

Guides: an Orc Warrior on the Horde guides from level 1 to 60 (2142 movable goals in 123 runs, 1585 walking minutes in
a 158 hour route).

| Order | Yards | Over the best goal order |
|---|---|---|
| guide | 649,423 | 4.7% |
| live | 640,203 | 3.2% |
| steps | 638,172 | 2.8% |
| goals | 620,527 | 0% |

The best goal order saves 69 minutes over the guide and 47 minutes over the live reorder. Re-entries fall from 181 to
162 and there-and-back trips from 66 to 45.

Simulated runs: 15 characters, 3 seeds each, two hours, no events. Walking above the best order of the same goals:

| Mode | Median | Worst | Median minutes a run | Worst minutes a run |
|---|---|---|---|---|
| Guide order | 8.6% | 22.2% | 2.0 | 8.5 |
| Fastest XP | 6.6% | 19.3% | 1.6 | 7.2 |

Deep guides lose most: level 52 in Un'Goro Crater 19.3% and level 33 in Stranglethorn Vale 16.9% with Fastest XP.

## What the numbers say

The order of the goals is not where most time goes. On the full route, the best order of single goals saves 0.7% of the time. Walking is about one sixth of the time. The rest is fights, waiting, and other work.

A larger window for the step reorder gains little: the best order of whole steps is 0.3% shorter than the live
reorder. The gain is in moving single goals between steps, for example to hand in two quests in one visit.

No setting change moved the time to level by more than 0.5% in the simulator: XP-first order off, reorder
threshold 8 seconds, detour 6 seconds, While-here radius 100.
