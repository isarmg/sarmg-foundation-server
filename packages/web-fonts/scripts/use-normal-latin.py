"""Select the pinned official Normal NL upright faces; leave CJK shards intact."""
import hashlib
import io
import json
import re
import sys
import zipfile
from pathlib import Path
from fontTools.ttLib import TTFont

root = Path(__file__).resolve().parents[1]
archive = Path(sys.argv[1]).read_bytes()
digest = "6fd6c8668657d4b55f108a115e59fa7a081757f29f599ebfc30a8558b85725bf"
assert hashlib.sha256(archive).hexdigest() == digest
manifest = json.loads((root / "provenance.json").read_text())
css = [line for line in (root / "fonts.css").read_text().splitlines() if 'src:url("./cjk/' in line]
covered = set()
for line in css:
    for part in re.search(r"unicode-range:([^}]+)", line)[1].split(","):
        bounds = part.removeprefix("U+").split("-")
        covered.update(range(int(bounds[0], 16), int(bounds[-1], 16) + 1))
def ranges(points):
    result = []
    for point in sorted(points):
        if result and point == result[-1][1] + 1:
            result[-1][1] = point
        else:
            result.append([point, point])
    return ",".join(f"U+{start:X}" + (f"-{end:X}" if end != start else "") for start, end in result)
with zipfile.ZipFile(io.BytesIO(archive)) as source:
    for style, weight in [("Regular", 400), ("Bold", 700)]:
        data = source.read(f"MapleMonoNormalNL-{style}.ttf.woff2")
        font = TTFont(io.BytesIO(data))
        assert font["post"].italicAngle == 0
        assert font["OS/2"].usWeightClass == weight
        name = f"MapleMonoNormalNL-{style}.woff2"
        (root / name).write_bytes(data)
        manifest["assets"][name] = hashlib.sha256(data).hexdigest()
        css.append(f'@font-face{{font-family:"Sarmg Maple";src:url("./{name}") format("woff2");font-style:normal;font-weight:{weight};font-display:block;unicode-range:{ranges(set(font.getBestCmap()) - covered)}}}')
    data = source.read("LICENSE.txt")
    (root / "NORMAL-LICENSE.txt").write_bytes(data)
    manifest["assets"]["NORMAL-LICENSE.txt"] = hashlib.sha256(data).hexdigest()
features = 'font-variant-ligatures:none;font-feature-settings:"calt" 0,"liga" 0,"clig" 0,"dlig" 0;font-synthesis:none'
css += [f':root{{--sarmg-font-ui:"Sarmg Maple",ui-monospace,monospace;--sarmg-font-mono:"Sarmg Maple",ui-monospace,monospace;{features}}}',
        f'body,button,input,select,textarea,code,pre,kbd,samp{{font-family:var(--sarmg-font-ui);font-style:normal;{features}}}']
data = ("\n".join(css) + "\n").encode()
(root / "fonts.css").write_bytes(data)
manifest["assets"]["fonts.css"] = hashlib.sha256(data).hexdigest()
manifest["latin"] = {"variant": "Normal NL", "style": "upright", "handwriting": False,
    "release": "v7.9", "url": "https://github.com/subframe7536/maple-font/releases/download/v7.9/MapleMonoNormalNL-Woff2.zip", "sha256": digest}
(root / "provenance.json").write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + "\n")
print("Selected official Normal NL Regular/Bold, italic angle 0; CJK unchanged")
