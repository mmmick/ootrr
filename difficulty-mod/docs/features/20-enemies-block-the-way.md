# Enemies block the way

Setting `enemy_gates` (number 129), checkbox "Enemies block the way" on the Difficulty page, default off. Built on `feature/enemy-gates`. Experimental, never loaded in the game. Code: `crates/core/src/difficulty/gates.rs`.

## Request
"enemies block progression. not all enemies, but many, should block the way-forward. this could be by adding a new lock to the existing door, or by putting up some temporary wall. the idea here is that if enemies are too hard to defeat, you should retreat and come back when you're stronger. it should never make a game impossible to beat, or lock you without the option to return. obv in areas like hyrule field this doesn't make sense, and some rooms already have this (like the stalfos at the end of the lost woods) but there's some obvious places where this would make sense, like the deku babas between the kokiri forrest and the deku tree"

Then, simplifying it: "do we really need to perma-save the state of these enemies-block-passage? can't it just be that if a particular enemy is alive, the wall is in-place, and when the enemies are defeated, it goes away? seems pretty stateless to me."

## What it does
No saved state and no new flag. The wall exists exactly while its guarding enemies are alive. If the area loads again and the enemies are back, the wall is back. The earlier blocker (needing a persistent flag) is gone.

## Gates (an explicit list, `GATES` in gates.rs)
1. The two Deku Babas on the path from Kokiri Forest to the Deku Tree (child map only). Babas at (39.12, 113.19, -18.58) and (37.74, 113.26, -19.59). The path there runs north to south, so one invisible wall (`FldObj_AirWall_Player_GenerateNavMesh`, the kind Mido's blocker uses) stands across it at z = -19.08, 16 wide, with the two babas on it. The other three babas of that map are untouched.

I did not add more gates. None of the other candidates could be checked as safe for the same reasons (see below).

## Mechanism
- **Death signal:** a Logic graph. The nodes are `ActorLogicSimpleActor` ones, the same kind the game uses for an enemy whose `Dead` output is read (the wolfos in `Gimmick_DefeatEnemy_Open_03`) and for air walls (`FldObj_AirWall_Player_Unridable` in the Death Mountain Trail graph). For each guard the build adds two nodes: a guard node, and a wall node whose `Logic_Delete` input is wired to the guard node's `Dead` output. With two guards there are two walls stacked at the same place, so the passage opens only when both are dead, with no AND node needed.
- **Host graph:** no vanilla graph has the shape "N enemies dead, then delete one plain actor" without a save flag or a typed enemy node. The closest are `Gimmick_AllDay_Skulltula_115c` (typed to the Gold Skulltula) and the counter rooms, which all set a persistent flag. So the build adds the four nodes to `Gimmick_AllDay_Skulltula_115c`, the smallest graph, exactly the way `minibosses.rs` adds a node, and binds them through a new `AiGroups` entry in the Kokiri Forest child static map, the way `ambush.rs` does. Other maps that use the same graph get four extra nodes bound to nothing.
- **Dynamic to static:** the five babas live in `KokiriForest_Child_Dynamic`, which has no `AiGroups`, and across 40 maps no group refers to a dynamic actor. Graphs only ever bind static actors. So the two guard babas are moved, unchanged, with their `Presence` flag, from the dynamic map into the static one. None is added or lost (the game test counts them).
- **Same flag on the wall:** each wall copies the baba's `Presence` (`KokiriForest_DeleteDekubaba`, negated). Once the Stone of Forest is yours or the Kokiri 0150 event has played, the babas are not placed and neither is the wall, whatever the graph says about a guard that was never created.
- **Wall choice:** invisible, because the vanilla visible barriers (webs, brambles, Mido's gate) burn, break or need an event, and a barrier that can be bypassed is not a gate. An invisible wall is confusing, so the build report says where it is.

## Never trapping the player
- The babas stand in the wall (within 0.5 of its plane), so a sword swing, the boomerang, a slingshot shot or a bomb reaches them from both sides. Coming out of the Deku Tree with the babas alive again you meet the wall from behind and can kill the babas from there.
- Warp songs, Farore's Wind and "Return with Navi" do not land in this corridor. Entrance shuffle can put you out of the tree on the far side, so the gate is skipped when entrances are shuffled.
- If you have no weapon at all, Deku Sticks and the tree's own contents still give one.

## Never making a seed unbeatable
`KF Outside Deku Tree` needs `is_adult or open_forest == 'open' or 'Showed Mido Sword & Shield'`, and the Mido event needs the Kokiri Sword. With a closed or closed-Deku forest a child already needs the sword (and the logic already counts it as the way to kill these babas: "Deku Baba Sticks" needs it or the Boomerang), so the gate adds nothing. With `open_forest` open a child needs nothing, so the gate is skipped there. Adult Kokiri Forest is untouched. The build report says when the setting is on but nothing is gated.

## Earlier findings that still matter
- Only Logic graphs react to deaths; map actors have no link that does.
- `FldObj_AirWall_*` actors take a `Presence` block. Mido's wall is `FldObj_AirWall_Player_GenerateNavMesh` at (25.17, 110.84, 3.79), scale (1.11, 1.5, 0.12), next to `Land_Kokiri_FieldProp_StoneGate_A_01`.
- The Deku Tree entrance is at about (90, 115.5, -28). Coming out of the tree puts you east of the wall.
- Counter rooms (Spirit 04_03, Ice Cavern 01 and 04, Forest Temple 06 and 18, Jabu, Shadow) set persistent flags, which is why none can be reused.

## How it was tested
- `cargo test -p ootrr-core difficulty` (96 pass, with `OOTRR_ROMFS` set) and `cargo test -p ootrr-core settings` (107 pass), `cargo build -p ootrr`.
- Unit tests: guards found by position and moved with their flag, missing or flagless guard refused, a wall per guard with the guard's flag, sorted unique hashes, round trip of the map, skipping under open forest or shuffled entrances, setting read and settings string unchanged.
- Game-file test (gated on `OOTRR_ROMFS`): guard count preserved across the two maps, walls added, old actors untouched, groups sorted, graph audits clean, each wall's delete input wired to its guard's `Dead` output, same output on every build.
- A real build with the setting on completed; the mod holds both Kokiri Forest child maps and the edited graph.

## In the game: what to check first
1. Kokiri Forest (child, closed forest) loads at all, with the Deku Babas where they were and no crash. A crash on loading the forest points at the moved babas or the new graph nodes.
2. Walk up from Mido's gate: is the invisible wall at the second baba pair and does it stop you? Is anything bypassed around its ends?
3. Kill both babas: does the wall go? Kill only one: it should stay.
4. Go through the tree and come out: you are east of the wall, the babas are back, so is the wall; hit a baba from this side.
5. With the Stone of Forest, the wall must not be there.
If the engine refuses a simple actor node bound to a baba, the next step is a vanilla typed node or a counter room.


## Redesign (branch `feature/gates-v2`, never played)
User, after the hands-on session: "there was no wall. don't make it invisible. use the burnable spiderweb wall but give it the purple glow that invincible enemies have." Earlier: "the wall shouldn't be invisible. make it visible. make it mido, or maybe the spiderweb wall but with the purple glowing effect."

### What was built
- Each guard now has, besides the air wall, a visible web: `Obj_InsideTheDekuTree_SpiderWeb_A_02` (the solid, elastic wall web used in the Deku Tree, Hyrule Field and the Shadow Temple) plus its cosmetic partner `Obj_InsideTheDekuTree_InteractiveSpiderWebParts_A_02` (instance name `SpiderwebPart`), joined by a `Reference` link from the web to the part, as in the game's own maps. Two webs stand on the wall line at x 36.4 and 40.4, one per guard, at scale 1 (the game places these with no scale).
- The air wall stays as the real blocker and is thicker (z scale 0.12, like Mido's blocker, instead of 0.03). The old wall was thin and Link crossed it in the hands-on session.
- Each web, part and wall is deleted by the Dead output of its guard through the same graph nodes (`ActorLogicSimpleActor`, `Logic_Delete`), so the webs of a guard vanish when that guard dies and the air wall vanishes when both are dead. Webs and parts copy the guard's `Presence` flag.

### Fire-proof: not done
The web dies by fire through its chemical parameters (`RequestDeathIfBurned` and `RequestDeathIfExploded` are true in `Obj_SpiderWeb`) and through its AI graph `Spiderweb_Wall` (burning leads to a death request; a lit Deku Stick sends an ignite message). Changing that needs a private copy of the actor (a new actor class with its own pack, rows in the actor info tables, the tag table and the resource size table, and an edited AI graph), as the cow clone feature does. That is data only but large and cannot be checked here, so it was not built. Instead the air wall holds when the web burns: fire removes the picture, never the blocker. The closest alternative if the user wants an unburnable visible barrier: a cosmetic web prop (`Land_Deku_SpiderWebProp_*`, no chemical parameters, no collision) in front of the air wall; its size is unknown.

### Purple glow: not done
The tint is not a model or actor parameter. It is applied by the native node `ExecuteSetEnemyDamageColorAndLight` in `AI/EnemyCommon.common.reactiondamagecolor.module.ainb` (the node's only data is its name; the colour lives in game code) and runs from the enemy graphs. No map actor key, model info or material parameter that a non-enemy object can carry was found. A glow would need game code or a rebuilt model, so the webs have none.

### Why the user saw no wall
Not the open-forest setting: `open_forest` defaults to "closed", so the gate is on by default, and the preset in the hands-on session set `enemy_gates` with entrances off. The map file of that session's mod could not be inspected afterwards (the mod folder was replaced), so the cause is not proven. Candidates: the wall was only 0.3 thick (now 1.2), a guard counted as dead (Dead true) before it was spawned, or the mod lacked the map. Nothing else in the difficulty code edits the Kokiri Forest map files, so a later merge overwriting it is ruled out. The test must print the mod's map and graph first (see `playtest/gates-v2/TESTS.md`).

### Unverified
Everything in the game: that the map loads, the web size and facing, that the air wall blocks, the deletion by the guards, the return of the webs.


## Session fixes (18 Nov, feature/session-fixes, not played)
User, with the gate v2 build: the web is visible and blocks, a bomb destroys it, and after that, with both guards alive, he could walk through.
- Why the wall failed: the invisible wall was `FldObj_AirWall_Player_GenerateNavMesh`, whose box is 4 by 4 by 4 before scaling (half extents 2). At scale (1.6, 1.5, 0.12) it was 6.4 wide, 6 high and 0.5 thick, not the 16 wide the earlier text said, so it was narrower than the two webs together (34.4 to 42.4). The game places that class in `_Dynamic` maps (Mido's blocker); our wall is in the static map, where the game uses `FldObj_AirWall_Common_Cube` (six in this map). The earlier script test that "stopped" Link at z -19.9 was within a metre of a guard Baba and may have hit the Baba.
- Change: the wall is now `FldObj_AirWall_Common_Cube` (a 1 unit cube standing on its origin), at (38.43, 112.2, -19.08), scale (16, 7, 1.2): x 30.4 to 46.4, y 112.2 to 119.2, z -19.68 to -18.48. Both guards and both webs are inside it. Same deletion wiring (both guards dead).
- Webs: still the Deku Tree wall web and its part. A bomb or fire can still destroy the web, but the wall stays, so the way stays shut until both guards are dead. A bombed web leaves an invisible wall.
- Not built: a private bomb-proof web. It needs a new actor class (pack, actor info rows, tag table, size table, edited AI and life data) and a wrong entry would crash Kokiri Forest on load with no way to check here. The air wall does the blocking. The purple glow is still not done.
- Test: `the_wall_covers_the_webs_and_the_guards_and_uses_a_class_the_static_map_already_holds`.
- In the game: see `<playtest folder>\session-fixes\TESTS.md`.


## Redesign v4 (branch `feature/gates-v3`, never played)
User, after v3: the gates were "the wrong way around" and Link got stuck at the Death Mountain one. Redesign, in the user's words: "ideally, if possible, the gates will not be there, and then when you approach the guard, it does the little cutscene it does to show the gate appearing -- like it does w/ the stalfos at the grove -- and then when you kill the guard it does the little cutscene again to show it go down. another good side-effect of this is that we'll never spawn the gate unless you're already on the guard's side of it."

### The game's own construct
The Sacred Forest Meadow wolfos fight (`Gimmick_DefeatEnemy_Open_03`) is one AiGroup: a player-sensor area (`ActorLogicAreaActor`, `HoldActorObserverBySensorHitInfo`, `OperateANDFixInputTrue`) that creates the enemy, an `ActorLogicEnemyCounter` for the purple barrier (`Obj_EnemyCounter_A_08`), and an event request actor that plays the short scene when the enemy dies. The barrier's own graph counts the enemies linked to it, deletes itself when there are none at start, and on `Finish` plays its end effect and deletes itself. The vanilla group also sets a saved flag and its scene is placed at that one spot.

### What was reused
Area, sensor and AND nodes (built field by field as in `ForestTemple_Level_Room_01_Prefab_c0f3`), the counter node, the barrier actor and its reference link. Per gate the build adds an `Area` actor (box 20 x 8 x 20 round the guard mark, `RADIUS` 10), one barrier piece per 4 units of wall (marked ends plus 1.5 each side), and wires each counter's `Logic_Create` to the AND output and `Finish` to the guard's `Dead`. The barrier does not exist until the player is in the guard's box; a guard killed from outside makes it delete itself on creation.
- Not reused: the two scenes. Their event request actor and camera belong to the one meadow spot and the saved flag would be shared by every gate. The barrier rises and drops with its own animation and effect, no camera. Unverified in the game: that the rise animation plays when a piece is created late.
- Facing: the pieces face the guard spot. The side and arrival inference of v3 is gone (the earlier uncommitted edit, facing the guard spot whatever the mark order, was kept and tested).
- Guards: enemies within 8 units of the guard mark (also ones with a presence flag, like the withered Babas); harmless scrubs and Gold Skulltulas are ignored. New: Lizalfos in Zora's River (`ZORA_GUARD`), Wolfos in Lost Woods (`WOODS_GUARD`; that map holds no fighting enemy), a Dead Hand with its four hands in the Market (`MARKET_GUARD`), `Enemy_MiddleBoss_GerudoThief` in Gerudo Valley (`GERUDO_GUARD`; in the fortress it runs in a carpenter-set group with extra links, so outside it is unverified), an Iron Knuckle awake from the start in Hyrule Field (`FIELD_GUARD`, same wake patch as the special encounters). Dead Hand, Iron Knuckle and the Gerudo are always new, never an existing enemy.

### Never trapping the player
The barrier rises only with the player in the guard's box. Flagged: gate 4 (the guard stands beyond the wall for a player walking east; from the west the barrier rises behind you); gate 2 (climbable wall beside the barrier, where Link was wedged in v3: check it); gates 9 and 10 (two doors of one small room: both barriers rise, the player is shut in until the guards die, as in the game's fight locks and as marked). Gate 8 (no guard, same wall as 9) is dropped. Gate 6 stays closed-forest only: with an open forest a child may have no weapon.

### Tests
`cargo test -p ootrr-core difficulty` and `settings` pass: geometry, facing whatever the mark order, trigger box, hands inside the box, guard selection, graph audit with the area, sensor and AND nodes, create wired to the AND, finish wired to the guard, determinism, and the game-file test over every gate map. In the game: `<playtest folder>\gates-v4\TESTS.md`.

## v5 (branch `feature/gates-v3`, after the user's hands-on of v4, not played)
v4 in the game: gate 3 worked exactly as designed (nothing until near, barrier rose, dropped when the Lizalfos died). Faults found, in the user's words: "the guard spawns immediately but the gate only appears when i get nearby. the guard should spawn when the gate comes up"; at gate 5 the barrier rose with no Lizalfos present and killing the nearby Tektites did not open it; at gate 1 an extra Mad Scrub sat exactly on top of one of the game's two.

- New guards are created by the barrier's trigger. Every new guard (Lizalfos, Wolfos, Dead Hand and its four hands, Gerudo fighter, Iron Knuckle) has a graph node whose `Logic_Create` is wired to the sensor AND output, the same wire the barrier counters use and the same way the meadow wolfos is created. A gate that makes its guards never takes a vanilla enemy near the mark as its guard (gate 5's blue Tektites).
- Gate 1's extra scrub: the game's Mad Scrubs are already owned by a vanilla AiGroup. v4 added a second node for them with `Logic_Create` constant true, so the engine created them again. Now a guard that a vanilla group references gets a node with `Logic_Create` constant false (creates nothing); a guard nothing owns (the withered Deku Babas moved from the dynamic map) keeps a constant true, as before. Tektites (gate 2) and scrubs (gate 1) are owned, so they get false. Unverified: that the false node still reports `Dead` when the vanilla owner kills the enemy. If gate 1 or 2 never opens, that is the cause.
- Marks: gate 3 is replaced by wall 14 (left -60.62 100.11 -8.67, right -64.0 99.85 -7.09, guard -64.05 99.71 -14.09). Gate 5 keeps its wall, with two Lizalfos at the marks (34.73, 106.31, -24.81) and (29.84, 106.68, -20.6), both created by the trigger; each has its own barrier stack, so both must die.
- Private graph per gate: each gate writes its own copy of the logic graph (`Logic/Gimmick_EnemyGate_NN_xxxx.logic.module.ainb`, module `Logic/Module/Gimmick_EnemyGate_NN/...`) and its AiGroup points at it. The vanilla `Gimmick_AllDay_Skulltula_115c` graph is no longer written. A new file has no size table entry, so `staging::grow_out` now adds one for a path that is missing from the table when `old` is 0.
- Gate 6 (review finding 2 and 3): skipped when the forest is open, and when `open_kakariko` is open with a forest that is not closed (that story state takes Mido away, so the child could reach the babas with no sword). With a closed forest or Closed Deku and Mido present, the child has already shown Mido the Kokiri Sword, so a weapon is guaranteed. Not done: the other gates whose guards need a weapon (3 to 5, 7, 9 to 13) do not tell the logic; they sit on routes a child or adult with the usual gear passes, and this stays an accepted requirement, to be reviewed.
- Gate 4: left built. Its guard is beyond the wall for a player walking up-river, so the barrier rises behind them; harmless now that the guard is created with it, but the user should re-mark it.

### A barrier with no living guard
Covered now: the guard is created at the moment the barrier is, so it cannot have wandered or fallen away before the player arrived; the barrier counter deletes itself when it has no enemy at start; and, if the game's wiring follows the wolfos fight, leaving the box turns the trigger off and removes both barrier and guard, entering again brings fresh ones.
Not covered: a guard that leaves the area or is lost after it was created while the player stays inside. The counter's `Finish` is wired to the guard's `Dead` output only. A "no longer alive" test (the `Logic_Alive` output) was not wired: before the guard exists it would read as not alive and could drop the barrier the moment it rises, and what `Dead` and `Logic_Alive` read for an unloaded or despawned actor cannot be learned from the files. Test it in the game.

## Round 2 (branch `feature/gates-r2`, from the user's hands-on of v5, not played)
v5 in the game: gates 3, 4, 5 and 13 worked, gate 6 "perfect". Gates 1 and 2 never opened, gate 11 stood on the entrance, the Gerudo fighter and the Lost Woods guards needed changing.

- Gates 1 and 2 no longer adopt the game's enemies (the "creates nothing" node never reported their death). Each now creates its own guard with the trigger, like gates 3 to 5: a Mad Scrub (gate 1, at (-6.4, 104.81, -53.4), about 2.5 units from the nearest vanilla scrub) and a Red Tektite (gate 2, at its old mark). The vanilla scrubs and tektites are left exactly as the game has them. Only gate 6 still adopts existing enemies (its withered Deku Babas, wiring unchanged). The new scrub has no leaf bush with it.
- Same-side rule, every gate: the trigger box is still centred on the guard mark, 20 x 20, but is shrunk (never moved) until all four corners lie at least 1 unit on the guard's side of the wall line (a wall line is infinite here, so it is stricter than the segment). Minimum half-width 2. Marks nearer the wall than 11 units change: gates 2, 3, 4 and 6 (7 to 9 units away, box about 11 to 16 wide on the wall side) and the room, gate 9 (box 7 wide, between the two doors). Gate 11's mark is 19.6 units from its wall, so its box stays 20 x 20 (z 22.6 to 42.6); the wall (z about 52.2, x -4.5 to 0.65) is the south entrance, the one the player walks in through towards -z, so entering and walking to the Dead Hand raises the barrier behind the player. Not checked against the map's spawn points.
- Gate 12: two Stalfos (`Enemy_Stalfos`, the Forest Temple kind) 3 units apart at the mark, both must die. Gerudo fighter as a guard: to fix later (cutscene on defeat, jail on loss); the user likes the fight.
- Gates 9 and 10 are one gate (id 9): one Stalfos at the middle of the small room, one barrier stack for each door, both fed by the single guard.
- Gate 13: barrier pieces lowered by 0.5. The marked ends are at height 136.64 and 135.53; pieces stand at the lower mark minus 0.5, 135.03.
- Red guards (not found): the vanilla Wolfos fight carries no tint-related key. Its barrier actor has only `IsSkipClose`, its Wolfos only `Drop__DropTable` None and `IsCheckHidingHoleAsDungeonRoom`, and the link is a plain Reference. The gate sets nothing on a guard. Only the adopted guards of gate 6 were seen red (the user did not report the created Lizalfos as red), and the withered Deku Baba's own model has a red-brown head and red leaves; the screenshot shows the Z-target arrows, which are red for enemies. Open: compare a withered baba in an unmodified game.

## Review findings from 18 Nov (history, addressed in v5 above)
From the code review of 18 Nov (`docs\code-review-2026-11-18.md`, findings 2 and 4). The review read the old `gates.rs`; I checked `feature/gates-v3` (commit 81597f3, `crates/core/src/difficulty/gates.rs`). The same problems are all still there. Nothing was fixed; v4 is unmerged and unplayed.

1. Nodes added to a logic graph other maps use (still present).
   - `gates.rs:16-17` name the vanilla module `Gimmick_AllDay_Skulltula_115c` and its graph.
   - `gates.rs:337-345` give each new AiGroup `Logic` = that module.
   - `gates.rs:354-430` (`wire`) add per gate a guard node per guard, an `Area` node, a sensor node, an AND node and a counter node. `gates.rs:466-507` (`stage`) read that vanilla graph and write it back under the same path (`outs.push(Out { path: GRAPH ... })` at 507).
   - Every other group in the game that names this module now also runs those nodes, whose `InstanceName` values appear in none of that group's References. `minibosses.rs` refuses this case ("a new node would break the other room"). Only Kokiri Forest and the gate maps were load-tested.
   - Fix: write the nodes into a new graph file with its own name (and size table entry, as backup does for its holder files) and point the new groups at it; or list every map that references the module, load-test them, and record them here.
2. The forest gate assumes the player has a weapon (still present).
   - `gates.rs:77` (gate 6, the path from Kokiri Forest to the Deku Tree, `closed_forest_only: true`) and `gates.rs:159-161` (`active`): the gate is skipped only for an open forest or shuffled entrances.
   - The guards are the two withered Deku Babas. A child with no sword or slingshot cannot pass them, yet the seed logic may put an item behind the Deku Tree that needs nothing. The same holds, more widely, for every gate whose guard needs a weapon: gates 3 to 5 (Lizalfos), 7, 9 and 10 (Wolfos), 11 (Dead Hand), 12 (Iron Knuckle from the Gerudo thief), 13 (the Iron Knuckle knight). None of them tells the logic.
   - Fix: skip a gate unless the seed guarantees a weapon at that point, or accept the requirement and say so in the note; prove it per gate.
3. `closed_deku` with an open Kakariko gate can remove Mido (still present, not changed by v4).
   - `active` takes only `forest_open` and `entrances`. The Kakariko gate setting says an open gate "takes Mido away", and that is allowed together with `closed_deku`. If the guards' `Presence` flag is still true in that story state, the gate stands where the game itself would let the child through.
   - Fix: also skip gate 6 when any setting removes Mido, or prove from the map files that the guards' `Presence` flag is off in that state and write that here.

## Round 3 (19 Nov): the trigger starts well past the wall

Seen on 19 Nov: gate 12 worked perfectly and is now shipped (`HELD` is `[1, 2, 7, 9, 11]`). Gate 11 raised its barrier while the player was still outside it, at the south entrance, and the game's own respawn point after a death is on that line.

- Rule from the user, every gate: the player must be at least 2 units past the wall on the guard's side before the barrier rises and the guard is created.
- Margin now `INSET` = 2.5 + `BARRIER_HALF` (1.0, a conservative half-depth of the barrier piece) = 3.5, measured along the wall's normal toward the guard, to every corner of the trigger box. The box is shrunk (floor half-width 2); if that is not enough its centre is moved away from the wall; if two walls leave no room (gate 9) the floor drops to 0.5. A test checks all gates.
- Gate 11 evidence: the box was 20 x 20 round a mark 19.6 units from the wall, yet it fired at the wall. 32.58 + 20 = 52.58 matches the respawn z of 52.6, so the game very likely reads the area `Scale` as half-extents, not full size. The `Scale` written is now the computed half-width (so the real box is the same or smaller than computed, whichever reading is true). If triggers feel too small, double it.
- Nothing persists after a death or Try Again: the map reloads and the barrier pieces and created guards come only from the graph's `Logic_Create` when the trigger fires; no flag or save entry is written for them. Gate 6 adopts the game's own Deku Babas (their own presence flags are the game's) and its barrier is created the same way, so the same holds.
- Gate 13's lowered barrier and the same-side rule are unchanged.
- Not verified in the game.

## Round 4 (19 Nov)

Gates 2 (Death Mountain Trail) and 9 (the small Lost Woods room, one guard, two gates) were seen working and ship. `HELD` is now `[7]`: only gate 7 (Lost Woods east passage) is held, not tried. Gate 2's guard changed from a created Red Tektite to a created `Enemy_Dodongo` (the fire-breathing one, not `Enemy_BabyDodongo`) at the same guard position on the ground; the new-guard kind is `DODONGO_GUARD`. The actor pack exists and the game creates it in other maps without room logic. The Dodongo version itself is not played yet.

## rc 13 scripted check (19 Nov)
- Gate 2 (Death Mountain Trail): the map loads with the mod; after a warp to (-54.67, 106.0, 10.4) on the guard's side an `Enemy_Dodongo` (hp 6) stood at (-54.4, 106.4, 3.2) and moved under 1 unit in 25 s. Barrier and fight not observed. Details: `playtest
c13\RESULTS.md`.
