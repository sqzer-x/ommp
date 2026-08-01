#!/usr/bin/env bash
# Capture ommp screenshots for the landing page.
#
# Runs the real binary in a real terminal: album art only renders as an image
# under a graphics protocol, and Ghostty is the only installed terminal that
# speaks one. Anything else would show the block-character fallback.
#
# The window is fullscreen for about fifteen seconds and then closes. Your
# player settings are backed up and restored — the app rewrites them on exit,
# and this script deliberately changes a few for the shot.
set -euo pipefail

export XDG_RUNTIME_DIR=/run/user/1000
export WAYLAND_DISPLAY=wayland-1

HERE=$(cd "$(dirname "$0")" && pwd)
OUT=${OUT:-/tmp/ommp-shots}
FONT_SIZE=${FONT_SIZE:-20}
OMMP=${OMMP:-/home/himesama/Documents/ommp/target/release/ommp}
MONITOR=${MONITOR:-DP-1}
STATE="$HOME/.config/ommp/state.json"

mkdir -p "$OUT"
BAK="$OUT/state.json.bak"
[ -f "$STATE" ] && [ ! -f "$BAK" ] && cp -a "$STATE" "$BAK"

cleanup() {
  pkill -x ommp 2>/dev/null || true
  sleep 0.4
  [ -f "$BAK" ] && cp -a "$BAK" "$STATE"
}
trap cleanup EXIT

python3 "$HERE/prep-state.py" "$STATE"

# --background matches the logo's #0A051E so padding is not a different black.
# --palette pins the ANSI colours the app draws selections and the progress bar
# with, so the shot does not depend on whatever theme happens to be configured.
ghostty \
  --font-family="JetBrainsMono Nerd Font" \
  --font-size="$FONT_SIZE" \
  --background=0A051E \
  --foreground=E8E6F0 \
  --palette=6=#00ffff --palette=14=#00ffff \
  --palette=5=#c850ff --palette=13=#c850ff \
  --fullscreen=true \
  --window-decoration=none \
  --window-padding-x=16 --window-padding-y=12 \
  --confirm-close-surface=false \
  -e "$OMMP" &

# The splash runs a fixed two seconds; wait it out plus a beat for the first draw.
sleep 5

shoot() { grim -o "$MONITOR" "$OUT/$1.png" && echo "  -> $1.png"; }

# Mute the player's audio stream at the server, not in the app: the volume
# meter is part of the shot, so it has to read a real level while staying
# silent. The stream exists from startup, before anything plays.
SINK_INPUT=""
for _ in $(seq 20); do
  SINK_INPUT=$(pactl -f json list sink-inputs 2>/dev/null \
    | python3 -c 'import json,sys
try:
    for si in json.load(sys.stdin):
        if si["properties"].get("application.process.binary") == "ommp":
            print(si["index"]); break
except Exception:
    pass')
  [ -n "$SINK_INPUT" ] && break
  sleep 0.25
done
if [ -n "$SINK_INPUT" ]; then
  pactl set-sink-input-mute "$SINK_INPUT" 1
  echo "  muted sink-input $SINK_INPUT"
else
  echo "  WARNING: could not find the player audio stream; it will be audible" >&2
fi

# Start playback so the status bar reads Playing with a partly filled progress
# bar, rather than Stopped at 0:00.
wtype " "
sleep 9

shoot 01-main

wtype -M ctrl -k h -m ctrl; sleep 0.8; shoot 02-help;   wtype -k Escape; sleep 0.5
wtype -M ctrl -k s -m ctrl; sleep 0.8; wtype -d 40 "yorushika"; sleep 1.0
shoot 03-search; wtype -k Escape; sleep 0.5

echo "done: $OUT"
