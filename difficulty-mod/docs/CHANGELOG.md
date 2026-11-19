# Difficulty settings: changelog

One entry per release candidate (rc), newest first. Each rc is one commit on the `difficulty` branch. "Played" means seen in the running game, by hand or by a script through Longshot's control bridge (see [TESTING.md](TESTING.md)). Feature details are in [features/](features/).

## rc 15

Changes from the first hands-on session with rc 14 (19 Nov, at 10x). A second hands-on session the same day played this build: "everything I tested was perfect", with the spawn buttons and the extra enemies named; it was not recorded which other items below were looked at, so only those marked "by hand" count as seen.

- **Spawn enemies: new buttons.** Hold both stick clicks, then D-pad Down for a random enemy and D-pad Up for the same one again. Works by hand on a real controller. The old L + R never worked by hand: both open the game's item wheel.
- **Special encounters is no longer its own setting.** Any more-enemies value above 1x includes what it did (Dead Hand in two holes, the awake Iron Knuckle, the Lake Hylia Like Likes, grass and pot surprises, the Guays over child Hyrule Field). Old settings strings still load.
- **More enemies, from play at 10x** ("about the right number" in child Hyrule Field; by hand):
  - Peaceful places get no added enemies: child Kokiri Forest, Kakariko Village, child Market, child Hyrule Castle, Gerudo Fortress, Zora's Domain, Goron City; enemies keep away from Lon Lon Ranch and the Temple of Time.
  - Iron Knuckles: at most two in each Hyrule Field, one in any other map.
  - Large enemies (Like Like, Wolfos, Stalfos, Dinolfos, ReDead, Moblins and the like) stay well apart; small ones may cluster.
  - Keese of every kind are only placed on a perch (a tree outdoors, a wall indoors), since they sit still until approached.
  - Walkers only go on ground Link can reach; no more enemies standing in the lava of Dodongo's Cavern. Fire Keese were added there.
  - No added Dodongos or Baby Dodongos in Dodongo's Cavern (their explosion can clear a bombable wall).
  - Tougher enemies are steered toward landmarks such as trees and holes ("very good stuff" in the first session).
- **Enemies call for backup: a lesser kind.** Seen by hand in rc 14: Stalfos and Wolfos calling a second of their own kind ("nice", but too much), and a Lizalfos' helper falling into lava. Now a caller never calls its own kind (Stalfos a Stalchild, Wolfos two Keese, White Wolfos a Wolfos, Lizalfos and Dodongos a Keese), and the helper appears where the caller stands. Not played yet in this form.
- **Gate on Death Mountain Trail with the Dodongo guard: "perfect", by hand.**
- **Bosses spawn minibosses and minions (Gohma):** warped into the room by the test tool, the Stalfos and Keese never appeared (they wait for the door scene, which a warp skips). Entering through the door is still not played.
- **Not done:** an on-screen effect when enemies regenerate, are healed by a fairy or call for backup. The node that emits an effect is known; no effect that every enemy has loaded was found yet.

## rc 14

Results come from scripted runs in the game (19 Nov). Nothing in this entry has been played by hand yet.

- **Merged upstream 0.7.3.**
- **More enemies is now a slider from 1x (off) to 10x.** The value is a ceiling, not a target: each area gets as many extra enemies as sensibly fit, up to that multiple. Up to 2x adds more of an area's own enemies; above that come kinds that suit the ground (flyers over open ground, Octoroks in water, wall and ceiling kinds in dungeons, Leevers only on sand) and, from about 5x, kinds that never live there and the odd miniboss (Stalfos, Dinolfos pairs, an Iron Knuckle, White Wolfos) on flat open ground. Spots come from the game's own navigation data, with a slope limit per kind. See [features/10](features/10-more-enemies.md).
  - Safe zones: nothing is added near any place Link can arrive (entrances, exits, hole exits, warp pads, restart points), and only a couple of slow enemies in a wider ring, always leaving a way out.
  - Limits: 240 enemies per map, 120 within 60 units. At 10x that is about 1,800 added enemies over 39 maps; 9 of 172 areas reach the full 10x.
  - Seen: 37 of 38 edited maps load at 10x (the 38th is black without the mod too and is no longer edited). Kokiri Forest, both Hyrule Fields, Dodongo's Cavern and the Fire Temple hold 58 to 60 frames a second at 10x. Dying and retrying in Hyrule Field and Dodongo's Cavern: no damage in the first five seconds.
  - Seen going wrong: Freezards fell through floors, so they are not placed. In 30 seconds at 10x one Tektite fell and a few enemies dropped off the list.
  - Not tested: rooms whose door opens when the enemies are beaten; safe zones in Kokiri Forest and other dungeons; how it plays.
- **Enemies call for backup: "bring a friend".** The helper is now the same kind as the caller or a peer (a Stalfos calls a Stalfos, a Wolfos a Wolfos, a Deku Baba a Mad Scrub), once, 10 to 25 seconds into the fight, and a called helper never calls. The table is in [enemy-table.md](enemy-table.md), option 3.
  - Seen (32 callers spawned in Hyrule Field, no crash): working for Stalfos, Wolfos, Lizalfos, Dodongo, club Moblin, both Tektites, Deku Baba, Mad Scrub, Gohma larva, Floormaster, Like Like, Poe, green and red Bubble, Torch Slug, Octorok, Shell Blade. Keese and ReDeads call nobody. No helper called in turn.
  - Seen short: the spear Moblin and the Bari brought one helper, not two.
  - Nothing came in 75 seconds for Stalchild, Freezard, Skulltula, Wallmaster, Armos, Spike, Tailpasaran, Biri. Several of these do not fight normally when dropped in an open field, so this may be the test and not the feature.
  - Not tested: weak Stalfos, Gibdo, white Bubble, the water and Jabu-Jabu kinds.
- **Everything on together** (more enemies at 5x): Kokiri Forest, Hyrule Field, Dodongo's Cavern and the Deku Tree load, and three minutes in Hyrule Field ran without a crash at 46 to 56 frames a second.
- Testing tools: the `enemies` command now scans the whole heap (it missed most enemies before, which looked like a mod fault); frame rates are only measured with one copy of the game running.

## rc 13

Results come from scripted runs in the game (19 Nov) unless they say otherwise.

- **Merged upstream 0.7.1** (the title logo option). All upstream tests and ours pass.
- **Enemy regen, new timing, seen working:** the first heal comes 15 seconds after the last hit (15.0 and 15.2 measured), then 1 health about every 5 seconds.
- **Enemies call for backup, reworked, seen working.** Each kind of enemy calls its own kind of helper, after a random 10 to 25 seconds, once. Seen: a Stalfos called one Keese (15.9 and 24.5 seconds), a Lizalfos one Baby Dodongo (24.4 and 25.4 seconds); Keese and a ReDead called nobody for 90 seconds; no crash. The full table is in [enemy-table.md](enemy-table.md). Peahat larvae and Leevers are no longer used as helpers. See [features/17](features/17-enemies-call-for-backup.md).
- **Gates: eleven of twelve are on.** Played by hand on 19 Nov: gates 1, 2, 3, 4, 5, 6, 9, 11, 12 and 13. A gate now only appears once the player is well past its line on the guard's side. The Death Mountain Trail gate has a Dodongo as its guard: the map loads and he stands at his spot (script); the fight is not played. Gate 7 (Lost Woods east passage) stays held back, never tried.
- **Spawn enemies with L + R + D-pad** works by scripted presses in this build with every other difficulty setting on (six presses, five to ten enemies). By hand on a real controller it did nothing in the previous build; not retried by hand.
- **Special encounters:** 24 Guays over child Hyrule Field in six flocks. Seen: eight in range, flying, by day and by night (they were meant to be night-only; they are not). The Like Like near the lakeside laboratory is removed. Grass that always gave a bug joins the surprise pool: confirmed by hand.
- **Bosses spawn minibosses and minions:** in Gohma's room the Stalfos and Keese are now created when the door shuts, not when the room loads. The dungeon loads; the door scene is not played (a script cannot walk through the door yet). Other boss rooms are unchanged.
- **Zora's Domain ambush:** trigger adjusted. The map loads with the three Skulltulas present; the walk through the cave is not played.
- **Fairies help enemies:** confirmed by hand. The circling effect on the healed enemy is not possible with data edits.
- **Enemy speed, measured for the first time.** A script spawns each enemy beside a standing Link and tracks its position. At 2x only the Lizalfos comes close to double (1.72); Stalfos, Wolfos, Dinolfos, Freezard and Guay gain 30 to 50 percent; Tektites, Peahats, Poes, Bubbles, Shell Blades and Floormasters do not change. The table is in [enemy-speed.md](enemy-speed.md).
- **Not in this rc:** the "more enemies" slider rework (up to 10x, placed from the maps' navigation data). It is built and loads 37 of 38 maps at 10x, but many planned enemies do not appear in five maps and the frame rate halves at 10x. See [features/10](features/10-more-enemies.md).
- Testing tools: Longshot updated to the latest upstream with our tools re-applied; a new `enemies` command lists live enemies with positions and health; two copies of the game can run side by side for tests. See [TESTING.md](TESTING.md).

## rc 12

Results here come from scripted runs in the game unless they say otherwise. Enemy health can now be read during tests, which is what made several of these checks possible.

- **Enemies call for backup: fixed and seen working.** It crashed the game seconds after an enemy appeared; the cause was how its two flags were stored. Now: no crash through 90 seconds beside two Lizalfos, and each called one Peahat larva, once, at 10 and 16 seconds. The wait is about 12 seconds, not the 25 intended. See [features/17](features/17-enemies-call-for-backup.md).
- **Spawn enemies with L + R + D-pad: fixed and seen working** with scripted button presses: 12 presses gave 12 enemies in front of Link, D-pad Up repeated the last kind, and further presses gave nothing (12 alive at once). Before, only the first press ever worked. Not yet tried on a real controller. See [features/25](features/25-spawn-enemies-combo.md).
- **Enemy regen: seen working.** A Stalfos hit from 18 to 11 waited four seconds, then healed 1 about every two seconds back to 18; with the setting off it stayed at 12 for a minute.
- **Enemy health: seen working by the numbers.** At 2x a Stalfos has 36 and a Lizalfos 18.
- **Like Likes at Lake Hylia: present.** The map loads and they are at their spots.
- **"Red gate guards" was not a bug.** The big Deku Baba on the Deku Tree path has a red head with every difficulty setting off.
- **Gates:** unchanged from rc 11 (six on). The six held back all load their maps and show their guards; killing the guards and seeing the barrier drop is still to be done by hand.

## rc 11

- **Merged upstream** (32 commits; the repository layout changed: `crates/` is now `randomizer/`). All upstream tests and ours pass.
- **Surprises from grass and pots** are now 1 in 50 for each kind, and grass releases the wall Skulltula (on the ground) or a little Floormaster hand; pots a Poe or a little hand. Played by hand at a raised test rate: "perfect". The clumps of grass that always give a bug (for example round the Dead Hand hole in Hyrule Field) now join the pool; not played yet.
- **Cuccos take revenge at once** now covers Kakariko's cuccos too. Played: one hit brings the swarm.
- **Dead Hand** in both holes: placed further back and his rise distance shortened. Not played since; before this he still rose near the entrance.
- **Gates:** seven were in rc 10; six are in this rc. The Lost Woods room was changed to one Stalfos for both barriers and is held back until that is seen working, along with the five held back before (which got their fixes: own guards for the meadow and the trail, two Stalfos in Gerudo Valley, a rule that a gate only triggers from the guard's side, the Hyrule Field barrier half a unit lower). None of the fixes is played yet.
- **Enemies call for backup crashes the game** (found by a scripted test, twice). Leave it off. Regen alone and fairies alone did not crash.
- Docs: the status board, the capture playbook and the code review are now in this folder too.

## rc 10

Played by hand unless it says otherwise. Things known not to work are listed in [TODO.md](TODO.md).

- **Enemies block the way, rebuilt.** A gate is the game's own purple barrier. Nothing is there until you come near its guard; then the barrier rises and the guard appears with it; it drops when the guards are dead. Seven gates are in: three on Zora's River (Lizalfos; the upper one has two), the Deku Tree path (the withered Deku Babas, closed forest only), the small room in the Lost Woods (two barriers), and the road to Gerudo Valley in adult Hyrule Field (an Iron Knuckle). All seven were seen working. Five more are built but held back until they work (see TODO). See [features/20](features/20-enemies-block-the-way.md).
- **New in special encounters: grass and pot surprises.** Cut grass can release a wall Skulltula or a little Floormaster hand; a broken pot a Poe or a little hand; and about one plain pot in ten is a flying pot. The chance is 1 in 100 in this rc (1 in 50 is planned). Grass surprises were seen working at a raised test rate. Pots in the Market guard house gave nothing; not understood yet. See [features/23](features/23-grass-and-pot-surprises.md).
- **New in special encounters: ten Like Likes around Lake Hylia.** Not played yet.
- **Special encounters:** the Iron Knuckle in Hyrule Field is awake from the start ("perfect"). Dead Hand in the Hyrule Field hole was moved back twice and still rises near the entrance.
- **New: Cuccos take revenge at once.** One hit brings the swarm. Seen working in Kakariko. The two minigame cucco kinds are left out. See [features/24](features/24-cucco-revenge.md).
- **Enemy regen, fairies help enemies, call for backup** now run beside the game's damage-flash routine instead of inside it. Not confirmed: with these on, killing a gate guard crashed the game once. Leave them off for now.
- **New setting that does not work yet: spawn enemies with L + R + D-pad.** The checkbox is there; pressing the buttons does nothing. See [features/25](features/25-spawn-enemies-combo.md).
- Review fixes: a broken gate or shop setting is reported instead of ignored; the battle-music change is pinned to its five enemies; tips corrected.

## rc 9

- **Lose heart on death now works.** The heart is taken after the Try Again reload, in an event the game runs once the map is back. Played by script and by hand: 6 → 5 → 4 → 3 hearts, a further death stays at 3, a fairy revival costs no heart. Limit: the loss lasts for the play session; quitting and continuing restores the hearts. See [features/07](features/07-lose-heart-on-death.md).
- **No bottles during combat** also refuses bottles while miniboss music plays (it only knew the ordinary battle music). Played by hand against the Iron Knuckle: refused while he is aggro, allowed again at a distance, and a bottled fairy still revives Link in the fight. See [features/08](features/08-no-bottles-in-combat.md).
- **New: More enemies: special encounters** (one checkbox). A Dead Hand waits in two holes (Hyrule Field east, Death Mountain Trail) and an Iron Knuckle stands in the open part of Hyrule Field, awake from the start. Played: both maps load as child and adult; both Dead Hands rise and attack; the knight wakes without being hit. Dead Hand was then moved further from the landing spot, and "awake from the start" replaced "wakes when near"; those two changes are not played yet. See [features/22](features/22-special-encounters.md).
- **Enemies block the way:** the wall became two spider webs in front of a solid wall. Played by hand: the web blocks but can be bombed, and it sat on top of its guards. It is being rebuilt on positions marked in the game; treat this setting as unfinished in this rc. See [features/20](features/20-enemies-block-the-way.md).
- Confirmed in play with no code change: no Z-targeting, no dupe items (a second fairy cannot be bottled), no gossip fairies (six songs tried), no heart drops and no ammo drops (about 20 grass: rupees and bugs only), the Bean Seller's pitch.
- Known problem: with enemy regen, call for backup or fairies help enemies on, enemies can look tinted red. Cause not found yet.
- Not played yet: enemy speed, enemy regen, more enemies, boss minibosses and minions, enemies respawn sooner, fairies help enemies, call for backup, Zora's Domain ambush.

## rc 8

- **Enemies call for backup**, reworked. See [features/17](features/17-enemies-call-for-backup.md).
  - Strictly once per enemy: no second call, even after the helper dies.
  - Bosses, minibosses, Iron Knuckles, Dead Hand and Phantom Ganon never call.
  - The helper fits the caller: Shabom for Jabu-Jabu's enemies, a flying pot for the Ice Cavern, Shadow and Spirit Temple enemies, a Peahat larva for the rest.
- Played: nothing yet.

## rc 7

- **No bottles during combat:** Leevers and Moblins, the only enemies that fought without battle music, now start it when this setting is on, so the music check covers them. See [features/08](features/08-no-bottles-in-combat.md).
- **No dupe items:** a refused catch or hand-over now ends properly (head shake, then the same closing steps the game uses) instead of ending the event at its start. See [features/13](features/13-no-dupe-items.md).
- Played: nothing yet.

## rc 6

- **Lose heart on death, third version:** the heart is taken through the player's own "set max health" action on Retry. See [features/07](features/07-lose-heart-on-death.md).
- **New: Enemies block the way.** First gate: the path to the Deku Tree stays walled while the two withered Deku Babas beside it are alive. Nothing is saved. Skipped with an open forest or entrance shuffle. See [features/20](features/20-enemies-block-the-way.md).
- Played by script (rc.4 Longshot):
  - Lose heart on death still fails: 6 hearts before and after one game over and Try Again. The save flag read 0 before, at game over and after, so the added steps did not run at all.
  - Enemies block the way: Kokiri Forest loads, and the wall stops Link at the expected line with both Deku Babas alive (three attempts). Killing the Babas and walking through was not tested.
  - King Dodongo's black screen (rc 4 builder) comes from the edited static map file of Dodongo's Cavern: the cave loads without it, and stays black without either of the other two edited files.
  - Not tested: red enemies, Zora's Domain ambush.

## rc 5

- **Bosses spawn minibosses** no longer touches King Dodongo: his dungeon loaded to a black screen with it on.
- Played by script (rc.4 Longshot):
  - Enemy damage 3x works: a Deku Baba bite took 2 quarter hearts at 1x and 6 at 3x.
  - Dodongo's Cavern loads with boss minibosses and minions on.
  - Combat music is named `BGM_EnemyBattle`; it starts when an enemy notices Link and stays on until the map changes or Link dies.
  - Lose heart on death failed: 6 hearts before and after two game overs.

## rc 4

- Four more settings, 19 in total: **Enemies call for backup**, **Zora's Domain ambush**, **Fairies help enemies too**, **Bean Seller's sales pitch**.
- Played by script (rc.3 Longshot), load test only: Kokiri Forest, Zora's Domain, Hyrule Field, the Deku Tree and Fire Temple load with nine map and enemy settings on. Dodongo's Cavern went to a black screen, traced to boss minibosses.

## rc 3

- First combined candidate, 15 settings. Added: **Enemies regen health**, **No Z-targeting**, **More enemies**, **Enemies respawn sooner**, **No dupe items**, **Bosses spawn minibosses**, **Bosses spawn minions**.
- Edits that touch the same file now build on each other.
- Played: nothing.

## rc 2

- Fixes from the rc 1 playtest:
  - **Bottles:** the combat check was inverted, and a refused bottle froze Link.
  - **No heart / ammo drops:** the game's global drop lottery added drop chance at low health or ammo; those boosts are zeroed and the lottery's heart and ammo actors point at the green rupee.
  - **Lose heart on death:** Retry now asks the game to save before it reloads.
- The enemy sliders read as multipliers.
- Played: nothing.

## rc 1

- New Difficulty page with eight settings: **enemy health**, **enemy damage**, **enemy speed**, **no heart drops**, **no ammo drops**, **no gossip fairies**, **lose heart on death**, **no bottles during combat**.
- Played by hand: enemy health scaling works. Bottles never worked and froze Link, hearts and ammo still dropped, lose heart on death did not stick.
