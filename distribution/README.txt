OpenFO3 - open-source Fallout 3 engine
by RPGKING The Mad Modder
======================================

You need your own copy of Fallout 3 (Steam, GOG or disc). No game files
are included.

This is version 1.0 and it isn't finished. Bugs will be common, including
crashes and stuck quests.


GETTING STARTED

  1. Run "OpenFO3 Launcher".
  2. It should find Fallout 3 by itself. If it doesn't, pick the game's Data
     folder (the one with Fallout3.esm) on the Data Files page.
  3. Set your window and resolution on the Display page.
  4. Press PLAY.

  The Import page copies your resolution, subtitles and active plugins from
  the original game. It only reads those files.

  You can also run OpenFO3.exe with the Data folder as its first argument.
  "OpenFO3.exe --help" lists its options and controls.


WINDOWS AND ANTIVIRUS WARNINGS

  The programs aren't code-signed, so Windows SmartScreen or your antivirus
  may flag them.

  If you see "Windows protected your PC", click "More info", then "Run anyway".
  If your antivirus quarantines OpenFO3.exe or the launcher, restore the file
  and allow it.

  SHA-256 checksums are on the release page.


FILES

  OpenFO3 doesn't change your Fallout 3 install, settings or saves.

  Saves and game log    Documents\My Games\OpenFO3
  Launcher settings     %APPDATA%\OpenFO3

  OpenFO3 saves don't load in the original game. Original game saves don't
  load in OpenFO3 yet.


CONTROLS

  W A S D               move
  mouse                 look
  left mouse            attack
  right mouse / L-Alt   aim
  E                     talk, take, open, use
  Tab                   Pip-Boy (hold for light), end a conversation
  V                     V.A.T.S.
  R                     reload (hold to draw or holster)
  Space                 jump
  Shift / Ctrl          walk / sneak
  Caps Lock, Q          always run, auto move
  F5 / F9               quicksave / quickload
  F12                   save a bug report to the reports folder (a screenshot
                        and the game state; write the problem in its note.txt)
  [ and ]               darker / brighter
  Esc                   pause menu


WORKING SO FAR

  Vault 101, Megaton and the start of Following in His Footsteps.

  Not done: the rest of the main quest, side quests, DLC content and Fallout
  3's combat AI. Mods that need the script extender don't work.


CREDITS

  RPGKING The Mad Modder     OpenFO3
  nv-rs contributors         the Fallout: New Vegas engine OpenFO3 is built on
  OpenMW                     inspiration
  Bevy, egui, Inno Setup     game engine, launcher UI and installer

  Other libraries are listed in THIRD-PARTY-NOTICES.txt.


LEGAL

  OpenFO3 is free software under the GNU General Public License, version 3
  or later (LICENSE.txt). It isn't affiliated with or endorsed by Bethesda
  Softworks, ZeniMax Media or Microsoft. "Fallout" is their trademark and is
  only used to say which game this works with. See LEGAL.md.

  Copyright (C) 2026 RPGKING The Mad Modder and the nv-rs contributors.
