//! Small, intentionally restricted SVG reader for our hand-authored room assets.
//! Geometry is parsed once and drawn as vectors at the current display resolution.
#[derive(Clone, Debug)]
pub enum Shape {
    Rect {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        radius: f32,
    },
    Ellipse {
        x: f32,
        y: f32,
        rx: f32,
        ry: f32,
    },
    Polygon(Vec<(f32, f32)>),
}
#[derive(Clone, Debug)]
pub struct Element {
    pub shape: Shape,
    pub color: [u8; 3],
}
#[derive(Clone, Debug)]
pub struct Scene {
    pub elements: Vec<Element>,
}
fn attribute<'a>(tag: &'a str, key: &str) -> Option<&'a str> {
    let needle = format!("{key}=\"");
    let start = tag
        .match_indices(&needle)
        .find(|(i, _)| *i == 0 || tag.as_bytes()[i - 1].is_ascii_whitespace())?
        .0
        + needle.len();
    tag[start..].split('"').next()
}
fn number(tag: &str, key: &str, default: f32) -> Result<f32, &'static str> {
    let Some(v) = attribute(tag, key) else {
        return Ok(default);
    };
    let n: f32 = v.parse().map_err(|_| "invalid coordinate")?;
    if n.is_finite() {
        Ok(n)
    } else {
        Err("nonfinite coordinate")
    }
}
impl Scene {
    pub fn parse(svg: &str) -> Result<Self, &'static str> {
        if !svg.contains("viewBox=\"0 0 390 844\"") {
            return Err("unsupported viewBox");
        }
        let mut elements = Vec::new();
        for part in svg.split('<').skip(1) {
            let tag = part.split('>').next().ok_or("unclosed element")?;
            let kind = tag.split_whitespace().next().ok_or("empty element")?;
            if matches!(kind, "svg" | "/svg") {
                continue;
            }
            let n = |k, d| number(tag, k, d);
            let shape = match kind {
                "rect" => Shape::Rect {
                    x: n("x", 0.)?,
                    y: n("y", 0.)?,
                    w: n("width", 0.)?,
                    h: n("height", 0.)?,
                    radius: n("rx", 0.)?,
                },
                "circle" => {
                    let r = n("r", 0.)?;
                    Shape::Ellipse {
                        x: n("cx", 0.)?,
                        y: n("cy", 0.)?,
                        rx: r,
                        ry: r,
                    }
                }
                "ellipse" => Shape::Ellipse {
                    x: n("cx", 0.)?,
                    y: n("cy", 0.)?,
                    rx: n("rx", 0.)?,
                    ry: n("ry", 0.)?,
                },
                "polygon" => {
                    let points: Result<Vec<f32>, _> = attribute(tag, "points")
                        .ok_or("missing points")?
                        .split(|c: char| c == ',' || c.is_whitespace())
                        .filter(|s| !s.is_empty())
                        .map(str::parse::<f32>)
                        .collect();
                    let points = points.map_err(|_| "invalid polygon")?;
                    if points.len() < 6
                        || points.len() % 2 != 0
                        || points.iter().any(|p| !p.is_finite())
                    {
                        return Err("invalid polygon");
                    }
                    Shape::Polygon(
                        points
                            .as_chunks::<2>()
                            .0
                            .iter()
                            .map(|p| (p[0], p[1]))
                            .collect(),
                    )
                }
                _ => return Err("unsupported SVG element"),
            };
            let fill = attribute(tag, "fill").ok_or("missing fill")?;
            if fill.len() != 7 || !fill.starts_with('#') || !fill.is_ascii() {
                return Err("expected hex color");
            }
            let mut color = [0; 3];
            for (i, c) in color.iter_mut().enumerate() {
                *c = u8::from_str_radix(&fill[1 + i * 2..3 + i * 2], 16)
                    .map_err(|_| "invalid color")?;
            }
            match &shape {
                Shape::Rect { w, h, radius, .. } if *w < 0. || *h < 0. || *radius < 0. => {
                    return Err("negative size");
                }
                Shape::Ellipse { rx, ry, .. } if *rx < 0. || *ry < 0. => {
                    return Err("negative size");
                }
                _ => {}
            }
            elements.push(Element { shape, color });
        }
        if elements.is_empty() {
            return Err("empty SVG");
        }
        Ok(Self { elements })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_room_is_editable_vector_geometry_without_raster_images() {
        for svg in [
            include_str!("../assets/fih/kitchen.svg"),
            include_str!("../assets/fih/bathroom.svg"),
            include_str!("../assets/fih/bedroom.svg"),
            include_str!("../assets/fih/playroom.svg"),
            include_str!("../assets/fih/clinic.svg"),
        ] {
            let scene = Scene::parse(svg).unwrap();
            assert!(scene.elements.len() >= 15);
            assert!(!svg.contains("image") && !svg.contains("data:"));
        }
    }
    #[test]
    fn invalid_geometry_and_external_elements_are_rejected() {
        for tag in [
            "<image href=\"https://example.com\"/>",
            "<circle r=\"NaN\" fill=\"#ffffff\"/>",
            "<polygon points=\"0,0 1,1\" fill=\"#ffffff\"/>",
            "<rect width=\"-1\" fill=\"#ffffff\"/>",
        ] {
            assert!(Scene::parse(&format!("<svg viewBox=\"0 0 390 844\">{tag}</svg>")).is_err());
        }
    }
}
