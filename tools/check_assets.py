#!/usr/bin/env python3
"""Validate the bundled audited font files and passive monochrome SVG icons."""

import hashlib
import json
from pathlib import Path
import sys
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]


def check() -> list[str]:
    errors = []
    fonts = json.loads((ROOT / "assets/fonts/manifest.json").read_text(encoding="utf-8"))
    for entry in fonts["fonts"]:
        payload = (ROOT / entry["path"]).read_bytes()
        if len(payload) != entry["size_bytes"] or hashlib.sha256(payload).hexdigest() != entry["sha256"]:
            errors.append(f"Font differs from audited source: {entry['path']}")
        if entry["license"] != "OFL-1.1":
            errors.append(f"Unexpected font license: {entry['path']}")
        license_path = ROOT / Path(entry["path"]).parent / "OFL.txt"
        if "SIL OPEN FONT LICENSE Version 1.1" not in license_path.read_text(encoding="utf-8"):
            errors.append(f"Missing OFL notice: {entry['path']}")
    icons = json.loads((ROOT / "assets/icons/manifest.json").read_text(encoding="utf-8"))
    if "ISC License" not in (ROOT / "assets/icons/LICENSE.txt").read_text(encoding="utf-8"):
        errors.append("Missing icon license notice")
    allowed = {"svg", "path", "circle", "ellipse", "line", "polyline", "polygon", "rect", "g"}
    for entry in icons["icons"]:
        payload = (ROOT / entry["path"]).read_bytes()
        if hashlib.sha256(payload).hexdigest() != entry["sha256"]:
            errors.append(f"Icon differs from audited derivative: {entry['path']}")
        svg = ET.fromstring(payload)
        if svg.attrib.get("viewBox") != "0 0 24 24" or svg.attrib.get("stroke-width") != "1.75":
            errors.append(f"Icon grid/stroke differs: {entry['path']}")
        if svg.attrib.get("stroke") != "currentColor":
            errors.append(f"Icon does not use a theme color: {entry['path']}")
        for element in svg.iter():
            if element.tag.rsplit("}", 1)[-1] not in allowed:
                errors.append(f"Active/unsupported SVG node: {entry['path']}")
            for key, value in element.attrib.items():
                if key.rsplit("}", 1)[-1] == "href" or key.lower().startswith("on"):
                    errors.append(f"Active/external SVG attribute: {entry['path']}")
                if key in {"fill", "stroke"} and value not in {"none", "currentColor"}:
                    errors.append(f"Hardcoded SVG color: {entry['path']}")
    return errors


if __name__ == "__main__":
    errors = check()
    if errors:
        print("\n".join(errors), file=sys.stderr)
        raise SystemExit(1)
    print("Audited assets intact: 8 fonts, passive 24px SVGs, currentColor and 1.75 stroke")
