#!/usr/bin/env bash
# scripts/release.sh — cut a new bubble release in one step.
#
# Usage:
#   scripts/release.sh 0.4.22 ["One-line release note for metainfo.xml"] [notes.md]
#
# notes.md (Markdown) becomes the annotated tag message, which CI publishes
# as the GitHub release body. Without it the tag just says "Release X.Y.Z".
#
# What it does, in order:
#   1. Bumps every in-tree version reference:
#        - CMakeLists.txt   project(bubble VERSION X.Y.Z ...)
#        - io.github.tattvaorg.Bubble.yml     bubble source `tag: vX.Y.Z`
#        - dist/*.metainfo.xml   screenshot URLs + new <release> entry
#   2. Runs a quick cmake build as a smoke test (if build/ exists).
#   3. Creates a "chore: release vX.Y.Z" commit on the current branch.
#   4. Creates an annotated git tag vX.Y.Z on that commit.
#   5. Stops BEFORE pushing — prints the exact push commands so you can
#      review `git log -1` / `git show HEAD` first and then push by hand.
#
# Rerunning is idempotent as long as the tag doesn't already exist.
set -euo pipefail

if [ $# -lt 1 ] || [ "${1:-}" = "-h" ] || [ "${1:-}" = "--help" ]; then
    sed -n '2,/^set -euo/p' "$0" | sed '$d'
    exit 0
fi

VERSION="$1"
NOTE="${2:-}"
NOTES_FILE="${3:-}"

if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "release.sh: version must be X.Y.Z (got '$VERSION')" >&2
    exit 1
fi

TAG="v$VERSION"

REPO_ROOT=$(git rev-parse --show-toplevel)
cd "$REPO_ROOT"

CMAKE_FILE="CMakeLists.txt"
FLATPAK_FILE="io.github.tattvaorg.Bubble.yml"
META_FILE="dist/io.github.tattvaorg.Bubble.metainfo.xml"

for f in "$CMAKE_FILE" "$FLATPAK_FILE" "$META_FILE"; do
    [ -f "$f" ] || { echo "release.sh: missing $f" >&2; exit 1; }
done

if [ -n "$(git status --porcelain --untracked-files=no --ignore-submodules=all)" ]; then
    echo "release.sh: working tree has uncommitted tracked changes. Commit or stash first." >&2
    git status --short --ignore-submodules=all
    exit 1
fi

if [ -n "$NOTES_FILE" ] && [ ! -s "$NOTES_FILE" ]; then
    echo "release.sh: notes file '$NOTES_FILE' is missing or empty" >&2
    exit 1
fi

if git rev-parse --verify --quiet "refs/tags/$TAG" >/dev/null; then
    echo "release.sh: tag $TAG already exists. Pick a new version or delete it first." >&2
    exit 1
fi

echo "==> Cutting release $VERSION"

# 1. CMakeLists.txt ---------------------------------------------------------
python3 - "$CMAKE_FILE" "$VERSION" <<'PY'
import re, sys, pathlib
path, version = pathlib.Path(sys.argv[1]), sys.argv[2]
text = path.read_text()
new, n = re.subn(
    r'(project\(bubble\s+VERSION\s+)\d+\.\d+\.\d+',
    rf'\g<1>{version}',
    text,
)
if n != 1:
    raise SystemExit(f"release.sh: expected 1 project(bubble VERSION ...) in {path}, found {n}")
path.write_text(new)
PY

# 2. Flatpak manifest -------------------------------------------------------
#    Only the bubble git source gets bumped (not qtwayland / wl-clipboard /
#    bundled fd etc.). We locate it by the immediately-preceding URL line.
python3 - "$FLATPAK_FILE" "$TAG" <<'PY'
import re, sys, pathlib
path, tag = pathlib.Path(sys.argv[1]), sys.argv[2]
text = path.read_text()
new, n = re.subn(
    r'(url:\s*https://github\.com/TattvaOrg/Bubble\.git\s*\n(?:[^\n]*\n)*?\s*tag:\s*)v\d+\.\d+\.\d+',
    rf'\g<1>{tag}',
    text,
)
if n != 1:
    raise SystemExit(f"release.sh: expected 1 bubble tag: line in {path}, found {n}")
path.write_text(new)
PY

# 3. metainfo.xml -----------------------------------------------------------
#    - rewrite screenshot URLs: bubble/v<old>/docs/screenshots → bubble/v<new>/…
#    - prepend a new <release version="X.Y.Z" date="YYYY-MM-DD"> entry if
#      the version isn't already listed.
python3 - "$META_FILE" "$VERSION" "$TAG" "$NOTE" <<'PY'
import re, sys, datetime, pathlib
path, version, tag, note = (pathlib.Path(sys.argv[1]),) + tuple(sys.argv[2:])
text = path.read_text()

# Screenshot URLs
text = re.sub(
    r'(raw\.githubusercontent\.com/TattvaOrg/Bubble/)v\d+\.\d+\.\d+',
    rf'\g<1>{tag}',
    text,
)

# Release entry
if f'<release version="{version}"' not in text:
    today = datetime.date.today().isoformat()
    note = note or f'Release {version}.'
    # Escape &<> for XML
    note = (note.replace('&', '&amp;').replace('<', '&lt;').replace('>', '&gt;'))
    entry = (
        f'    <release version="{version}" date="{today}">\n'
        f'      <description>\n'
        f'        <p>{note}</p>\n'
        f'      </description>\n'
        f'    </release>\n'
    )
    new, n = re.subn(r'(<releases>\s*\n)', r'\1' + entry, text, count=1)
    if n != 1:
        raise SystemExit(f"release.sh: <releases> not found in {path}")
    text = new

path.write_text(text)
PY

# 4. Smoke build (only if a build tree already exists) ---------------------
if [ -d build ]; then
    echo "==> Smoke build (reusing existing build/)"
    cmake --build build --target bubble >/dev/null
fi

# 5. Commit + tag ----------------------------------------------------------
git add "$CMAKE_FILE" "$FLATPAK_FILE" "$META_FILE"
git commit -m "chore: release $TAG"
if [ -n "$NOTES_FILE" ]; then
    # verbatim: Markdown headings start with "#", which git would otherwise strip as comments
    git tag -a "$TAG" --cleanup=verbatim -F "$NOTES_FILE"
else
    git tag -a "$TAG" -m "Release $VERSION"
fi

echo
echo "==> Release $TAG prepared on $(git rev-parse --abbrev-ref HEAD) as $(git rev-parse --short HEAD)"
echo
echo "Push with:"
echo "    git push origin $(git rev-parse --abbrev-ref HEAD) && git push origin $TAG"
echo
echo "To undo locally before pushing:"
echo "    git tag -d $TAG && git reset --hard HEAD^"
