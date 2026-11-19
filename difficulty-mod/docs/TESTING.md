# Testing the difficulty settings in the game

Nothing here is needed to use the randomizer. It is how we test. The mod itself is data only and works wherever OOTRR works; Longshot is just the most convenient place to script a playtest.

## By hand

1. Build a seed with one or two difficulty settings on (see the feature file for what to turn on).
2. Put the mod in the game's mods folder and play to the place the feature file names.
3. Report the release candidate number, the settings that were on and exactly what happened.

Change as little as possible per test. With everything on, a crash tells you nothing.

## Scripted, through Longshot's control bridge

[Longshot](https://git.magicbean.dev/MagicBeanDev/Longshot) is a PC runtime for the game, built on your own PC from your own copy of the game. It has a local control bridge that lets a script press buttons, take screenshots, teleport and read the game's state. It is not part of this repo and none of it is copied here.

### Set up Longshot

- Install it with its launcher, or build it from a clone with its own script. Its README section "Build from source" has the commands; in short: `python -I nx2/make_game.py --nup <your game file> --out <game folder> --language en-US`. It needs a recent Python, its compiler tool set and about 60 GB free while building.
- A recent version is needed for the bridge to report health, position, music and save flags. Older versions report only the map, age and whether an event is playing; with those, read the hearts from a screenshot.

### The bridge

- It is off unless the game is started with the environment variable `HOS_CONTROL` set. It is local only and for the current user only.
- The client is `scripts/longshot_ctl.py` in the Longshot repo (standard library only). Longshot's `docs/control.md` documents every command.
- Useful commands: `launch <game folder> --user <save folder>`, `status`, `press A --frames 8`, `stick L 0 1 --frames 90`, `shot <file.png>`, `teleport <map or id>`, `age adult`, `watch --events oot --flags <names>`, `quit`.

### Two local additions to the bridge

We added two commands to our own Longshot source build. They are not in Longshot itself and not in this repo; they are kept as a patch beside the Longshot clone and re-applied after an update.

- `warp X Y Z`: moves Link to a position in the current map (same units as `status`'s position). Refused while an event plays or a map loads. Seen working for short and 100-unit jumps; a jump to a spot with no ground made Link fall out, lose a heart and respawn.
- `caption --test "..." --expect "..." --result "..."`: a small panel at the bottom left of the game window saying what is being tested, what is expected and the result. Every scripted test uses it so a person watching can follow. It does not appear in `shot` screenshots.

### Enemy positions: `enemies` (our source build only)

`longshot_ctl.py --json enemies` (or `enemies --no-scan`) lists every live enemy actor as JSON: `t_ms` (host clock, monotonic), `frame`, `link` [x,y,z], `life`, `max_life`, and `enemies`: `name`, `id` (the actor's address, stable per instance), `x`, `y`, `z`, `hp`, `max_hp`. The first call scans memory (a hitch); `--no-scan` re-reads the last scan's enemies and is fast enough to sample at 10 to 20 Hz. Speed is horizontal distance between two samples over the `t_ms` difference, in the units `status` uses per second. In Python, `lib.enemies()` and `lib.horizontal_speed(a, b)` (playtest `encounters\lib.py`) wrap it. `docs\enemy-speed.md` has a whole survey built on it (`<playtest folder>\speed\survey.py`).

Run the game windowed (`HOS_FULLSCREEN=0`) so it can be watched beside other windows.

### Keep the real install safe

Never test in the folder you play in. Make a shadow game folder:

1. A new empty folder.
2. Directory junctions inside it for the large read-only folders of the real game folder (the game's `romfs` and the like).
3. Copies of the small top-level files and the executable.
4. Its own empty `mods` folder, and its own save folder passed with `--user`.

One randomizer mod at a time in the mods folder; mods load only at game start. Use a fresh save folder for each new game so starting settings apply.

### A typical scripted check

1. `ootrr generate <preset.json> "<seed>" <out.ootrr>` then `ootrr build <out.ootrr> <game romfs> <shadow mods folder> <name> --host longshot`. A preset is `{"id":"t","name":"t","tip":"t","order":99,"settings":{...}}`.
   - Put the randomizer's time savers in every test preset: `"skip_intro": true, "skip_cutscenes": "all", "faster_chests": true`. Leave them off only when the scene itself is under test.
2. Launch the shadow game, press A through the title until `status` says `play`.
3. Read the starting state, do the thing, read the state again, take a screenshot.
4. Quit the game before starting the next run.

### Rules

- One copy of the game at a time.
- "Loads" is not "works": say which you saw.
- Teleports less than about eight seconds apart are refused.
- A new mod needs a restart of the game.

## What has been tested so far

See `CHANGELOG.md` for the results per release candidate, and each feature file for its own status.
