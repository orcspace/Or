#!/usr/bin/env python3
"""Audit exact experimental Cargo pins against current primary registry data.

This records declared licenses, not a legal compatibility conclusion.
It resolves no versions, builds nothing and writes no production model.
"""

from __future__ import annotations

import concurrent.futures
from datetime import datetime, timedelta, timezone
import hashlib
import json
from pathlib import Path
import re
import sys
import tomllib
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
LOCK = ROOT / "spikes/crdt/Cargo.lock"
OUT = ROOT / "docs/dependencies/crdt-spike.json"


def inspect(package: dict, now: datetime) -> dict:
    name, version = package["name"], package["version"]
    if not re.fullmatch(r"[A-Za-z0-9_-]+", name) or not re.fullmatch(r"[0-9A-Za-z.+-]+", version):
        raise ValueError("Unexpected Cargo package identifier")
    url = f"https://crates.io/api/v1/crates/{name}/{version}"
    request = urllib.request.Request(url, headers={"User-Agent": "TrineDocs dependency audit"})
    with urllib.request.urlopen(request, timeout=30) as response:
        record = json.load(response)["version"]
    published = datetime.fromisoformat(record["created_at"].replace("Z", "+00:00"))
    license_name = record.get("license") or "UNKNOWN"
    errors = []
    if now - published < timedelta(days=14):
        errors.append("release_younger_than_14_days")
    if record["yanked"]:
        errors.append("yanked")
    if "-" in version.split("+", 1)[0]:
        errors.append("prerelease")
    if license_name == "UNKNOWN" or "AGPL" in license_name.upper():
        errors.append("missing_or_forbidden_license")
    if record.get("checksum") != package.get("checksum"):
        errors.append("registry_checksum_differs_from_lock")
    return {"name": name, "version": version, "declared_license": license_name,
            "published_at": record["created_at"], "source": url,
            "checksum": package.get("checksum"), "errors": errors}


def main() -> int:
    lock_bytes = LOCK.read_bytes()
    lock = tomllib.loads(lock_bytes.decode("utf-8"))
    now = datetime.now(timezone.utc)
    packages = [p for p in lock["package"] if "source" in p]
    with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
        entries = list(pool.map(lambda p: inspect(p, now), packages))
    report = {"checked_at": now.isoformat(), "lock_sha256": hashlib.sha256(lock_bytes).hexdigest(),
              "policy": "14 full days, stable, not yanked, no AGPL, exact checksums",
              "license_scope": "Primary registry declarations; source notices and release compliance remain separate checks",
              "packages": sorted(entries, key=lambda e: (e["name"], e["version"]))}
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    problems = [e for e in entries if e["errors"]]
    for entry in problems:
        print(entry["name"], entry["version"], ",".join(entry["errors"]))
    print(f"Audited {len(entries)} exact experimental dependencies; blocked: {len(problems)}")
    return 1 if problems else 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, KeyError) as error:
        print(f"Dependency audit could not complete: {error}", file=sys.stderr)
        raise SystemExit(1)
