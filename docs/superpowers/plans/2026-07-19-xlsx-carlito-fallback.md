# XLSX Carlito Sans-Serif Fallback Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Keep XLSX text that requests unavailable Carlito in a metric-compatible sans-serif family so issue #329 matches Excel more closely.

**Architecture:** Extend the existing static substitution table instead of adding XLSX-specific renderer logic. Preserve Carlito as the requested face, provide Calibri and broadly available sans-compatible fallbacks, and rely on the existing font search context to rank installed Office/system families before unavailable candidates.

**Tech Stack:** Rust, office2pdf font search context, Typst font fallback arrays, Cargo tests, Poppler, ImageMagick

---

## Chunk 1: Fallback behavior

### Task 1: Define and rank Carlito fallbacks

**Files:**
- Modify: `crates/office2pdf/src/render/font_subst.rs`
- Test: `crates/office2pdf/src/render/font_subst_tests.rs`

- [ ] **Step 1: Write failing substitution tests**

Assert that `substitutes("Carlito")` returns only sans-compatible candidates, that the generated Typst font array contains the original family plus those candidates, and that a context containing only Arial ranks Arial before unavailable alternatives.

- [ ] **Step 2: Run the focused tests and verify RED**

Run: `cargo test --offline -p office2pdf render::font_subst::tests::test_carlito -- --nocapture`

Expected: FAIL because Carlito currently has no substitution entry and produces a single quoted family.

- [ ] **Step 3: Add the general Carlito substitution chain**

Add Calibri, Liberation Sans, Arimo, and Arial candidates to the existing substitution table. Do not special-case the contributor fixture or XLSX renderer.

- [ ] **Step 4: Re-run focused tests and verify GREEN**

Run: `cargo test --offline -p office2pdf render::font_subst::tests::test_carlito -- --nocapture`

Expected: all Carlito substitution and ranking tests pass.

### Task 2: Lock the contributor fixture output

**Files:**
- Test: `crates/office2pdf/tests/xlsx_fixtures.rs`

- [ ] **Step 1: Add an XLSX fixture acceptance test**

Generate Typst for `pr_186_contributor_acceptance.xlsx` and assert the Carlito header emits a fallback array containing a sans family rather than a single unresolved family.

- [ ] **Step 2: Run the acceptance test**

Run: `cargo test --offline -p office2pdf --test xlsx_fixtures acceptance_pr_186_contributor_acceptance_carlito_fallback -- --nocapture`

Expected: PASS after the substitution-table change and fail on the parent main baseline.

- [ ] **Step 3: Run font, XLSX, and full workspace regressions**

Run the office2pdf font substitution tests, all XLSX fixture tests, and `cargo test --offline --workspace`.

Expected: all pass on native targets and the WASM check remains compatible.

## Chunk 2: Visual contract and delivery

### Task 3: Generate and audit #329 evidence

**Files:**
- Create: `assets/bugfixes/issue-329/gt.jpg`
- Create: `assets/bugfixes/issue-329/before.jpg`
- Create: `assets/bugfixes/issue-329/after.jpg`

- [ ] Render the same Excel page and current-main before/branch after PDFs at 150 DPI.
- [ ] Inspect full-resolution text crops, the 5% fuzz diff, hairlines, and weight/emphasis.
- [ ] Confirm the header is sans-serif and every unrelated remaining deviation references #330 or #331.
- [ ] Encode progressive quality-86 JPEGs with stripped metadata and validate the PR contract.

### Task 4: Publish and merge

**Files:**
- Review all files above

- [ ] Run rustfmt, Clippy, full tests, visual checks, and `git diff --check`.
- [ ] Commit with `git commit -s` using the configured developer0hye identity.
- [ ] Verify GitHub auth immediately before push and PR creation, then publish an English ready-for-review PR related to #329.
- [ ] Require all CI green, review the GitHub diff, merge with `--merge`, verify main, close #329, and clean the worktree when starting #330.
