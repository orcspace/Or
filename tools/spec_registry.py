#!/usr/bin/env python3
"""Maintain stable requirement IDs and fail on missing or stale traceability.

Uses only Python's standard library. This is development tooling, not app code.
"""

from __future__ import annotations

import argparse
from collections import Counter
import json
from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[1]
SOURCES = {"AGENTS.md": "", "docs/MCP.md": "M", "docs/SPEC-AUDIT.md": "A"}
ID_PATTERN = re.compile(r"<!-- (REQ-(?:[MA])?\d{2}-\d{3}) -->")
SECTION_PATTERN = re.compile(r"^## (\d+)\.")
TESTS = {
    0: ["T-00-SPEC", "T-00-BUILD", "T-00-UI"],
    1: ["T-01-CRDT", "T-01-LAYOUT", "T-01-INPUT", "T-01-RECOVERY"],
    2: ["T-02-FILES", "T-02-ROUNDTRIP", "T-02-PRINT"],
    3: ["T-03-MCP", "T-03-AI", "T-SECURITY"],
    4: ["T-04-SYNC", "T-SECURITY"],
    5: ["T-05-CALC", "T-05-GRID", "T-02-ROUNDTRIP"],
    6: ["T-06-SLIDES", "T-02-ROUNDTRIP", "T-02-PRINT"],
    7: ["T-07-LEGACY", "T-07-GOOGLE", "T-SECURITY"],
    8: ["T-08-PARITY"],
    9: ["T-09-RELEASE", "T-SECURITY"],
}


def clean_clause(line: str) -> str:
    return ID_PATTERN.sub("", line).strip()


def points(content: str):
    """Yield every non-heading clause, excluding Markdown table headers/rules."""
    section = 0
    heading = "Введение"
    lines = content.splitlines()
    for index, line in enumerate(lines):
        stripped = line.strip()
        if not stripped or stripped == "---":
            continue
        match = SECTION_PATTERN.match(stripped)
        if match:
            section = int(match.group(1))
        if stripped.startswith("#"):
            heading = stripped.lstrip("# ")
            continue
        if re.fullmatch(r"\|[ :|\-]+\|", stripped):
            continue
        if index + 1 < len(lines) and re.fullmatch(r"\|[ :|\-]+\|", lines[index + 1].strip()):
            continue
        yield index, section, heading, line, ID_PATTERN.findall(line)


def route(source: str, section: int, heading: str, text: str):
    if source.endswith("SPEC-AUDIT.md"):
        return [0], ["T-00-SPEC", "T-FACTS"]
    if source.endswith("MCP.md"):
        phase = 3
        if "6.4 Sheets" in heading:
            phase = 5
        elif "6.5 Slides" in heading:
            phase = 6
        return [phase], TESTS[phase]
    phase_map = {0: 0, 1: 0, 2: 0, 3: 1, 4: 1, 5: 0, 6: 1, 7: 2,
                 8: 2, 9: 7, 10: 2, 11: 1, 12: 3, 13: 3, 14: 4,
                 15: 4, 16: 0, 17: 1, 18: 0, 19: 0, 20: 0}
    phase = phase_map.get(section, 0)
    if section in {8, 10, 11}:
        if heading.startswith("Sheets"):
            phase = 5
        elif heading.startswith("Slides"):
            phase = 6
        elif text.startswith("P1:"):
            phase = 8
        elif text.startswith("P2:"):
            phase = 9
        elif heading.startswith("Свои"):
            phase = 8
    if section == 20 and text.strip().startswith("|"):
        phase_match = re.match(r"\|\s*(\d+)\.", text)
        if phase_match:
            phase = int(phase_match.group(1))
    return [phase], TESTS[phase]


def initialize(root: Path = ROOT) -> None:
    registry_path = root / "docs/requirements.json"
    if registry_path.exists():
        raise ValueError("Registry already exists. Preserve IDs and edit entries explicitly.")
    records = []
    for source, prefix in SOURCES.items():
        path = root / source
        content = path.read_text(encoding="utf-8")
        lines = content.splitlines()
        counts: Counter[int] = Counter()
        for index, section, heading, line, existing in points(content):
            if existing:
                raise ValueError(f"Already annotated: {source}:{index + 1}")
            counts[section] += 1
            req_id = f"REQ-{prefix}{section:02d}-{counts[section]:03d}"
            marker = f"<!-- {req_id} -->"
            if line.strip().startswith("|"):
                # Put the marker inside the first cell, preserving the GFM table.
                pipe = line.index("|")
                lines[index] = line[:pipe + 1] + marker + line[pipe + 1:]
            else:
                lines[index] = line + " " + marker
            phases, tests = route(source, section, heading, line)
            records.append({"id": req_id, "source": source, "section": section,
                            "heading": heading, "clause": line.strip(),
                            "phases": phases, "tests": tests,
                            "status": "planned", "evidence": []})
        path.write_text("\n".join(lines) + "\n", encoding="utf-8")
    registry_path.write_text(json.dumps(records, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def md(text: str) -> str:
    return text.replace("|", "\\|").replace("\n", " ")


def render_traceability(records: list[dict]) -> str:
    rows = ["# Трассировка требований", "",
            "ID закреплены в HTML-комментариях источников. IDs не перенумеровывать; изменённую формулировку и маршруты обновлять явно. Подпункты интерфейсных функций дополнительно перечислены с PAR-ID в parity. Проверки — запланированные методы из testing/acceptance.md, не заявления об их прохождении.", "",
            f"Всего пунктов: **{len(records)}**. Глобальный DoD выполнен: **{sum(r['status'] == 'done' for r in records)}**.", "",
            "| ID | Источник / раздел | Требование | Фазы | Проверки (план) | Статус |",
            "|---|---|---|---|---|---|"]
    for record in records:
        rows.append(f"| {record['id']} | {record['source']} §{record['section']} | "
                    f"{md(record['clause'])} | {','.join(map(str, record['phases']))} | "
                    f"{', '.join(record['tests'])} | {record['status']} |")
    return "\n".join(rows) + "\n"


def render_parity(features: list[dict]) -> str:
    rows = ["# Матрица паритета", "",
            "Первичный feature inventory по заданию и первым проверенным пробелам. Полная сверка всех вкладок Office ещё не завершена. Внутри каждого PAR-пункта варианты поведения проверяются отдельно. Ни одна функция не объявлена готовой. P2 не удаляется из конечного объёма; поздние фазы ждут решения владельца ADR-0005.", "",
            f"Всего функций/вариантов: **{len(features)}**.", "",
            "| PAR ID | Режим | Функция | P | Фаза | REQ | MCP / пробел | Import | Export | Статус |",
            "|---|---|---|---|---|---|---|---|---|---|"]
    for f in features:
        rows.append(f"| {f['id']} | {f['mode']} | {md(f['feature'])} | {f['priority']} | "
                    f"{f['phase']} | {f['requirement']} | {md(f['mcp'])} | "
                    f"{f['import']} | {f['export']} | {f['status']} |")
    return "\n".join(rows) + "\n"


def validate(root: Path = ROOT) -> list[str]:
    errors = []
    records = json.loads((root / "docs/requirements.json").read_text(encoding="utf-8"))
    ids = [r["id"] for r in records]
    if len(ids) != len(set(ids)):
        errors.append("Duplicate registry IDs")
    by_id = {r["id"]: r for r in records}
    seen: Counter[str] = Counter()
    test_plan = (root / "docs/testing/acceptance.md").read_text(encoding="utf-8")
    for source in SOURCES:
        for index, section, heading, line, annotations in points((root / source).read_text(encoding="utf-8")):
            if len(annotations) != 1:
                errors.append(f"{source}:{index + 1}: require exactly one REQ ID")
                continue
            req_id = annotations[0]
            seen[req_id] += 1
            record = by_id.get(req_id)
            if record is None:
                errors.append(f"Unregistered {req_id}")
            elif record["source"] != source or record["clause"] != clean_clause(line) or record["section"] != section:
                errors.append(f"Source drift for {req_id}")
    for record in records:
        req_id = record["id"]
        if seen[req_id] != 1:
            errors.append(f"{req_id} occurs {seen[req_id]} times in sources")
        if not record["phases"] or any(p not in range(10) for p in record["phases"]):
            errors.append(f"{req_id} needs a valid phase")
        if not record["tests"] or any(t not in test_plan for t in record["tests"]):
            errors.append(f"{req_id} needs a documented verification method")
        if record["status"] not in {"planned", "blocked", "partial", "done"}:
            errors.append(f"{req_id} invalid status")
        if record["status"] == "done" and not record["evidence"]:
            errors.append(f"{req_id} done without evidence")
    features = json.loads((root / "docs/parity.json").read_text(encoding="utf-8"))
    feature_ids = [f["id"] for f in features]
    if len(feature_ids) != len(set(feature_ids)):
        errors.append("Duplicate PAR IDs")
    for feature in features:
        if feature["requirement"] not in by_id or feature["priority"] not in {"P0", "P1", "P2"}:
            errors.append(f"{feature['id']} invalid requirement/priority")
        if feature["phase"] not in range(10) or not feature["mcp"]:
            errors.append(f"{feature['id']} missing phase/MCP or gap")
    outputs = {"docs/traceability.md": render_traceability(records), "docs/parity.md": render_parity(features)}
    for name, expected in outputs.items():
        path = root / name
        if not path.exists() or path.read_text(encoding="utf-8") != expected:
            errors.append(f"{name} stale: run render")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["init", "render", "check"])
    args = parser.parse_args()
    if args.command == "init":
        initialize()
        return 0
    records = json.loads((ROOT / "docs/requirements.json").read_text(encoding="utf-8"))
    features = json.loads((ROOT / "docs/parity.json").read_text(encoding="utf-8"))
    if args.command == "render":
        (ROOT / "docs/traceability.md").write_text(render_traceability(records), encoding="utf-8")
        (ROOT / "docs/parity.md").write_text(render_parity(features), encoding="utf-8")
        return 0
    errors = validate()
    if errors:
        print("\n".join(errors), file=sys.stderr)
        return 1
    print(f"Spec registry OK: {len(records)} REQ clauses, {len(features)} PAR features. Product acceptance remains incomplete.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

