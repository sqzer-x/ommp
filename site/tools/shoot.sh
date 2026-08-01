#!/usr/bin/env bash
# Capture the ommp screenshots the landing page uses.
#
# Runs the real binary in a real terminal: album art only renders as an image
# under a graphics protocol, and Ghostty is the only installed terminal that
# speaks one. Anything else would show the block-character fallback.
#
# One font size for every shot. 14 is the density the program actually runs at
# here: everything fits, including all thirteen fields of the info panel with
# the sample rate and bit depth. Shooting the modals larger made the app look
# stretched next to the wide shot, which is worse than a modal with some room
# inside it.
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
# Playing here too: the status bar shows through beside every modal, and a shot
# that reads Stopped next to two that read Playing looks like a mistake.
python3 "$HERE/prep-state.py" "$STATE"
pactl set-sink-mute "$SINK" 1
launch 14
wtype " "
sleep 8
wtype -M ctrl -k h -m ctrl; sleep 1.2; grim -o "$MONITOR" "$RAW/02-help.png"
wtype -k Escape; sleep 0.5
wtype -M ctrl -k s -m ctrl; sleep 0.8; wtype -d 40 "yorushika"; sleep 1.2
grim -o "$MONITOR" "$RAW/03-search.png"
stop
pactl set-sink-mute "$SINK" 0

# ── Crop and encode ───────────────────────────────────────────────────────
# Kept at the captured 3840-wide scale rather than downsampled: the pixel grid
# of terminal text is already sharp, and resampling it both blurs the glyphs
# and *inflates* the PNG by inventing intermediate colours.
#
# Whole frames, not crops: the page shows each screenshot beside the copy that
# explains it, at a size where the full window still reads.
for n in 01-main 02-help 03-search; do
  magick "$RAW/$n.png" -strip "$RAW/$n-stripped.png"
  avifenc -q 84 -s 0 "$RAW/$n-stripped.png" "$OUT/$n.avif" >/dev/null
  rm -f "$RAW/$n-stripped.png"
done

for f in "$OUT"/0*.avif; do
  printf "  %-16s %-11s %s\n" "$(basename "$f")" \
    "$(magick identify -format '%wx%h' "$f")" "$(du -h "$f" | cut -f1)"
done
