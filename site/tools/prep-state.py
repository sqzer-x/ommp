"""Shoot-only player settings; the caller restores the originals afterwards."""
import json, sys
p = sys.argv[1]
d = json.load(open(p))
d["volume"] = 0.72           # a realistic meter; silence comes from muting the stream
d["pane_widths"] = [20, 50, 30]  # wider right column: track info stops wrapping
d["right_split"] = 46        # a little more room for the info fields
d["info_view"] = "AlbumArt"
json.dump(d, open(p, "w"), indent=2)
