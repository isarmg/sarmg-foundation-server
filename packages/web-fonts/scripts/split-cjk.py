"""Refine generated CJK shards to fit the unchanged 256 KiB native asset budget."""
import hashlib
import io
import json
from pathlib import Path
import runpy

from fontTools import subset
from fontTools.ttLib import TTFont

ROOT = Path(__file__).resolve().parents[1]
ranges = runpy.run_path(str(ROOT / "scripts/build-cjk.py"))["ranges"]
provenance = json.loads((ROOT / "provenance.json").read_text())
css = (ROOT / "fonts.css").read_text()
for name, expected in list(provenance["assets"].items()):
    if not name.startswith("cjk/"):
        continue
    path = ROOT / name
    assert path.is_file() and not path.is_symlink()
    data = path.read_bytes()
    assert hashlib.sha256(data).hexdigest() == expected
    if len(data) <= 240 * 1024:
        continue
    points = sorted(TTFont(io.BytesIO(data)).getBestCmap())
    weight = 700 if "-Bold-" in name else 400
    stem = name.rsplit("-", 2)[0]
    replacement = []

    def emit(chunk):
        font = TTFont(io.BytesIO(data), recalcTimestamp=False)
        options = subset.Options()
        options.layout_features = []
        worker = subset.Subsetter(options=options)
        worker.populate(unicodes=chunk)
        worker.subset(font)
        font.flavor = "woff2"
        output = io.BytesIO()
        font.save(output)
        encoded = output.getvalue()
        if len(encoded) > 240 * 1024:
            middle = len(chunk) // 2
            assert middle > 0
            emit(chunk[:middle])
            emit(chunk[middle:])
            return
        child = f"{stem}-{chunk[0]:X}-{chunk[-1]:X}.woff2"
        assert not (ROOT / child).exists(), child
        (ROOT / child).write_bytes(encoded)
        assert set(TTFont(io.BytesIO(encoded)).getBestCmap()) == set(chunk)
        provenance["assets"][child] = hashlib.sha256(encoded).hexdigest()
        replacement.append(f'@font-face{{font-family:"Sarmg Maple";src:url("./{child}") format("woff2");font-style:normal;font-weight:{weight};font-display:swap;unicode-range:{ranges(chunk)}}}')

    middle = len(points) // 2
    emit(points[:middle])
    emit(points[middle:])
    original = next(line for line in css.splitlines() if f'url("./{name}")' in line)
    css = css.replace(original, "\n".join(replacement))
    del provenance["assets"][name]
    path.unlink()  # Only this generator's verified intermediate shard is replaced.
(ROOT / "fonts.css").write_text(css)
provenance["assets"]["fonts.css"] = hashlib.sha256((ROOT / "fonts.css").read_bytes()).hexdigest()
provenance["cjk"]["maximum_asset_bytes"] = 240 * 1024
(ROOT / "provenance.json").write_text(json.dumps(provenance, indent=2) + "\n")
print("CJK source snapshot refined without dropping codepoints or increasing asset budgets")
