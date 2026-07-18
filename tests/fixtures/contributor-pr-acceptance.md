# Contributor PR Acceptance Fixtures

These synthetic fixtures isolate the user-visible behavior proposed in PRs #186, #187, and #188. They were authored for this repository and contain no third-party or confidential data.

`Active` expectations already pass on current `main`. `Pending` expectations are executable ignored tests that define the remaining adaptation work without making this fixture-only change fail CI.

## PR #186: XLSX alignment, sizing, borders, and page setup

Fixture: `xlsx/pr_186_contributor_acceptance.xlsx`

| Status | Source construct | Observable expectation |
|---|---|---|
| Active | Explicit Left, Center, Right, and Justify cell alignment | The corresponding paragraph alignment reaches IR. |
| Pending | A General numeric value beside the text value `0050` | The number is right-aligned while the numeric-looking text keeps General alignment. |
| Active | Column B width of 20 Excel character units | The IR column width is `(20 * 7 + 5) * 0.75 = 108.75pt`. |
| Active | A Double top border | The parser preserves `BorderLineStyle::Double`. |
| Pending | The same Double border | Typst uses a 2.5x solid stroke rather than a dashed approximation. |
| Active | Explicit margins on the Statement sheet and default margins on the Executive sheet | Margins reach IR in points. |
| Pending | Statement paper in landscape and Executive paper in portrait | Each sheet's source paper size and orientation reach `SheetPage::size`. |

## PR #187: DOCX cell properties and style-inherited lists

Fixture: `docx/pr_187_contributor_acceptance.docx`

| Status | Source construct | Observable expectation |
|---|---|---|
| Active | Table cell defaults plus a cell-level top/left margin override | Overridden sides replace defaults and unspecified right/bottom sides remain inherited. |
| Active | A custom table style with dark whole-table shading | Cells without explicit shading inherit the dark fill. |
| Pending | An explicit `FFFFFF` cell fill over the dark table style | The explicit white color remains present in IR. |
| Pending | `List Bullet` and `List Number` paragraph styles with `numPr`, while paragraphs have only `pStyle` | Paragraphs become unordered and ordered list blocks without inline `numPr`. |

## PR #188: PPTX background inheritance and page reset

Fixtures:

- `pptx/pr_188_layout_gradient.pptx`
- `pptx/pr_188_master_bg_ref.pptx`
- `pptx/pr_188_page_fill_reset.pptx`

| Status | Source construct | Observable expectation |
|---|---|---|
| Active | A slide without `p:bg` whose layout has a two-stop `a:gradFill` | The slide inherits both gradient stops. |
| Active | A slide and layout without `p:bg` whose master has `p:bgRef idx="1001"` | The reference resolves through the theme background fill list and placeholder color. |
| Active | A red slide followed by a slide with no background at any inheritance layer | The parser reports red then no source background. |
| Pending | The same two-slide sequence in generated Typst | The second page declaration explicitly uses `fill: white`, preventing previous-page fill leakage. |

## Follow-up workflow

Adapt contributor code in one focused format PR at a time. For each accepted behavior, remove only its matching `#[ignore]`, run the targeted fixture test, and preserve contributor authorship in the adapted commit history or PR attribution.
