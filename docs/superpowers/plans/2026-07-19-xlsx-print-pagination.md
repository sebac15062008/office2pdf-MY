# XLSX Print Pagination Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Preserve Excel's horizontal print pagination by converting character-unit column widths with the worksheet font's print metric before packing columns into pages.

**Architecture:** Keep the existing generic width-based pagination. Infer the worksheet's normal font family from its populated cells, choose the corresponding maximum-digit print metric, and convert stored OOXML widths without fixture-specific page breaks. This fixes the column geometry that feeds pagination rather than special-casing page count.

**Tech Stack:** Rust, umya-spreadsheet, office2pdf XLSX IR, Typst, Cargo tests, Poppler, ImageMagick

---

## Chunk 1: Font-aware print widths

### Task 1: Characterize the native contributor fixture

**Files:**
- Test: `crates/office2pdf/src/parser/xlsx_cells.rs`
- Test: `crates/office2pdf/tests/xlsx_fixtures.rs`

- [ ] Add failing unit tests for Carlito widths 26, 20, and 24 mapping to 156pt, 120pt, and 144pt.
- [ ] Add a failing fixture acceptance test asserting the first statement page contains A-C and the overflow page contains D.
- [ ] Run the focused tests and retain the RED output.

### Task 2: Feed font metrics into width conversion

**Files:**
- Modify: `crates/office2pdf/src/parser/xlsx_cells.rs`

- [ ] Infer the dominant worksheet font family from existing cells.
- [ ] Use an 8px maximum-digit width for Carlito print geometry while preserving the existing default metric for other workbooks.
- [ ] Convert widths to points before the existing greedy pagination step.
- [ ] Re-run focused tests and all XLSX fixture tests.

## Chunk 2: Visual contract and delivery

### Task 3: Generate and audit #330 evidence

**Files:**
- Create: `assets/bugfixes/issue-330/gt.jpg`
- Create: `assets/bugfixes/issue-330/before.jpg`
- Create: `assets/bugfixes/issue-330/after.jpg`

- [ ] Render native, parent-main, and branch page 1 from the same fixture at 150 DPI.
- [ ] Inspect page count/order, full-resolution crops, 5% fuzz diff, hairlines, and font emphasis.
- [ ] Verify column D moves to the overflow page and classify all remaining deviations with open issues.
- [ ] Encode progressive quality-86 JPEGs with metadata stripped.

### Task 4: Validate and publish

- [ ] Run rustfmt, Clippy, full workspace tests, WASM checks, visual validator tests, and `git diff --check`.
- [ ] Commit with `git commit -s` using developer0hye identity.
- [ ] Verify GitHub auth immediately before every write, open the English PR, require all CI green, review the GitHub diff, and merge with `--merge`.
- [ ] Verify merged main and close #330 before starting #331.
