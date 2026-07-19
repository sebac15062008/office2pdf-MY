#!/usr/bin/env python3
"""Fail a pull request when its visual audit or evidence is incomplete."""

from __future__ import annotations

import argparse
import json
import os
import re
import struct
import subprocess
import sys
import urllib.error
import urllib.request
from dataclasses import dataclass
from pathlib import Path


VISUAL_ROOT = Path("assets/bugfixes")
EVIDENCE_PATH = re.compile(
    r"^assets/bugfixes/issue-(?P<issue>\d+)/(?P<name>gt|before|after|compare)\.(?P<ext>[^/]+)$"
)
AUDIT_ROWS = (
    "Page count/order",
    "Element presence",
    "Position/size",
    "Rotation/flip",
    "Fill",
    "Stroke/border",
    "Text content",
    "Font family/weight/style",
    "Text color",
    "Alignment",
    "Line/paragraph spacing",
    "Clipping/overflow",
)
INSPECTION_ITEMS = (
    "Rendered all evidence at 150 DPI or higher",
    "Stored progressive JPEG quality 86 assets with metadata stripped",
    "Inspected matched region crops at full resolution",
    "Ran the 5% fuzz pixel-difference sweep",
    "Inventoried hairlines and border dash styles",
    "Inventoried font weight, italic, and underline emphasis",
)
ALLOWED_RESULTS = ("Matches GT", "Fixed", "No deviation observed")


@dataclass(frozen=True)
class JpegInfo:
    width: int
    height: int
    progressive: bool
    density_dpi: tuple[float, float] | None
    metadata_markers: tuple[str, ...]


def extract_section(body: str, heading: str) -> str:
    pattern = re.compile(
        rf"(?ms)^{re.escape(heading)}\s*$\n(?P<section>.*?)(?=^##\s|\Z)"
    )
    match = pattern.search(body)
    return match.group("section") if match else ""


def checked(section: str, label: str) -> bool:
    return bool(re.search(rf"(?mi)^- \[[xX]\] {re.escape(label)}\s*$", section))


def field(section: str, name: str) -> str | None:
    match = re.search(rf"(?mi)^- {re.escape(name)}:\s*(.*?)\s*$", section)
    if not match:
        return None
    value = match.group(1).strip().strip("`")
    if not value or "<!--" in value:
        return None
    return value


def audit_table(section: str) -> dict[str, str]:
    rows: dict[str, str] = {}
    for line in section.splitlines():
        if not line.startswith("|"):
            continue
        cells = [cell.strip() for cell in line.strip().strip("|").split("|")]
        if len(cells) == 2 and cells[0] in AUDIT_ROWS:
            rows[cells[0]] = cells[1]
    return rows


def validate_pr_body(body: str, changed_paths: list[str]) -> list[str]:
    errors: list[str] = []
    impact = extract_section(body, "## Visual impact")
    no_change = checked(impact, "No rendered PDF change")
    visual = checked(impact, "Rendered PDF change or visual evidence added")
    visual_assets_changed = any(path.startswith(f"{VISUAL_ROOT.as_posix()}/") for path in changed_paths)

    if no_change == visual:
        errors.append("Select exactly one Visual impact checkbox.")
        return errors

    if no_change:
        reason = field(impact, "Reason")
        if not reason:
            errors.append("Explain why the PR has no rendered PDF change in Visual impact > Reason.")
        if visual_assets_changed:
            errors.append("assets/bugfixes changes require 'Rendered PDF change or visual evidence added'.")
        return errors

    audit = extract_section(body, "## Visual audit")
    if not audit:
        return ["Rendered changes require a ## Visual audit section."]

    issue_value = field(audit, "Issue")
    issue_match = re.fullmatch(r"#(\d+)", issue_value or "")
    if not issue_match:
        errors.append("Visual audit > Issue must be a single issue reference such as #123.")
        issue_number = None
    else:
        issue_number = issue_match.group(1)

    for name in ("Fixture", "Page(s)"):
        if not field(audit, name):
            errors.append(f"Visual audit > {name} is required.")

    renderer = field(audit, "Renderer and DPI")
    dpi_match = re.search(r"(?i)(\d+(?:\.\d+)?)\s*DPI", renderer or "")
    if not dpi_match or float(dpi_match.group(1)) < 150:
        errors.append("Visual audit > Renderer and DPI must record at least 150 DPI.")

    mode = field(audit, "Evidence mode")
    if mode not in {"fix", "defect"}:
        errors.append("Visual audit > Evidence mode must be `fix` or `defect`.")
    elif issue_number:
        expected = (
            ("GT", "gt"),
            ("Before", "before"),
            ("After", "after"),
        ) if mode == "fix" else (("Compare", "compare"),)
        for field_name, basename in expected:
            expected_path = f"assets/bugfixes/issue-{issue_number}/{basename}.jpg"
            if field(audit, field_name) != expected_path:
                errors.append(f"Visual audit > {field_name} must be `{expected_path}`.")

    for item in INSPECTION_ITEMS:
        if not checked(audit, item):
            errors.append(f"Required visual inspection is not checked: {item}.")

    rows = audit_table(audit)
    for row in AUDIT_ROWS:
        result = rows.get(row, "")
        if not result or "<!--" in result:
            errors.append(f"Deviation audit row is incomplete: {row}.")
        elif result.startswith("Remaining:"):
            if not re.search(r"#\d+", result):
                errors.append(f"Remaining deviation must reference an issue: {row}.")
        elif not result.startswith(ALLOWED_RESULTS):
            errors.append(
                f"Deviation audit row '{row}' must start with Matches GT, Fixed, "
                "No deviation observed, or Remaining: #N."
            )

    if not visual_assets_changed:
        errors.append("Rendered visual changes require evidence under assets/bugfixes/issue-<number>/.")

    return errors


def remaining_issue_numbers(body: str) -> set[int]:
    audit = extract_section(body, "## Visual audit")
    numbers: set[int] = set()
    for result in audit_table(audit).values():
        if result.startswith("Remaining:"):
            numbers.update(int(number) for number in re.findall(r"#(\d+)", result))
    return numbers


def validate_open_issues(issue_numbers: set[int], repository: str, token: str | None) -> list[str]:
    if not issue_numbers:
        return []
    if not token:
        return ["GITHUB_TOKEN is required to verify remaining visual issues."]

    errors: list[str] = []
    for number in sorted(issue_numbers):
        url = f"https://api.github.com/repos/{repository}/issues/{number}"
        request = urllib.request.Request(
            url,
            headers={
                "Accept": "application/vnd.github+json",
                "Authorization": f"Bearer {token}",
                "User-Agent": "office2pdf-visual-pr-gate",
                "X-GitHub-Api-Version": "2022-11-28",
            },
        )
        try:
            with urllib.request.urlopen(request, timeout=15) as response:
                issue = json.load(response)
        except (urllib.error.HTTPError, urllib.error.URLError, TimeoutError) as exc:
            errors.append(f"Could not verify remaining visual issue #{number}: {exc}.")
            continue
        if "pull_request" in issue:
            errors.append(f"Remaining reference #{number} is a pull request, not an issue.")
        elif issue.get("state") != "open":
            errors.append(f"Remaining visual issue #{number} is not open.")
    return errors


def read_jpeg_info(path: Path) -> JpegInfo:
    data = path.read_bytes()
    if not data.startswith(b"\xff\xd8"):
        raise ValueError("not a JPEG file")

    offset = 2
    width = height = None
    progressive = False
    density: tuple[float, float] | None = None
    metadata: list[str] = []

    while offset + 1 < len(data):
        if data[offset] != 0xFF:
            offset += 1
            continue
        while offset < len(data) and data[offset] == 0xFF:
            offset += 1
        if offset >= len(data):
            break
        marker = data[offset]
        offset += 1
        if marker in {0xD8, 0xD9, 0x01} or 0xD0 <= marker <= 0xD7:
            continue
        if offset + 2 > len(data):
            raise ValueError("truncated JPEG segment")
        length = struct.unpack(">H", data[offset : offset + 2])[0]
        if length < 2 or offset + length > len(data):
            raise ValueError("invalid JPEG segment length")
        payload = data[offset + 2 : offset + length]
        offset += length

        if marker == 0xE0 and payload.startswith(b"JFIF\x00") and len(payload) >= 12:
            units = payload[7]
            x_density = struct.unpack(">H", payload[8:10])[0]
            y_density = struct.unpack(">H", payload[10:12])[0]
            if units == 1:
                density = (float(x_density), float(y_density))
            elif units == 2:
                density = (x_density * 2.54, y_density * 2.54)
        elif marker in {0xE1, 0xE2, 0xED, 0xFE}:
            metadata.append({0xE1: "APP1", 0xE2: "APP2", 0xED: "APP13", 0xFE: "COM"}[marker])
        elif marker in {0xC0, 0xC1, 0xC2} and len(payload) >= 5:
            height = struct.unpack(">H", payload[1:3])[0]
            width = struct.unpack(">H", payload[3:5])[0]
            progressive = marker == 0xC2
        elif marker == 0xDA:
            break

    if width is None or height is None:
        raise ValueError("JPEG dimensions were not found")
    return JpegInfo(width, height, progressive, density, tuple(metadata))


def validate_jpeg(path: Path) -> list[str]:
    try:
        info = read_jpeg_info(path)
    except (OSError, ValueError) as exc:
        return [f"{path}: {exc}"]

    errors: list[str] = []
    if not info.progressive:
        errors.append(f"{path}: evidence must be a progressive JPEG.")
    if not info.density_dpi or min(info.density_dpi) < 150:
        errors.append(f"{path}: JPEG density must be at least 150 DPI.")
    if info.metadata_markers:
        markers = ", ".join(info.metadata_markers)
        errors.append(f"{path}: metadata was not stripped ({markers}).")
    if path.name == "compare.jpg" and info.width % 2:
        errors.append(f"{path}: side-by-side comparison width must split into equal panels.")
    return errors


def validate_evidence(changed_paths: list[str], root: Path) -> list[str]:
    errors: list[str] = []
    touched: dict[str, set[str]] = {}

    for raw_path in changed_paths:
        if not raw_path.startswith(f"{VISUAL_ROOT.as_posix()}/"):
            continue
        match = EVIDENCE_PATH.fullmatch(raw_path)
        if not match:
            errors.append(
                f"{raw_path}: visual evidence must be gt.jpg, before.jpg, after.jpg, or compare.jpg "
                "under assets/bugfixes/issue-<number>/."
            )
            continue
        issue = match.group("issue")
        name = match.group("name")
        if match.group("ext").lower() != "jpg":
            errors.append(f"{raw_path}: visual evidence must use the .jpg extension.")
            continue
        touched.setdefault(issue, set()).add(name)

    for issue, names in touched.items():
        issue_dir = root / VISUAL_ROOT / f"issue-{issue}"
        required = {"gt", "before", "after"} if names & {"gt", "before", "after"} else set()
        for name in sorted(required | ({"compare"} if "compare" in names else set())):
            path = issue_dir / f"{name}.jpg"
            if not path.is_file():
                errors.append(f"{path}: required evidence file is missing.")
            else:
                errors.extend(validate_jpeg(path))

    return errors


def git_changed_paths(base: str, head: str, root: Path) -> list[str]:
    result = subprocess.run(
        ["git", "diff", "--name-only", "--diff-filter=ACMR", base, head],
        cwd=root,
        check=True,
        capture_output=True,
        text=True,
    )
    return [line for line in result.stdout.splitlines() if line]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--event", type=Path, required=True)
    parser.add_argument("--base", required=True)
    parser.add_argument("--head", required=True)
    parser.add_argument("--repository", required=True)
    parser.add_argument("--root", type=Path, default=Path.cwd())
    args = parser.parse_args()

    event = json.loads(args.event.read_text(encoding="utf-8"))
    body = event.get("pull_request", {}).get("body") or ""
    changed_paths = git_changed_paths(args.base, args.head, args.root)
    errors = validate_pr_body(body, changed_paths)
    errors.extend(validate_evidence(changed_paths, args.root))
    errors.extend(
        validate_open_issues(
            remaining_issue_numbers(body),
            args.repository,
            os.environ.get("GITHUB_TOKEN"),
        )
    )

    if errors:
        for error in errors:
            print(f"::error::{error}")
        return 1

    print("Visual pull request contract is complete.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
