use super::*;

// ── Helpers ──────────────────────────────────────────────────────────

/// Theme XML with a `<a:fmtScheme>` carrying fill and background fill styles.
/// `bgFillStyleLst` entries: (1001) solid phClr, (1002) solid phClr with 50%
/// shade, (1003) two-stop phClr gradient. `fillStyleLst` entry 1 is solid phClr.
fn make_theme_xml_with_fmt_scheme(colors: &[(&str, &str)]) -> String {
    let mut color_xml = String::new();
    for (name, hex) in colors {
        if *name == "dk1" || *name == "lt1" {
            color_xml.push_str(&format!(
                r#"<a:{name}><a:sysClr val="windowText" lastClr="{hex}"/></a:{name}>"#
            ));
        } else {
            color_xml.push_str(&format!(r#"<a:{name}><a:srgbClr val="{hex}"/></a:{name}>"#));
        }
    }
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><a:themeElements><a:clrScheme name="Test">{color_xml}</a:clrScheme><a:fontScheme name="Test"><a:majorFont><a:latin typeface="Calibri Light"/></a:majorFont><a:minorFont><a:latin typeface="Calibri"/></a:minorFont></a:fontScheme><a:fmtScheme name="Test"><a:fillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:gradFill><a:gsLst><a:gs pos="0"><a:schemeClr val="phClr"/></a:gs><a:gs pos="100000"><a:schemeClr val="phClr"/></a:gs></a:gsLst><a:lin ang="5400000" scaled="0"/></a:gradFill><a:gradFill><a:gsLst><a:gs pos="0"><a:schemeClr val="phClr"/></a:gs><a:gs pos="100000"><a:schemeClr val="phClr"/></a:gs></a:gsLst><a:lin ang="5400000" scaled="0"/></a:gradFill></a:fillStyleLst><a:lnStyleLst><a:ln w="6350"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln><a:ln w="12700"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln><a:ln w="19050"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln></a:lnStyleLst><a:effectStyleLst><a:effectStyle><a:effectLst/></a:effectStyle><a:effectStyle><a:effectLst/></a:effectStyle><a:effectStyle><a:effectLst/></a:effectStyle></a:effectStyleLst><a:bgFillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:solidFill><a:schemeClr val="phClr"><a:shade val="50000"/></a:schemeClr></a:solidFill><a:gradFill rotWithShape="1"><a:gsLst><a:gs pos="0"><a:schemeClr val="phClr"/></a:gs><a:gs pos="100000"><a:schemeClr val="phClr"><a:shade val="50000"/></a:schemeClr></a:gs></a:gsLst><a:lin ang="5400000" scaled="0"/></a:gradFill></a:bgFillStyleLst></a:fmtScheme></a:themeElements></a:theme>"#
    )
}

fn make_empty_slide() -> String {
    r#"<?xml version="1.0" encoding="UTF-8"?><p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree></p:cSld></p:sld>"#
        .to_string()
}

fn make_layout_with_bg(bg_xml: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld>{bg_xml}<p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree></p:cSld></p:sldLayout>"#
    )
}

/// Master with an optional `<p:bg>` and a `<p:clrMap>` that swaps background
/// and text colors (dark-theme mapping), as real dark masters do.
fn make_master_with_bg_and_dark_clr_map(bg_xml: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld>{bg_xml}<p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree></p:cSld><p:clrMap bg1="dk1" tx1="lt1" bg2="dk2" tx2="lt2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/></p:sldMaster>"#
    )
}

fn make_plain_master() -> String {
    make_master_with_bg_and_dark_clr_map("")
}

fn parse_first_page_bg(data: &[u8]) -> (Option<Color>, Option<GradientFill>) {
    let parser = PptxParser;
    let (doc, _warnings) = parser.parse(data, &ConvertOptions::default()).unwrap();
    let page = first_fixed_page(&doc);
    (page.background_color, page.background_gradient.clone())
}

// ── bgRef resolution ─────────────────────────────────────────────────

#[test]
fn test_layout_bg_ref_solid_resolves_theme_background_fill() {
    // onemaster-twolayouts.pptx page 1: layout bgRef idx=1001 with bg2,
    // dark clrMap maps bg2 -> dk2.
    let layout = make_layout_with_bg(
        r#"<p:bg><p:bgRef idx="1001"><a:schemeClr val="bg2"/></p:bgRef></p:bg>"#,
    );
    let data = build_test_pptx_with_theme_layout_master(
        SLIDE_CX,
        SLIDE_CY,
        &make_empty_slide(),
        &layout,
        &make_plain_master(),
        &make_theme_xml_with_fmt_scheme(&standard_theme_colors()),
    );

    let (color, _gradient) = parse_first_page_bg(&data);
    // dk2 in standard_theme_colors is 1F4D78.
    assert_eq!(color, Some(Color::new(0x1F, 0x4D, 0x78)));
}

#[test]
fn test_master_bg_ref_solid_used_when_layout_has_no_bg() {
    // onemaster-twolayouts.pptx page 2: layout has no <p:bg>; master bgRef
    // idx=1001 with bg1, dark clrMap maps bg1 -> dk1 (black).
    let layout = make_layout_with_bg("");
    let master = make_master_with_bg_and_dark_clr_map(
        r#"<p:bg><p:bgRef idx="1001"><a:schemeClr val="bg1"/></p:bgRef></p:bg>"#,
    );
    let data = build_test_pptx_with_theme_layout_master(
        SLIDE_CX,
        SLIDE_CY,
        &make_empty_slide(),
        &layout,
        &master,
        &make_theme_xml_with_fmt_scheme(&standard_theme_colors()),
    );

    let (color, _gradient) = parse_first_page_bg(&data);
    assert_eq!(color, Some(Color::new(0x00, 0x00, 0x00)));
}

#[test]
fn test_bg_ref_applies_color_transforms_from_theme_fill() {
    // bgFillStyleLst entry 2 (idx=1002) shades phClr by 50%.
    let layout = make_layout_with_bg(
        r#"<p:bg><p:bgRef idx="1002"><a:schemeClr val="accent1"/></p:bgRef></p:bg>"#,
    );
    let data = build_test_pptx_with_theme_layout_master(
        SLIDE_CX,
        SLIDE_CY,
        &make_empty_slide(),
        &layout,
        &make_plain_master(),
        &make_theme_xml_with_fmt_scheme(&standard_theme_colors()),
    );

    let (color, _gradient) = parse_first_page_bg(&data);
    // accent1 4472C4 shaded 50% -> 22 39 62.
    assert_eq!(color, Some(Color::new(0x22, 0x39, 0x62)));
}

#[test]
fn test_bg_ref_gradient_resolves_theme_background_fill() {
    // bgFillStyleLst entry 3 (idx=1003) is a two-stop phClr gradient.
    let layout = make_layout_with_bg(
        r#"<p:bg><p:bgRef idx="1003"><a:schemeClr val="accent1"/></p:bgRef></p:bg>"#,
    );
    let data = build_test_pptx_with_theme_layout_master(
        SLIDE_CX,
        SLIDE_CY,
        &make_empty_slide(),
        &layout,
        &make_plain_master(),
        &make_theme_xml_with_fmt_scheme(&standard_theme_colors()),
    );

    let (_color, gradient) = parse_first_page_bg(&data);
    let gradient = gradient.expect("expected gradient background from bgRef");
    assert_eq!(gradient.stops.len(), 2);
    assert_eq!(gradient.stops[0].color, Color::new(0x44, 0x72, 0xC4));
    assert_eq!(gradient.stops[1].color, Color::new(0x22, 0x39, 0x62));
}

// ── Layer precedence and regressions ─────────────────────────────────

#[test]
fn test_slide_bg_pr_still_wins_over_layout_bg_ref() {
    let slide = r#"<?xml version="1.0" encoding="UTF-8"?><p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld><p:bg><p:bgPr><a:solidFill><a:srgbClr val="FF0000"/></a:solidFill><a:effectLst/></p:bgPr></p:bg><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree></p:cSld></p:sld>"#;
    let layout = make_layout_with_bg(
        r#"<p:bg><p:bgRef idx="1001"><a:schemeClr val="bg2"/></p:bgRef></p:bg>"#,
    );
    let data = build_test_pptx_with_theme_layout_master(
        SLIDE_CX,
        SLIDE_CY,
        slide,
        &layout,
        &make_plain_master(),
        &make_theme_xml_with_fmt_scheme(&standard_theme_colors()),
    );

    let (color, _gradient) = parse_first_page_bg(&data);
    assert_eq!(color, Some(Color::new(255, 0, 0)));
}

#[test]
fn test_layout_bg_pr_gradient_is_inherited_by_slide() {
    // Gradient backgrounds must inherit from layout, not only from the slide.
    let layout = make_layout_with_bg(
        r#"<p:bg><p:bgPr><a:gradFill><a:gsLst><a:gs pos="0"><a:srgbClr val="112233"/></a:gs><a:gs pos="100000"><a:srgbClr val="445566"/></a:gs></a:gsLst><a:lin ang="5400000" scaled="0"/></a:gradFill><a:effectLst/></p:bgPr></p:bg>"#,
    );
    let data = build_test_pptx_with_theme_layout_master(
        SLIDE_CX,
        SLIDE_CY,
        &make_empty_slide(),
        &layout,
        &make_plain_master(),
        &make_theme_xml_with_fmt_scheme(&standard_theme_colors()),
    );

    let (_color, gradient) = parse_first_page_bg(&data);
    let gradient = gradient.expect("expected gradient background inherited from layout");
    assert_eq!(gradient.stops.len(), 2);
    assert_eq!(gradient.stops[0].color, Color::new(0x11, 0x22, 0x33));
}

#[test]
fn test_no_background_anywhere_stays_none() {
    let data = build_test_pptx_with_theme_layout_master(
        SLIDE_CX,
        SLIDE_CY,
        &make_empty_slide(),
        &make_layout_with_bg(""),
        &make_plain_master(),
        &make_theme_xml_with_fmt_scheme(&standard_theme_colors()),
    );

    let (color, gradient) = parse_first_page_bg(&data);
    assert_eq!(color, None);
    assert!(gradient.is_none());
}
