# Legal

This is not legal advice.

## Not affiliated

OpenFO3 is a personal project by RPGKING The Mad Modder. It isn't affiliated with, endorsed by or sponsored by Bethesda Softworks, ZeniMax Media, Microsoft or their affiliates.

It started as a fork of [nv-rs](https://github.com/slaterain/nv-rs), an open-source Fallout: New Vegas engine.

## Trademarks

"Fallout", "Fallout 3" and "Bethesda" are trademarks of their owners. They're only used here to say which game this works with. OpenFO3 uses no Fallout logos or art.

## No game files

This repo and its releases contain no game files. That means no plugins (`.esm`, `.esp`), archives (`.bsa`), models (`.nif`, `.kf`), textures (`.dds`), sound (`.ogg`, `.wav`, `.mp3`, `.lip`), movies (`.bik`), saves (`.fos`), shader packages (`.sdp`), game executables or recordings. Tests make their own data. `crates/nvinspect/tests/repo_hygiene.rs` rejects these file types.

You need your own legal copy of Fallout 3. OpenFO3 reads its `Data` folder and doesn't change the game, its settings or its saves.

## Where the rules come from

OpenFO3 copies how Fallout 3 behaves. That comes from the game's data files and from decompiling `Fallout3.exe` 1.7.0.3. Comments in the code give the function address for each rule. No decompiler output is in this repo.

Code carried over from nv-rs also uses the following, marked where used:

- Rules from `FalloutNV.exe` that haven't been redone from Fallout 3 yet. These get replaced over time.
- Names and types from the debug symbols of Xbox 360 development builds that are publicly available.
- Logic translated from the decompiled program, mostly in `crates/physics`.

nv-rs's bulk translation of the New Vegas program is not in OpenFO3's source or releases.

## License

Copyright (C) 2026 RPGKING The Mad Modder and the nv-rs contributors.

Original code here is licensed under the GNU General Public License, version 3 or (at your option) any later version. See [LICENSE](LICENSE). This only covers contributors' original work. It gives no rights to the game, its files, its program, anything translated from it, or anyone's trademarks.

Code contributed before 2026-10-16 was released under MIT (or Apache 2.0, at the user's option) and is included here under the MIT terms:

> Copyright (c) nv-rs contributors
>
> Permission is hereby granted, free of charge, to any person obtaining a copy
> of this software and associated documentation files (the "Software"), to deal
> in the Software without restriction, including without limitation the rights
> to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
> copies of the Software, and to permit persons to whom the Software is
> furnished to do so, subject to the following conditions:
>
> The above copyright notice and this permission notice shall be included in
> all copies or substantial portions of the Software.
>
> THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
> IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
> FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
> AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
> LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
> OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
> THE SOFTWARE.

## Third-party code

- `crates/vorbis` follows the public source of libvorbis 1.2.0, libvorbisfile 1.2.0 and libogg 1.1.3 (Xiph.Org Foundation, BSD license) and keeps their window and floor tables, so it decodes the same way as the library the game ships with. The license is in [crates/vorbis/LICENSE-libvorbis](crates/vorbis/LICENSE-libvorbis). Each file that follows them says so at the top.
- `third_party/winit-0.30.13` is winit 0.30.13 (Apache 2.0) with one fix to Windows mouse input, marked in `src/platform_impl/windows/event_loop.rs`. The license is in [third_party/winit-0.30.13/LICENSE](third_party/winit-0.30.13/LICENSE).
- Other libraries and their licenses are listed in `distribution/THIRD-PARTY-NOTICES.txt` and `distribution/licenses/`.

## Rights holders

If you have a concern, open an issue on this repo or contact RPGKING The Mad Modder. Marked material can be found and removed quickly.
