+++
title = "ommp — Oh My Music Player"
description = "A terminal music player written in Rust. It plays the files already on your disk, in the terminal you already have open."
template = "landing.html"

[extra]
kicker    = "Terminal music player"
headline  = "Play your own music"
headline2 = "without leaving the terminal."
lede = "ommp plays the files already on your disk: the CDs you ripped, the albums you went looking for, the folder you have been adding to for years. One command, and it plays."

install_primary_label   = "Arch Linux"
install_primary         = "yay -S ommp"
install_secondary_label = "Anywhere else"
install_secondary       = "cargo install ommp"

# ── 1. The artwork ───────────────────────────────────────────────────────
art_title  = "That is your album art, in a terminal."
art_lede   = "The cover comes out of the file itself: the same image your tags have been carrying since you ripped the disc or paid for the download."
art_body_1 = "In Ghostty, Kitty, WezTerm, Konsole, foot, xterm and Contour, ommp hands the artwork to the terminal's own graphics protocol and it arrives as a real image, grain and gradient intact, sitting in a shell next to your prompt. Terminals without one draw the same sleeve in block characters, coarser, and still recognisably the record you know."
art_body_2 = "Beside it, thirteen lines saying exactly what is playing, including the ones you went to some trouble over: <b>48.0 kHz</b>, <b>24-bit</b>, <b>Stereo</b>. Nothing there was fetched or guessed. All of it came off your disk, where it has been the whole time."

# ── 2. Nothing to start ──────────────────────────────────────────────────
start_title = "Nothing to start first."
start_lede  = "No daemon in the background, no service to enable, no file to write before the first note. You type <code>ommp</code>, it reads <code>~/Music</code>, and it plays."
start_body  = "Quit it and nothing of it keeps running. Everything it plays is already on your disk, so there is nothing to sign into, nothing to subscribe to, and nothing that stops working when a company changes its mind."
skipped = [
  "Create an account",
  "Import your library",
  "systemctl --user enable mpd",
  "~/.config/mpd/mpd.conf",
]

# ── 3. Browse ────────────────────────────────────────────────────────────
browse_title  = "Ask it seven ways, or just type."
browse_lede   = "Press <kbd>1</kbd> to <kbd>7</kbd> to move along the tab strip: the queue everything lands in, the directory tree you built yourself, then artists, albums, genre, format and playlists. The same library, seven questions."
browse_body_1 = "Directories keeps the folders you organised, in the order you organised them. Genre is whatever you wrote in it. Format counts what you have of each: FLAC, MP3, M4A, OGG, WAV, Opus, AAC, WMA."
browse_body_2 = "When you already know the name, <kbd>Ctrl</kbd>+<kbd>S</kbd> and start typing. The list narrows on every keystroke. Put <code>artist:</code>, <code>album:</code> or <code>genre:</code> in front to say which you meant, or type <code>*.flac</code> for when you own something twice and want the good copy."

# ── 4. Nothing lost ──────────────────────────────────────────────────────
lost_title = "Nothing quietly disappears."
lost_lede  = "The small decisions are the ones you notice on the second day."
lost_close = "Volume, shuffle, repeat, your playlists and the pane widths you dragged all come back the way you left them. The track does not. You choose that again."

# ── 5. Keys ──────────────────────────────────────────────────────────────
keys_title  = "h j k l, Space, n."
keys_lede   = "The keys already under your hands in every other window you have open."
keys_body_1 = "<kbd>j</kbd> and <kbd>k</kbd> move the list, <kbd>g</kbd> and <kbd>G</kbd> jump to the ends, <kbd>h</kbd> and <kbd>l</kbd> move between panes and <kbd>Tab</kbd> does the same. <kbd>Space</kbd> plays and pauses, <kbd>n</kbd> and <kbd>N</kbd> step through the queue, <kbd>+</kbd> and <kbd>-</kbd> take the volume, the arrows seek, <kbd>s</kbd> shuffles, <kbd>r</kbd> repeats, and <kbd>b</kbd> drops the playing track into a playlist. <kbd>Ctrl</kbd>+<kbd>H</kbd> lists every key there is."
keys_body_2 = "The mouse works too, when your hand is already there: click a tab, double-click a track, <kbd>Ctrl</kbd>-drag a border to resize a pane."

# ── 6. Install ───────────────────────────────────────────────────────────
install_title  = "Install it."
install_close  = "Then put some music in <code>~/Music</code> and run <code>ommp</code>."
splash_note    = "The notes behind the top of this page are the ones ommp scatters across its splash screen, generated from the same hash the program uses."

# The seven tabs, each in the accent colour it actually uses in
# src/ui/widgets/tab_bar.rs, so the page and the app agree.
[[extra.tabs]]
name = "Queue"
color = "#64DCFF"
[[extra.tabs]]
name = "Directories"
color = "#78FFB4"
[[extra.tabs]]
name = "Artists"
color = "#FFB464"
[[extra.tabs]]
name = "Albums"
color = "#C882FF"
[[extra.tabs]]
name = "Genre"
color = "#FF7896"
[[extra.tabs]]
name = "Format"
color = "#78C8FF"
[[extra.tabs]]
name = "Playlists"
color = "#FFDC64"

[[extra.details]]
color = "#FFB464"
body = "A file with no tags at all lands under <b>Unknown Artist</b>, where you can still find it."
[[extra.details]]
color = "#C882FF"
body = "An album is paired with its album artist where one is tagged and the track artist where one is not, so two records called <i>Greatest Hits</i> stay two records."
[[extra.details]]
color = "#FFDC64"
body = "A playlist entry pointing at a drive that was not mounted when you started is left alone rather than pruned, and it works again when the drive comes back."
[[extra.details]]
color = "#78FFB4"
body = "Add files to the folder while ommp is running and they turn up. Take some away and they go."
+++
