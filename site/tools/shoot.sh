#!/usr/bin/env bash
# Capture ommp screenshots for the landing page, and crop them to what the
# page actually shows.
#
# Runs the real binary in a real terminal: album art only renders as an image
# under a graphics protocol, and Ghostty is the only installed terminal that
# speaks one. Anything else would show the block-character fallback.
#
# Two font sizes, because the two kinds of shot want opposite things:
#
#   14  the wide shot. Everything fits — full album names, and all thirteen
#       fields of the info panel including the sample rate and bit depth.
#       At 20 the panel stops at "Disc #" and the best detail is lost.
#   20  the modal close-ups. A modal is sized as a percentage of the terminal
#       but its contents are a fixed number of characters, so a wider terminal
#       just adds empty space inside it. 20 is where the columns sit tight.
#
# The window is fullscreen for about fifteen seconds and then closes. Your
# player settings and your default sink's mute state are backed up and
# restored — the app rewrites the former on exit, and this script changes both.
set -euo pipefail

export XDG_RUNTIME_DIR=${XDG_RUNTIME_DIR:-/run/user/1000}
export WAYLAND_DISPLAY=${WAYLAND_DISPLAY:-wayland-1}

HERE=$(cd "$(dirname "$0")" && pwd)
RAW=${RAW:-$HOME/.cache/ommp-shoot/shots}
OUT=${OUT:-$HERE/../static/img}
OMMP=${OMMP:-$HERE/../../target/release/ommp}
MONITOR=${MONITOR:-DP-1}
STATE="$HOME/.config/ommp/state.json"

mkdir -p "$RAW" "$OUT"
BAK="$RAW/state.json.bak"
[ -f "$STATE" ] && [ ! -f "$BAK" ] && cp -a "$STATE" "$BAK"

SINK=$(pactl get-default-sink)
SINK_WAS_MUTED=$(pactl get-sink-mute "$SINK" | grep -o 'yes\|no')

cleanup() {
  pkill -x ommp 2>/dev/null || true
  sleep 0.5
  [ -f "$BAK" ] && cp -a "$BAK" "$STATE"
  [ "$SINK_WAS_MUTED" = yes ] || pactl set-sink-mute "$SINK" 0
}
trap cleanup EXIT

# --background matches the logo's #0A051E so padding is not a different black.
# --palette pins the two ANSI colours the app draws selections and the progress
# bar with, so the shot matches the page's palette rather than whatever theme
# happens to be configured.
launch() {
  ghostty \
    --font-family="JetBrainsMono Nerd Font" \
    --font-size="$1" \
    --background=0A051E \
    --foreground=E8E6F0 \
    --palette=6=#00ffff --palette=14=#00ffff \
    --palette=5=#c850ff --palette=13=#c850ff \
    --fullscreen=true \
    --window-decoration=none \
    --window-padding-x=16 --window-padding-y=12 \
    --confirm-close-surface=false \
    -e "$OMMP" >/dev/null 2>&1 &
  # The splash runs a fixed two seconds; wait it out plus a beat for the draw.
  sleep 5
}

stop() { pkill -x ommp 2>/dev/null || true; sleep 1; }

# ── The wide shot ─────────────────────────────────────────────────────────
# Playback has to be running for the status bar to read Playing over a partly
# filled progress bar. rodio does not create its sink-input until that moment,
# so there is no per-stream target to mute in advance — mute the whole sink
# across the shot instead and put it back afterwards.
python3 "$HERE/prep-state.py" "$STATE"
pactl set-sink-mute "$SINK" 1
launch 14
wtype " "
sleep 9
grim -o "$MONITOR" "$RAW/01-main.png"
stop
pactl set-sink-mute "$SINK" 0

# ── The modals ────────────────────────────────────────────────────────────
# Nothing plays here, so nothing is audible.
python3 "$HERE/prep-state.py" "$STATE"
launch 20
wtype -M ctrl -k h -m ctrl; sleep 1.2; grim -o "$MONITOR" "$RAW/02-help.png"
wtype -k Escape; sleep 0.5
wtype -M ctrl -k s -m ctrl; sleep 0.8; wtype -d 40 "yorushika"; sleep 1.2
grim -o "$MONITOR" "$RAW/03-search.png"
stop

# ── Crop and encode ───────────────────────────────────────────────────────
# Kept at the captured 3840-wide scale rather than downsampled: the pixel grid
# of terminal text is already sharp, and resampling it both blurs the glyphs
# and *inflates* the PNG by inventing intermediate colours.
#
# The modals are cropped to themselves. Uncropped they are a small rectangle in
# the middle of a 4K frame, and no amount of page width makes them readable.
crop() { magick "$RAW/$1.png" -crop "$2" +repage -strip "$OUT/$3.png"; }

magick "$RAW/01-main.png" -strip "$OUT/01-main.png"
crop 02-help   3100x1700+380+250 02-help
crop 03-search 2400x1560+720+300 03-search

for f in "$OUT"/0*.png; do
  printf "  %-16s %-11s %s\n" "$(basename "$f")" \
    "$(magick identify -format '%wx%h' "$f")" "$(du -h "$f" | cut -f1)"
done
