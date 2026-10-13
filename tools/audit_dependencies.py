#!/usr/bin/env python3
"""Audit locked registry crates and emit reproducible source license notices.

No dependency resolution, downloads of crates, builds or Cargo.toml mutations.
Online refresh queries exact crates.io versions and exact upstream VCS commits.
Offline rebuild uses the registry archives and evidence embedded in the report.
"""
from __future__ import annotations

import argparse
import base64
import gzip
import io
import concurrent.futures
from datetime import datetime, timedelta, timezone
import functools
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import sys
import subprocess
import tarfile
import time
import tomllib
import urllib.error
import urllib.parse
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
MARKER = "<!-- dependency-audit-data -->"
MAX_HTTP_BYTES = 8 * 1024 * 1024
SAFE_ID = re.compile(r"[A-Za-z0-9_.+-]+\Z")
LICENSE_PREFIXES = ("LICENSE", "LICENCE", "COPYING", "COPYRIGHT", "NOTICE", "AUTHORS")
PERMISSIVE_IDS = {"MIT", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause",
                  "ISC", "0BSD", "Zlib", "BSL-1.0", "Unlicense", "Unicode-3.0"}
PREFER = {"MIT": 0, "Apache-2.0": 1, "BSD-3-Clause": 2, "BSD-2-Clause": 3,
          "ISC": 4, "0BSD": 5, "Zlib": 6, "BSL-1.0": 7, "Unlicense": 8,
          "Unicode-3.0": 9}


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def iso(value: str) -> datetime:
    result = datetime.fromisoformat(value.replace("Z", "+00:00"))
    if result.tzinfo is None:
        raise ValueError("Audit time requires an explicit timezone")
    return result.astimezone(timezone.utc)


@functools.cache
def request_bytes(url: str) -> bytes:
    request = urllib.request.Request(
        url, headers={"User-Agent": "TrineDocs exact dependency/license audit",
                      "Accept": "application/json" if "api." in url else "*/*"})
    for attempt in range(4):
        try:
            with urllib.request.urlopen(request, timeout=25) as response:
                body = response.read(MAX_HTTP_BYTES + 1)
            if len(body) > MAX_HTTP_BYTES:
                raise ValueError(f"HTTP response too large: {url}")
            return body
        except urllib.error.HTTPError as error:
            if error.code not in (429, 500, 502, 503, 504) or attempt == 3:
                raise
            time.sleep(min(2 ** attempt, 8))
        except (TimeoutError, urllib.error.URLError):
            if attempt == 3:
                raise
            time.sleep(min(2 ** attempt, 8))
    raise RuntimeError("Unreachable retry state")


def primary_version(name: str, version: str) -> dict:
    url = f"https://crates.io/api/v1/crates/{name}/{urllib.parse.quote(version, safe='')}"
    data = json.loads(request_bytes(url))["version"]
    return {"published_at": data["created_at"], "yanked": bool(data["yanked"]),
            "registry_checksum": data["checksum"],
            "registry_license": data.get("license"), "metadata_url": url}


def license_name(path: str) -> bool:
    name = PurePosixPath(path).name.upper()
    return name.startswith(LICENSE_PREFIXES)


def crate_sources(cargo_root: Path, name: str, version: str) -> tuple[Path | None, Path | None]:
    basename = f"{name}-{version}"
    sources = sorted((cargo_root / "registry" / "src").glob(f"*/{basename}"))
    archives = sorted((cargo_root / "registry" / "cache").glob(f"*/{basename}.crate"))
    return (sources[0] if sources else None, archives[0] if archives else None)


def archive_data(archive: Path, name: str, version: str) -> tuple[dict, list[dict], dict]:
    prefix = f"{name}-{version}/"
    documents = []
    with tarfile.open(archive, mode="r:gz") as tar:
        manifest_entry = tar.extractfile(prefix + "Cargo.toml")
        if manifest_entry is None:
            raise ValueError("Registry archive has no Cargo.toml")
        manifest = tomllib.loads(manifest_entry.read().decode("utf-8"))["package"]
        vcs_entry = tar.extractfile(prefix + ".cargo_vcs_info.json") if prefix + ".cargo_vcs_info.json" in tar.getnames() else None
        vcs = json.load(vcs_entry) if vcs_entry else {}
        for entry in sorted(tar.getmembers(), key=lambda e: e.name):
            if not entry.isfile() or not entry.name.startswith(prefix):
                continue
            relative = entry.name[len(prefix):]
            if ".." in PurePosixPath(relative).parts:
                raise ValueError("Invalid registry archive member")
            if not license_name(relative) and relative != manifest.get("license-file"):
                continue
            if entry.size > 1024 * 1024:
                raise ValueError(f"Unexpectedly large license document: {relative}")
            stream = tar.extractfile(entry)
            if stream is None:
                raise ValueError("Missing registry archive member stream")
            body = stream.read()
            text = body.decode("utf-8")
            if PurePosixPath(relative).name.upper().startswith("AUTHORS") and not any(grant in text for grant in ("Permission is hereby granted", "Licensed under the Apache License", "This project is triple-licensed")):
                continue
            documents.append({"path": relative, "sha256": digest(body),
                              "text": text,
                              "origin": "checksum-verified-registry-archive"})
    return manifest, documents, vcs


def upstream_documents(manifest: dict, vcs: dict) -> list[dict]:
    repository = manifest.get("repository") or ""
    match = re.match(r"https?://github\.com/([^/]+)/([^/#?]+)", repository)
    if not match or not vcs:
        return []
    owner, repository_name = match.groups()
    repository_name = repository_name.removesuffix(".git")
    commit = vcs.get("git", {}).get("sha1")
    if not isinstance(commit, str) or not re.fullmatch(r"[0-9a-fA-F]{40}", commit):
        return []
    prefix = PurePosixPath(vcs.get("path_in_vcs", ""))
    ancestors = {prefix.as_posix()}
    ancestors.update(p.as_posix() for p in prefix.parents)
    ancestors.add(".")
    base = f"https://api.github.com/repos/{owner}/{repository_name}/git/trees/{commit}?recursive=1"
    tree = json.loads(request_bytes(base))
    if tree.get("truncated"):
        raise ValueError(f"Truncated upstream tree: {base}")
    paths = []
    for item in tree.get("tree", []):
        path = item.get("path", "")
        parent = PurePosixPath(path).parent.as_posix()
        if item.get("type") == "blob" and license_name(path) and parent in ancestors:
            paths.append(path)
    result = []
    for path in sorted(paths):
        url = f"https://raw.githubusercontent.com/{owner}/{repository_name}/{commit}/{urllib.parse.quote(path)}"
        body = request_bytes(url)
        result.append({"path": path, "sha256": digest(body),
                       "text": body.decode("utf-8"), "origin": "exact-upstream-commit",
                       "source_url": url, "vcs_commit": commit})
    return result


def selected_expression(expression: str) -> str:
    """Select a declared permissive alternative; retain every AND obligation."""
    normalized = expression.replace("/", " OR ")
    tokens = re.findall(r"\(|\)|[A-Za-z0-9.+-]+", normalized)
    index = 0

    def atom() -> list[str]:
        nonlocal index
        if index >= len(tokens):
            raise ValueError("Incomplete SPDX expression")
        if tokens[index] == "(":
            index += 1
            result = disjunction()
            if index >= len(tokens) or tokens[index] != ")":
                raise ValueError("Missing SPDX parenthesis")
            index += 1
            return result
        license_id = tokens[index]
        index += 1
        if license_id not in PERMISSIVE_IDS:
            raise ValueError(f"License option needs explicit review: {license_id}")
        if index < len(tokens) and tokens[index] == "WITH":
            index += 1
            if index >= len(tokens) or tokens[index] != "LLVM-exception":
                raise ValueError("Unreviewed SPDX exception")
            license_id += " WITH LLVM-exception"
            index += 1
        return [license_id]

    def conjunction() -> list[str]:
        nonlocal index
        result = atom()
        while index < len(tokens) and tokens[index] == "AND":
            index += 1
            result += atom()
        return result

    def disjunction() -> list[str]:
        nonlocal index
        options = []
        begin = index
        # Unknown GPL/LGPL alternatives are legal declarations but not selected.
        # Try each OR branch with a full parser, retaining allowed branches.
        try:
            options.append(conjunction())
        except ValueError:
            while index < len(tokens) and tokens[index] not in ("OR", ")"):
                index += 1
        while index < len(tokens) and tokens[index] == "OR":
            index += 1
            try:
                options.append(conjunction())
            except ValueError:
                while index < len(tokens) and tokens[index] not in ("OR", ")"):
                    index += 1
        if not options or index == begin:
            raise ValueError("No reviewed permissive license choice")
        return min(options, key=lambda branch: (sum(PREFER.get(x.split()[0], 20) for x in branch), len(branch)))

    choice = disjunction()
    if index != len(tokens):
        raise ValueError("Unsupported SPDX expression")
    return " AND ".join(sorted(set(choice)))


def load_evidence(path: Path) -> dict:
    if not path.is_file():
        return {}
    content = path.read_text(encoding="utf-8")
    tail = content.split(MARKER, 1)
    if len(tail) != 2:
        return {}
    match = re.search(r"~~~json\s*\n([\s\S]+?)\n~~~", tail[1])
    data = json.loads(match.group(1)) if match else {}
    if data.get("encoding") == "gzip-base64-json":
        compressed = base64.b64decode(data["payload"], validate=True)
        with gzip.GzipFile(fileobj=io.BytesIO(compressed)) as stream:
            expanded = stream.read(16 * 1024 * 1024 + 1)
        if len(expanded) > 16 * 1024 * 1024:
            raise ValueError("Audit evidence exceeds 16 MiB")
        if digest(expanded) != data["payload_sha256"]:
            raise ValueError("Audit evidence payload hash differs")
        data = json.loads(expanded)
    document_store = data.get("documents", {})
    for package in data.get("packages", []):
        for document in package.get("license_documents", []):
            if "text" not in document and document["sha256"] in document_store:
                document["text"] = document_store[document["sha256"]]
    return data


def inspect(package: dict, cargo_root: Path, existing: dict, online: bool, as_of: datetime) -> dict:
    name, version = package["name"], package["version"]
    if not SAFE_ID.fullmatch(name) or not SAFE_ID.fullmatch(version):
        raise ValueError("Invalid Cargo package identifier")
    record = {"name": name, "version": version, "lock_checksum": package.get("checksum"),
              "source": package["source"], "errors": []}
    errors = record["errors"]
    old = existing.get((name, version), {})
    _, archive = crate_sources(cargo_root, name, version)
    if archive is None:
        if not online and old.get("lock_checksum") == package.get("checksum") and old.get("archive_verified"):
            record = dict(old)
            record["errors"] = errors = [e for e in old.get("errors", []) if e != "release_younger_than_14_days"]
            for document in record.get("license_documents", []):
                if digest(document["text"].encode("utf-8")) != document["sha256"]:
                    errors.append("cached_notice_hash_differs")
            if not record.get("published_at"):
                errors.append("unverified_publication_age")
            elif as_of - iso(record["published_at"]) < timedelta(days=14):
                errors.append("release_younger_than_14_days")
            if record.get("yanked"):
                errors.append("yanked")
            if "AGPL" in (record.get("declared_license") or "").upper():
                errors.append("forbidden_AGPL")
            if not record.get("license_documents"):
                errors.append("missing_source_license_text")
            return record
        errors.append("missing_registry_archive_and_evidence")
        return record
    if digest(archive.read_bytes()) != package.get("checksum"):
        errors.append("archive_checksum_differs")
        return record
    manifest, documents, vcs = archive_data(archive, name, version)
    record["archive_verified"] = True
    record["declared_license"] = manifest.get("license")
    record["repository"] = manifest.get("repository")
    record["authors"] = manifest.get("authors", [])
    try:
        if not record["declared_license"]:
            errors.append("missing_declared_license")
        elif "AGPL" in record["declared_license"].upper():
            errors.append("forbidden_AGPL")
        else:
            record["selected_license"] = selected_expression(record["declared_license"])
    except ValueError as error:
        errors.append(str(error))
    old = existing.get((name, version), {})
    if online:
        try:
            record.update(primary_version(name, version))
        except (OSError, ValueError, KeyError) as error:
            errors.append(f"registry_metadata_unavailable:{type(error).__name__}")
    elif old.get("lock_checksum") == package.get("checksum"):
        for key in ("published_at", "yanked", "registry_checksum", "registry_license", "metadata_url"):
            if key in old:
                record[key] = old[key]
    if not record.get("published_at"):
        errors.append("unverified_publication_age")
    else:
        if as_of - iso(record["published_at"]) < timedelta(days=14):
            errors.append("release_younger_than_14_days")
        if record.get("yanked"):
            errors.append("yanked")
        if record.get("registry_checksum") != package.get("checksum"):
            errors.append("registry_checksum_differs_from_lock")
        if record.get("registry_license") != record.get("declared_license"):
            errors.append("registry_manifest_license_disagree")
    if "-" in version.split("+", 1)[0]:
        errors.append("prerelease")
    if not documents and old.get("lock_checksum") == package.get("checksum"):
        documents = old.get("license_documents", [])
        for document in documents:
            if digest(document["text"].encode("utf-8")) != document["sha256"]:
                errors.append("supplemental_license_hash_differs")
    if not documents and online:
        try:
            documents = upstream_documents(manifest, vcs)
        except (OSError, ValueError, KeyError) as error:
            errors.append(f"upstream_notices_unavailable:{type(error).__name__}")
    if not documents:
        errors.append("missing_source_license_text")
    record["license_documents"] = documents
    return record


def target_closure(target: str) -> dict:
    command = ["cargo", "metadata", "--format-version", "1", "--locked", "--offline",
               "--filter-platform", target]
    completed = subprocess.run(command, cwd=ROOT, capture_output=True,
                               encoding="utf-8", check=False)
    if completed.returncode:
        raise ValueError(f"Read-only cargo metadata failed: {completed.stderr.strip()}")
    metadata = json.loads(completed.stdout)
    packages = {p["id"]: p for p in metadata["packages"]}
    nodes = {n["id"]: n for n in metadata["resolve"]["nodes"]}
    roots = [p["id"] for p in metadata["packages"] if p["name"] == "trinedocs" and p["id"] in metadata["workspace_members"]]
    if len(roots) != 1:
        raise ValueError("Expected exactly one application root crate")
    pending = list(roots)
    visited = set()
    while pending:
        current = pending.pop()
        if current in visited:
            continue
        visited.add(current)
        for dependency in nodes[current].get("deps", []):
            if any(kind["kind"] in (None, "build") for kind in dependency["dep_kinds"]):
                pending.append(dependency["pkg"])
    included = sorted((packages[p]["name"], packages[p]["version"])
                      for p in visited if packages[p].get("source"))
    return {"target": target, "roots": ["trinedocs"], "registry_packages": included,
            "method": "cargo metadata --locked --offline --filter-platform; normal/build edges, dev excluded",
            "scope": "conservative target dependency graph including build/proc-macro crates; not linker-object proof"}


def gate_errors(package: dict) -> list[str]:
    licensing = ("missing_declared_license", "missing_source_license_text",
                 "upstream_notices_unavailable", "registry_manifest_license_disagree",
                 "License option", "No reviewed", "SPDX", "Unreviewed")
    return [error for error in package["errors"] if package.get("target_included") or
            not error.startswith(licensing)]


def markdown_cell(value: object) -> str:
    return str(value or "").replace("|", "\\|").replace("\n", " ")


def license_markdown(data: dict) -> str:
    rows = data["packages"]
    result = ["# Зависимости и лицензии",
              "",
              f"Полный production Cargo.lock: {len(rows)} registry packages. "
              f"Проверка exact versions на {data['as_of']}.",
              f"Cargo.lock SHA-256: {data['lock_sha256']}.",
              "",
              "Реестр консервативно включает все target-specific и build-time зависимости lock; "
              "это не заявление, что каждый crate связан с Windows exe. Qt/CRDT spike не входят в этот граф.",
              "Декларации лицензий взяты из checksum-verified crates, даты/yank/checksum — из crates.io API. "
              "Отсутствующие в package LICENSE/NOTICE дополнены только с exact upstream commit из .cargo_vcs_info.json.",
              "",
              "Выбор permissive OR альтернативы указан явно; все AND obligations сохранены. "
              "Notices сохраняют все найденные исходные LICENSE/COPYRIGHT/NOTICE тексты, включая альтернативы. "
              "Это проверка происхождения/наличия текстов, а не юридическое заключение о достаточности распространения.",
              "",
              "Генератор: [tools/audit_dependencies.py](../tools/audit_dependencies.py). "
              "Полные проверяемые evidence и supplemental тексты: [audit/dependencies.md](audit/dependencies.md).",
              "",
              "Offline сборка notices: python -B tools/audit_dependencies.py --offline "
              "--check --notices target/dist/THIRD-PARTY-NOTICES.txt",
              "Online refresh: python -B tools/audit_dependencies.py --refresh",
              "",
              "| Package | Pin | Declared SPDX | Selected | Published UTC | Source texts | Windows | Gate |",
              "|---|---|---|---|---|---|---|---|"]
    for item in rows:
        metadata = item.get("metadata_url", "")
        name = f"[{item['name']}]({metadata})" if metadata else item["name"]
        documents = item.get("license_documents", [])
        origin = ", ".join(sorted({d["origin"] for d in documents})) or "MISSING"
        result.append("| " + " | ".join(markdown_cell(x) for x in (
            name, item["version"], item.get("declared_license"), item.get("selected_license"),
            item.get("published_at"), f"{len(documents)}; {origin}", item.get("target_included"),
            "; ".join(gate_errors(item)) or ("pass; source notices absent, not Windows" if item["errors"] else "pass"))) + " |")
    result += ["", "## Assets, toolchain и остальные компоненты", "",
               "- Системные Windows Arial/Segoe загружаются из ОС и не входят в installer. PDF subsets "
               "нуждаются в embedding flags/rights audit системного шрифта; это отдельная проверка.",
               "- Inter 4.1 / JetBrains Mono 2.304 (OFL-1.1), Lucide 1.48.0 (ISC) находятся в source assets. "
               "Их прежние exact SHA/notices — в assets/manifests; текущие Rust exe не встраивают эти fonts/icons.",
               "- Loro CRDT spike изолирован: docs/dependencies/crdt-spike.{md,json}. "
               "Он не включён в production Cargo.lock, и его notices не подменяют этот граф.",
               "- Rust 1.98.1, Microsoft Windows SDK/MSVC CRT и NSIS — build/platform components. "
               "Для static CRT installer применяются отдельно подтверждённые redistribution terms; "
               "этот Cargo реестр не является их аудитом.",
               "- CI actions и NSIS script проверяются отдельным release review. "
               "AGPL запрещён. Age/yank/prerelease/checksum gate применяется ко всему lock; "
               "license/source-text gate — к Windows target graph. Non-Windows source gaps сохраняются честно в audit report.",
               ""]
    return "\n".join(result)


def compact_evidence(data: dict) -> str:
    packed = dict(data)
    packed["packages"] = []
    packed["documents"] = {}
    for package in data["packages"]:
        entry = dict(package)
        entry["license_documents"] = []
        for document in package.get("license_documents", []):
            packed["documents"][document["sha256"]] = document["text"]
            entry["license_documents"].append({k: v for k, v in document.items() if k != "text"})
        packed["packages"].append(entry)
    expanded = json.dumps(packed, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
    compressed = gzip.compress(expanded, mtime=0)
    envelope = {"encoding": "gzip-base64-json", "payload_sha256": digest(expanded),
                "payload": base64.b64encode(compressed).decode("ascii")}
    return json.dumps(envelope, ensure_ascii=False, indent=2)


def report_markdown(data: dict) -> str:
    problems = [p for p in data["packages"] if gate_errors(p)]
    issues = [p for p in data["packages"] if p["errors"]]
    result = ["# Dependency audit — exact production lock", "",
              f"As of: {data['as_of']}; Cargo.lock SHA-256: {data['lock_sha256']}.",
              f"Registry packages: {len(data['packages'])}; full source issues: {len(issues)}; gate failures: {len(problems)}.",
              f"Windows payload: {len(data['target_closure']['registry_packages'])} target graph crates; target {data['target_closure']['target']}.",
              data["target_closure"]["scope"],
              "",
              "Все версии lock проверяются отдельно, включая crates с несколькими версиями. "
              "Этот report не заменяет linked-target SBOM, binary third-party source audit или legal review.",
              "Данные registry immutable versions + timestamps зафиксированы для offline воспроизведения. "
              "Fresh yanked status требует --refresh, а не доверия старому report.",
              ""]
    if issues:
        result += ["| Package | Pin | Target included | Source issues | Gate |", "|---|---|---|---|---|"]
        result += ["| " + " | ".join(markdown_cell(x) for x in
                    (p["name"], p["version"], p.get("target_included"), "; ".join(p["errors"]), "; ".join(gate_errors(p)) or "pass (not target license payload)")) + " |" for p in issues]
    else:
        result.append("Declared license, source notices, checksum and 14-day age gates passed for this snapshot.")
    result += ["", "## Reproduction", "",
               "python -B tools/audit_dependencies.py --offline --check "
               "--notices target/dist/THIRD-PARTY-NOTICES.txt",
               "",
               "The notices artifact includes full original texts with exact package, version, checksum, "
               "origin and document SHA-256. Registry source license texts remain in checksum-verified Cargo cache. "
               "Supplemental upstream texts are embedded losslessly below as deterministic gzip/base64 JSON; "
               "offline regeneration needs no GitHub request. Loader limits expanded evidence to16MiB.",
               "",
               MARKER, "~~~json", compact_evidence(data), "~~~", ""]
    return "\n".join(result)


def notices(data: dict) -> str:
    result = ["TrineDocs — third-party license notices", "=" * 72,
              f"Cargo.lock SHA-256: {data['lock_sha256']}",
              f"Audit as of: {data['as_of']}",
              "Windows target normal/build dependency closure; excludes non-Windows-only crates.",
              f"Target: {data['target_closure']['target']}",
              "All original license options/AND notices are retained. Selected option is metadata only.", ""]
    for item in data["packages"]:
        if not item.get("target_included"):
            continue
        result += ["=" * 72, f"{item['name']} {item['version']}",
                   f"Package checksum: {item['lock_checksum']}",
                   f"Declared license: {item.get('declared_license')}",
                   f"Selected license: {item.get('selected_license')}",
                   f"Source: {item.get('metadata_url', item['source'])}",
                   f"Authors: {', '.join(item.get('authors', []))}",
                   f"Audit gate: {'; '.join(item['errors']) or 'pass'}", ""]
        for document in item.get("license_documents", []):
            result += [f"--- {document['path']} ---",
                       f"Document SHA-256: {document['sha256']}",
                       f"Origin: {document.get('source_url', document['origin'])}", "",
                       document["text"].rstrip(), ""]
    return "\n".join(result) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--lock", type=Path, default=ROOT / "Cargo.lock")
    parser.add_argument("--cargo-home", type=Path,
                        default=Path(os.environ.get("CARGO_HOME", str(Path.home() / ".cargo"))))
    parser.add_argument("--report", type=Path, default=ROOT / "docs/audit/dependencies.md")
    parser.add_argument("--licenses", type=Path, default=ROOT / "docs/licenses.md")
    parser.add_argument("--notices", "--output", dest="notices", type=Path)
    parser.add_argument("--refresh", action="store_true")
    parser.add_argument("--offline", action="store_true")
    parser.add_argument("--check", action="store_true", help="Do not rewrite documentation; optional notices output still writes")
    parser.add_argument("--check-age", action="store_true", help="Use current UTC time for the publication-age gate; no network unless --refresh")
    parser.add_argument("--as-of", help="ISO-8601 UTC policy time; offline defaults to existing report time")
    parser.add_argument("--workers", type=int, default=4)
    parser.add_argument("--target", default="x86_64-pc-windows-msvc")
    args = parser.parse_args()
    if args.refresh and args.offline:
        parser.error("Choose refresh or offline")
    evidence = load_evidence(args.report)
    if args.notices and not args.refresh:
        args.check = True
    if not args.refresh and evidence:
        args.offline = True
    as_of = iso(args.as_of) if args.as_of else (
        iso(evidence["as_of"]) if args.offline and evidence else datetime.now(timezone.utc))
    policy_time = datetime.now(timezone.utc) if args.check_age else as_of
    existing = {(p["name"], p["version"]): p for p in evidence.get("packages", [])}
    lock_bytes = args.lock.read_bytes()
    lock = tomllib.loads(lock_bytes.decode("utf-8"))
    packages = [p for p in lock["package"] if p.get("source", "").startswith("registry+")]
    non_registry = [p for p in lock["package"] if "source" in p and not p["source"].startswith("registry+")]
    if non_registry:
        raise ValueError("Git/other registry source needs a separate immutable provenance audit")
    online = args.refresh or not args.offline
    lock_hash = digest(lock_bytes)
    if args.offline and evidence.get("lock_sha256") == lock_hash and evidence.get("target_closure", {}).get("target") == args.target:
        closure = evidence["target_closure"]
    else:
        closure = target_closure(args.target)
    included = {tuple(p) for p in closure["registry_packages"]}
    with concurrent.futures.ThreadPoolExecutor(max_workers=max(1, min(args.workers, 8))) as pool:
        pending = {pool.submit(inspect, p, args.cargo_home, existing, online, policy_time): p for p in packages}
        rows = []
        for future in concurrent.futures.as_completed(pending):
            entry = future.result()
            entry["target_included"] = (entry["name"], entry["version"]) in included
            rows.append(entry)
            if len(rows) % 50 == 0:
                print(f"Audited {len(rows)}/{len(packages)} locked crates", flush=True)
    if digest(args.lock.read_bytes()) != digest(lock_bytes):
        raise ValueError("Cargo.lock changed during audit; rerun against the final lock")
    data = {"schema": 1, "as_of": as_of.isoformat(), "lock_sha256": digest(lock_bytes),
            "policy": "14 full days, stable, not yanked, exact checksums, no AGPL, source notices",
            "scope": "complete production lock age/source audit; Windows target license payload",
            "target_closure": closure,
            "packages": sorted(rows, key=lambda p: (p["name"], p["version"]))}
    if not args.check:
        for path, text in ((args.report, report_markdown(data)), (args.licenses, license_markdown(data))):
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8", newline="\n")
    if args.notices:
        args.notices.parent.mkdir(parents=True, exist_ok=True)
        args.notices.write_text(notices(data), encoding="utf-8", newline="\n")
    problems = [p for p in rows if gate_errors(p)]
    for p in sorted(problems, key=lambda p: (p["name"], p["version"])):
        print(f"{p['name']} {p['version']}: {', '.join(gate_errors(p))}")
    print(f"Audited {len(rows)} locked registry packages; Windows payload {len(included)}; gated failures: {len(problems)}", flush=True)
    return 1 if problems else 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, KeyError, RuntimeError) as error:
        print(f"Dependency audit failed: {error}", file=sys.stderr)
        raise SystemExit(2)
