import io
import unittest
from pathlib import Path
from unittest.mock import patch

from scripts.check_visual_pr import (
    AUDIT_ROWS,
    INSPECTION_ITEMS,
    read_jpeg_info,
    validate_evidence,
    validate_open_issues,
    validate_pr_body,
)


ROOT = Path(__file__).resolve().parents[2]


def visual_body(result_overrides=None):
    results = {row: "Matches GT" for row in AUDIT_ROWS}
    results.update(result_overrides or {})
    inspections = "\n".join(f"- [x] {item}" for item in INSPECTION_ITEMS)
    rows = "\n".join(f"| {row} | {results[row]} |" for row in AUDIT_ROWS)
    return f"""## Visual impact

- [ ] No rendered PDF change
- [x] Rendered PDF change or visual evidence added
- Reason:

## Visual audit

- Issue: #186
- Fixture: tests/fixtures/xlsx/pr_186_contributor_acceptance.xlsx
- Page(s): 1
- Renderer and DPI: pdftoppm, 150 DPI
- Evidence mode: `fix`
- GT: `assets/bugfixes/issue-186/gt.jpg`
- Before: `assets/bugfixes/issue-186/before.jpg`
- After: `assets/bugfixes/issue-186/after.jpg`

### Required inspection

{inspections}

### Deviation audit

| Check | Result |
| --- | --- |
{rows}
"""


class PullRequestBodyTests(unittest.TestCase):
    def test_non_visual_pr_requires_reason(self):
        body = """## Visual impact

- [x] No rendered PDF change
- [ ] Rendered PDF change or visual evidence added
- Reason: Documentation-only workflow change
"""
        self.assertEqual(validate_pr_body(body, ["README.md"]), [])

    def test_complete_visual_audit_passes(self):
        errors = validate_pr_body(
            visual_body({"Fill": "Remaining: #328"}),
            ["assets/bugfixes/issue-186/after.jpg"],
        )
        self.assertEqual(errors, [])

    def test_remaining_deviation_requires_issue(self):
        errors = validate_pr_body(
            visual_body({"Fill": "Remaining: still different"}),
            ["assets/bugfixes/issue-186/after.jpg"],
        )
        self.assertTrue(any("Remaining deviation must reference an issue" in error for error in errors))

    def test_visual_assets_cannot_be_marked_non_visual(self):
        body = """## Visual impact

- [x] No rendered PDF change
- [ ] Rendered PDF change or visual evidence added
- Reason: Documentation only
"""
        errors = validate_pr_body(body, ["assets/bugfixes/issue-186/after.jpg"])
        self.assertTrue(any("assets/bugfixes changes require" in error for error in errors))


class EvidenceTests(unittest.TestCase):
    def test_repository_evidence_is_progressive_150_dpi_jpeg(self):
        path = ROOT / "assets/bugfixes/issue-186/gt.jpg"
        info = read_jpeg_info(path)
        self.assertTrue(info.progressive)
        self.assertEqual(info.density_dpi, (150.0, 150.0))
        self.assertEqual(info.metadata_markers, ())

    def test_changed_trio_validates_all_three_files(self):
        errors = validate_evidence(
            ["assets/bugfixes/issue-186/after.jpg"],
            ROOT,
        )
        self.assertEqual(errors, [])

    def test_png_evidence_is_rejected(self):
        errors = validate_evidence(
            ["assets/bugfixes/issue-186/after.png"],
            ROOT,
        )
        self.assertTrue(any(".jpg extension" in error for error in errors))


class OpenIssueTests(unittest.TestCase):
    @patch("scripts.check_visual_pr.urllib.request.urlopen")
    def test_remaining_issue_must_be_open(self, urlopen):
        response = io.BytesIO(b'{"state":"closed"}')
        urlopen.return_value = response
        errors = validate_open_issues({328}, "developer0hye/office2pdf", "token")
        self.assertEqual(errors, ["Remaining visual issue #328 is not open."])


if __name__ == "__main__":
    unittest.main()
