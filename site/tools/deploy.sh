#!/usr/bin/env bash
# Build the landing page and publish it to the gh-pages branch.
#
# gh-pages holds only the built output and shares no history with main; the
# source is site/ on main. Requires sqzass on PATH (yay -S sqzass, or
# cargo install --git https://github.com/sqzer-x/sqzass).
set -euo pipefail

REPO=$(git rev-parse --show-toplevel)
WORK=${WORK:-"${TMPDIR:-/tmp}/ommp-ghpages"}

command -v sqzass >/dev/null || { echo "sqzass not found on PATH" >&2; exit 1; }

sqzass build -i "$REPO/site"
sqzass doctor -i "$REPO/site"

git -C "$REPO" worktree remove "$WORK" --force 2>/dev/null || true
rm -rf "$WORK"
git -C "$REPO" fetch origin gh-pages
git -C "$REPO" worktree add "$WORK" gh-pages

# Mirror rather than merge: files deleted from the site must disappear here too.
rsync -a --delete --exclude '.git' "$REPO/site/public/" "$WORK/"

git -C "$WORK" add -A
if git -C "$WORK" diff --cached --quiet; then
  echo "nothing changed"
else
  git -C "$WORK" commit -q -m "Deploy the landing page"
  git -C "$WORK" push origin gh-pages
  echo "published → https://ommp.sqzer.com/"
fi
git -C "$REPO" worktree remove "$WORK" --force
