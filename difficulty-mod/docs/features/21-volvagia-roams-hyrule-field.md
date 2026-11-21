# Volvagia roams Hyrule Field

Setting `volvagia_roams` (number 133, default off). Branch `feature/volvagia-field2` in `worktree volvagia`.
**Built in the files (Route A, custom actor), not yet run in the game.** Test plan: `<playtest folder>\volvagia\PLAN.md`.
(The first attempt, `volvagia_field` on `feature/volvagia-field`, built nothing; its findings are kept below under "First investigation".)

## Request
"let volvagia roam above hyrule field as an adult." On the earlier "not possible" verdict (20 Nov): "yes, try a few things. What's the limitation here? Can't we just make our own logic if we need to?"

## Interpretation
In adult Hyrule Field a Volvagia flies about and attacks. It must not block anything, must not make a seed unbeatable, must not crash the field and must have no story effect. Killing it gives nothing.

## How it works (Route A: our own actor class)
**Vanilla precedent.** The game already clones Volvagia once: `Dm_Enemy_DungeonBoss_Volvagia` is the Volvagia pack plus one ActorParam whose `$parent` is `Work/Actor/Enemy_DungeonBoss_Volvagia...ActorParam.gyml` and which overrides one reference (its model). Its ActorInfo row is a copy of Volvagia's with `ActorName`, model, ELink/SLink user names still `Enemy_DungeonBoss_Volvagia`. The randomizer's `cow_clone` module registers new actor classes the same way (new pack, ActorInfo, GameActorInfo and Tag rows, size table entries). We follow both.

**The clone `Enemy_Volvagia_Roam`** (`difficulty/volvagia.rs`): new pack = the whole Volvagia pack (211 files, unchanged) + four files of its own:
- ActorParam: `$parent` Volvagia's; overrides `GameParameterTableRef` (own table) and `BattlePresentationRef` → `EnemyCommon` (normal enemy battle music `BGM_EnemyBattle` instead of `EnemyDungeonBoss`, which carries `BGM_DungeonBoss` and the boss presentation class).
- GameParameterTable: Volvagia's, with `DungeonBossCommonParam` and `DungeonBossVolvagiaParam` pointed at own copies.
- DungeonBossCommonParam: no `Appear` entry; `Die` kept with an empty event name and the start condition the game uses for Appear. Why this is safe: `BossCommon.StartEvent.module` switches on the condition: case 0 never, case 1 "AI condition met" (an input), case 2 "is dead". Volvagia's brain calls it for `Die` with that input fixed to false. Die's own value must be "is dead"; Appear's (a different valid value) is case 0 or 1, so the Die request can never fire, and even if it did it names no event. Nothing in the actor's graphs asks for "Appear"; the vanilla intro is started by the boss room.
- DungeonBossVolvagiaParam: Volvagia's, the seven circle/leave distances times the scale switch (default 1).
- Rows in `RSDB/ActorInfo` (heap grown by the pack growth), `RSDB/GameActorInfo`, `RSDB/Tag` copied from Volvagia's; size table entries for the pack and the four files (learned from their Volvagia peers, as cow clones do). Done in its own step after the late files and cow clones (`slice/edit.rs`), because the generic late path cannot size a pack that does not exist in the game.

**Placement.** The 8 holes (`Zero`..`Eight`, there is no `Four`) and `RoomCenter` are read at build time from the dream world Volvagia stage (same layout as the Fire Temple room: a test checks this) and placed around the arena point with the same links, plus the actor, in one far-delete and one simultaneous group. No `IsIntendedForEventAppear`, no room logic, no lava plane link, no falling rock manager, no heart container. Holes are 5 to 14 units from the centre; the AI's circle moves use the room centre and the distances in its own parameters.

**Two stages** (env var `OOTRR_VOLVAGIA_STAGE`): `1` = ruined Market plaza `Market_Adult` (-1.9, 100.07, 12.4), a throwaway test map; otherwise adult Hyrule Field at the open mid-field spot (123.7, 154.2, 159.0). `OOTRR_VOLVAGIA_LIFT` (0 to 60) raises the whole ring; `OOTRR_VOLVAGIA_SCALE` (0.5 to 4) spreads holes and circle distances.

**"Attacks".** It keeps all of Volvagia's AI: jump out of a hole, fly arcs and circle the centre, fire breath (with its fire field), air dive, dive back into a hole, chance time on the ground. Falling rocks need the room's rock manager, which is not placed, so that attack should do nothing.

## What is certain from the files and what is not
Certain: the clone pack, rows and sizes are built and parse back; the clone's boss parameters contain no Sage of Fire or BossIn event; the real Volvagia's files are untouched (its pack, its rows); the Fire Temple and dream world maps are untouched; the hole layout matches vanilla; the map gets exactly 10 new actors with unique hashes.
Unknown until the game runs: whether a Volvagia without its room logic appears at all (its hidden start may wait for the room), what the native hole queries do outside the boss room, whether the empty Die event name is harmless if the death path asks for it anyway, whether the body is removed after death, how it behaves on uneven field ground, and the heap/crash risk in the field. Also: killing it may count toward `HaveDefeatedEnemy_Enemy_DungeonBoss_Volvagia` / `DefeatedEnemyNum.Enemy_DungeonBoss_Volvagia` (enemy dictionary), because it keeps Volvagia's enemy parameters and `ActorName`.

## Route B (our own AI) and Route C
Route B not built: it is only needed if stage 1 shows the native hole queries do not work outside the boss room. The mechanism is ready: the clone has its own ActorParam, so it can point `BSARef` at its own `BSAParam` whose `BrainAinPath`, `ActionSlotMainAin` and `UtilityAinPath` name our own graphs (the ainb module already grafts nodes into graphs for regen and backup). Writing a whole brain plus action graph from scratch is a larger job than this time box; it should start from the result of stage 1. Route C (a re-skinned ordinary flyer) was not built, as asked: A is not proven impossible.

## How it was tested
Unit and game-file tests in `difficulty/volvagia.rs` (5): switches; the clone pack keeps every Volvagia file byte for byte and adds four with the right references, no story events, scaled distances; the ring matches both the dream world and Fire Temple rooms; rows and tags copy Volvagia and refuse a second insert; both stages place one actor with 9 linked anchors at the right height and spread, groups grown by one, sizes for every new file. `cargo test -p ootrr-core difficulty`, `settings`, release build. Nothing run in the game.

## In the game
See `<playtest folder>\volvagia\PLAN.md`: stage 1 Market load test, appearance, events, music, kill and flag reads; lift/scale variants; stage 2 field with travel and kill tests.

## First investigation (19 Nov, kept)
- `Enemy_DungeonBoss_Volvagia` is a normal enemy actor with a 211-file pack; flight comes from native queries `QueryVolvagiaGetHolePos`, `...NearOrderByInputNumber`, `...GetRoomCenterPos`, `...CalcCircleMoveTargetPos` that read the named links. `flymove` only picks a turn animation.
- The room logic `Enemy_DungeonBoss_Volvagia_Main_e0cf` creates/deletes it and drives rocks, lava, palette and warp.
- The story hooks are in the shared `DungeonBossCommonParam` (Appear `Dm_FireTemple_BossIn`, Die `SageOfFire_DefeatBoss`), which is why a field copy needs its own actor class: done above.
- The adult field has no flyer rails; Guay flocks are native.
