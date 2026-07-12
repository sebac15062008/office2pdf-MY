use std::fmt::Write;

const PLACEABLE_KEY: u32 = 0x9AC6_CDD7;
const META_EOF: u16 = 0x0000;
const META_SET_POLY_FILL_MODE: u16 = 0x0106;
const META_SELECT_OBJECT: u16 = 0x012D;
const META_DELETE_OBJECT: u16 = 0x01F0;
const META_SET_WINDOW_EXT: u16 = 0x020C;
const META_CREATE_PEN_INDIRECT: u16 = 0x02FA;
const META_CREATE_BRUSH_INDIRECT: u16 = 0x02FC;
const META_POLYGON: u16 = 0x0324;

const BS_SOLID: u16 = 0;
const BS_NULL: u16 = 1;
const PS_NULL: u16 = 5;

#[derive(Clone, Copy)]
struct RgbColor {
    red: u8,
    green: u8,
    blue: u8,
}

impl RgbColor {
    fn from_colorref(colorref: u32) -> Self {
        Self {
            red: (colorref & 0xff) as u8,
            green: ((colorref >> 8) & 0xff) as u8,
            blue: ((colorref >> 16) & 0xff) as u8,
        }
    }

    fn as_svg_hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.red, self.green, self.blue)
    }
}

#[derive(Clone, Copy)]
enum WmfObject {
    Brush(Option<RgbColor>),
    Pen { color: Option<RgbColor>, width: i16 },
}

struct WmfSvgConverter {
    objects: Vec<Option<WmfObject>>,
    brush: Option<RgbColor>,
    pen: Option<RgbColor>,
    pen_width: i16,
    uses_even_odd_fill: bool,
    has_inverted_y_axis: bool,
    elements: String,
}

impl WmfSvgConverter {
    fn new() -> Self {
        Self {
            objects: Vec::new(),
            brush: None,
            pen: None,
            pen_width: 1,
            uses_even_odd_fill: true,
            has_inverted_y_axis: false,
            elements: String::new(),
        }
    }

    fn insert_object(&mut self, object: WmfObject) {
        if let Some(slot) = self.objects.iter_mut().find(|slot| slot.is_none()) {
            *slot = Some(object);
        } else {
            self.objects.push(Some(object));
        }
    }

    fn handle_record(&mut self, record_type: u16, parameters: &[u8]) -> Option<()> {
        match record_type {
            META_CREATE_BRUSH_INDIRECT => {
                let style: u16 = read_u16(parameters, 0)?;
                let color: Option<RgbColor> = if style == BS_SOLID {
                    Some(RgbColor::from_colorref(read_u32(parameters, 2)?))
                } else {
                    None
                };
                self.insert_object(WmfObject::Brush(if style == BS_NULL {
                    None
                } else {
                    color
                }));
            }
            META_CREATE_PEN_INDIRECT => {
                let style: u16 = read_u16(parameters, 0)? & 0x000f;
                let width: i16 = read_i16(parameters, 2)?.unsigned_abs().max(1) as i16;
                let color: Option<RgbColor> = if style != PS_NULL {
                    Some(RgbColor::from_colorref(read_u32(parameters, 6)?))
                } else {
                    None
                };
                self.insert_object(WmfObject::Pen { color, width });
            }
            META_SELECT_OBJECT => {
                let index: usize = read_u16(parameters, 0)? as usize;
                match self.objects.get(index).copied().flatten()? {
                    WmfObject::Brush(color) => self.brush = color,
                    WmfObject::Pen { color, width } => {
                        self.pen = color;
                        self.pen_width = width;
                    }
                }
            }
            META_DELETE_OBJECT => {
                let index: usize = read_u16(parameters, 0)? as usize;
                if let Some(slot) = self.objects.get_mut(index) {
                    *slot = None;
                }
            }
            META_SET_POLY_FILL_MODE => {
                self.uses_even_odd_fill = read_u16(parameters, 0)? == 1;
            }
            META_SET_WINDOW_EXT => {
                self.has_inverted_y_axis = read_i16(parameters, 0)? < 0;
            }
            META_POLYGON => self.write_polygon(parameters)?,
            _ => {}
        }
        Some(())
    }

    fn write_polygon(&mut self, parameters: &[u8]) -> Option<()> {
        let point_count: usize = read_u16(parameters, 0)? as usize;
        if point_count < 2 || 2usize.checked_add(point_count.checked_mul(4)?)? > parameters.len() {
            return None;
        }

        self.elements.push_str("<path d=\"M");
        for index in 0..point_count {
            let offset: usize = 2 + index * 4;
            let x: i16 = read_i16(parameters, offset)?;
            let y: i16 = read_i16(parameters, offset + 2)?;
            if index > 0 {
                self.elements.push(' ');
            }
            write!(self.elements, "{x} {y}").ok()?;
        }
        self.elements.push_str("Z\"");
        match self.brush {
            Some(color) => write!(self.elements, " fill=\"{}\"", color.as_svg_hex()).ok()?,
            None => self.elements.push_str(" fill=\"none\""),
        }
        match self.pen {
            Some(color) => write!(
                self.elements,
                " stroke=\"{}\" stroke-width=\"{}\"",
                color.as_svg_hex(),
                self.pen_width
            )
            .ok()?,
            None => self.elements.push_str(" stroke=\"none\""),
        }
        if self.uses_even_odd_fill {
            self.elements.push_str(" fill-rule=\"evenodd\"");
        }
        self.elements.push_str("/>\n");
        Some(())
    }
}

pub(super) fn convert_wmf_to_svg(data: &[u8]) -> Option<Vec<u8>> {
    if read_u32(data, 0)? != PLACEABLE_KEY {
        return None;
    }
    let left: i16 = read_i16(data, 6)?;
    let top: i16 = read_i16(data, 8)?;
    let right: i16 = read_i16(data, 10)?;
    let bottom: i16 = read_i16(data, 12)?;
    let width: i32 = i32::from(right) - i32::from(left);
    let height: i32 = i32::from(bottom) - i32::from(top);
    if width <= 0 || height <= 0 || read_u16(data, 24)? != 9 {
        return None;
    }

    let mut converter: WmfSvgConverter = WmfSvgConverter::new();
    let mut offset: usize = 40;
    let mut saw_eof: bool = false;
    while offset.checked_add(6)? <= data.len() {
        let record_words: usize = read_u32(data, offset)? as usize;
        let record_size: usize = record_words.checked_mul(2)?;
        if record_size < 6 || offset.checked_add(record_size)? > data.len() {
            return None;
        }
        let record_type: u16 = read_u16(data, offset + 4)?;
        converter.handle_record(record_type, &data[offset + 6..offset + record_size])?;
        offset += record_size;
        if record_type == META_EOF {
            saw_eof = true;
            break;
        }
    }
    if !saw_eof || converter.elements.is_empty() {
        return None;
    }

    let elements: String = if converter.has_inverted_y_axis {
        let vertical_translation: i32 = i32::from(top) + i32::from(bottom);
        format!(
            "<g transform=\"translate(0 {vertical_translation}) scale(1 -1)\">\n{}</g>\n",
            converter.elements
        )
    } else {
        converter.elements
    };
    Some(
        format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{left} {top} {width} {height}\">\n{elements}</svg>"
        )
        .into_bytes(),
    )
}

fn read_u16(data: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        data.get(offset..offset + 2)?.try_into().ok()?,
    ))
}

fn read_i16(data: &[u8], offset: usize) -> Option<i16> {
    Some(i16::from_le_bytes(
        data.get(offset..offset + 2)?.try_into().ok()?,
    ))
}

fn read_u32(data: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        data.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(record_type: u16, parameters: &[u8]) -> Vec<u8> {
        let mut bytes: Vec<u8> = Vec::new();
        bytes.extend_from_slice(&((parameters.len() + 6) as u32 / 2).to_le_bytes());
        bytes.extend_from_slice(&record_type.to_le_bytes());
        bytes.extend_from_slice(parameters);
        bytes
    }

    fn simple_wmf(colorref: u32, points: &[(i16, i16)]) -> Vec<u8> {
        let mut records: Vec<u8> = Vec::new();
        let mut window_extent: Vec<u8> = Vec::new();
        window_extent.extend_from_slice(&(-100i16).to_le_bytes());
        window_extent.extend_from_slice(&100i16.to_le_bytes());
        records.extend(record(META_SET_WINDOW_EXT, &window_extent));
        let mut brush: Vec<u8> = Vec::new();
        brush.extend_from_slice(&BS_SOLID.to_le_bytes());
        brush.extend_from_slice(&colorref.to_le_bytes());
        brush.extend_from_slice(&0u16.to_le_bytes());
        records.extend(record(META_CREATE_BRUSH_INDIRECT, &brush));
        records.extend(record(META_SELECT_OBJECT, &0u16.to_le_bytes()));
        let mut polygon: Vec<u8> = Vec::new();
        polygon.extend_from_slice(&(points.len() as u16).to_le_bytes());
        for (x, y) in points {
            polygon.extend_from_slice(&x.to_le_bytes());
            polygon.extend_from_slice(&y.to_le_bytes());
        }
        records.extend(record(META_POLYGON, &polygon));
        records.extend(record(META_EOF, &[]));

        let mut data: Vec<u8> = Vec::new();
        data.extend_from_slice(&PLACEABLE_KEY.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&(-10i16).to_le_bytes());
        data.extend_from_slice(&(-20i16).to_le_bytes());
        data.extend_from_slice(&90i16.to_le_bytes());
        data.extend_from_slice(&80i16.to_le_bytes());
        data.extend_from_slice(&1440u16.to_le_bytes());
        data.extend_from_slice(&0u32.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&1u16.to_le_bytes());
        data.extend_from_slice(&9u16.to_le_bytes());
        data.extend_from_slice(&0x0300u16.to_le_bytes());
        data.extend_from_slice(&((18 + records.len()) as u32 / 2).to_le_bytes());
        data.extend_from_slice(&1u16.to_le_bytes());
        data.extend_from_slice(&0u32.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend(records);
        data
    }

    #[test]
    fn converts_placeable_wmf_polygon_to_svg() {
        let data: Vec<u8> = simple_wmf(0x0033_2211, &[(0, 0), (30, 0), (15, 40)]);
        let svg: String = String::from_utf8(convert_wmf_to_svg(&data).unwrap()).unwrap();

        assert!(svg.contains("viewBox=\"-10 -20 100 100\""));
        assert!(svg.contains("transform=\"translate(0 60) scale(1 -1)\""));
        assert!(svg.contains("M0 0 30 0 15 40Z"));
        assert!(svg.contains("fill=\"#112233\""));
    }

    #[test]
    fn converts_different_polygon_without_fixture_specific_assumptions() {
        let data: Vec<u8> = simple_wmf(0x0000_80ff, &[(-5, 6), (7, 8), (9, -10), (2, 1)]);
        let svg: String = String::from_utf8(convert_wmf_to_svg(&data).unwrap()).unwrap();

        assert!(svg.contains("M-5 6 7 8 9 -10 2 1Z"));
        assert!(svg.contains("fill=\"#ff8000\""));
    }

    #[test]
    fn rejects_non_wmf_data() {
        assert!(convert_wmf_to_svg(b"not a metafile").is_none());
    }
}
