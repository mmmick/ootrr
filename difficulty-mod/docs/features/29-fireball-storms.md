# 29. Fireball storms

Setting `fireball_storms` (number 140), toggle, default off. Difficulty page, new section "Hazards" (it is a danger of the place, not an enemy). Branch `feature/fireball-storms`, worktree `wt\fireballs`. Not played yet.

## What was asked (user, 20 Nov)

"Random fireball storms. In the vanilla game there's one sequence the first time you go up the volcano where it erupts and lots of fireballs fly down. Can we make that storm of fireballs fall down at other times too? Maybe when you're sneaking into Hyrule Castle courtyard, or in some of the boring-er parts of Hyrule Field."

## How the Death Mountain Trail eruption works (read from the game files)

Both `DeathMountainTrail_Child` and `_Adult` `_Static` maps hold the same set of falling-rock managers. All are plain map actors with a Logic module; none is a Death-Mountain-only actor.

| Actor | Count | What it does |
|---|---|---|
| `Obj_FallingRockManager_A_04` | 5 | The burning-rock storm. Launches a rock from its own map position to each linked `DestinationAnchor` in turn. Rock `Obj_FallingRock_A_04` carries a sub-actor `Obj_FallingRock_Fire_A_01` (fire damage, attack power 2; the rock itself 4). |
| `Obj_FallingRockManager_A_05` | 1 | A fan of rocks (`Obj_FallingRock_A_02`, up to 25) every 2 to 6 frames, 80 to 100 units from the manager, within a wide angle. The large "demo" storm. Gated by the saved flag `MountainTrail_ChildFallingRockDemoActivated`. |
| `Obj_FallingRockManager_A_01` | 1 | Rocks (`Obj_FallingRock_A_05`, up to 20) along a rail, near the first slope. |

What starts and stops them. Each manager has a small AI Logic module (for example `DeathMountainTrail_Child_Prefab_7fa0`): an `Area` actor (layer Player) feeds a `HoldActorObserverBySensorHitInfo` node, and its `IsHit` output is wired to the manager's `IsActive` input. So the storm runs while Link stands in the area. The area's `Logic_Delete` is wired to an OR of the saved flag `MountainTrail_HasReachedPeak_Child` and a Great Fairy flag, which switches the storms off for good once the peak is reached. That is the only story coupling.

How the A_04 manager works (its AI, `Obj_FallingRockManager_A_04.root.ainb`). While `IsActive` is true: a loop with a counter (`IntervalFrame` from the map actor's `Dynamic`, 15 to 120 frames in vanilla); each time it fires it reads the `Reference` link whose destination actor is named with the current index ("0", "1", ...), launches a rock from the manager's home position to that anchor with flight time `FallTime` (3.5 to 4 s), and adds one to the index. When no link has that name the index returns to 0. At most 5 rocks are held at once. Landing spots are the anchors, so a storm is deterministic, readable and dodgeable. In vanilla the managers hang 65 to 150 units above and about 140 units away from anchors packed within 15 units of the path.

Damage. The rock hits for 4 base attack power (one heart), the fire sub-actor for 2. Unchanged by this setting. (Enemy damage multipliers do not apply: it is a map object.)

Packs and effects. Nothing new is loaded by hand: the manager's `ActorHolderParam` lists the rock (`DependencyWithHolder`), and the rock's holder lists the fire actor, so the engine pulls the packs in with the manager, as for every actor the build places that a map did not have (Dead Hand, Like Like, Iron Knuckle). Effect keys are shared in `ELink2` and keyed by actor, so the visuals travel with the rock. If a map shows no burning rock but the manager is active, this is the first thing to suspect.

## Design

Place the A_04 manager and its anchors in new maps and copy the vanilla Logic module's wiring. Data edits only: map placement files (`Banc/.../_Static`) and one new Logic graph per spot. Nothing is a code patch, nothing is Longshot-specific, nothing touches the volcano maps (the test checks that the Death Mountain files are not written and still hold 5 managers).

Per spot the build adds, to every map that carries it:

- `Obj_FallingRockManager_A_04`, high above and 60 to 110 units to the side of the spot (the same shape as on the mountain), with `Dynamic` `FallTime` and `IntervalFrame`, and `Reference` links to the anchors.
- 5 or 6 `DestinationAnchor` actors named "0", "1", ... on the ground around the spot (the landing points).
- One `Area` actor, the trigger box.
- `FarDeleteGroups` and `SimultaneousGroups` entries (when the map has them) holding all of the above, so the engine loads and unloads them together.
- An `AiGroups` entry whose Logic module is a new graph `Logic/Hazard_FireballStorm_<spot>_<tag>.logic.module.ainb`, built from the gates graph template with `ambush`/`gates` machinery: Area node, player sensor, AND, and the manager node (`ActorLogicFallingRockManager`) with `Logic_Create` true and `IsActive` wired to the AND output. So the storm runs only while Link is in the box, stops when he leaves and starts again on re-entry. The manager is created with the map and is never deleted by logic, exactly like the vanilla prefab.

Never tied to the volcano flags, never blocks anything (the manager, anchors and area have no links to doors, events or enemy counters).

Not done: a random chance per visit. The AI graph has random-number nodes, but a query re-rolls every frame, so a chance gate would flicker the storm on and off instead of deciding once per visit. A spot is therefore always on while the setting is on. A cheaper randomness is to ship fewer spots; the table below is where to cut. (Possible later: a rare-spawn variant that uses the A_05 manager's own random fan.)

## Spots

All coordinates are map coordinates. Heights come from the navigation mesh (child and adult agree within 0.3). Landing spots sit 0.5 above the ground. Each trigger box is 32 x 12 x 30 (width, height, depth) in the field, 44 x 10 x 26 in the castle.

| Spot | Maps | Trigger centre | Manager | Landing spots (x, z) | Interval, fall |
|---|---|---|---|---|---|
| `field_middle_road` | `HyruleField_Child`, `_Adult` | (75, 152.9, 95) | (170, 216, 60) | (75,95) (85,100) (65,100) (80,85) (70,105) (85,90) | 45 frames, 3.5 s |
| `field_south_central` | same | (55, 163, 190) | (150, 226, 160) | (55,190) (65,195) (45,195) (60,180) (50,200) (65,185) | 45, 3.5 |
| `field_west_road` | same | (-170, 137.1, 170) | (-75, 200, 135) | (-170,170) (-160,175) (-180,175) (-165,160) (-175,180) (-160,165) | 45, 3.5 |
| `castle_front_garden` | `HyruleCastle` | (14, 118, -76) | (14, 186, -165) | (25,-74) (28,-80) (4,-71) (25,-89) (-14,-86), all at height 114.5 | 60, 3.5 |

How the field spots were chosen (survey of the navigation mesh, child and adult together): open, flat ground (every point within 15 units on one floor, within 4 of the centre height, slope at most 18 degrees), at least 70 units from every arrival point, map exit and entrance request actor, and at least 45 from every hole, gossip stone, Great Fairy hole, Business Scrub, Gold Skulltula, Kaepora, ranch actor, Lake Hylia Like Like site and our special encounters (the Iron Knuckle at (134, 157.7, 168.5), the Dead Hand hole, the Guay flocks are all further than that). Middle road is the open stretch between the Market gate (0, 151, 32) and the Kakariko road; south-central is the grass south of the Iron Knuckle between the Lon Lon and Lost Woods roads; west road is the lowland on the way to Gerudo Valley (arrival at (-307, 137, 166), 120 away).

Castle spot. The child castle (`HyruleCastle`, one map) has stationary guards on `AnchorAction1` posts: the front garden holds six `Npc_Hylia_Soldier_*` posts at (13,-64), (-5,-77), (16,-82), (-5,-95), (3,-95), (35,-93), height 114, and the courtyard holds five more, from (-4,-132) on. The front garden is the sneaking stretch before the courtyard. The landing spots are all at least 9 units from every soldier post (checked by the test); the trigger box spans the garden between the gate guard (22, 108, -56) and the first guard line. The area, manager and anchors have no link to the guards' `FoundBySecurity` events. Open risks: a fireball landing in a guard's line of sight may or may not make him react (unknown, never seen), and Link may be knocked back into a guard's view. Cut the entry from `SPOTS` if that happens; the other three are independent. The approach from the entrance to the gate guard (Malon's wagon at (17.6,105.5,-25.1), the gossip stone at (21.4,112.2,-35.5)) is deliberately left alone: NPC event actors stand there and Malon is a quest NPC.

## Setting and menu

Number 134: free on upstream main and on every open branch and worktree (130 to 132 used by the base, 133 Volvagia, 136 to 139 Bowling, enemy randomizer and others; 134 and 135 were unused). The retired list (`RETIRED` in `settings/list.rs`) holds 55, 63, 92, the old shop settings 59 to 62 and 130 (old special encounters), so 134 is not retired either. Id `fireball_storms`, label "Fireball storms", tooltip "Burning rocks rain down on a few quiet stretches of Hyrule Field and the Hyrule Castle garden, like on Death Mountain. Dodge them." (23 words). Placed in a new "Hazards" section after "Boss fights" on the Difficulty page. Default off; the default settings string is unchanged (tested).

## How it was tested

`cargo test -p ootrr-core difficulty`: 214 passed (4 are new, in `difficulty/fireballs.rs`, run against the game files with `OOTRR_ROMFS`):

- Spot layout: 3 to 8 landing spots, manager 60 to 110 units away and 55 to 80 above, every landing spot within 30 of the box.
- The generated graph parses back, audits clean, has 4 more nodes than the template, the manager node has its `IsActive` input on the AND output, built twice gives the same bytes.
- Map files: only the 4 `_Static` files below are written (`HyruleField_Child`, `HyruleField_Adult`, `HyruleCastle`... see the test), the old actors are untouched and first in the list, no hash is used twice, AiGroups stay sorted, each manager links all its anchors, anchors are named "0".., no landing spot is within 9 units of an NPC or enemy actor at the same height, and the Death Mountain Trail maps are not written and still hold 5 managers.
- Every landing spot is on the navigation mesh within 1 unit in both ages.
- Setting off: `stage` writes nothing; setting on writes exactly the four graphs and the three map files.
- `cargo test -p ootrr-core settings`: 106 passed (default string unchanged, menu places every setting).

Not tested in the game. Nothing about the actual fall, the damage, the picture or the castle guards has been seen.

## How to test it in the game

See `<playtest folder>\fireballs\PLAN.md`: load each touched map first (3 maps: child Field, adult Field, Castle; Death Mountain Trail child as the control), then stand in each box and watch.

## Seen in the game (20 Nov)

The field_middle_road storm works: rocks arc in and burn on the ground (user's clip, handson-6). Expect the first impact about 5 s after stepping into the area (45-frame interval plus 3.5 s flight); after that a rock about every 0.75 to 1.5 s. It stops when Link leaves, though rocks already in flight still land. The setting number is 140 (134 is taken by door of time). The castle garden spot is not checked yet.

## Release (rc 16, 20 Nov)
In rc 16 as setting 140 (the "Setting and menu" section above still says 134, which upstream's door of time took). Number check on upstream 0.8.0: 133 and 134 door of time, 135 sword, 136 used upstream; 139 is bowling grass; 141 is held for faster blocks.

- **Played by hand** (session 6, 20 Nov): the middle-road storm (rocks arc in and burn; slow to start, about 5 s) and the castle garden storm (the guards behave).
- **Seen:** the frame rate in child Hyrule Field was about 30 to 37 with more enemies at 5x plus the storm (60 before). Not measured with the storm alone.
- **Not played:** `field_south_central`, `field_west_road`, and the adult field.
