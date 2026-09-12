"""Build reviewed, unicode-ranged CJK WOFF2 assets (fonttools 4.60.1, brotli 1.1.0)."""
import hashlib
import io
import json
from pathlib import Path
import sys
import zipfile

from fontTools import subset
from fontTools.ttLib import TTFont

ROOT = Path(__file__).resolve().parents[1]
ARCHIVE_SHA256 = "80bf6db8920b2999d900e08f9e5031f7686baeb72dbdb80891f2c1ae9cec606f"


def ranges(points):
    result = []
    start = end = points[0]
    for value in points[1:]:
        if value == end + 1:
            end = value
        else:
            result.append(f"U+{start:X}" if start == end else f"U+{start:X}-{end:X}")
            start = end = value
    result.append(f"U+{start:X}" if start == end else f"U+{start:X}-{end:X}")
    return ",".join(result)


def main():
    archive_bytes = Path(sys.argv[1]).read_bytes()
    assert hashlib.sha256(archive_bytes).hexdigest() == ARCHIVE_SHA256
    destination = ROOT / "cjk"
    destination.mkdir(exist_ok=True)
    assert not list(destination.iterdir()), "refuse to overwrite an existing font generation"
    latin = TTFont(ROOT / "MapleMono-Italic.woff2").getBestCmap()
    faces = []
    assets = {}
    coverage = []
    with zipfile.ZipFile(io.BytesIO(archive_bytes)) as archive:
        (ROOT / "CJK-LICENSE.txt").write_bytes(archive.read("LICENSE.txt"))
        for style, weight in [("Regular", 400), ("Bold", 700)]:
            source = archive.read(f"MapleMonoNL-CN-{style}.ttf")
            points = sorted(set(TTFont(io.BytesIO(source)).getBestCmap()) - set(latin))
            assert all(ord(character) in points for character in "中文日本語かなカナ")
            if not coverage:
                coverage = points
            else:
                assert points == coverage

            def emit(chunk):
                font = TTFont(io.BytesIO(source), recalcTimestamp=False)
                options = subset.Options()
                options.flavor = "woff2"
                options.layout_features = []
                options.name_IDs = [0, 1, 2, 3, 4, 5, 6, 13, 14]
                worker = subset.Subsetter(options=options)
                worker.populate(unicodes=chunk)
                worker.subset(font)
                font.flavor = "woff2"
                output = io.BytesIO()
                font.save(output)
                data = output.getvalue()
                if len(data) > 500 * 1024:
                    middle = len(chunk) // 2
                    assert middle > 0
                    emit(chunk[:middle])
                    emit(chunk[middle:])
                    return
                name = f"cjk/MapleMonoNL-CN-{style}-{chunk[0]:X}-{chunk[-1]:X}.woff2"
                (ROOT / name).write_bytes(data)
                assert set(TTFont(io.BytesIO(data)).getBestCmap()) == set(chunk)
                assets[name] = hashlib.sha256(data).hexdigest()
                faces.append(f'@font-face{{font-family:"Sarmg Maple";src:url("./{name}") format("woff2");font-style:normal;font-weight:{weight};font-display:block;unicode-range:{ranges(chunk)}}}')
                print(f"{name}: {len(data)} bytes", flush=True)

            for offset in range(0, len(points), 2048):
                emit(points[offset:offset + 2048])
    latin_ranges = ranges(sorted(latin))
    # Composite unicode faces must have identical weight descriptors. A variable
    # 100..900 face alongside static CJK 400/700 makes browsers select a family
    # subset lacking Latin glyphs and silently fall back to a system font.
    for style in ["normal", "italic"]:
        for weight in [400, 700]:
            faces.append(f'@font-face{{font-family:"Sarmg Maple";src:url("./MapleMono-Italic.woff2") format("woff2");font-style:{style};font-weight:{weight};font-display:block;unicode-range:{latin_ranges}}}')
    faces.append(':root{--sarmg-font-ui:"Sarmg Maple",ui-monospace,monospace;--sarmg-font-mono:"Sarmg Maple",ui-monospace,monospace;font-variant-ligatures:none;font-feature-settings:"calt" 0,"liga" 0,"clig" 0,"dlig" 0,"ss06" 1}')
    faces.append('body,button,input,select,textarea,code,pre,kbd,samp{font-family:var(--sarmg-font-ui);font-variant-ligatures:none;font-feature-settings:"calt" 0,"liga" 0,"clig" 0,"dlig" 0,"ss06" 1}')
    (ROOT / "fonts.css").write_text("\n".join(faces) + "\n")
    provenance = json.loads((ROOT / "provenance.json").read_text())
    provenance["cjk"] = {"upstream": "https://github.com/subframe7536/maple-font/releases/tag/v7.9", "archive": "MapleMonoNL-CN-unhinted.zip", "sha256": ARCHIVE_SHA256, "fonttools": "4.60.1", "brotli": "1.1.0", "additional_codepoints": len(coverage), "weights": [400, 700]}
    for name in ["fonts.css", "OFL.txt", "CJK-LICENSE.txt"]:
        assets[name] = hashlib.sha256((ROOT / name).read_bytes()).hexdigest()
    provenance["assets"].update(assets)
    (ROOT / "provenance.json").write_text(json.dumps(provenance, indent=2) + "\n")


if __name__ == "__main__":
    main()
