# Enemies regen health

Setting `enemy_regen` (number 118), checkbox. Code: `crates/core/src/difficulty/regen.rs`. Built by a sub-agent on `feature/enemy-regen` (`d582934`).

## Request
"enemies regen health"

## Interpretation
While alive, enemies slowly recover health, never above their maximum. I left the rate to the agent, asking for something small every few seconds.

## Investigation
- No parameter for regeneration exists, so it had to be added to enemy AI.
- Nearly every enemy's AI calls one tiny shared graph, `AI/EnemyCommon.common.reactiondamagecolor.module.ainb`, in the global AI pack. Stalfos use an identical private copy in four packs. That graph is the hook: one edit reaches about 123 actors.
- The heal itself is the node `OneShotLifeSetOwnLife`. The player's "heal" event uses it with mode 0 (add and clamp to max), and a Floormaster graph proves it works on enemies.
- A looping frame counter node is used the same way in Twinrova's own graph.

## Implementation
- The graph gets a new root that runs the old node alongside a 150-frame looping timer. Each time it fires: has a life component, is not dead, add 1 health.
- Every actor that runs the graph gets a little more AI memory (`InstanceHeapSize` + 0x4000), as the Bunny Hood feature does.
- Size table entries grow for the graphs and packs.

## How it was tested
- rc 13 (19 Nov, merged build, script, `playtest
c13\RESULTS.md`): a hit Stalfos regained +1 hp 15.2 s and 15.0 s after the hit, then every 4.9 and 5.4 s. The source of the hits was not identified (19 sword swings did no damage).
- 22 difficulty tests, including checks that each added node has the same shape as a game-authored node of the same kind.
- A real build with health at 2x and regen on; the built graph has 11 nodes in both places.
- Found and worked around a bug: the repo misreads the compression dictionary of the global AI pack.

## In the game
Never played. To test:
- Turn this on with enemy health at 3x or more, so there is health to regain.
- Hit a Stalfos or Lizalfos a few times, back off for 30 seconds, then count the hits to kill it against a fresh one.
- Pass: it takes noticeably more hits after the wait.
- Watch bosses with health phases (Volvagia, King Dodongo, Twinrova): healing back over a phase line may replay a phase.
- A crash when any enemy loads means the graph edit is rejected; report the map.

## Notes
- Rate is one point per 150 frames, about five seconds if the game counts 30 per second. That assumption is unverified.
- Enemies with 1 or 2 health show no effect.
- Not covered: Beamos, Anubis, flying pots, Shabom, Peahat larvae, Gohma eggs, and boss parts such as Morpha's tentacle.
- This shared graph is the best hook found for "make every enemy do X". Fairies-help-enemies and call-for-backup reuse it. Order matters: regen must be applied first.

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
The code review (finding 1) read the red tint as the wrapper: each feature made a Parallel the root of the damage colour module, so the module could never return and the colour node might never be left. What the files show: the module is one native node; the callers (16 shared graphs in the AI pack, and 85 actor packs that carry their own graph) run it as one child of a per-frame Parallel next to other modules, and none of them reads a result from it or does anything when it returns. So the module is unconditional and not "only entered on a hit", but whether a Parallel that never ends changes how the native node behaves cannot be told from the files.
- Built: the module graphs (`EnemyCommon...reactiondamagecolor` and the Stalfos one) are no longer edited, in any combination. Regen adds its looping timer as one more child of the Parallel that holds the module call, in every caller graph (`difficulty/hook.rs` is the one shared helper, all three features use it). Nodes are the same as before minus the old wrapper. Heap growth is unchanged.
- Test: `the_vanilla_damage_color_module_is_untouched_in_every_combination_of_the_three` builds all 8 on/off combinations of regen, fairy and backup and checks that both module graphs are byte-identical and the call itself is untouched.
- In the game: with regen, fairy and backup on, are enemies normal-coloured before and after being hit; does regen still heal. See `<playtest folder>\review-fixes\TESTS.md`.

## Wait before healing (branch `feature/regen-wait`, code only)
The user fought two Stalfos with regen on: "they're recovering health so fast I can't kill them!" Decision: "10 seconds wait, 1 hp every 5 seconds". An enemy starts healing only after 10 seconds without being hit, then heals 1 point every 5 seconds until full. A new hit restarts the 10 seconds.
- Hit signal: `QueryLifeIsExternalDamaged` (the life component's "damaged from outside" flag), no keys, no blackboard. Vanilla precedent: the Stalfos root graph (`AI/Enemy_Stalfos.utility.root.ainb`, element 5 and the Switch 38 that reads it) sits in the same kind of per-frame Parallel we host in; `EnemyCommon.common.ragdolldie.module` and Freezard, Wolfos and Bubble graphs use it the same way. The nodes are copied from those; a test compares their shapes.
- Wait: a Switch on the hit query. True runs an idle node. False runs a 600-frame non-looping `SelectorGenericFrameCounter` (60 frames a second). When a hit flips the Switch, the counter is left and starts from zero when it is entered again. Precedent for a counter under a Switch branch: `AI/Kokko.utility.root.ainb` (Switch 106, False to counter 60).
- Heal: the old looping 300-frame timer (`HEAL_EVERY`, 5 s) is the counter's Finish child, so it runs only after the wait and restarts with it. Its Finish runs the old guarded heal (has life, not dead, add 1). The first heal comes 15 s after the last hit (10 s wait, then one 5 s period), then every 5 s.
- Nodes added per hosted call: 14 (was 9). No keys are read or written, so the placement-key and blackboard problems of the backup feature do not apply; a test checks the count of Blackboard nodes in the patched caller is unchanged.
- Not verified in the game: that the flag is true on the frame an enemy takes a hit (the Stalfos graph reacts to it, so it should be), and that the Switch re-enters the counter from zero. To check: debug overlay enemy health on a Stalfos; hit it, 10 s of no change, then +1 per 5 s (first at about 15 s); hit again at 8 s and the wait restarts.

## Effects experiment (19 Nov, branch `feature/fx-r1`, investigated, nothing built)
- **Request.** Use particle effects to show a regen tick, a fairy heal and a backup call.
- **Found.** The only generic effect node enemy graphs use is `OneShotXLinkSearchAndEmit` (170 uses; props `ExecuteEveryFrame`; inputs `ELinkCall`, `SLinkCall`, `XLinkKeyName` as a constant string; output `HandleSet`). Example: `Enemy_Leever.utility.postsensor.root` emits `Upheave`. The key names (`Death_Start`, `Wakeup`, `Dig`, ...) are looked up in the actor's own XLink user, which lives in the global effect-link resource, not the actor pack. Notes from another modder confirm that particle sets belong to actor bundles and that a set not loaded for the scene does not appear.
- **Not built (time box).** A key every enemy's XLink user has, and that reads as a heal or alert, was not identified; emitting a key such as `Death_Start` on a heal would read as dying. Next step: read the global XLink resource, list the keys shared by every enemy user, pick one, and add one `OneShotXLinkSearchAndEmit` beside the heal / call nodes (no blackboard keys needed).
