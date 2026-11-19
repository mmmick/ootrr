# Code review: Difficulty settings, 18 Nov 2026

Scope: `playtest/difficulty-all` (HEAD `5eadac3`) against upstream `a32352f`. Read only: no build, no tests run, no game launched. Paths are relative to `crates/core/src/`.

## Status of each finding (review-fixes branch, 18 Nov)

Commits are on `feature/review-fixes` in `wt
eview-fixes`, from `playtest/difficulty-all`. Nothing is confirmed in the game.

| # | Finding | Status |
|---|---|---|
| 1 | Damage colour module never returns (High) | Fixed (a581e86): regen, fairy and backup now add their nodes to the caller graphs; both module graphs stay byte-identical in all 8 combinations. Whether the tint is gone needs the game |
| 2 | Gate edits a shared logic graph | Written up for v4, not fixed in the old code (docseatures-enemies-block-the-way.md). Same problem confirmed in `feature/gates-v3` |
| 3 | No ammo drops with shuffled shops | Fixed in part (8ccc3c0): the note and the tip warn when Shopsanity is on. Options for a guarantee in suggested-improvements E1 |
| 4 | Forest gate assumes a weapon | Written up for v4 (still present in `feature/gates-v3`), widened to every guarded gate |
| 5 | Battle music edit not limited | Checked on the game files: exactly five packs change (Leever, Leever_Big, Leever_Fixed, Moblin, Moblin_Club). Test added (c42b831). No restriction needed |
| 6 | Backup eligibility key in some copies only | Fixed (a581e86): all 157 copies of the table get the keys; bosses are excluded by graph. Two actors that share a graph remain (see feature 17) |
| 7 | Size table grown once per pack | Skipped: needs a change in upstream `staging.rs`; suggested-improvements E2 |
| 8 | Later steps overwrite a difficulty edit | Skipped: no overlap today, build-level check is a design change; E3 |
| 9 | Gate settings read fail-open | Fixed (8ccc3c0): errors go to `bad` and skip the gate; test added |
| 10 | Lose-heart boot gate replaces the vanilla chain | Skipped: changes how the flow ends and cannot be checked without the game (a flow that ends wrongly freezes Link) |
| 11 | Health capped at 99 | Fixed (289a770): the tip says so |
| 12 | Damage rounding across parent and child | Skipped: behaviour change; E5 |
| 13 | Encounters note describes the knight mode that is off | Note fixed (3eea92c). Dead branch `SeatedUntilNear` kept (tests use it); E6 |
| 14 | Build cost of the pack passes | Skipped: needs a single combined pass; the new hook module is a first step |
| (a) | Red enemies | See finding 1 |
| (b) | Dodongo black screen | Written up (docseatures	-boss-minibosses.md) with the experiment |
| (c) | Lose heart persistence | Written up (docseatures-lose-heart-on-death.md); the fix needs a wait step and the game, not built |
| Clean-up | Duplicated helpers in regen, fairy, backup | Done: `difficulty/hook.rs` |
| Clean-up | `keep` parameter; not-called-outside-tests functions; README | Done (3eea92c): `lose_heart_with` lost the parameter, `minibosses::stage` deleted, `lottery::boosts` and `actor_of` and `encounters::around` moved into their tests, README updated. `gates::pairs_of` left (the old gates code is being replaced) |
| Clean-up | `qol::source` and nine other copies of the "read through have" helper; four fresh-hash functions; `NODE` and friends duplicated in minibosses and gates; frame counts; fairy flag line | Skipped: wide mechanical change, E6 and E7 |
| Clean-up | Missing tests | battle_bgm set and the broken gate setting read: added. Build-level check for a path staged twice: skipped (E3) |

## Summary

The code is careful and consistent with upstream style: no `unwrap`, `expect`, panics or comments outside tests, every local is annotated, every edit reads back what it wrote, and missing names fail with a clear "the game files differ" error instead of doing nothing. Settings default off, `worked` returns early when nothing is on, and the edits are deterministic per seed. The weak point is not the Rust but untested assumptions about how the game runs the data: the shared enemy graph hook (regen, fairy, backup), the gate's shared logic graph, and same-path files that differ between packs. Maintainability debt is concentrated in three near-identical modules (`regen.rs`, `fairy.rs`, `backup.rs`) and a read-through helper written about ten times.

Counts: 1 High, 5 Medium, 8 Low, plus a clean-up list.

CONFIRMED = traced in the code. PLAUSIBLE = follows from the code but needs a test or a look in the game.

## Findings

### 1. The graph hook keeps the damage colour module running for ever
- Severity: High. PLAUSIBLE (the red tint itself was seen by the user).
- Where: `difficulty/regen.rs:88-89`, `difficulty/fairy.rs:112-116`, `difficulty/backup.rs:192-193`.
- What: each feature makes a Parallel (`EndPolicy` 0, `ResultPolicy` 0) the new root of `EnemyCommon.common.reactiondamagecolor.module`, with the native colour node as first child and a second child that never finishes (a looping timer for regen, a switch resting on `ExecuteGenericDummyAction` for fairy and backup). In the vanilla file the colour node is the whole module, so the module ends when that node ends and the caller leaves it. After the edit the module can never end. If the node clears the tint when it is left, it is never left.
- Failure: any of the three settings on, hit a Deku Baba once: it turns red and stays red. A second consequence if the module is only entered on a damage reaction (the name suggests so): regen, fairy heal and the backup timer do not run at all until the enemy has been hit once.
- Fix: check in the game whether enemies are red before the first hit or only after it. If only after, stop using this module as the host: wrap the root of each caller graph (the set `callers()` and `runs()` already find) or another graph that lives as long as the actor. The test added in `005c093` only proves the node's bytes are unchanged, which this reading does not dispute.

### 2. The gate adds nodes to a logic graph that other maps use
- Severity: Medium. CONFIRMED in code, effect PLAUSIBLE.
- Where: `difficulty/gates.rs:14-15`, `gates.rs:277-283`; compare the guard in `difficulty/minibosses.rs:149-151`.
- What: `gates::stage` writes four nodes per guard into `Logic/Gimmick_AllDay_Skulltula_115c.logic.module.ainb`, a vanilla graph bound by other `AiGroups`. Those groups now run eight nodes whose `InstanceName` is in none of their references, and the new Kokiri group runs the vanilla Gold Skulltula node unbound. `minibosses.rs` refuses exactly this ("a new node would break the other room"). Only Kokiri Forest was load-tested.
- Failure: `enemy_gates` on, enter any other map whose group uses this module: unknown behaviour, from nothing to a load failure like the Dodongo one.
- Fix: write the gate nodes into a new graph file with its own name and size table entry (as backup does for its holder files), or load-test every map that references `Gimmick_AllDay_Skulltula_115c` and list them in the feature file.

### 3. No ammo drops can leave a seed without any ammo source
- Severity: Medium. PLAUSIBLE.
- Where: `difficulty/drops.rs:44-62`, `difficulty/lottery.rs:164-178`, tip in `settings/list.rs` (`no_ammo_drops`).
- What: every random source of arrows, seeds, bombs, nuts and sticks is zeroed, including the "most depleted resource" help, and logic is unchanged. The tip tells the player to buy ammo, but nothing checks that ammo can be bought.
- Failure: `no_ammo_drops` on with shop contents shuffled so that no shop sells arrows: the quiver runs dry and the Ganondorf fight (Light Arrows) cannot be finished.
- Fix: refuse or warn in `read`/`notes` when the setting is combined with shuffled shops, or keep one guaranteed source (for example leave the depleted-resource help for arrows).

### 4. The forest gate assumes everyone who reaches it has a weapon
- Severity: Medium. PLAUSIBLE.
- Where: `difficulty/gates.rs:65-67`, `difficulty.rs:262-263`.
- What: `active` skips the gate only for `open_forest = open` or shuffled entrances. The `kakariko` gate tip in `settings/list.rs:262` says an open Kakariko gate "takes Mido away", and that setting is allowed with `closed_deku`. If the guards' `Presence` flag is still true in that story state, a child can reach the wall without sword or shield.
- Failure: `enemy_gates` on, `open_forest = closed_deku`, Kakariko gate open, logic places an item in the Deku Tree reachable with nothing: the player must first find something that kills two withered Deku Babas, a requirement the seed does not know.
- Fix: also skip the gate when any setting removes Mido, or prove from the files that the guards' `Presence` flag is off in that state and note it in the feature file.

### 5. The battle music edit is not limited to Leevers and Moblins
- Severity: Medium. PLAUSIBLE.
- Where: `difficulty/battle_bgm.rs:15-28` and `56-70`, called for every pack at `difficulty.rs:312-319`.
- What: `edit` gives `BGM_EnemyBattle` to every battle presentation file that is linked from any actor in any pack and has no `BgmName` in its parent chain. Nothing restricts it to `Enemy_` actors and no game-file test lists which files change. The setting text names only Leevers and Moblins.
- Failure: `no_bottles_in_combat` on: any other actor with a silent presentation (a boss part, a business scrub, an object with a hit reaction) starts the battle theme near Link, and bottles are refused there.
- Fix: add a game-file test that asserts the exact set of edited `(pack, file)` pairs, and restrict `linked` to actor names starting with `Enemy_` if the set is wider than intended.

### 6. Backup's eligibility key is in some copies of a shared file and not in others
- Severity: Medium. CONFIRMED in code, effect PLAUSIBLE.
- Where: `difficulty/backup.rs:413-414`.
- What: `OotrrBackupEligible` and `OotrrBackupUsed` are added to `Component/Blackboard/BlackboardParamTable/EnemyCommon...bgyml` only in packs of actors that may call. Boss and non-`Enemy_` packs keep the vanilla copy under the same path. If the game keeps one copy per path, load order decides which one every actor gets.
- Failure: `enemy_backup` on in a room with a boss and ordinary enemies. Boss copy wins: nobody is eligible and the feature silently does nothing. Enemy copy wins: the boss reads Eligible true and runs the holder queries on a `Backup` key it does not have.
- Fix: put the two keys in every copy of the table and express the exclusion per actor (for example an override in the actor's own blackboard file, or no `Backup` holder entry plus a check that tolerates a missing key). Same pattern, but harmless either way, in `wake.rs` (`Sit` graph and knight table edited in the Silver pack only).

### 7. The size table entry of a file shared by many packs is grown once per pack
- Severity: Low. CONFIRMED mechanism, effect PLAUSIBLE.
- Where: `staging.rs:57-75` (upstream), driven by `slice/edit.rs:789-800`; `rsizetable.rs:222-234`.
- What: `grow_out` grows the entry of every in-pack file that is larger than vanilla, and `grow` adds the delta plus a 0x400 margin on each call. Backup edits the same `EnemyCommon` table path in about 81 packs and the same actor and holder files in several, so each of those entries is grown once per pack.
- Failure: `enemy_backup` on: the entry for a table of a few kilobytes ends up more than 80 KB too large, reserved each time the file is loaded.
- Fix: in the `late` loop, remember the in-pack names already grown and grow each once.

### 8. Later build steps would overwrite a difficulty edit of the same file
- Severity: Low (latent). CONFIRMED mechanism; no overlap found among the fixed file names today.
- Where: `slice/edit.rs:778-788`, `story_skip.rs:676-680` and `1021-1025`, `deku_bridge.rs:52-56`.
- What: `story_skip::worked`, `worked_endings` and `deku_bridge::worked` run after `difficulty::worked` but read only the work folder on disk, not `late`. Their output is appended with `late.extend`, and the write loop lets the last entry for a path win.
- Failure: a future difficulty edit of a flow that story skip also edits, or a difficulty hit inside one of the two Deku Tree floor packs (all actor packs are scanned when `packs()` is true): the difficulty edit is lost with no error.
- Fix: let those steps read through `late` as `difficulty::worked` does, or fail the build when `late` holds one path twice.

### 9. Gate settings are read fail-open
- Severity: Low. CONFIRMED.
- Where: `difficulty.rs:262-263`.
- What: `s.text(FOREST)` and `s.text(ENTRANCES)` use `unwrap_or(false)`, unlike every other read in the function, which reports to `bad`. An error means "forest closed, entrances vanilla", the case where the gate is placed.
- Failure: either setting renamed upstream: the gate is built under an open forest or shuffled entrances.
- Fix: push the error to `bad`, or treat an error as "skip the gate".

### 10. The lose-heart boot gate replaces the vanilla boot chain after a retry
- Severity: Low. PLAUSIBLE.
- Where: `difficulty/flows.rs:205-236`.
- What: when `Sequence_LatestRetryType` is Normal the flow runs a copy of the first event, the heart ladder, and ends. The rest of the vanilla `ShieldHylianNaviHintBootEvent` chain is skipped, although `AfterLoadSaveData` may have reserved this event for its own purpose during the same reload.
- Failure: child with a Hylian Shield dies and retries: the Navi hint that was due is not shown that time.
- Fix: end the ladder at the original second event instead of `NONE` (the ladder runs first, then the vanilla chain continues).

### 11. Health above 5x of 20 is capped at 99
- Severity: Low. CONFIRMED in code, reach depends on data.
- Where: `difficulty/stats.rs:31-37`.
- What: `scaled_life` clamps to `SCRIPTED_LIFE - 1`. Any enemy or boss with 20 or more health gets less than the chosen factor at 5x (24 becomes 99, not 120), and the tip promises five times.
- Fix: say so in the tip, or lift the clamp for values that started below 100 if the game has no meaning for 100 and above beyond the scripted ones.

### 12. Damage rounding goes wrong across parent and child files
- Severity: Low. PLAUSIBLE.
- Where: `difficulty/stats.rs:70-95`.
- What: the rounding remainder is put into the `DamageMultiplier` values of the same file. A child file that sets `BaseAttackPower` but inherits `AttackParams` (or the reverse) gets its own rounded base with the parent's remainder. Attack entries with no `DamageMultiplier` key lose the remainder.
- Failure: 1.5x, parent base 2 (exact 3) with the multipliers, child base 1 (rounded to 2): the child deals 2x instead of 1.5x. At 0.5x a base of 1 with no multiplier key stays 1.
- Fix: resolve the parent chain inside the pack before scaling (as `battle_bgm::silent` does), or accept and document it.

### 13. The encounters note describes the knight mode that is switched off
- Severity: Low. CONFIRMED.
- Where: `difficulty/encounters.rs:20` against `difficulty/wake.rs:31`.
- What: `NOTE` says the Iron Knuckle "gets up when you come within about 9 units"; `MODE` is `Wake::Awake`, so he is up from the start. The `SeatedUntilNear` branch (`wake.rs:49-72`) is now dead code.
- Fix: correct the note; delete the branch or make the mode a real setting.

### 14. Build cost of the pack passes
- Severity: Low. CONFIRMED.
- Where: `difficulty.rs:363-370` and `388-396`; `regen.rs:207-228`, `fairy.rs:234-255`, `backup.rs:492-525`.
- What: with the pack settings plus regen, fairy and backup, every actor pack is decompressed and parsed in `pack_out`, once each in regen and fairy, twice in backup, and twice more in `grow_out`. `step`'s `seen` closure searches `outs` linearly and clones the whole compressed file on each hit.
- Fix: one pass that opens each pack once and applies all graph edits (falls out of the clean-up below), and a map from path to index for `outs`.

## Open problems

**(a) Red enemies.** See finding 1. My reading is the wrapper, not the heal: the module used to be one short-lived node and now never returns, so the colour node is never left (tint sticks) or is restarted by the Parallel while its sibling stays busy (tint re-triggers). Backup-only and fairy-only builds should show red too. The decisive observation is whether an enemy is red before it is first hit. The fix is to host the new nodes in a graph that lives for the actor's life, not to adjust the Parallel's policies.

**(b) Dodongo black screen.** A hang with no crash line fits an actor that never becomes ready, not a bad file. Two things differ from the four spots that load. First, the kind: the cavern only ever uses `Enemy_MiddleBoss_Lizalfos`, placed with `Links`, a `Name` and a battle-start key, and plain `Enemy_Lizalfos` exists in one vanilla map; it probably waits for a partner or arena link that our bare entry (room index only) does not give, while a Stalfos needs nothing. Second, the spot: it was about 34 units from the boss's own position, against 5 to 9 units for the four working spots, so it may lie outside the boss room whose index it carries. Variant D in the feature file (a Keese at the same spot) separates the two; also copy the full vanilla entry of the Spirit Temple's plain Lizalfos and compare its keys. Simplest way out: use a kind that already loads with a bare entry (Stalfos or Dinolfos) at a spot checked against the room.

**(c) Lose heart after quitting.** The in-session design is sound (traced: the stale counter is always either overwritten or already correct). The loss can only persist if the cut value reaches a real save, and the only test quit straight after the respawn. Check first whether an ordinary later save (walk through a loading zone, or save by hand) already keeps the cut: the second death reads the cut value from game data, so the player actor does sync it. The autosave variant most likely failed because `AfterChangeBanc` had just autosaved or the request came while the boot event was still running. If a later save does keep it, the remaining hole is "die, quit at once"; to close it with data only, write `PlayerStatus.MaxLife` with `EventTriggerSetGameDataInt` next to the player action (both then agree), wait some frames, then request the autosave. If no save keeps it, the fallback is the revive-in-place design already ranked third in the feature file. `Sequence_RequestLastGameOverSave` was tried and did not help.

## Areas that are fine
- Flow switches: boolean queries use case 0 for true and int queries 0/1/2 throughout (`flows.rs`, `dupes.rs`, `bean.rs`).
- Composition on the shared enemy graph: regen, then fairy, then backup, each wrapping the previous root, heap growth adds up; order is fixed in `stage` and tested.
- Drops against the lottery: both are edited; shared lottery rows are skipped on purpose.
- More enemies: deterministic per seed and file, hashes checked against every map, enemies referenced by any group or link are never copied, later hand-placed actors are added after it.
- Default output: nothing is staged with all settings off; the settings string test covers the defaults.
- Error handling: no swallowed failures apart from finding 9.

## Clean-up list
- `regen.rs`, `fairy.rs` and `backup.rs` each carry the same `switch`, `flag`, `has`, `calls`, `is_caller`, `callers`, `graph_of`, `names`, `runs`, `actor_of`, `heap_of`, `grow_heap`, `zipped`, `visit` and the same constants (about 110 lines, three times). Move them to one shared module; `wake.rs` already imports from `backup.rs`.
- The "read through `have`, else the game file" helper exists as `qol::source`, `regen::zipped`, `fairy::zipped`, `backup::zipped`, `extras::raw`, `ambush::source`, `encounters::source`, `gates::source` and inline in `difficulty.rs`, `minibosses.rs`, `minions.rs`, `wake.rs`.
- Four fresh-hash functions (`extras`, `minibosses`, `minions`, `ambush`) with different uniqueness scopes; `minibosses::fresh` checks only the static file.
- `NODE`, `NAME`, `CREATE`, `DELETE`, `RESPAWN` and `named` are duplicated between `minibosses.rs` and `gates.rs`.
- Lose-heart leftovers: `Keep::CountSave`, `AUTOSAVE` and its note text describe the autosave variant that failed in the game; the `keep` parameter of `lose_heart_with` (`flows.rs:183`) is unused and will raise a compiler warning.
- Not called outside tests: `minibosses::stage`, `gates::pairs_of`, `lottery::boosts`, `lottery::actor_of`, `encounters::around`.
- `README.md` still describes the first lose-heart design ("takes four quarter hearts ... when you pick Retry") and lists only the first eight settings.
- Frame counts assume 30 per second (`regen::FRAMES`, `fairy::FRAMES`, `backup::FRAMES`); unverified, one timing in the game settles all three.
- The fairy flag (`MiniGame_Diving_Test`) stays set when no enemy is awake to clear it, so the next enemies loaded get the heal; harmless, worth a line in the feature file.
- Missing tests: the exact file set of `battle_bgm` on the game files (finding 5); a build-level check that no path is staged twice across `late` (finding 8); `read` with a broken forest or entrance setting (finding 9).
