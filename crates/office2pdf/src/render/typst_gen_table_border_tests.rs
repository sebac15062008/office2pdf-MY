use super::*;

#[test]
fn test_table_all_borders() {
    let cell = TableCell {
        content: vec![Block::Paragraph(Paragraph {
            style: ParagraphStyle::default(),
            runs: vec![Run {
                text: "All borders".to_string(),
                style: TextStyle::default(),
                href: None,
                footnote: None,
            }],
        })],
        border: Some(CellBorder {
            top: Some(BorderSide {
                width: 1.0,
                color: Color::black(),
                style: BorderLineStyle::Solid,
            }),
            bottom: Some(BorderSide {
                width: 1.0,
                color: Color::black(),
                style: BorderLineStyle::Solid,
            }),
            left: Some(BorderSide {
                width: 1.0,
                color: Color::black(),
                style: BorderLineStyle::Solid,
            }),
            right: Some(BorderSide {
                width: 1.0,
                color: Color::black(),
                style: BorderLineStyle::Solid,
            }),
        }),
        ..TableCell::default()
    };
    let table = Table {
        rows: vec![TableRow {
            cells: vec![cell],
            height: None,
        }],
        column_widths: vec![100.0],
        ..Table::default()
    };
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(result.contains("top:"), "Expected top border in: {result}");
    assert!(
        result.contains("bottom:"),
        "Expected bottom border in: {result}"
    );
    assert!(
        result.contains("left:"),
        "Expected left border in: {result}"
    );
    assert!(
        result.contains("right:"),
        "Expected right border in: {result}"
    );
}

#[test]
fn test_table_dashed_border_codegen() {
    let cell = TableCell {
        content: vec![Block::Paragraph(Paragraph {
            style: ParagraphStyle::default(),
            runs: vec![Run {
                text: "Dashed".to_string(),
                style: TextStyle::default(),
                href: None,
                footnote: None,
            }],
        })],
        border: Some(CellBorder {
            top: Some(BorderSide {
                width: 1.0,
                color: Color::black(),
                style: BorderLineStyle::Dashed,
            }),
            bottom: Some(BorderSide {
                width: 1.0,
                color: Color::new(255, 0, 0),
                style: BorderLineStyle::Dotted,
            }),
            left: None,
            right: None,
        }),
        ..TableCell::default()
    };
    let table = Table {
        rows: vec![TableRow {
            cells: vec![cell],
            height: None,
        }],
        column_widths: vec![100.0],
        ..Table::default()
    };
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(
        result.contains("dash: \"dashed\""),
        "Expected dashed dash pattern in: {result}"
    );
    assert!(
        result.contains("dash: \"dotted\""),
        "Expected dotted dash pattern in: {result}"
    );
}

#[test]
fn test_table_double_borders_render_two_oriented_rules() {
    let cell = TableCell {
        content: vec![Block::Paragraph(Paragraph {
            style: ParagraphStyle::default(),
            runs: vec![Run {
                text: "Double".to_string(),
                style: TextStyle::default(),
                href: None,
                footnote: None,
            }],
        })],
        border: Some(CellBorder {
            top: Some(BorderSide {
                width: 0.8,
                color: Color::new(10, 20, 30),
                style: BorderLineStyle::Double,
            }),
            bottom: Some(BorderSide {
                width: 0.8,
                color: Color::new(10, 20, 30),
                style: BorderLineStyle::Double,
            }),
            left: Some(BorderSide {
                width: 0.8,
                color: Color::new(10, 20, 30),
                style: BorderLineStyle::Double,
            }),
            right: Some(BorderSide {
                width: 0.8,
                color: Color::new(10, 20, 30),
                style: BorderLineStyle::Double,
            }),
        }),
        ..TableCell::default()
    };
    let table = Table {
        rows: vec![
            TableRow {
                cells: vec![TableCell::default(), TableCell::default()],
                height: None,
            },
            TableRow {
                cells: vec![TableCell::default(), cell],
                height: None,
            },
        ],
        column_widths: vec![50.0, 50.0],
        ..Table::default()
    };
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let output = generate_typst(&doc).unwrap();
    let result = &output.source;

    assert_eq!(
        result.matches("stroke: 0.8pt + rgb(10, 20, 30)").count(),
        8,
        "each double side should render as two one-width rules: {result}"
    );
    assert!(
        result.contains(
            "#place(top + left, dx: -5pt, dy: -5.8pt, line(length: 100% + 10pt, angle: 0deg"
        ),
        "the outer horizontal rule should sit one width above the cell edge: {result}"
    );
    assert!(
        result.contains(
            "#place(top + left, dx: -5pt, dy: -4.2pt, line(length: 100% + 10pt, angle: 0deg"
        ),
        "the inner horizontal rule should sit one width below the cell edge: {result}"
    );
    assert!(
        result.contains(
            "#place(top + left, dx: -5.8pt, dy: -5pt, line(length: 100% + 10pt, angle: 90deg"
        ),
        "the outer vertical rule should sit one width before the cell edge: {result}"
    );
    assert!(
        result.contains(
            "#place(top + left, dx: -4.2pt, dy: -5pt, line(length: 100% + 10pt, angle: 90deg"
        ),
        "the inner vertical rule should sit one width after the cell edge: {result}"
    );
    assert!(
        result.contains(
            "#place(bottom + left, dx: -5pt, dy: 4.2pt, line(length: 100% + 10pt, angle: 0deg"
        ),
        "the inner bottom rule should sit one width above the cell edge: {result}"
    );
    assert!(
        result.contains(
            "#place(bottom + left, dx: -5pt, dy: 5.8pt, line(length: 100% + 10pt, angle: 0deg"
        ),
        "the outer bottom rule should sit one width below the cell edge: {result}"
    );
    assert!(
        result.contains(
            "#place(top + right, dx: 4.2pt, dy: -5pt, line(length: 100% + 10pt, angle: 90deg"
        ),
        "the inner right rule should sit one width before the cell edge: {result}"
    );
    assert!(
        result.contains(
            "#place(top + right, dx: 5.8pt, dy: -5pt, line(length: 100% + 10pt, angle: 90deg"
        ),
        "the outer right rule should sit one width after the cell edge: {result}"
    );

    #[cfg(not(target_arch = "wasm32"))]
    {
        let pdf = crate::render::pdf::compile_to_pdf(
            &output.source,
            &output.images,
            None,
            &[],
            false,
            false,
        )
        .expect("double-border Typst should compile");
        assert!(pdf.starts_with(b"%PDF"));
    }
}

#[test]
fn test_shape_dashed_stroke_codegen() {
    let doc = make_doc(vec![make_fixed_page(
        960.0,
        540.0,
        vec![make_shape_element(
            10.0,
            10.0,
            100.0,
            100.0,
            ShapeKind::Rectangle,
            Some(Color::new(0, 128, 255)),
            Some(BorderSide {
                width: 2.0,
                color: Color::black(),
                style: BorderLineStyle::Dashed,
            }),
        )],
    )]);
    let output = generate_typst(&doc).unwrap();
    assert!(
        output.source.contains("dash: \"dashed\""),
        "Expected dashed stroke in: {}",
        output.source
    );
}

#[test]
fn test_shape_dash_dot_stroke_codegen() {
    let doc = make_doc(vec![make_fixed_page(
        960.0,
        540.0,
        vec![make_shape_element(
            10.0,
            10.0,
            100.0,
            100.0,
            ShapeKind::Ellipse,
            None,
            Some(BorderSide {
                width: 1.0,
                color: Color::new(0, 0, 255),
                style: BorderLineStyle::DashDot,
            }),
        )],
    )]);
    let output = generate_typst(&doc).unwrap();
    assert!(
        output.source.contains("dash: \"dash-dotted\""),
        "Expected dash-dotted stroke in: {}",
        output.source
    );
}

#[test]
fn test_border_line_style_to_typst_mapping() {
    assert_eq!(border_line_style_to_typst(BorderLineStyle::Solid), "solid");
    assert_eq!(
        border_line_style_to_typst(BorderLineStyle::Dashed),
        "dashed"
    );
    assert_eq!(
        border_line_style_to_typst(BorderLineStyle::Dotted),
        "dotted"
    );
    assert_eq!(
        border_line_style_to_typst(BorderLineStyle::DashDot),
        "dash-dotted"
    );
    assert_eq!(
        border_line_style_to_typst(BorderLineStyle::DashDotDot),
        "dash-dotted"
    );
    assert_eq!(border_line_style_to_typst(BorderLineStyle::Double), "solid");
    assert_eq!(border_line_style_to_typst(BorderLineStyle::None), "solid");
}

#[test]
fn test_solid_border_no_dash_param() {
    let cell = TableCell {
        content: vec![Block::Paragraph(Paragraph {
            style: ParagraphStyle::default(),
            runs: vec![Run {
                text: "Solid".to_string(),
                style: TextStyle::default(),
                href: None,
                footnote: None,
            }],
        })],
        border: Some(CellBorder {
            top: Some(BorderSide {
                width: 1.0,
                color: Color::black(),
                style: BorderLineStyle::Solid,
            }),
            bottom: None,
            left: None,
            right: None,
        }),
        ..TableCell::default()
    };
    let table = Table {
        rows: vec![TableRow {
            cells: vec![cell],
            height: None,
        }],
        column_widths: vec![100.0],
        ..Table::default()
    };
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(
        !result.contains("dash:"),
        "Solid border should not have dash parameter in: {result}"
    );
    assert!(
        result.contains("1pt + rgb(0, 0, 0)"),
        "Expected simple solid format in: {result}"
    );
}
