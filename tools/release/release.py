"""Makes an OpenFO3 release. By RPGKING The Mad Modder.

    python tools/release/release.py all [--all-tests]
    python tools/release/release.py source | build | notices | package

The steps, in order:

  source   OUT/source, the public source tree: the engine's files as git lists
           them (tracked, and new ones not ignored) without the parts that
           aren't published (EXCLUDE), with tools/ and OpenFO3's GitHub files;
           the workspace and viewer manifests rewritten for the crates left
           out.
  build    the game and the launcher built from that tree (release), and the
           repository hygiene test run on it (--all-tests: every engine
           crate's tests).
  notices  THIRD-PARTY-NOTICES.txt and licenses/ for every library the two
           programs are built with, from that tree's cargo metadata.
  package  OUT/OpenFO3 (the game folder), and in OUT/dist the zip of it and
           the installer (Inno Setup 6's ISCC).

OUT is Desktop\\OpenFO3-release (OPENFO3_RELEASE changes it). Builds go to
OPENFO3_TARGET (default D:/open-fallout3-target/release). Nothing outside OUT
and the build folder is written.
"""

import datetime
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import zipfile
from pathlib import Path

PROJECT = Path(__file__).resolve().parents[2]
ENGINE = PROJECT / "engine"
OUT = Path(os.environ.get("OPENFO3_RELEASE") or Path(os.environ["USERPROFILE"]) / "Desktop" / "OpenFO3-release")
SOURCE = OUT / "source"
APP = OUT / "OpenFO3"
DIST = OUT / "dist"
NOTICES = OUT / "notices"
TARGET = Path(os.environ.get("OPENFO3_TARGET") or "D:/open-fallout3-target/release")
MARKER = OUT / ".openfo3-release"

GAME_EXE = "OpenFO3.exe"
LAUNCHER_EXE = "OpenFO3 Launcher.exe"
PLATFORM = "x86_64-pc-windows-msvc"

# Left out of the public source, and why.
EXCLUDE = [
    (r"^crates/engine(-[^/]+)?/", "the New Vegas engine translated from FalloutNV.exe"),
    (r"^crates/host/", "runs that translated engine on FalloutNV.exe"),
    (r"^crates/play-patch/", "patches to the translated engine"),
    # Except the frame map the world crate's tests check its frame order against.
    (r"^research/(?!engine-map/frame(_wiring)?\.tsv$)", "New Vegas research data"),
    (r"^scripts/", "New Vegas measurement and packaging scripts"),
    (r"^characters/", "New Vegas (Dead Money) character notes"),
    (r"^distribution/(?!README\.txt$)", "the earlier package files; OpenFO3's notices are made by the notices step"),
    # No documentation folder in the public repository (the README's pictures
    # are in media/).
    (r"^docs/", "the documentation folder (development notes)"),
    (r"^CONTRIBUTING\.md$", "the earlier contributing guide; OpenFO3's own is added"),
    (r"^\.github/", "the earlier GitHub settings (funding, owners); OpenFO3's own CI is added"),
    (r"(^|/)\.(?!github/|gitignore$|gitattributes$)[^/]+/", "local tool state (hidden folders)"),
]
# This machine's own names (tools/release/local-names.json, never exported):
# more files to leave out, .gitignore lines to drop, the hygiene test's
# local-state folder to rename, and the words MENTIONS.txt looks for.
LOCAL_NAMES = Path(__file__).with_name("local-names.json")
LOCAL = json.loads(LOCAL_NAMES.read_text(encoding="utf-8")) if LOCAL_NAMES.is_file() else {}
EXCLUDE += [tuple(e) for e in LOCAL.get("extra_exclude", [])]
# The crates left out, by package name (for the manifests).
LEFT_OUT_CRATES = re.compile(r"^(engine(-[a-z]+)?|host|play-patch)$")
# Of the project's own tools/, not published.
TOOLS_SKIP = {"rea", "rea.sh", "__pycache__", "github", "public", "local-names.json"}
# Names the export is checked for after each run (MENTIONS.txt): credits,
# licence notices and code calling a library are expected there.
MENTION_WORDS = LOCAL.get("mention_words", [])


def run(cmd, cwd, target=None, capture=False):
    env = dict(os.environ)
    if target is not None:
        env["CARGO_TARGET_DIR"] = str(target)
    print(">", " ".join(str(c) for c in cmd), f"(in {cwd})", flush=True)
    r = subprocess.run([str(c) for c in cmd], cwd=cwd, env=env, capture_output=capture, text=capture)
    if r.returncode != 0:
        if capture:
            print(r.stdout, r.stderr)
        sys.exit(f"failed: {' '.join(str(c) for c in cmd)}")
    return r.stdout if capture else None


def fresh(path: Path):
    """Empties one of OUT's own folders (never anything outside OUT)."""
    assert OUT in path.parents, path
    path.mkdir(parents=True, exist_ok=True)
    # Its contents, not the folder itself (it may be open in Explorer or a shell),
    # and never a git repository's own folder.
    for child in path.iterdir():
        if child.name == ".git":
            continue
        if child.is_dir() and not child.is_symlink():
            shutil.rmtree(child)
        else:
            child.unlink()


def start_out():
    if OUT.exists() and any(OUT.iterdir()) and not MARKER.exists():
        sys.exit(f"{OUT} exists and isn't a release folder: refusing to write in it")
    OUT.mkdir(parents=True, exist_ok=True)
    MARKER.write_text("Made by tools/release/release.py: its folders are rebuilt on each run.\n")


def version() -> str:
    text = (ENGINE / "launcher" / "Cargo.toml").read_text(encoding="utf-8")
    return re.search(r'^version\s*=\s*"([^"]+)"', text, re.M).group(1)


def git_files():
    out = []
    for extra in ([], ["--others", "--exclude-standard"]):
        listing = subprocess.run(
            ["git", "ls-files", "-z", *extra], cwd=ENGINE, capture_output=True, check=True
        ).stdout
        out += [p.decode("utf-8") for p in listing.split(b"\0") if p]
    return sorted(set(out))


def excluded(rel: str):
    for pattern, why in EXCLUDE:
        if re.search(pattern, rel):
            return why
    return None


# ---------------------------------------------------------------- source


def rewrite_workspace(text: str) -> str:
    def members(m):
        kept = [x for x in re.findall(r'"([^"]+)"', m.group(1))
                if not LEFT_OUT_CRATES.match(x.split("/")[-1])]
        return "members = [" + ", ".join(f'"{x}"' for x in kept) + "]"
    new = re.sub(r"members\s*=\s*\[([^\]]*)\]", members, text, count=1)
    assert new != text
    return new


def rewrite_viewer_manifest(text: str) -> str:
    lines = text.split("\n")
    out = []
    i = 0
    while i < len(lines):
        line = lines[i]
        header = re.match(r"^\[profile\.[a-z-]+\.package\.([A-Za-z0-9_-]+)\]\s*$", line)
        if header and LEFT_OUT_CRATES.match(header.group(1)):
            # The comment block right above it goes with it.
            while out and out[-1].startswith("#"):
                out.pop()
            i += 1
            while i < len(lines) and lines[i].strip() and not lines[i].startswith("["):
                i += 1
            while i < len(lines) and not lines[i].strip():
                i += 1
            continue
        out.append(line)
        i += 1
    text = "\n".join(out)
    # The New Vegas host dependency and its feature.
    text, n = re.subn(r"(?m)^# The New Vegas host:.*\n(^#.*\n)*^host = \{[^\n]*\}\n", "", text)
    assert n == 1, "the host dependency"
    text, n = re.subn(r'(?m)^nv-host = \["dep:host"\]\n', "", text)
    assert n == 1, "the nv-host feature"
    assert not re.search(r"(?m)^[^#\n]*crates/(host|play-patch|engine)", text), "a left-out crate in the manifest"
    return text


def rewrite_viewer_main(text: str) -> str:
    old = re.search(
        r"// The New Vegas host \(`crates/host`\).*?#\[cfg\(feature = \"nv-host\"\)\]\npub\(crate\) use ::host;\n"
        r"#\[cfg\(not\(feature = \"nv-host\"\)\)\]\n",
        text,
        re.S,
    )
    assert old, "the host's cfg block in viewer/src/main.rs"
    return text[: old.start()] + "// A stand-in for the New Vegas host, which OpenFO3 doesn't ship.\n" + text[old.end():]


def rewrite_gitignore(text: str) -> str:
    """Without this machine's local entries (LOCAL's gitignore_drop)."""
    drop = set(LOCAL.get("gitignore_drop", []))
    return "".join(line for line in text.splitlines(keepends=True) if line.rstrip("\n") not in drop)


def rewrite_hygiene_test(text: str) -> str:
    """The hygiene test's local-state folder under a neutral name (the same rule)."""
    for a, b in LOCAL.get("hygiene_replace", []):
        text = text.replace(a, b)
    for word in LOCAL.get("hygiene_forbid", []):
        assert word not in text.lower(), f"'{word}' left in the hygiene test"
    return text


def edit(path: Path, f):
    raw = path.read_bytes().decode("utf-8")
    crlf = "\r\n" in raw
    text = f(raw.replace("\r\n", "\n"))
    path.write_bytes((text.replace("\n", "\r\n") if crlf else text).encode("utf-8"))


def source():
    start_out()
    fresh(SOURCE)
    left_out = {}
    count = 0
    for rel in git_files():
        why = excluded(rel)
        if why:
            left_out[why] = left_out.get(why, 0) + 1
            continue
        src = ENGINE / rel
        if not src.is_file():
            continue  # deleted in the working tree
        dst = SOURCE / rel
        dst.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(src, dst)
        count += 1
    # The project's tools.
    for root, dirs, files in os.walk(PROJECT / "tools"):
        dirs[:] = [d for d in dirs if d not in TOOLS_SKIP]
        for f in files:
            if f in TOOLS_SKIP or f.endswith(".pyc"):
                continue
            src = Path(root) / f
            dst = SOURCE / "tools" / src.relative_to(PROJECT / "tools")
            dst.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(src, dst)
            count += 1
    shutil.copytree(PROJECT / "tools" / "release" / "github", SOURCE / ".github")
    shutil.copy2(PROJECT / "tools" / "release" / "public" / "CONTRIBUTING.md", SOURCE / "CONTRIBUTING.md")
    edit(SOURCE / ".gitignore", rewrite_gitignore)
    edit(SOURCE / "crates" / "nvinspect" / "tests" / "repo_hygiene.rs", rewrite_hygiene_test)
    edit(SOURCE / "Cargo.toml", rewrite_workspace)
    edit(SOURCE / "viewer" / "Cargo.toml", rewrite_viewer_manifest)
    edit(SOURCE / "viewer" / "src" / "main.rs", rewrite_viewer_main)
    report = [f"{count} files in {SOURCE}", "Left out:"]
    report += [f"  {n:5} files: {why}" for why, n in left_out.items()]
    (OUT / "SOURCE-EXPORT.txt").write_text("\n".join(report) + "\n", encoding="utf-8")
    print("\n".join(report))
    mentions()


def mentions():
    """Where the export still names another project or tool, for review."""
    found = []
    for f in sorted(SOURCE.rglob("*")):
        if not f.is_file() or ".git" in f.parts or f.suffix.lower() not in (
                ".md", ".txt", ".yml", ".toml", ".rs", ".py", ".iss", ".java", ".sh"):
            continue
        rel = f.relative_to(SOURCE).as_posix()
        if rel.startswith(("third_party/", "distribution/licenses/", "tools/release/upstream-licenses/")):
            continue
        text = f.read_text(encoding="utf-8", errors="replace")
        for word in MENTION_WORDS:
            n = len(re.findall(r"(?<![A-Za-z])" + re.escape(word) + r"(?![A-Za-z])", text))
            if n:
                found.append((rel, word, n))
    lines = [f"{n:5}  {word:12}  {rel}" for rel, word, n in found]
    (OUT / "MENTIONS.txt").write_text("\n".join(lines) + "\n", encoding="utf-8")
    total = {}
    for _, word, n in found:
        total[word] = total.get(word, 0) + n
    print("Mentions (MENTIONS.txt):", ", ".join(f"{w} {n}" for w, n in sorted(total.items())) or "none")


# ---------------------------------------------------------------- build


def build(all_tests=False):
    run(["cargo", "build", "--release"], SOURCE / "viewer", TARGET / "viewer")
    run(["cargo", "build", "--release"], SOURCE / "launcher", TARGET / "launcher")
    run(["cargo", "test", "--release"], SOURCE / "launcher", TARGET / "launcher")
    if all_tests:
        run(["cargo", "test", "--release", "--workspace", "--no-fail-fast"], SOURCE, TARGET / "workspace")
    else:
        run(["cargo", "test", "--release", "-p", "nvinspect", "--test", "repo_hygiene"], SOURCE, TARGET / "workspace")


# ---------------------------------------------------------------- notices

NOTICE_NAMES = ("LICENSE", "LICENCE", "COPYING", "NOTICE", "COPYRIGHT", "UNLICENSE", "AUTHORS", "PATENTS")


def notice_files(pkg_dir: Path, license_file):
    found = []
    for f in sorted(pkg_dir.iterdir()):
        if f.is_file() and f.name.upper().startswith(NOTICE_NAMES):
            found.append(f)
    if license_file:
        lf = (pkg_dir / license_file).resolve()
        if lf.is_file() and lf not in found:
            found.append(lf)
    return found


def nested_notice_files(pkg_dir: Path):
    """Licences kept in a subfolder, such as the bundled fonts' (epaint_default_fonts/fonts)."""
    out = []
    for sub in ("fonts", "licenses", "LICENSES", "license"):
        d = pkg_dir / sub
        if d.is_dir():
            out += [f for f in sorted(d.iterdir()) if f.is_file() and f.suffix.lower() in (".txt", ".md")]
    return out


# Licence texts fetched from a package's upstream repository (at the commit its
# archive records), kept so a release doesn't need the network again.
UPSTREAM = PROJECT / "tools" / "release" / "upstream-licenses"
UPSTREAM_NAMES = ["LICENSE-MIT", "LICENSE-APACHE", "LICENSE", "LICENSE.txt", "LICENSE.md",
                  "LICENSE_1_0.txt", "COPYING", "NOTICE"]


def raw_url(repo: str, sha: str, path: str):
    repo = repo.split("/tree/")[0].split("/blob/")[0].rstrip("/")
    repo = repo[:-4] if repo.endswith(".git") else repo
    if repo.startswith("https://github.com/"):
        return f"https://raw.githubusercontent.com/{repo[len('https://github.com/'):]}/{sha}/{path}"
    if repo.startswith("https://gitlab.com/"):
        return f"{repo}/-/raw/{sha}/{path}"
    return None


def upstream_files(p, pkg_dir: Path, folder: str):
    cache = UPSTREAM / folder
    if cache.is_dir():
        files = [f for f in sorted(cache.iterdir()) if f.is_file() and not f.name.startswith("_")]
        why = (cache / "_provenance.txt").read_text(encoding="utf-8").strip()
        return files, why
    vcs = pkg_dir / ".cargo_vcs_info.json"
    if not vcs.is_file() or not p.get("repository"):
        return [], None
    info = json.loads(vcs.read_text(encoding="utf-8"))
    sha, sub = info.get("git", {}).get("sha1"), info.get("path_in_vcs", "")
    import urllib.request
    for path in dict.fromkeys([sub, ""]):
        got = {}
        for name in UPSTREAM_NAMES:
            url = raw_url(p["repository"], sha, f"{path}/{name}".lstrip("/"))
            if url is None:
                return [], None
            try:
                with urllib.request.urlopen(url, timeout=20) as r:
                    got[name] = r.read()
            except Exception:
                pass
        if got:
            cache.mkdir(parents=True)
            for name, data in got.items():
                (cache / name).write_bytes(data)
            why = (f"no licence file in the package archive; fetched from {p['repository']} at commit {sha}"
                   f" ({path or 'the repository root'})")
            (cache / "_provenance.txt").write_text(why + "\n", encoding="utf-8")
            return upstream_files(p, pkg_dir, folder)
    return [], None


def notices():
    start_out()
    fresh(NOTICES)
    licenses = NOTICES / "licenses"
    licenses.mkdir()
    packages = {}
    for program in ("viewer", "launcher"):
        meta = json.loads(run(
            ["cargo", "metadata", "--format-version", "1", "--filter-platform", PLATFORM],
            SOURCE / program, capture=True,
        ))
        used = {n["id"] for n in meta["resolve"]["nodes"]}
        for p in meta["packages"]:
            if p["id"] not in used:
                continue
            manifest = Path(p["manifest_path"])
            in_tree = p["source"] is None
            if in_tree and "third_party" not in manifest.parts:
                continue  # OpenFO3's own crates
            key = (p["name"], p["version"])
            packages.setdefault(key, (p, manifest.parent, in_tree, set()))[3].add(program)
    entries, missing = [], []
    old = ENGINE / "distribution" / "licenses"
    for (name, ver), (p, pkg_dir, in_tree, programs) in sorted(packages.items()):
        folder = f"{name}-{ver}"
        files = notice_files(pkg_dir, p.get("license_file"))
        provenance = "copied from the package's own files"
        if not files and (old / folder).is_dir():
            files = sorted(f for f in (old / folder).iterdir() if f.is_file())
            provenance = "no licence file in the package archive; a checked copy from its upstream repository"
        if not files:
            files, why = upstream_files(p, pkg_dir, folder)
            provenance = why or provenance
        nested = nested_notice_files(pkg_dir)
        if nested:
            provenance += "; the licences of files it bundles from its own subfolders"
        files = files + nested
        bundled = []
        if files:
            (licenses / folder).mkdir()
            for f in files:
                saved = f"{f.parent.name}_{f.name}" if f in nested else f.name
                shutil.copy2(f, licenses / folder / saved)
                bundled.append(saved)
        else:
            missing.append(f"{name} {ver} ({p.get('license')})")
            provenance = "no licence file in the package archive: its licence expression applies"
        source = "this repository (third_party/)" if in_tree else (
            "crates.io" if "crates.io" in (p["source"] or "") else p["source"])
        lines = [
            f"{name} {ver}",
            f"  License expression: {p.get('license') or '(none given)'}",
            f"  Repository: {p.get('repository') or '-'}",
            f"  Source: {source}",
            f"  Used by: {', '.join(sorted(programs))}",
            f"  Provenance: {provenance}",
        ]
        if files:
            lines.append("  Bundled license/notice/attribution files:")
            lines += [f"    {folder}/{saved}" for saved in bundled]
        entries.append("\n".join(lines))
    header = f"""THIRD-PARTY NOTICES: OpenFO3 for Windows (x64)

OpenFO3 (by RPGKING The Mad Modder) is built with the {len(entries)} external
packages below: the game program (OpenFO3.exe, the "viewer") and the
launcher (OpenFO3 Launcher.exe, the "launcher"), as cargo resolves them for
{PLATFORM}, including the packages used only while building. Names,
versions and licence expressions come from cargo metadata of the released
source tree.

Each package's licence, notice and attribution files are in
licenses/<package>-<version>/ beside this file. No package source and no game
files are included.
"""
    (NOTICES / "THIRD-PARTY-NOTICES.txt").write_text(
        header + "\n" + "\n\n".join(entries) + "\n", encoding="utf-8")
    locks = []
    for program in ("viewer", "launcher"):
        lock = SOURCE / program / "Cargo.lock"
        locks.append(f"{hashlib.sha256(lock.read_bytes()).hexdigest()}  {program}/Cargo.lock")
    (NOTICES / "DEPENDENCY-LOCK.sha256").write_text("\n".join(locks) + "\n", encoding="utf-8")
    # Into the source tree's distribution/ too.
    dist = SOURCE / "distribution"
    for f in ("THIRD-PARTY-NOTICES.txt", "DEPENDENCY-LOCK.sha256"):
        shutil.copy2(NOTICES / f, dist / f)
    if (dist / "licenses").exists():
        shutil.rmtree(dist / "licenses")
    shutil.copytree(licenses, dist / "licenses")
    print(f"{len(entries)} packages; {len(missing)} without a licence file")
    for m in missing:
        print("  missing:", m)


# ---------------------------------------------------------------- package


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def find_iscc():
    for p in [os.environ.get("ISCC"),
              Path(os.environ.get("LOCALAPPDATA", "")) / "Programs" / "Inno Setup 6" / "ISCC.exe",
              Path(r"C:\Program Files (x86)\Inno Setup 6\ISCC.exe"),
              Path(r"C:\Program Files\Inno Setup 6\ISCC.exe")]:
        if p and Path(p).is_file():
            return Path(p)
    return None


def package():
    start_out()
    ver = version()
    fresh(APP)
    DIST.mkdir(exist_ok=True)
    shutil.copy2(TARGET / "viewer" / "release" / "nv-viewer.exe", APP / GAME_EXE)
    shutil.copy2(TARGET / "launcher" / "release" / "openfo3-launcher.exe", APP / LAUNCHER_EXE)
    shutil.copy2(SOURCE / "distribution" / "README.txt", APP / "README.txt")
    shutil.copy2(SOURCE / "LICENSE", APP / "LICENSE.txt")
    shutil.copy2(SOURCE / "LEGAL.md", APP / "LEGAL.md")
    shutil.copy2(NOTICES / "THIRD-PARTY-NOTICES.txt", APP / "THIRD-PARTY-NOTICES.txt")
    shutil.copytree(NOTICES / "licenses", APP / "licenses")
    # The public repository's commit when the source tree is one (commit it
    # before packaging), else the working tree's.
    repo = SOURCE if (SOURCE / ".git").exists() else ENGINE
    head = subprocess.run(["git", "rev-parse", "--short", "HEAD"], cwd=repo,
                          capture_output=True, text=True).stdout.strip()
    dirty = subprocess.run(["git", "status", "--porcelain"], cwd=repo,
                           capture_output=True, text=True).stdout.strip()
    (APP / "BUILD.txt").write_text(
        f"OpenFO3 {ver}\n"
        "by RPGKING The Mad Modder\n"
        f"Built: {datetime.date.today().isoformat()}\n"
        f"Source: {head}{' with uncommitted changes' if dirty else ''}\n"
        "Platform: Windows x64\n"
        f"{GAME_EXE} SHA256: {sha256(APP / GAME_EXE)}\n"
        f"{LAUNCHER_EXE} SHA256: {sha256(APP / LAUNCHER_EXE)}\n",
        encoding="utf-8",
    )
    zip_path = DIST / f"OpenFO3-{ver}-win64.zip"
    with zipfile.ZipFile(zip_path, "w", zipfile.ZIP_DEFLATED, compresslevel=9, strict_timestamps=False) as z:
        for f in sorted(APP.rglob("*")):
            if f.is_file():
                z.write(f, Path("OpenFO3") / f.relative_to(APP))
    print("zip:", zip_path, f"{zip_path.stat().st_size / 1e6:.1f} MB")
    iscc = find_iscc()
    if iscc is None:
        print("Inno Setup 6 not found: no installer (set ISCC to its ISCC.exe)")
        return
    run([sys.executable, PROJECT / "tools" / "release" / "make_icon.py", "--wizard", SOURCE / "installer"], PROJECT)
    run([iscc, "/Q", f"/DAppVersion={ver}", f"/DSourceDir={APP}", f"/DOutputDir={DIST}",
         SOURCE / "installer" / "OpenFO3.iss"], SOURCE / "installer")
    setup = DIST / f"OpenFO3-{ver}-setup.exe"
    print("installer:", setup, f"{setup.stat().st_size / 1e6:.1f} MB")


def main():
    args = sys.argv[1:]
    if not args or args[0] not in ("all", "source", "build", "notices", "package"):
        sys.exit(__doc__)
    step, all_tests = args[0], "--all-tests" in args
    if step in ("all", "source"):
        source()
    if step in ("all", "build"):
        build(all_tests)
    if step in ("all", "notices"):
        notices()
    if step in ("all", "package"):
        package()


if __name__ == "__main__":
    main()
