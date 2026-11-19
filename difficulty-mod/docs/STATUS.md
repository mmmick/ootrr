# Difficulty features: status board

Last updated: 19 Nov. rc 14 cut and waiting for review; Longshot contribution measurements next.

Goal set by the user: every requested feature is either **Done** (seen working in the game, polished) or **Blocked** (investigated, with a written reason and the question for the user). Nothing stays at "built, never played".

## Right now

| What | State |
|---|---|
| **Ready for the user to review and push** | `difficulty` in `ootrr\` (the IDE's checkout): **rc 13** (ebd28ef) and **rc 14** (build `builds_2026-11-19_rc14`), 2 commits ahead of the fork. rc 14: more enemies slider to 10x, backup "bring a friend", upstream 0.7.3 |
| **Has the game** | Nobody |
| **Next** | Longshot contribution: in-game measurements for the three branches in `<Longshot checkout>-pr` (caption, warp, record/replay); then the user's Discord sign-in and push. Then: Gerudo fighter as a placed enemy; Volvagia and other bosses outside their rooms |
| **For the user to review** | `enemy-table.md`, `enemy-speed.md`, `capture-playbook.md` (hands-on items: Gohma's door, Zora cave, Dodongo guard fight, spawn combination on a real controller, a room-clear door at 5x, safe retry in Kokiri Forest) |
| **Waiting on the user** | Longshot: `python -I <local path>orge.py login` (Discord), and a hello in their `#dev-work` |
| **Layout** | Branches: `main`, `difficulty` (release), `playtest/next` (collects finished features), `test/gates-all`. Merged feature branches and worktrees are deleted |

## Features

States: **Done** = seen working in the game. **Works, finishing** = seen working, polish or follow-up checks under way. **Built, untested** = never seen in the game. **Half tested** = part seen. **Broken** = seen failing, being fixed. **Not possible** = judged not doable with data edits.

| # | Feature | State | What was seen in the game | Next step |
|---|---|---|---|---|
| 01 | Enemy health | Done | Scaling seen (19 Nov): at enemy_health 200 the overlay shows Stalfos 36/36 and Lizalfos 18/18, exactly double vanilla 18 and 9 | Peahat larva not read |
| 02 | Enemy damage | Done | 3x: a bite took 6 quarter hearts instead of 2 (script, rc 5) | Grabs (ReDead, Like Like) not covered; parked |
| 03 | Enemy speed | Built, untested | Nothing (batch 1 aborted before this) | Batch 1 |
| 04 | No heart drops | Done (small sample) | User (18 Nov): about 20 grass cut in the Lost Woods, only rupees and bugs dropped | Hearts the map places on the ground are left alone (user: fine unless they respawn; to check from the files) |
| 05 | No ammo drops | Done (small sample) | Same run as 04: no ammo dropped | Pots and enemy kills not sampled yet. With Shopsanity on, the note now warns that a special deal can replace an ammo shelf (review finding 3) |
| 06 | No gossip fairies | Done | User (18 Nov): none of the six songs produced a fairy at a gossip stone | None |
| 07 | Lose heart on death | Done, with one limit | 6 → 5 → 4 → 3 hearts over three deaths; a 4th death stays at 3; no side effects. The final build was replayed: 6 → 5 → 4 (script, 18 Nov). Confirmed by hand too (user died without a fairy: back with 5 hearts). Merged into `playtest/difficulty-all` | Limit: the loss does not survive quitting and continuing (accepted as "simplest for now"). Fairy revival costs no heart: confirmed by the user (revived at 6 hearts max) |
| 08 | No bottles in combat | Done | Confirmed by the user at the controls (18 Nov): a bottled fairy could NOT be used while the Iron Knuckle was aggro and COULD once far enough away; the refusal animation is right; a bottled fairy still revived Link automatically in the fight (6 hearts kept). Found and fixed on the way: the block ignored miniboss music (commit 630870b) | Not seen against a full boss; low risk |
| 09 | Enemies regen health | Built, waiting for a playtest | Old timer seen (19 Nov): +1 hp every 2 s, too fast. Now 10 s without a hit, then +1 hp every 5 s (branch feature/regen-wait, playtest/regen-wait) rc13 (19 Nov, merged build, script): hit a Stalfos; +1 hp came 15.2 s and 15.0 s after the hit, then every 4.9 to 5.4 s. Matches 15 s then 5 s. Hit source unidentified (sword swings did no damage). | Not seen yet: the hit flag resetting the wait fx-r1 (19 Nov): effect for regen ticks investigated, not built (`OneShotXLinkSearchAndEmit` found; no shared heal key yet). |
| 10 | More enemies (slider 1x to 10x) | Half tested (19 Nov, 10x) | Old 2x maps loaded (rc 4). New 19 Nov (`feature/horde` a6e2ce3, exe `<playtest folder>\horde\ootrr.exe`): slider 1x to 10x; 1.5x and 2x unchanged; above 2x enemies are placed on the game's own navigation mesh by ground class (flat, gentle slope, any, flier, water, ceiling, sand, miniboss), new kinds, groups and minibosses from 5x; caps 240 per map and 120 within 60 units, 12 within 15 units; safe zones around every arrival point (none within 15, at most 2 slow kinds within 30, a 120 degree way out); the multiplier is a ceiling per room or ground patch, limited by usable floor. 1,802 added at 10x over 39 maps (copies included). Second game test: all added enemies present with a wide scan, 59 to 60 fps at 10x with one game copy. Game test 19 Nov: 37 of 38 maps load at 10x; fix round done (Freezard and Guay dropped, child Wasteland dropped, caps lowered); Second test 19 Nov (exe 66bb967 build): 10x frame rate 59 to 60 at every spot tried (control 60); the "missing" enemies were mostly the `enemies` scan window (wider scan: Gerudo Valley adult 35 live instead of 6 to 8, 10x minus 1x live counts match planned); Hyrule Field child death reload with no damage; room-clear doors and Kokiri reload not tested. Played 19 Nov (`<playtest folder>\horde\RESULTS.md`): 37 of 38 edited maps load at 10x, Dodongo included; nothing crashed; slider rises 5x to 10x where enemies appear; frame rate 28 at the busiest spots (child Field, Dodongo; control 60); planned enemies missing in Gerudo Valley (0 of 106), Water Temple, Field adult, Lost Woods adult, Fire Temple; some Freezards fall; Try Again test passed at the Dodongo entrance only; door logic and fights not tested; Third test 19 Nov (rc 14 exe, `<playtest folder>\horde\RESULTS.md`): 10x loads in child Kokiri, child and adult Field, Dodongo, Fire Temple, 58.5 to 60.2 fps at every spot (1x control 60), live added enemies +18 to +127 near the spots, nothing vanished or fell except 1 Tektite and 2 Guays at one Field spot each; all difficulty settings on (5x): Kokiri, Field, Dodongo, Deku Tree load and Link moves, 3 minutes in Field with enemies: up 180 s, 46 to 56 fps (average 49.7) | Scripted load test by risk: `<playtest folder>\horde\PLAN.md` (Dodongo's Cavern first, then Hyrule Field at the cap) |
| 11 | Bosses spawn minibosses | Built, in the game once | Stalfos seen in Gohma's room standing there before the introduction (19 Nov); now created as the door-shut scene ends in Gohma's room, the other three rooms unchanged rc13 (19 Nov): Deku Tree loads (not black). Stalfos listed at (16.5, 47.2, 74.0) right after the map loads, before any door scene; whether it is visible or active then is not seen. Door walk NOT TESTED (capture playbook). | Walk through the Gohma door on feature/boss-adds (build in <playtest folder>\boss-adds); Dodongo stays out |
| 12 | Bosses spawn minions | Built, in the game once | Keese seen in Gohma's room before the introduction (19 Nov); Gohma's Keese now appear as the door-shut scene ends, the other rooms unchanged rc13 (19 Nov): the 3 Keese are listed at map load too (20.0/15.7/24.3, 50.5, 62.8/70.3/70.3); door walk NOT TESTED. | Walk through the Gohma door on feature/boss-adds; then Dodongo's babies and the other rooms |
| 13 | No dupe items | Done (bottled fairy) | Confirmed by the user (18 Nov): a second fairy could not be bottled while holding one, and the refusal animation is right | Other duplicate cases (gifts, other bottle contents) not tried |
| 14 | Enemies respawn sooner | Built, untested | Nothing | Batch 2 |
| 15 | No Z-targeting | Done | Confirmed by the user at the controls (18 Nov): enemies cannot be locked on to | None |
| 16 | Fairies help enemies | Seen working | Confirmed in the game: the fairy healed the enemy (19 Nov) | Circling-fairy visual on enemies not possible: no enemy-side effect node (see feature file) fx-r1 (19 Nov): heal effect investigated, not built (same reason). |
eview-fixes, not yet seen in the game) |
| 17 | Enemies call for backup | Option 3 seen in the game by script (19 Nov): no crash on 32 callers; 19 pairs gave the expected helper once (15 to 30 s), 2 gave one helper where two were expected (spear Moblin, Bari), 9 callers called nobody in 75 s (Stalchild, Freezard, Skulltula, Wallmaster, Amos, Spike, Tailpasaran, Biri) , no helper called in turn | Results in `playtestackup-r3\RESULTS.md`; Gibdo, weak Stalfos and white Bubble not spawnable in the field fx-r1 (19 Nov, 91c3d0a, code only): never calls its own kind, a weaker one; helper appears at the caller, raised 0.5; no Dodongo kinds as helpers; flying pot and Shabom dropped (no backup keys).
eview-fixes, not yet seen in the game) |
| 18 | Zora's Domain ambush | Seen, fix built | Seen in game: wall Skulltulas great, droppers nearly right but triggered underneath. Fixed in polish-r5 (trigger box at the back, droppers 1.25 nearer the middle) rc13 (19 Nov): Zora's Domain loads. The three hanging Skulltulas are already listed at map load (before any trigger), so the trigger is not confirmed; cave walk NOT TESTED. | Retest: nothing drops until you reach the back of a side cave |
| 19 | Bean Seller's pitch | Done | User (18 Nov): the dialogue works | None |
| 20 | Enemies block the way | Round 4: gates 2, 9, 11, 12 ship; 7 held | Gates 2 and 9 seen working (gate 9: "lost woods 1 guard 2 gates confirmed"; gate 2 "worked perfect"). Gate 2 guard is now a created Enemy_Dodongo (not played yet). Only gate 7 stays held. Build in playtest	weaks-r4 rc13 (19 Nov): gate 2 guard seen: Death Mountain Trail loads, an Enemy_Dodongo (hp 6) stands at (-54.4, 106.4, 3.2) after warping to the guard's side. | Play the Dodongo guard at gate 2; try gate 7 (Lost Woods east passage); gate 11 from the south |
| 21 | Volvagia over Hyrule Field | Not possible | Not built | Write-up in `features\21-volvagia-roams-hyrule-field.md` |
| 22 | Special encounters (one checkbox): Dead Hand in hole 9 (Hyrule Field) and hole 23 (Death Mountain Trail), Iron Knuckle in the open part of Hyrule Field | Done, two tweaks wanted | Lake Hylia Like Likes confirmed in the game (19 Nov); the one by the lab removed because of the steep slope, nine remain. Dead Hand and Iron Knuckle confirmed earlier. Build in playtest	weaks-r4 Guays (24, child Hyrule Field only) added on `feature/guays`, build in <playtest folder>\guays, not seen in the game. rc13 (19 Nov): Guays over the field: 8 in load range, flying, none vanished or fell, but equally active by day as by night (night-only not seen). | Tweaks as before |
| 23 | Grass and pot surprises (part of special encounters): 1 in 100 that cut grass or a broken pot spawns a Skulltula (grass) or Poe (pot), or a small crawling hand (both; branch `feature/surprise-hands`, exe `<playtest folder>\hands\ootrr.exe`, test hook env `OOTRR_SURPRISE_CHANCE` 1 to 20), and 1 in 10 plain pots are flying pots | Built, untested (code only) | Nothing. Commit `163112b` on `feature/surprises`; 138 difficulty and 107 settings tests pass | Play it: cut about 300 blades and break about 200 pots; look for fake pots in Zora's Domain and Kakariko. Riskiest: whether a drop can create an enemy. Steps in `<playtest folder>\surprises\TESTS.md`. Round 2 (`feature/enc-r2`): rate now 2 percent per surprise (1 in 50), the bug grass round the hole 9 Dead Hand (`Land_kokiri_Plant_A_02_DropFixed_03`) now gets the grass surprises; Market guard house pots are fixed rupee pots and stay as they are |
| 24 | Cucco revenge on the first hit (setting 131, `cucco_revenge`) | Built, untested (code only) | Nothing. Commit `f9b7c87` on `feature/surprises`. The hit threshold `StartSpecialAttackCount` goes 4 to 1 in the Cucco hit graphs; Anju's and the find-out Cuccos are left alone | Play it: one sword hit on an adult Kakariko Cucco; carry and throw one. If one hit is not enough, set `FIRST_HIT` to 0. Steps in `<playtest folder>\surprises\TESTS.md` |
| 25 | Spawn enemies with L + R + D-pad (setting 132, `enemy_spawn_combo`): Down spawns a random ordinary enemy in front of Link, Up spawns the same again | Fixed, scripted test passed | Opus fix (19 Nov, `feature/opus-fixes` 4efaaaf): every create used holder slot 0, so only the first press worked; now a slot counter. Bridge L+R+Down: 12 presses, 12 different enemies 3 m ahead (Up repeated the kind), the 13th and 14th nothing (cap 12). The user probably saw nothing because Longshot's own L+R+D-pad tool shortcut takes the D-pad rc13 (19 Nov, merged build, script): 6 combo presses gave 5 to 10 new enemies with the combo alone, with every other setting group on, and with all other settings on; could not reproduce the failure. | User: real pad with the Longshot shortcut off; check that a kill frees a slot fx-r1 (19 Nov, 91c3d0a, code only): combo is now both stick clicks (L3 + R3) + D-pad, user decision. |
| 26 | Small hands and floor Skulltulas from grass and pots (new request, saved for later) | Built in code, untested (see 23) | Requested 18 Nov: the small crawling hands (the Floormaster's little ones) and the floor Skulltulas sometimes appear instead of bugs or rupees when grass is cut or a pot is broken. Extends the grass and pot surprises (23) | After the current build is polished and out |
| 27 | "No ammo drops" becomes "Limited ammo" (change request, saved for later) | Not started | Requested 18 Nov: ammo still drops in boss fights and anywhere else the player cannot flee and would otherwise have no option but to die (the user thinks that is only bosses). Also answers the review's finding that no ammo drops plus shuffled shops can leave no arrow source | After the current build is polished and out |

## Queue for the game (one copy at a time)

Batch 1 (18 Nov) was stopped early with nothing confirmed: scripts cannot sword-fight, aim, or work the pause menu reliably. So testing is split by what each side is good at.

**Scripted (stand still, warp, read numbers, take pictures):**

1. **S1, running:** forest gate v2 (20), Iron Knuckle awake build (22), enemy speed (03), call for backup (17).
2. **S2:** boss minibosses and minions (11, 12) with the Dodongo variants, Zora ambush (18), more enemies pictures (10).
3. **S3:** respawn sooner (14), fairies help enemies (16) if a stand-still check exists; else to hands-on.
4. **S4, horde (10):** load test every edited map at 10x with `<playtest folder>\horde\ootrr.exe`, by risk, per `<playtest folder>\horde\PLAN.md`: Dodongo's Cavern, Hyrule Field (240 enemies), child Kokiri Forest, the dungeons, then the rest. Read `enemies` for 30 s per map; compare with `placements-10x.csv`.

**Hands-on with the user (script sets up and logs, the user plays; advance by button press):** no heart drops (04), no ammo drops (05), no gossip fairies (06), no bottles in combat and the fairy revival (08), enemy regen (09), no dupe items (13), bean pitch (19), gate: kill the guards (20), Dead Hand in hole 9 (22). The list with setup and what to look for is `capture-playbook.md`.

Then: fix what failed, replay only those, merge the passing branches and cut rc 9.

## Done today besides features

- **rc 12 cut (19 Nov):** commit `4d772af` on `difficulty`, build in `builds_2026-11-19_rc12`. Local only. Backup and the spawn combination fixed (Opus), regen and the health multiplier seen working by the numbers, Like Likes present, "red guards" shown to be the Deku Baba's normal colour.
- **rc 11 cut (19 Nov):** `difficulty` now has upstream merged (commit `a3550be`, 32 upstream commits, new `randomizer/` layout) and rc 11 on top (`c8d4b34`); build in `builds_2026-11-19_rc11`. Local only: rc 10 is on the user's fork, rc 11 waits for review and push. All docs are in `difficulty-mod/docs` (34 files, scanned for personal details).
- **Longshot debug panel shows enemy health** (`hp CUR / MAX`), seen working: a bomb took a Stalfos from 18 to 16.
- **rc 10 cut (18 Nov late):** commit `ecbdb04` on `difficulty`, build in `builds
_2026-11-18_rc10` with CHANGELOG.md and TODO.md. Local only: waiting for the user's review and push. Seven gates in, five held back; what does not work is in `docs\TODO.md`.
- **rc 11 (18 Nov evening):** `builds	_2026-11-18_rc9-spawn-combo`: rc 9 plus the Longshot tools with enemy spawning on L+R+D-pad (Down = random, Up = same again), every press counted. Confirmed by the user. Local only.
- **rc 10 (18 Nov evening):** `builds_2026-11-18_rc9-with-spawn-keys`: the rc 9 randomizer plus a launcher for the source-built Longshot with the spawn keys (F4 random enemy, F3 same again, F9 screenshot). The spawn keys are a Longshot tool, not part of the mod. The launcher itself is not tried yet.
- **rc 9 cut (18 Nov evening):** commit `e303654` on `difficulty`, build in `builds 9_2026-11-18_rc9`. Local only, not pushed. From now on a release candidate is cut each time a major feature finishes testing.
- Release branch `difficulty`: one commit per release candidate, rc 1 to rc 8, docs in `difficulty-mod/docs/`, nothing pushed. Builds 007 and 008 are in `builds\`.
- Longshot test tools (local patch, `<local path>`): warp to coordinates, on-screen captions, give item, input recorder. Recording real input is not yet tried.
- Hole catalogue: 28 of 30 holes pictured from inside, `<playtest folder>\holes\index.html`. Holes 27 and 29 (Gerudo) not reached.
- Capture playbook for things that need the user's hands: `capture-playbook.md`.

## Not yet merged

These branches hold work that is not in `playtest/difficulty-all` or on `difficulty` yet. They are merged when their test passes.

| Branch | What | Waiting for |
|---|---|---|
| `feature/heart-final` (from `heart-v5b`) | Lose heart on death, new design | The final test now running |
| `feature/dead-hand-holes` | Special encounters (feature 22) | Its load test |
| `feature/dodongo-fix` | Nothing yet (investigation only) | Batch 3 variants |

## Open questions for the user (none blocks the work)

- Holes 27 and 29 could not be photographed. Not needed unless you want them.
- Volvagia over Hyrule Field: judged not possible. Read the write-up and say if you want a different take on the idea.
