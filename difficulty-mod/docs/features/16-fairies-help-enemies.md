# Fairies help enemies too

Setting `fairy_helps_enemies` (number 125), checkbox. Code: `crates/core/src/difficulty/fairy.rs` and `flows.rs` (`fairy_used`). Built by a sub-agent on `feature/fairy-helps-enemies` (`df5f039`).

## Request
"fairies heal / respawn enemies. if a fairy gets used, all the enemies also heal, and the most recently defeated enemy gets resapwned"

## Interpretation
"Used" means releasing a bottled fairy to heal, or being revived by one on death. Two halves: (a) every living enemy nearby heals to full, (b) the last enemy you defeated comes back. I told the agent (b) was probably much harder and a solid (a) alone was acceptable.

## Investigation
- **(a)** needs a signal from the fairy events to every enemy. Events can set a game data flag; enemy AI can read one. The flag chosen is `MiniGame_Diving_Test`: it exists, is unsaved, defaults to false, and nothing in the game refers to it.
- "Heal to full" is the same heal node regen uses, in mode 1. The player's "heal to max" event is wired exactly that way.
- **(b)** A defeated enemy is deleted, so none of its own logic is left to run. No event action or AI node can name "the last one beaten" or recreate a placed enemy. A "this room's enemies come back" variant had no safe hook either.

## Implementation
- **Events raise the flag:** one added event in the release-fairy flow and one in the fairy revival chain of the game over flow.
- **Enemies poll it:** the shared enemy graph (the one regen uses) gains 15 nodes. While the flag is set, a living enemy heals to full. A 30-frame timer then clears the flag.
- Enemies clear the flag, not the event, because it is unknown whether enemy AI runs during the event.
- **(b) is not implemented.**

## How it was tested
- 26 difficulty tests, with node shapes compared against game-authored nodes.
- A real build: both flows carry the new event, `flow-loads` is clean, and with lose-heart and no-bottles also on each flow carries all edits.

## In the game
Never played. To test:
- Turn this on with enemy health at 3x. Damage a Stalfos or Lizalfos, then release a bottled fairy nearby.
- Pass: the enemy takes a full set of hits again.
- Also die with a fairy in a bottle next to a damaged enemy.
- Touching a wild fairy should not heal enemies.

## Notes
- If no enemy is awake when the fairy is used, the flag stays set and the next enemy to wake heals instead. A reload clears it.
- Not covered: Beamos and other enemies that do not run the shared graph.
- Must be applied after regen in the build (it wraps whatever it finds; regen expects the original graph).
- For (b), the control bridge's memory access or a code patch would be the only routes I can see.

## Red enemies (session fixes, 18 Nov, code only)
The user saw enemies tinted red with regen, fairy and backup on (screenshot of a Deku Baba in Kokiri Forest). Findings from the game files:
- The damage colour is made by one native node, `ExecuteSetEnemyDamageColorAndLight`, alone in `AI/EnemyCommon.common.reactiondamagecolor.module.ainb`. In the game's own file that node is the whole graph: no parameters, no inputs, no outputs, no links. Its colour logic is in game code, so the graph cannot be read for a cause.
- Regen, fairy and backup each wrap the old root in a Parallel (`EndPolicy` 0, `ResultPolicy` 0, the same values the game's own caller graphs use) with the old root as its first child. With all three the node sits three Parallels deep. Its own bytes and the link order are unchanged (new test `the_vanilla_damage_color_node_is_untouched_after_regen_fairy_and_backup`). Nothing writes a colour or damage key.
- Only regen writes to the life component (`OneShotLifeSetOwnLife`, add 1 every 150 frames, clamped at the maximum). The game uses that node only in the player's and NPCs' event graphs, never in an enemy graph.
- So no edit was found that changes the colour path, and none was made. Two causes remain:
  1. Regen: a heal, even one that changes nothing, may count as a life change that the colour node reads as a hit, giving a flash every 150 frames.
  2. The wrapper: the colour node runs as a child of a Parallel and not as the module root, and may behave differently there.
- Not caused by the speed setting (it scales keys ending in Speed and Interval only) or by the gate edit (the gate does not touch the guards' components).
- To tell them apart, build three mods with one setting each and look at a Deku Baba: `enemy_backup` only (wrapper but no heal), `enemy_regen` only (wrapper and heal), `enemy_speed` 200 only. Red with backup only means cause 2; red only with regen means cause 1; red with neither means another feature.

## Review fix: nodes live in the caller graphs (branch `feature/review-fixes`)
Same change as in 09: the poll switch is now an extra child of the Parallel that holds the damage colour module call in each caller graph, and the module graphs stay as the game has them. A caller counts as already patched when it reads or clears `MiniGame_Diving_Test` (vanilla callers use the same game data nodes for other flags). Test: the all-8-combinations test described in 09. In the game: see `<playtest folder>\review-fixes\TESTS.md`.

## Fairy circling the enemy (19 Nov, investigated, nothing built)
You, after seeing the heal work: "can we get it to play the fairy spinning around the character animation on all enemies that they're healing, too?"
Searched every AI graph in the game (loose files and those inside actor packs) for effect nodes:
- The circling fairy on Link is made by the player's own nodes (`EventPlayerOutBottleFairy`, `EventTriggerPlayerUseFairy`, `OneShotPlayerUseFairy`) and the `Dm_SY_OutBottleFairy` event, all tied to the player actor.
- The only generic effect nodes (`EventNodeEffectEmitOneShot`, `OneShotGenericEmitEventEffectInputActor`) live in the `EventEffectEmitter` actor's event graph and run only inside an event flow, which an enemy graph cannot start.
- Enemy graphs have only effect nodes made for one actor (Peahat, Wallmaster and similar), plus `OneShotEnemyHandleCreateEffectAndFadeIn`, the spawn fade-in, which is not a heal effect.
- No node plays a named effect at an arbitrary actor from an enemy graph, so no data-only route exists. Nothing was built. The remaining routes need a code patch or the control bridge, which the project rules forbid.

## Effects experiment (19 Nov, branch `feature/fx-r1`, investigated, nothing built)
- **Request.** Use particle effects to show a regen tick, a fairy heal and a backup call.
- **Found.** The only generic effect node enemy graphs use is `OneShotXLinkSearchAndEmit` (170 uses; props `ExecuteEveryFrame`; inputs `ELinkCall`, `SLinkCall`, `XLinkKeyName` as a constant string; output `HandleSet`). Example: `Enemy_Leever.utility.postsensor.root` emits `Upheave`. The key names (`Death_Start`, `Wakeup`, `Dig`, ...) are looked up in the actor's own XLink user, which lives in the global effect-link resource, not the actor pack. Notes from another modder confirm that particle sets belong to actor bundles and that a set not loaded for the scene does not appear.
- **Not built (time box).** A key every enemy's XLink user has, and that reads as a heal or alert, was not identified; emitting a key such as `Death_Start` on a heal would read as dying. Next step: read the global XLink resource, list the keys shared by every enemy user, pick one, and add one `OneShotXLinkSearchAndEmit` beside the heal / call nodes (no blackboard keys needed).
