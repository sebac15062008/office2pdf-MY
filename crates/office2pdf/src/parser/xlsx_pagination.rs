//! Column-wise pagination for sheets wider than the printable page.
//!
//! Excel prints columns that overflow the page width on subsequent pages
//! (default order: down, then over). office2pdf previously clipped them at
//! the right page edge, silently losing content.

use crate::ir::{SheetPage, Table, TableCell, TableRow};

/// Upper bound on overflow pages per sheet chunk. Pathological sheets (used
/// ranges thousands of columns wide) would otherwise explode into thousands
/// of pages and blow the Typst compiler's stack; columns beyond the cap stay
/// on the last page (clipped, the pre-pagination behavior).
const MAX_COLUMN_GROUPS: usize = 12;

/// Split a sheet page into column groups that each fit the printable width.
/// Returns the page unchanged when everything fits.
pub(super) fn split_sheet_page_by_width(page: SheetPage) -> Vec<SheetPage> {
    let printable_width: f64 = page.size.width - page.margins.left - page.margins.right;
    let total_width: f64 = page.table.column_widths.iter().sum();
    if total_width <= printable_width || page.table.column_widths.len() <= 1 {
        return vec![page];
    }

    let mut groups: Vec<(usize, usize)> = column_groups(&page.table.column_widths, printable_width);
    if groups.len() <= 1 {
        return vec![page];
    }
    if groups.len() > MAX_COLUMN_GROUPS {
        let column_count = page.table.column_widths.len();
        groups.truncate(MAX_COLUMN_GROUPS);
        if let Some(last) = groups.last_mut() {
            last.1 = column_count;
        }
    }

    let mut result: Vec<SheetPage> = Vec::with_capacity(groups.len());
    for (index, &(start, end)) in groups.iter().enumerate() {
        let table: Table = slice_table_columns(&page.table, start, end);
        result.push(SheetPage {
            name: page.name.clone(),
            size: page.size,
            margins: page.margins,
            table,
            header: page.header.clone(),
            footer: page.footer.clone(),
            // Charts anchor to rows of the first column group only.
            charts: if index == 0 {
                page.charts.clone()
            } else {
                Vec::new()
            },
        });
    }
    result
}

/// Greedily pack columns left-to-right into groups whose summed width fits
/// the printable width; every group holds at least one column.
fn column_groups(column_widths: &[f64], printable_width: f64) -> Vec<(usize, usize)> {
    let mut groups: Vec<(usize, usize)> = Vec::new();
    let mut start: usize = 0;
    let mut acc: f64 = 0.0;
    for (index, width) in column_widths.iter().enumerate() {
        if index > start && acc + width > printable_width {
            groups.push((start, index));
            start = index;
            acc = 0.0;
        }
        acc += width;
    }
    groups.push((start, column_widths.len()));
    groups
}

/// Build a table containing only columns `[start, end)`, truncating cell
/// spans at the group boundary. A merged cell that starts before the group
/// keeps its geometry (background/border) but not its content, matching how
/// Excel prints merge continuations on overflow pages.
fn slice_table_columns(table: &Table, start: usize, end: usize) -> Table {
    let column_count: usize = table.column_widths.len();
    // Tracks rows still covered by a row-spanning cell, per column.
    let mut rowspan_remaining: Vec<usize> = vec![0; column_count];

    let mut rows: Vec<TableRow> = Vec::with_capacity(table.rows.len());
    for row in &table.rows {
        let mut column_cursor: usize = 0;
        let mut cells: Vec<TableCell> = Vec::new();

        for cell in &row.cells {
            while column_cursor < column_count && rowspan_remaining[column_cursor] > 0 {
                rowspan_remaining[column_cursor] -= 1;
                column_cursor += 1;
            }
            if column_cursor >= column_count {
                break;
            }

            let span: usize = cell.col_span.max(1) as usize;
            let cell_start: usize = column_cursor;
            let cell_end: usize = (column_cursor + span).min(column_count);

            if cell.row_span > 1 {
                for occupied in rowspan_remaining.iter_mut().take(cell_end).skip(cell_start) {
                    *occupied = (cell.row_span - 1) as usize;
                }
            }

            let overlap_start: usize = cell_start.max(start);
            let overlap_end: usize = cell_end.min(end);
            if overlap_start < overlap_end {
                let mut sliced: TableCell = cell.clone();
                sliced.col_span = (overlap_end - overlap_start) as u32;
                if cell_start < start {
                    // Continuation of a merge that began on an earlier page.
                    sliced.content = Vec::new();
                }
                cells.push(sliced);
            }

            column_cursor = cell_end;
        }

        // Columns occupied only by rowspans still need their counters advanced.
        while column_cursor < column_count {
            if rowspan_remaining[column_cursor] > 0 {
                rowspan_remaining[column_cursor] -= 1;
            }
            column_cursor += 1;
        }

        rows.push(TableRow {
            cells,
            height: row.height,
        });
    }

    Table {
        rows,
        column_widths: table.column_widths[start..end].to_vec(),
        header_row_count: table.header_row_count,
        alignment: table.alignment,
        default_cell_padding: table.default_cell_padding,
        use_content_driven_row_heights: table.use_content_driven_row_heights,
    }
}

#[cfg(test)]
#[path = "xlsx_pagination_tests.rs"]
mod tests;
