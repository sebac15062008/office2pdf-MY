//! Placeholder inheritance for PPTX slides.
//!
//! A slide placeholder (`<p:sp>`/`<p:pic>` with `<p:ph>`) that omits
//! `<a:xfrm>` inherits its position and size from the matching placeholder
//! in the slide layout, which in turn may inherit from the slide master
//! (ECMA-376 §19.3.1.36). Placeholder text likewise stacks the master's
//! `<p:txStyles>` bucket, the master placeholder's `<a:lstStyle>`, and the
//! layout placeholder's `<a:lstStyle>` beneath slide-local properties.

use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};

use super::PptxTextBodyStyleDefaults;
use super::text::parse_pptx_list_style;
use super::theme::{ColorMapData, PptxMasterTextStyles, ThemeData};
use crate::parser::xml_util::{get_attr_i64, get_attr_str};

/// Resolved placeholder position and size, in EMU.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PlaceholderGeometry {
    pub(super) x: i64,
    pub(super) y: i64,
    pub(super) cx: i64,
    pub(super) cy: i64,
}

/// A placeholder declared by one inheritance layer (layout or master).
#[derive(Debug, Clone)]
struct LayerPlaceholder {
    ph_type: Option<String>,
    ph_idx: Option<String>,
    geometry: Option<PlaceholderGeometry>,
    /// Parsed `<a:lstStyle>` from the placeholder's own `<p:txBody>`.
    text_defaults: Option<PptxTextBodyStyleDefaults>,
}

/// Placeholder inheritance lookup table for one slide, built from its
/// layout and master XML.
#[derive(Debug, Default)]
pub(super) struct PlaceholderGeometryMap {
    layout: Vec<LayerPlaceholder>,
    master: Vec<LayerPlaceholder>,
    master_text_styles: PptxMasterTextStyles,
}

impl PlaceholderGeometryMap {
    pub(super) fn build(
        layout_xml: Option<&str>,
        master_xml: Option<&str>,
        theme: &ThemeData,
        layout_color_map: &ColorMapData,
        master_color_map: &ColorMapData,
        master_text_styles: PptxMasterTextStyles,
    ) -> Self {
        Self {
            layout: layout_xml
                .map(|xml| scan_layer_placeholders(xml, theme, layout_color_map))
                .unwrap_or_default(),
            master: master_xml
                .map(|xml| scan_layer_placeholders(xml, theme, master_color_map))
                .unwrap_or_default(),
            master_text_styles,
        }
    }

    /// Resolve the inherited text style defaults for a slide placeholder:
    /// the master `txStyles` bucket for the placeholder type, overlaid by the
    /// master placeholder's `<a:lstStyle>`, then the layout placeholder's.
    pub(super) fn text_defaults(
        &self,
        ph_type: Option<&str>,
        ph_idx: Option<&str>,
    ) -> PptxTextBodyStyleDefaults {
        let mut defaults: PptxTextBodyStyleDefaults = match normalized_master_type(ph_type) {
            "title" => self.master_text_styles.title.clone(),
            "body" => self.master_text_styles.body.clone(),
            _ => self.master_text_styles.other.clone(),
        };
        if let Some(overlay) =
            find_in_master(&self.master, ph_type).and_then(|entry| entry.text_defaults.as_ref())
        {
            defaults.merge_from(overlay);
        }
        if let Some(overlay) = find_in_layer(&self.layout, ph_type, ph_idx)
            .and_then(|entry| entry.text_defaults.as_ref())
        {
            defaults.merge_from(overlay);
        }
        defaults
    }

    /// Resolve the effective geometry for a slide placeholder:
    /// layout match first, then master fallback.
    pub(super) fn lookup(
        &self,
        ph_type: Option<&str>,
        ph_idx: Option<&str>,
    ) -> Option<PlaceholderGeometry> {
        if let Some(entry) = find_in_layer(&self.layout, ph_type, ph_idx) {
            if let Some(geometry) = entry.geometry {
                return Some(geometry);
            }
            // The layout declares the placeholder but omits <a:xfrm>;
            // continue the chain into the master using the layout's own type.
            if let Some(geometry) =
                find_in_master(&self.master, entry.ph_type.as_deref()).and_then(|m| m.geometry)
            {
                return Some(geometry);
            }
        }
        find_in_master(&self.master, ph_type).and_then(|m| m.geometry)
    }
}

/// `title` and `ctrTitle` are interchangeable for matching purposes.
fn is_title_family(ph_type: Option<&str>) -> bool {
    matches!(ph_type, Some("title") | Some("ctrTitle"))
}

/// Map any placeholder type onto the placeholder actually present on a
/// master: `title`, `dt`, `ftr`, `sldNum`, or `body` (all content types).
fn normalized_master_type(ph_type: Option<&str>) -> &'static str {
    match ph_type {
        Some("title") | Some("ctrTitle") => "title",
        Some("dt") => "dt",
        Some("ftr") => "ftr",
        Some("sldNum") => "sldNum",
        _ => "body",
    }
}

/// Match a slide placeholder against layout placeholders:
/// title family by type, `dt`/`ftr`/`sldNum` by type, everything else by
/// `idx` (defaulting to "0"), with a final fallback on the exact type.
fn find_in_layer<'a>(
    entries: &'a [LayerPlaceholder],
    ph_type: Option<&str>,
    ph_idx: Option<&str>,
) -> Option<&'a LayerPlaceholder> {
    if is_title_family(ph_type)
        && let Some(entry) = entries
            .iter()
            .find(|entry| is_title_family(entry.ph_type.as_deref()))
    {
        return Some(entry);
    }
    if matches!(ph_type, Some("dt") | Some("ftr") | Some("sldNum"))
        && let Some(entry) = entries
            .iter()
            .find(|entry| entry.ph_type.as_deref() == ph_type)
    {
        return Some(entry);
    }
    let idx: &str = ph_idx.unwrap_or("0");
    if let Some(entry) = entries
        .iter()
        .find(|entry| entry.ph_idx.as_deref().unwrap_or("0") == idx)
    {
        return Some(entry);
    }
    ph_type.and_then(|_| {
        entries
            .iter()
            .find(|entry| entry.ph_type.as_deref() == ph_type)
    })
}

/// Match against master placeholders by normalized type only: masters give
/// `dt`/`ftr`/`sldNum` placeholders idx values unrelated to layout/slide ones.
fn find_in_master<'a>(
    entries: &'a [LayerPlaceholder],
    ph_type: Option<&str>,
) -> Option<&'a LayerPlaceholder> {
    let wanted: &str = normalized_master_type(ph_type);
    entries
        .iter()
        .find(|entry| normalized_master_type(entry.ph_type.as_deref()) == wanted)
}

/// Collect every placeholder `<p:sp>`/`<p:pic>` in a layout/master layer
/// together with its explicit `<a:xfrm>` geometry and `<a:lstStyle>`, if any.
fn scan_layer_placeholders(
    xml: &str,
    theme: &ThemeData,
    color_map: &ColorMapData,
) -> Vec<LayerPlaceholder> {
    let mut reader: Reader<&[u8]> = Reader::from_str(xml);

    #[derive(Default)]
    struct Current {
        ph_type: Option<String>,
        ph_idx: Option<String>,
        has_ph: bool,
        x: Option<i64>,
        y: Option<i64>,
        cx: Option<i64>,
        cy: Option<i64>,
        in_sp_pr: bool,
        in_xfrm: bool,
        text_defaults: Option<PptxTextBodyStyleDefaults>,
    }

    fn handle_simple_start(current: &mut Option<Current>, e: &BytesStart) {
        match e.local_name().as_ref() {
            b"sp" | b"pic" => {
                *current = Some(Current::default());
            }
            b"ph" => {
                if let Some(state) = current.as_mut() {
                    state.has_ph = true;
                    state.ph_type = get_attr_str(e, b"type");
                    state.ph_idx = get_attr_str(e, b"idx");
                }
            }
            b"spPr" => {
                if let Some(state) = current.as_mut() {
                    state.in_sp_pr = true;
                }
            }
            b"xfrm" => {
                if let Some(state) = current.as_mut()
                    && state.in_sp_pr
                {
                    state.in_xfrm = true;
                }
            }
            b"off" => {
                if let Some(state) = current.as_mut()
                    && state.in_xfrm
                {
                    state.x = get_attr_i64(e, b"x");
                    state.y = get_attr_i64(e, b"y");
                }
            }
            b"ext" => {
                if let Some(state) = current.as_mut()
                    && state.in_xfrm
                {
                    state.cx = get_attr_i64(e, b"cx");
                    state.cy = get_attr_i64(e, b"cy");
                }
            }
            _ => {}
        }
    }

    let mut entries: Vec<LayerPlaceholder> = Vec::new();
    let mut current: Option<Current> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                if e.local_name().as_ref() == b"lstStyle" && current.is_some() {
                    let defaults: PptxTextBodyStyleDefaults =
                        parse_pptx_list_style(&mut reader, theme, color_map);
                    if let Some(state) = current.as_mut() {
                        state.text_defaults = Some(defaults);
                    }
                } else {
                    handle_simple_start(&mut current, e);
                }
            }
            Ok(Event::Empty(ref e)) => {
                handle_simple_start(&mut current, e);
            }
            Ok(Event::End(ref e)) => match e.local_name().as_ref() {
                b"sp" | b"pic" => {
                    if let Some(state) = current.take()
                        && state.has_ph
                    {
                        let geometry: Option<PlaceholderGeometry> =
                            match (state.x, state.y, state.cx, state.cy) {
                                (Some(x), Some(y), Some(cx), Some(cy)) => {
                                    Some(PlaceholderGeometry { x, y, cx, cy })
                                }
                                _ => None,
                            };
                        entries.push(LayerPlaceholder {
                            ph_type: state.ph_type,
                            ph_idx: state.ph_idx,
                            geometry,
                            text_defaults: state.text_defaults,
                        });
                    }
                }
                b"spPr" => {
                    if let Some(state) = current.as_mut() {
                        state.in_sp_pr = false;
                    }
                }
                b"xfrm" => {
                    if let Some(state) = current.as_mut() {
                        state.in_xfrm = false;
                    }
                }
                _ => {}
            },
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }

    entries
}
