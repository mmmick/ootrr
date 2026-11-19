# Enemy speed survey (Enemy speed setting, 1x against 2x)

Measured 2026-11-19 with the local Longshot build (origin/main 10fab15 plus our test patch) and the randomizer at `wt\next` (branch `playtest/next`; enemy speed setting 100 and 200). Numbers are what was measured; "uncertain" marks what the method cannot separate.

## How it works

- **Two test mods**, same preset apart from `enemy_speed` (100 and 200): `skip_intro`, `skip_cutscenes` "all", `faster_chests`, `open_forest` "open", `starting_hearts` 20. Presets: `<playtest folder>\speed\p100.json`, `p200.json`; build with `<playtest folder>\speed\mk.sh 100|200` from Git Bash. Only one mod is in the shadow game's `mods` folder at a time (`<playtest folder>\speed\game`, a shadow of the Longshot source build; saves in `<playtest folder>\speed\userv2_*`).
- **Positions come from the new bridge command `enemies`** (our Longshot source build only; see `docs\TESTING.md`): every live enemy actor with name, id (the actor's address), x y z, hp, max hp, the host clock `t_ms` taken when the game read them, and Link's position. The survey keeps one bridge connection and asks about 10 times a second (`scan` false after the first call).
- **Scenario per trial**: teleport into Hyrule Field (`HyruleField_Child`; this reloads the map and drops the previous enemy), `warp 123.68 159.5 159.04`, `health 20`, `spawn Enemy_NAME` 8 units in front of Link (a spawn 15 units away once fell through the floor), Link stands still (no input), sample for 10 s, `health 20` again every 3 s so he survives. Three trials per enemy per multiplier, 43 enemies. Every step is captioned on the game window from the first second.
- **Speed** is the horizontal distance (x and z only) between two samples at least 0.5 s apart divided by their `t_ms` difference, in the game's position units per second. "Typical" = median of the windows that moved at least 2 u/s; "peak" = the largest window. Ratio = typical at 2x over typical at 1x (medians over the trials). "Approached" = the enemy ended at least 3 units closer to Link than it started.
- **Verdict**: ratio 1.7 or more = follows the setting; 1.25 to 1.7 = partly; under 1.25 = ignores it; never over 2 u/s = does not move; "unclear" = it moved at one multiplier only. These cut-offs are our choice.
- **Close pass**: enemies that stood still at 8 units were run again 3 units from Link, 2 trials each (not 3, to save time), at both multipliers. Where that changed the verdict the note says so.
- **Limits**: an enemy that has not noticed Link does not chase, so "does not move" may mean "was not provoked", not "has no speed". Enemies that hop, glide, lunge or wait (Tektite, Peahat, Poe, Bubble, Floormaster) are paced by their own move cycle, so one typical figure blends moving and pausing; read the peak too. Spawned actors may not behave like ones placed by the map. Only movement was measured, not attack rate. Only the field map was used.

## Rerun

Build the two mods (`mk.sh 100`, `mk.sh 200`), keep only the one under test in `<playtest folder>\speed\game\mods` (the other goes to `mods_hold`), check that no `longshot.exe` is running, then from Git Bash in `<playtest folder>\speed`:

```
python -I survey.py run 100 --dur 10
python -I survey.py run 200 --dur 10          (after swapping the mod folder)
python -I survey.py run 200 --dist 3 --trials 2 --enemies Keese,Biri     (optional close pass, likewise at 100)
python -I survey.py report
```

`report` writes `docs\enemy-speed.csv` and `<playtest folder>\speed\table.md`. Results are appended to `<playtest folder>\speed\samples.jsonl`; finished trials are skipped on a rerun. `docs\enemy-speed.csv` holds every raw sample (multiplier, enemy, trial, time in s, x, y, z, hp, max hp, Link x and z, frame; the spawn-distance of a trial is not in it: rows with 2 trials per enemy are the 3-unit close pass).

## Results (units per second; median over the trials, range of the typical figure in brackets)

| Enemy | 1x typical / peak (u/s) | 2x typical / peak (u/s) | Ratio (typical) | Verdict | Note |
|---|---|---|---|---|---|
| Amos | 0.0 (0.0 to 0.0) / 0.0 | 0.0 (0.0 to 0.0) / 0.0 | - | does not move | stood still at 8 and at 3 units from Link |
| BabyDodongo | 2.9 (2.8 to 2.9) / 3.6 | 3.2 (3.2 to 3.3) / 6.0 | 1.11 | ignores it | peak ratio 1.66; typical ratio low but peak ratio 1.66: uncertain |
| Bari | 0.0 (0.0 to 0.0) / 0.0 | 0.0 (0.0 to 0.0) / 0.0 | - | does not move | stood still at 8 and at 3 units from Link |
| Beamos | 0.0 (0.0 to 0.0) / 0.0 | 0.0 (0.0 to 0.0) / 0.0 | - | does not move | no movement over 2 u/s in any trial (may simply not have noticed Link) |
| Biri | 1.5 (0.0 to 3.0) / 2.3 | 3.0 (3.0 to 3.0) / 3.8 | 2.01 | follows the setting | only when spawned 3 units from Link (stood still at 8); trials differ a lot; approached Link in 0/2 (1x) and 0/2 (2x) trials; peak ratio 1.62; it drifted without approaching Link, so this is idle movement, not pursuit: uncertain |
| Bubble_Blue | did not appear | did not appear | - | did not appear |  |
| Bubble_Green | 5.1 (4.1 to 5.3) / 8.1 | 5.0 (4.8 to 5.2) / 8.0 | 0.97 | ignores it | peak ratio 0.99 |
| Bubble_Red | 5.2 (0.0 to 5.5) / 8.0 | 5.6 (5.5 to 5.9) / 8.0 | 1.09 | ignores it | trials differ a lot; approached Link in 2/3 (1x) and 3/3 (2x) trials; peak ratio 1.00 |
| Dekubaba | 0.0 (0.0 to 0.0) / 0.0 | 0.0 (0.0 to 0.0) / 0.0 | - | does not move | stood still at 8 and at 3 units from Link |
| Dinolfos | 5.8 (5.7 to 6.3) / 7.1 | 7.6 (7.2 to 8.1) / 11.8 | 1.32 | partly | peak ratio 1.66 |
| Dodongo | 0.0 (0.0 to 0.0) / 1.7 | 2.9 (2.9 to 2.9) / 3.1 | - | unclear | moved at 2x only (typical under 1 u/s at the other), so no ratio; at 1x it barely moved (peak 1.7) |
| Floormaster | 5.2 (4.6 to 7.6) / 9.7 | 5.2 (4.4 to 7.9) / 10.2 | 0.99 | ignores it | trials differ a lot; peak ratio 1.06 |
| FlyingPot | 0.0 (0.0 to 0.0) / 0.0 | 0.0 (0.0 to 0.0) / 0.0 | - | does not move | no movement over 2 u/s in any trial (may simply not have noticed Link) |
| Freezard | 4.1 (4.1 to 4.2) / 6.0 | 5.6 (5.5 to 5.7) / 12.0 | 1.36 | partly | peak ratio 1.99 |
| Gibdo | did not appear | did not appear | - | did not appear |  |
| Guay | 3.0 (3.0 to 3.0) / 3.2 | 4.4 (4.3 to 4.4) / 6.1 | 1.48 | partly | peak ratio 1.94 |
| IronKnuckle_Silver | did not appear | did not appear | - | did not appear |  |
| Keese | 0.0 (0.0 to 0.0) / 1.9 | 3.0 (2.8 to 3.0) / 3.8 | - | unclear | moved at 2x only (typical under 1 u/s at the other), so no ratio; at 1x it barely moved (peak 1.9) |
| Keese_Fire | did not appear | did not appear | - | did not appear |  |
| Keese_Ice | 0.0 (0.0 to 0.0) / 1.9 | 3.0 (2.8 to 3.0) / 3.8 | - | unclear | moved at 2x only (typical under 1 u/s at the other), so no ratio; at 1x it barely moved (peak 1.9) |
| Leever | 0.0 (0.0 to 0.0) / 0.0 | 0.0 (0.0 to 0.0) / 0.0 | - | does not move | stood still at 8 and at 3 units from Link |
| Leever_Big | 0.0 (0.0 to 0.0) / 0.1 | 0.0 (0.0 to 0.0) / 0.1 | - | does not move | stood still at 8 and at 3 units from Link |
| LikeLike | 0.0 (0.0 to 0.0) / 0.6 | 0.0 (0.0 to 0.0) / 1.1 | - | does not move | stood still at 8 and at 3 units from Link |
| Lizalfos | 2.9 (2.9 to 3.0) / 4.6 | 5.1 (4.9 to 5.2) / 8.5 | 1.72 | follows the setting | peak ratio 1.84 |
| Moblin_Club | 0.0 (0.0 to 0.0) / 0.0 | 0.0 (0.0 to 0.0) / 0.0 | - | does not move | stood still at 8 and at 3 units from Link |
| Octorok | 0.0 (0.0 to 0.0) / 0.0 | 0.0 (0.0 to 0.0) / 0.0 | - | does not move | stood still at 8 and at 3 units from Link |
| Peahat | 3.9 (3.8 to 4.0) / 4.1 | 4.0 (3.9 to 4.0) / 4.0 | 1.02 | ignores it | peak ratio 0.97 |
| Poe | 3.4 (3.3 to 3.5) / 3.9 | 3.4 (0.0 to 3.6) / 4.0 | 1.01 | ignores it | trials differ a lot; approached Link in 3/3 (1x) and 2/3 (2x) trials; peak ratio 1.02 |
| Poe_Big | did not appear | did not appear | - | did not appear |  |
| ReDead | 0.0 (0.0 to 0.0) / 0.1 | 0.0 (0.0 to 0.0) / 0.1 | - | does not move | stood still at 8 and at 3 units from Link |
| Scrub_Mad | 0.0 (0.0 to 0.0) / 0.0 | 0.0 (0.0 to 0.0) / 0.0 | - | does not move | no movement over 2 u/s in any trial (may simply not have noticed Link) |
| ShellBlade | 6.0 (5.8 to 6.0) / 6.0 | 5.8 (5.7 to 6.0) / 6.0 | 0.97 | ignores it | peak ratio 1.00 |
| Skulltula | 0.0 (0.0 to 0.0) / 0.0 | 0.0 (0.0 to 0.0) / 0.0 | - | does not move | no movement over 2 u/s in any trial (may simply not have noticed Link) |
| Skullwalltula | 0.0 (0.0 to 0.0) / 0.0 | 0.0 (0.0 to 0.0) / 0.0 | - | does not move | no movement over 2 u/s in any trial (may simply not have noticed Link) |
| Stalchild | 0.0 (0.0 to 0.0) / 0.0 | 0.0 (0.0 to 0.0) / 0.0 | - | does not move | stood still at 8 and at 3 units from Link |
| Stalfos | 3.4 (3.0 to 3.6) / 6.9 | 5.0 (4.6 to 5.2) / 12.3 | 1.48 | partly | peak ratio 1.78 |
| Stinger | 3.7 (3.7 to 3.7) / 11.6 | 2.4 (2.4 to 2.5) / 11.3 | 0.66 | ignores it | only when spawned 3 units from Link (stood still at 8); approached Link in 0/2 (1x) and 0/2 (2x) trials; peak ratio 0.97; it drifted without approaching Link, so this is idle movement, not pursuit: uncertain |
| Tektite_Blue | 3.3 (3.2 to 3.3) / 4.7 | 3.2 (3.2 to 3.3) / 4.6 | 0.97 | ignores it | peak ratio 0.99 |
| Tektite_Red | 3.2 (3.1 to 3.3) / 4.5 | 3.4 (3.3 to 3.5) / 4.7 | 1.04 | ignores it | peak ratio 1.03 |
| TorchSlug | 2.8 (2.7 to 2.9) / 3.9 | 3.0 (2.9 to 3.0) / 4.0 | 1.05 | ignores it | only when spawned 3 units from Link (stood still at 8); approached Link in 0/2 (1x) and 0/2 (2x) trials; peak ratio 1.02; it drifted without approaching Link, so this is idle movement, not pursuit: uncertain |
| Wallmaster | 0.0 (0.0 to 0.0) / 0.1 | 0.0 (0.0 to 0.0) / 0.1 | - | does not move | stood still at 8 and at 3 units from Link |
| Wolfos | 6.5 (6.2 to 6.5) / 9.6 | 8.4 (8.4 to 8.9) / 13.8 | 1.30 | partly | peak ratio 1.44 |
| Wolfos_White | 6.2 (6.0 to 6.3) / 9.6 | 8.3 (8.0 to 8.4) / 14.5 | 1.34 | partly | peak ratio 1.51 |

Totals (43 enemies): follows the setting 2, partly 6, ignores it 11, unclear 3, does not move 16, did not appear 5 (Bubble_Blue, Gibdo, IronKnuckle_Silver, Keese_Fire, Poe_Big). Biri (follows), Stinger and TorchSlug (ignores) only drifted without approaching Link: count them as uncertain.

## What stands out

- Nothing scales a full 2x in pursuit. Best typical ratios: Lizalfos 1.72; by peak Freezard and Guay about 2.0. The walkers Stalfos, Wolfos, White Wolfos and Dinolfos gain 1.3 to 1.5 in typical speed and 1.4 to 1.8 in peak.
- Hoppers, floaters and rollers ignore the setting: both Tektites, Peahat, Poe, Green and Red Bubble, Shell Blade (a constant 6 u/s) and Floormaster measured the same at 1x and 2x.
- Keese, Ice Keese and Dodongo moved at 2x but hardly at 1x, so there is no honest ratio for them.
- 16 of 43 never moved in this setup (some are fixed in place by design: Beamos, Skulltula, Skullwalltula, Scrub_Mad, Dekubaba, FlyingPot; others such as Moblin, Octorok, Leever, ReDead probably need a trigger we did not give): treat them as not measured. 5 did not appear in the field at all.
- The setting's own tip says Octoroks and Skulltulas keep their pace; this survey could not test that because they did not move.
