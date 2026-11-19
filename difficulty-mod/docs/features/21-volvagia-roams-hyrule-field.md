# Volvagia roams Hyrule Field

Setting `volvagia_field` (number 130): **not built**. Nothing safe could be shown from the game files. Branch `feature/volvagia-field` is unchanged (`5d5b4f6`); no code was written.

## Request
"let volvagia roam above hyrule field as an adult."

## Interpretation
In the adult Hyrule Field map, a Volvagia flies about in the sky as a roaming threat. Best case it attacks; fallback a harmless-ish dragon circling overhead. It must not block anything, must not make a seed unbeatable, must not crash the field and must have no story effect.

## Investigation
**What Volvagia is.** `Enemy_DungeonBoss_Volvagia` is a normal enemy actor with a 211-file pack. `Dm_Enemy_DungeonBoss_Volvagia` is the same pack with only a different model file and an `ActorParam` that inherits from the enemy. Same brain, same AI graphs. It is not a lighter animation-only actor. No map places the `Dm_` actor; it is used only by the boss intro/outro events (`Dm_FireTemple_BossIn`, `Dm_FireTemple_BossOut`). There is no scenery version.

**How the boss room drives it** (`Banc/FireTemple/FireTemple_Static` and the identical-shape `Banc/DreamWorld/VolvagiaStage_Static`):
- The actor carries named links: `Zero` to `Eight` (nine `DestinationAnchor` points, the holes), `RoomCenter`, `Five`, and three more references including the lava plane. Dynamic parameter `IsIntendedForEventAppear: true`.
- Its AI asks native queries `QueryVolvagiaGetHolePos`, `QueryVolvagiaGetHolePosNearOrderByInputNumber`, `QueryVolvagiaGetRoomCenterPos` and `QueryVolvagiaCalcCircleMoveTargetPos` (7 uses of the room-centre query across the graphs). These read the links above. What they return with the links missing is not visible from the data.
- The room's logic module `Enemy_DungeonBoss_Volvagia_Main_e0cf` only creates and deletes it (`ActorLogicEnemyDungeonBossVolvagia`: create and delete inputs; outputs Dead, IsBattle, FallRockActivate). It also drives a falling-rock manager, lava floor, env palette and warp prefab. Dropping the logic binding would stop room control but not the actor's own behaviour.
- `DungeonBossVolvagia.action.flymove.module` is not a path follower. It only picks a left or right 180 turn animation from the target direction. Flight positions come from the anchor queries, not from a rail or a point list.

**Story hooks live on the actor, not only in the room.** `DungeonBossCommonParam` for Volvagia starts `Dm_FireTemple_BossIn` on Appear and `SageOfFire_DefeatBoss` on Die, and its death procedures table is `DungeonBossVolvagia_game__death__DeathReasonProcedures`. `OneShotDeathBossOnProcedures` and `BossCommon.StartEvent.module` ("Die") are in its brain. So a field Volvagia that is killed would start the Sage of Fire defeat event whether or not the boss room's logic is attached. Those parameters sit in the shared actor pack, so editing them for the field would also change the real fight.

**Hyrule Field's own flyers.** The adult field has no rails for flyers. Its only rails (24 entries in `HyruleField_Adult_Static`) are `Guide` rails. Guay flocks are `Enemy_FlockGuayManager` plus `GuaySpawnPosAnchor` points, native behaviour; Guay_Fixed are placed individually. Peahats are not placed (`Land_HrlField_PeahatGround` is scenery). There is no vanilla pattern of a map actor flying a patrol path that Volvagia could reuse.

**Flags.** `Enemy_DungeonBoss_Volvagia_BossAppear`, `HaveDefeatedEnemy_Enemy_DungeonBoss_Volvagia` and `Volvagia_TemporaryDelete` exist. The enemy museum and boss rush only place `Enemy_DungeonBoss_Volvagia_Dictionary`, a display actor, not the live boss.

## Why nothing was built
1. The only way to give the AI its holes and room centre is to copy the actor with its nine anchors and centre into the field. The result would be a full boss fight with its Appear event and no falling rocks, in an open map.
2. A kill cannot be made story-free with data edits to the map: the Die event comes from shared actor parameters.
3. A scenery-only fallback does not exist: the `Dm_` actor has the same AI and was never placed in a map; the loose flymove graph does not move it along anything.
4. The Dodongo's Cavern black screen is still unexplained; adding a 211-file boss actor with native, unseen queries to the most visited map is the wrong place to guess. A crash there blocks the whole game.

## Most promising route, if you still want it
- Test the reachable question first in a throwaway map, not the field: add one live Volvagia with its nine anchors and centre to a small adult map, with no boss logic, and see whether it loads, whether the Appear event fires, and where it flies. Longshot's control bridge (see `00-making-new-features.md`) can teleport and take screenshots, so this can be checked without playing.
- If the Appear event is gated by `IsIntendedForEventAppear` (omit it) and the Die event cannot be removed, the remaining route is a custom actor pack: a copy of the Volvagia pack with its own `DeathProcedures` and `EventStartParameters` pointing at no events, registered as a new actor name. That is a bigger change and needs the actor name tables understood first.

## How it was tested
Read-only inspection of the actor packs, AI graphs, Fire Temple and Dream World boss maps, the boss logic graph and the adult field maps. `cargo build -p ootrr` builds clean. Nothing was run in the game.

## In the game
Nothing to test.

## Notes
- Anchors, not rails, are the way Volvagia moves. Any future attempt has to supply the anchors.
- A shared parameter file is the reason a field-only change to its events is impossible without a new actor pack.
