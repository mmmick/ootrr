# OOTRR 0.8.1

A fix-up release. The main change: Windows Defender and other antivirus tools should stop flagging OOTRR, and there is now a portable version that installs nothing. Also two new time savers and several fixes.

## Windows: no more antivirus false alarms

- **Why it was flagged:** the "Browse" button for picking a folder started PowerShell with a hidden, encoded script. OOTRR's script was harmless, but malware works the same way, so Windows Defender and other antivirus tools treated OOTRR as suspicious.
- **The fix:** OOTRR now opens the normal Windows folder window directly, without PowerShell. It looks and works the same as before.
- `ootrr.exe` now carries its name, version and license in **Properties > Details**, like any normal program.
- Every download is built by GitHub from the public source and now comes with a build attestation. Anyone can check where a file came from with `gh attestation verify <file> --repo ootrando/ootrr`.
- The files are still not code-signed, so SmartScreen may still say "Windows protected your PC" the first time. Click **More info**, then **Run anyway**. If Defender ever removes `ootrr.exe` anyway, restore it from **Windows Security > Protection history** and report the false positive at https://www.microsoft.com/wdsi/filesubmission.

## New: portable Windows version

- Download `OOTRR-0.8.1-windows-x64-portable.zip`, unpack it anywhere (a USB stick works too) and double-click `ootrr.exe`.
- Nothing is installed and nothing is written to the registry. OOTRR keeps its files in a `data` folder next to `ootrr.exe`. To remove it, delete the folder.
- The installer is still there if you prefer it.

## New time savers

Both are off by default. They are on the **Time savers** page.

- **Skip first pickup text**: no "You got X!" box the first time you pick up a rupee, a recovery heart, a magic jar, arrows or Deku seeds. Chests, quest items and equipment keep theirs.
- **Faster sliding blocks**: pushable and pullable blocks slide twice as fast. They still stop on the same spots.

## Fixes

- **King Zora** no longer sits back down after you show him Ruto's Letter. This now holds in every seed, not only with Zora's Fountain open, and also with story skips on.
- **Darunia's Joy**: the fix for Darunia is no longer undone when story skips are on.
- **Door of Time**: a setting saved by an older version can no longer override what the Door of Time setting on the page shows. With the Song of Time not required, the door opens on the chosen requirement, and the logic matches.
- **Website:** building from a `.nup` or `.nx` package no longer fails with "The generator failed. unreachable". Package builds also no longer stop on missing title logo and Gerudo files (desktop app too).
- **Website on Firefox:** you can now drop the game folder onto the page instead of waiting minutes for Browse to list every file.
