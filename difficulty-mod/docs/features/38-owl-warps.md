# 38. More owl rides (Time savers)

## Requested (user, 21 Nov)

A time saver, its own small PR: more owl warps. (1) Kaepora Gaebora comes back to the Death Mountain Trail and to Lake Hylia and can be talked to for a ride again and again, not only once. (2) A new owl in Kakariko Village that flies Link to Kokiri Forest. (3) A small table, so more rides are easy to add. (4) A Time savers toggle, default off.

## Setting

`owl_warps`, number 153, "More owl rides", toggle, default off. Menu: Time savers page, new section **Travel**. 152 is taken by the Kaepora hint branch. The default settings string and default seeds are unchanged.

Tip: "The owls on the Death Mountain Trail and at Lake Hylia stay all game and offer their flights every time, and a new owl in Kakariko Village flies you to Kokiri Forest. Child Link only."

## What makes the vanilla owls leave (found in the game files)

- Both placed owls carry a map `Presence` entry: `HavePlayedEvent.Dm_TimeTemple_0086` with the negation flag. The game creates them only while that Temple of Time owl scene is unplayed. This is the only thing that removes them, and it is in the map data (`DeathMountainTrail_Child_Static`, owl `Npc_Kaepora_DeathMountainTrail`; `HyruleField_Child_Static`, owl `Npc_Kaepora_LakeHylia`). The adult maps have no owl.
- The Lake Hylia owl is also created by his own logic group (`HyruleField_LakeHylia_Child_Prefab_32f1`), which asks the same question (Temple of Time owl scene unplayed) and also wants the castle quest done, `Dm_HrlField_0050` played and the lake visited. With the setting on, that one check reads the credits scene flag (`Dm_Ending_0000`) instead, so the owl is no longer removed by the Temple of Time scene. The other conditions are story gates and stay (the owl first appears after the castle quest, as in the game).
- The Lake Hylia flow always offers the flight: every branch ends in the question and the flight scene. Played before or not, it asks again. Nothing to change.
- The Death Mountain Trail flow offers the flight only with Din's Fire (`IsGet.Item_DinFire`); without it he talks about the Great Fairy and offers nothing. A "recent talk" check (`TalkCheck`) can also show the short "Threads of Time" line instead of the offer.

## How it works

1. **Staying:** with the setting on, the `Presence` entry is removed from both owls, and the Lake Hylia logic check is redirected (see above). Nothing else in the map changes.
2. **Death Mountain Trail owl:** the Talk entry's mask talk (`Mask_All`) now continues into a map check (`Sequence_CurrentBanc`, the same query the Return with Navi feature uses). On any map that is not a ride map it goes straight to the game's own question (`Talk_04`, "heading down the mountain, I can carry you as far as Kakariko Village?") so the offer comes every time, with or without Din's Fire. The Great Fairy hint talk is therefore not heard with the setting on. This was my choice to meet "offer the ride every time". Say so if you want the hint talk kept.
3. **New owl in Kakariko Village:** one more `Npc_Kaepora_DeathMountainTrail` actor in `KakarikoVillage_Child_Static`, position `KAKARIKO_SPOT` in `qol/owl.rs`. It runs the same flow. On `KakarikoVillage_Child` the map check leads to a new question ("If you're heading for the forest, I can carry you as far as Kokiri Forest. What do you say?"), then the game's own yes talk, the game's own boarding line (`Talk_05`) and the game's own map change action (`EventReserveBancChange`), now to `KokiriForest_Child`, start `EntrancePos` (the arrival Return with Navi uses). The no answer reuses the game's own refusal path (the "no" animation and `Talk_06`, "Hoot."). Every path ends the way the vanilla paths end: the map change, or the refusal talk, then the end of the flow.
4. **Table:** `RIDES` in `randomizer/core/src/qol/owl.rs`: one row per new owl, with the map directory and map, the position and turn, the destination (map and start point) and the question text. Adding a ride is adding a row. The flow, the map placement, the staged files and the texts all follow the table.
5. **Text:** the question is added to the Death Mountain owl's message file in every language archive, in English (like Return with Navi).

## What it does not do (and why)

- The new ride skips the flight scene: the vanilla flight scenes (`Dm_DeathMtTrail_0030`, `0035`) are made for the Death Mountain Trail and Kakariko, with camera and owl placed at fixed coordinates, so they cannot be reused for another route. The new ride uses the owl's own question, the boarding line and the game's fade and map change. The two vanilla rides keep their own flight scenes.
- The vanilla owls elsewhere (Hyrule Field, Zora's River, the Lost Woods, Kokiri Forest) are cutscene owls and are not touched.
- No new saved flag; the map check reads the game's own current map.
- Data edits only: map files, one event flow, message files. No code patch, nothing Longshot specific.

## Tested

Unit tests (`cargo test -p ootrr-core qol::owl`, needs `OOTRR_ROMFS`): the table; the flow walked for every path (Kakariko yes and no, Death Mountain yes and no) ending in the map change or the refusal line, and refusing a second patch; the three maps keep both owls and gain one; the whole stage builds with the setting on and off.

In the game: see "Played" below. By hand (21 Nov), all three rides passed: the Death Mountain Trail owl, the Lake Hylia owl and the Kakariko to Kokiri Forest ride.

## Played

Longshot, one copy, fresh save, preset with `open_kakariko` open (so the castle quest is complete), by script with screenshots (21 Nov):

- Death Mountain Trail owl: rode three times in a row, each landing in Kakariko with control and full life; the third with `HavePlayedEvent.Dm_TimeTemple_0086` set (vanilla removes the owl then; he stood and carried Link). The first ride had no Din's Fire: the offer came anyway.
- Lake Hylia owl: stands on his post; rode twice, each flight ended in Hyrule Field (same map, new spot) with control. With the 0086 flag set he was still there (seen, not ridden a third time).
- Kakariko owl: new question text shown, "No" gave "Hoot." and control; "Yes" loaded Kokiri Forest (arrival at the Return with Navi spot) with control. Map loaded each time, no freeze, no black screen in Kakariko.
- Not played: the owl in a full playthrough with the Temple of Time scene really played, adult Link, other languages, other emulators and runtimes (data only, not run elsewhere). The Lake owl position near the pedestal needs the camera facing east to get the Talk prompt; a human has no such trouble.
- Kakariko owl position: set by the user from the hands-on test (21 Nov) to (-49.33, 118.84, 21.58), facing 121.5 degrees (`KAKARIKO_SPOT`, `KAKARIKO_TURN` 2.1206 rad). Checked against the map's navmesh: there is no navmesh floor at that x and z at all (the nearest floor at a similar height is about 9 units away), so it is a spot the user picked in the game, probably a raised perch. Not played at this spot since the move; the user played all three rides by hand with the earlier build and said "owl warp is amazing. game changer."

## How to test it in the game

1. Build with `owl_warps` on and the time savers (`skip_intro`, `skip_cutscenes` all, `faster_chests`).
2. Teleport to `DeathMountainTrail_Child`, walk to the owl on the upper trail (-7.4, 210, -137.6), talk, answer yes: the flight scene plays and you land in Kakariko. Go back up and do it again.
3. Teleport to `HyruleField_Child`, find the owl near the lake (-199.7, 138.6, 454.2), ride twice.
4. In Kakariko Village the new owl stands near the field entrance (-49.33, 118.84, 21.58). Talk, answer yes: you land in Kokiri Forest. Answer no: "Hoot." and control comes back.
