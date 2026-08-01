+++
title = "ommp — Oh My Music Player"
description = "A terminal music player written in Rust. No MPD, no daemon — point it at ~/Music and press play."
template = "landing.html"

[extra]
kicker    = "Terminal music player"
headline  = "Point it at ~/Music."
headline2 = "Press play."
lede = "No MPD. No daemon. No config file. One binary that reads your tags, draws your album art in the terminal, and keeps your playlists and layout between sessions."

install_primary_label = "Arch Linux"
install_primary       = "yay -S ommp"
install_secondary_label = "Anywhere else"
install_secondary       = "cargo install ommp"

# Each feature is keyed to the accent colour its tab actually uses in
# src/ui/widgets/tab_bar.rs, so the page and the app agree.
[[extra.features]]
name  = "Queue"
color = "#64DCFF"
body  = "Everything queues by default. Add an album without losing what is playing, drop a track with <code>d</code>, clear it with <code>c</code>."

[[extra.features]]
name  = "Directories"
color = "#78FFB4"
body  = "Browse the folders you actually organised, not a database's idea of them."

[[extra.features]]
name  = "Artists"
color = "#FFB464"
body  = "Grouped by the track artist, exactly as the file is tagged. Untagged files land under one heading instead of vanishing."

[[extra.features]]
name  = "Albums"
color = "#C882FF"
body  = "Paired with the album artist where it is tagged, the track artist where it is not — so two records called <em>Greatest Hits</em> stay two records."

[[extra.features]]
name  = "Genre"
color = "#FF7896"
body  = "Whatever your tags say. ommp reads them, it does not invent them."

[[extra.features]]
name  = "Format"
color = "#78C8FF"
body  = "FLAC, MP3, M4A, OGG, WAV, Opus, AAC, WMA — with a count beside each."

[[extra.features]]
name  = "Playlists"
color = "#FFDC64"
body  = "Create, rename, star a track with <code>b</code>. Saved between sessions, and entries survive a drive that was not mounted."

[[extra.numbers]]
value = "18 ms"
label = "to open a 5,000-track library"
note  = "Tags are cached and re-read only when a file's timestamp or size changes. The first scan runs across every core."

[[extra.numbers]]
value = "0.3 ms"
label = "per frame at 50,000 tracks"
note  = "Flat, not linear — the browse views are built once when the library loads, not on every redraw."

[[extra.numbers]]
value = "7 MB"
label = "one binary, three libraries"
note  = "Links against alsa-lib, glibc and gcc-libs. Nothing to configure, nothing running in the background."

[[extra.keys]]
key  = "Space"
desc = "Play / pause"
[[extra.keys]]
key  = "n / N"
desc = "Next / previous"
[[extra.keys]]
key  = "+ / -"
desc = "Volume"
[[extra.keys]]
key  = "s / r"
desc = "Shuffle / repeat"
[[extra.keys]]
key  = "1–7"
desc = "Switch tab"
[[extra.keys]]
key  = "Tab"
desc = "Cycle pane focus"
[[extra.keys]]
key  = "Ctrl+S"
desc = "Search"
[[extra.keys]]
key  = "Ctrl+H"
desc = "Every other key"

[[extra.protocols]]
name  = "Kitty graphics"
terms = "Ghostty, Kitty, WezTerm, Konsole"
[[extra.protocols]]
name  = "Sixel"
terms = "foot, WezTerm, xterm, Contour"
[[extra.protocols]]
name  = "iTerm2"
terms = "WezTerm, Konsole"
+++
