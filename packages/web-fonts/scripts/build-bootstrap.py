"""Build compact first-paint faces from the verified Latin and CJK assets."""
import hashlib
import json
import tempfile
from pathlib import Path

from fontTools import subset
from fontTools.merge import Merger
from fontTools.ttLib import TTFont

ROOT = Path(__file__).resolve().parents[1]
FAMILY = "Sarmg Maple Bootstrap"


def ranges(points):
    result = []
    for point in sorted(points):
        if result and point == result[-1][1] + 1:
            result[-1][1] = point
        else:
            result.append([point, point])
    return ",".join(
        f"U+{start:X}" + (f"-{end:X}" if end != start else "")
        for start, end in result
    )


def subset_file(source, keep, destination):
    font = TTFont(source, recalcTimestamp=False)
    options = subset.Options()
    options.layout_features = []
    options.name_IDs = [0, 1, 2, 3, 4, 5, 6, 13, 14]
    worker = subset.Subsetter(options=options)
    worker.populate(unicodes=keep)
    worker.subset(font)
    font.flavor = None
    font.save(destination)


def main():
    manifest = json.loads((ROOT / "provenance.json").read_text())
    requested = {ord(character) for character in (ROOT / "bootstrap-glyphs.txt").read_text().strip()}
    faces = []
    generated = {}
    for style, weight in [("Regular", 400), ("Bold", 700)]:
        latin_path = ROOT / f"MapleMonoNormalNL-{style}.woff2"
        sources = [latin_path, *sorted((ROOT / "cjk").glob(f"MapleMonoNL-CN-{style}-*.woff2"))]
        latin = set(TTFont(latin_path).getBestCmap())
        available = set().union(*(set(TTFont(source).getBestCmap()) for source in sources))
        expected = latin | (requested & available)
        assert requested <= available, f"bootstrap glyphs missing from {style} source"
        with tempfile.TemporaryDirectory() as temporary:
            fragments = []
            for index, source in enumerate(sources):
                source_points = set(TTFont(source).getBestCmap())
                keep = expected & source_points
                if not keep:
                    continue
                destination = Path(temporary) / f"{index}.ttf"
                subset_file(source, keep, destination)
                fragments.append(str(destination))
            merged = Merger().merge(fragments)
            merged.flavor = "woff2"
            name = f"MapleMonoBootstrap-{style}.woff2"
            destination = ROOT / name
            merged.save(destination)
        encoded = destination.read_bytes()
        assert encoded.startswith(b"wOF2") and len(encoded) <= 256 * 1024
        assert set(TTFont(destination).getBestCmap()) == expected
        manifest["assets"][name] = hashlib.sha256(encoded).hexdigest()
        generated[name] = {"bytes": len(encoded), "codepoints": len(expected)}
        faces.append(
            f'@font-face{{font-family:"{FAMILY}";src:url("./{name}") format("woff2");'
            f"font-style:normal;font-weight:{weight};font-display:block;unicode-range:{ranges(expected)}}}"
        )

    css_path = ROOT / "fonts.css"
    lines = [line for line in css_path.read_text().splitlines() if FAMILY not in line]
    root_index = next(index for index, line in enumerate(lines) if line.startswith(":root{"))
    lines[root_index:root_index] = faces
    lines = [
        line.replace(
            '--sarmg-font-ui:"Sarmg Maple",',
            f'--sarmg-font-ui:"{FAMILY}","Sarmg Maple",',
        ).replace(
            '--sarmg-font-mono:"Sarmg Maple",',
            f'--sarmg-font-mono:"{FAMILY}","Sarmg Maple",',
        )
        for line in lines
    ]
    css_path.write_text("\n".join(lines) + "\n")
    manifest["assets"]["fonts.css"] = hashlib.sha256(css_path.read_bytes()).hexdigest()
    manifest["bootstrap"] = {
        "family": FAMILY,
        "glyphs_sha256": hashlib.sha256((ROOT / "bootstrap-glyphs.txt").read_bytes()).hexdigest(),
        "ui_codepoints": len(requested),
        "faces": generated,
    }
    (ROOT / "provenance.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(f"Built two first-paint faces for {len(requested)} UI codepoints")


if __name__ == "__main__":
    main()
