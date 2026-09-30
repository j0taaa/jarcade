//! Fih is drawn entirely from curves and colored meshes, so each part can move.
use crate::fih_art::{COLORS, INK, PINK};
use jarcade::{fih::Fih, fih_interaction::Pose};
use macroquad::prelude::*;

fn mix(a: Color, b: Color, t: f32) -> Color {
    Color::new(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        a.a + (b.a - a.a) * t,
    )
}
fn cubic(a: Vec2, b: Vec2, c: Vec2, d: Vec2, t: f32) -> Vec2 {
    let u = 1. - t;
    a * u * u * u + b * 3. * u * u * t + c * 3. * u * t * t + d * t * t * t
}
fn contour(start: Vec2, curves: &[(Vec2, Vec2, Vec2)]) -> Vec<Vec2> {
    let mut points = vec![start];
    let mut a = start;
    for &(b, c, d) in curves {
        for i in 1..=16 {
            points.push(cubic(a, b, c, d, i as f32 / 16.));
        }
        a = d;
    }
    points
}
fn path(p: Vec2, r: f32, points: &[Vec2], center: Vec2, color: impl Fn(Vec2) -> Color) {
    let mut vertices = Vec::with_capacity(points.len() + 1);
    let mut indices = Vec::with_capacity(points.len() * 3);
    let v = p + center * r;
    vertices.push(Vertex::new(v.x, v.y, 0., 0., 0., color(center)));
    for &point in points {
        let v = p + point * r;
        vertices.push(Vertex::new(v.x, v.y, 0., 0., 0., color(point)));
    }
    for i in 0..points.len() {
        indices.extend([0, (i + 1) as u16, ((i + 1) % points.len() + 1) as u16]);
    }
    draw_mesh(&Mesh {
        vertices,
        indices,
        texture: None,
    });
}
fn gradient_ellipse(p: Vec2, r: Vec2, color: impl Fn(Vec2) -> Color) {
    const SIDES: usize = 72;
    const RINGS: usize = 6;
    let mut vertices = Vec::with_capacity(1 + SIDES * RINGS);
    let mut indices = Vec::with_capacity(SIDES * RINGS * 6);
    vertices.push(Vertex::new(p.x, p.y, 0., 0., 0., color(Vec2::ZERO)));
    for ring in 1..=RINGS {
        for i in 0..SIDES {
            let a = i as f32 * std::f32::consts::TAU / SIDES as f32;
            let q = vec2(a.cos(), a.sin()) * ring as f32 / RINGS as f32;
            let v = p + q * r;
            vertices.push(Vertex::new(v.x, v.y, 0., 0., 0., color(q)));
        }
    }
    for i in 0..SIDES {
        indices.extend([0, (i + 1) as u16, ((i + 1) % SIDES + 1) as u16]);
    }
    for ring in 1..RINGS {
        for i in 0..SIDES {
            let a = 1 + (ring - 1) * SIDES + i;
            let b = 1 + (ring - 1) * SIDES + (i + 1) % SIDES;
            let c = a + SIDES;
            let d = b + SIDES;
            indices.extend([a as u16, b as u16, c as u16, b as u16, d as u16, c as u16]);
        }
    }
    draw_mesh(&Mesh {
        vertices,
        indices,
        texture: None,
    });
}
fn stroke(p: Vec2, r: f32, curve: [Vec2; 4], width: f32, color: Color) {
    let [a, b, c, d] = curve;
    let mut prev = p + a * r;
    for i in 1..=24 {
        let point = p + cubic(a, b, c, d, i as f32 / 24.) * r;
        draw_line(prev.x, prev.y, point.x, point.y, width * r, color);
        prev = point;
    }
}
fn fin(p: Vec2, r: f32, side: f32, flap: f32, body: Color) {
    let points = contour(
        vec2(0.76, 0.25),
        &[
            (
                vec2(1.10, 0.14),
                vec2(1.45, 0.31 + flap),
                vec2(1.33, 0.54 + flap),
            ),
            (vec2(1.20, 0.79 + flap), vec2(1.02, 0.96), vec2(0.84, 0.78)),
            (vec2(0.73, 0.66), vec2(0.68, 0.40), vec2(0.76, 0.25)),
        ],
    );
    let points: Vec<_> = points.into_iter().map(|v| vec2(v.x * side, v.y)).collect();
    let dark = mix(body, color_u8!(209, 71, 26, 255), 0.32);
    path(p, r, &points, vec2(side * 1.02, 0.48), |v| {
        mix(dark, body, ((v.y - 0.18) * 1.5).clamp(0., 1.))
    });
    for i in 0..5 {
        let t = i as f32 / 4.;
        let end = vec2(1.29 - t * 0.35, 0.40 + t * 0.40 + flap * (1. - t));
        stroke(
            p,
            r,
            [
                vec2(side * 0.80, 0.30),
                vec2(side * 0.99, 0.34 + t * 0.13),
                vec2(side * end.x, end.y - 0.05),
                vec2(side * end.x, end.y),
            ],
            0.035,
            dark,
        );
    }
}
pub fn fish(p: Vec2, r: f32, pet: &Fih, pose: Pose, art: &crate::fih_art::Art) {
    let phase = pose.phase;
    let r = r * (1. + phase.sin() * 0.009);
    let p = p + vec2(
        (phase * 6.).sin() * pose.refuse * r * 0.07,
        phase.sin() * r * 0.026 - pose.delight * r * 0.075,
    );
    let body = mix(
        COLORS[usize::from(pet.color)],
        color_u8!(187, 203, 178, 255),
        pose.ill * 0.30,
    );
    let dark = mix(
        body,
        color_u8!(202, 80, 34, 255),
        if pet.color == 0 { 0.36 } else { 0.14 },
    );
    let light = mix(body, color_u8!(255, 223, 165, 255), 0.24);
    let flap = phase.sin() * 0.055;
    // Tail and dorsal fin sit behind the body; side fins have independent pivots.
    gradient_ellipse(p + vec2(0., r * 0.89), vec2(r * 0.16, r * 0.22), |q| {
        mix(dark, body, (q.y + 1.) * 0.4)
    });
    for side in [-1., 1.] {
        let pts = contour(
            vec2(0., 0.94),
            &[
                (
                    vec2(0.48 * side, 1.13),
                    vec2(0.65 * side, 1.52),
                    vec2(0.46 * side, 1.59),
                ),
                (
                    vec2(0.34 * side, 1.46),
                    vec2(0.05 * side, 1.37),
                    vec2(0., 1.11),
                ),
                (
                    vec2(-0.02 * side, 1.01),
                    vec2(-0.02 * side, 0.96),
                    vec2(0., 0.94),
                ),
            ],
        );
        path(
            p + vec2(phase.sin() * r * 0.035, 0.),
            r,
            &pts,
            vec2(0.24 * side, 1.27),
            |v| mix(dark, body, (v.y - 1.).clamp(0., 0.5) * 2.),
        );
        for i in 0..3 {
            let x = (0.18 + i as f32 * 0.12) * side;
            stroke(
                p,
                r,
                [
                    vec2(0., 1.02),
                    vec2(x * 0.3, 1.19),
                    vec2(x, 1.23 + i as f32 * 0.045),
                    vec2(x, 1.28 + i as f32 * 0.045),
                ],
                0.026,
                dark,
            );
        }
    }
    let dorsal = contour(
        vec2(-0.31, -0.77),
        &[
            (vec2(-0.28, -1.15), vec2(-0.08, -1.47), vec2(0.05, -1.48)),
            (vec2(0.21, -1.47), vec2(0.29, -1.04), vec2(0.30, -0.77)),
            (vec2(0.15, -0.70), vec2(-0.15, -0.70), vec2(-0.31, -0.77)),
        ],
    );
    path(p, r, &dorsal, vec2(0., -1.05), |v| {
        mix(dark, body, ((v.x + 0.3) * 1.7).clamp(0., 1.))
    });
    for x in [-0.16, -0.02, 0.12] {
        stroke(
            p,
            r,
            [
                vec2(x, -0.78),
                vec2(x + 0.06, -1.),
                vec2(x + 0.04, -1.26),
                vec2(x, -1.28),
            ],
            0.026,
            dark,
        );
    }
    fin(p, r, -1., flap, body);
    fin(p, r, 1., -flap, body);
    gradient_ellipse(p, vec2(r * 0.94, r * 0.90), |q| {
        let base = mix(
            mix(body, dark, ((0.2 - q.y) * 0.32).clamp(0., 0.4)),
            light,
            ((q.y + 0.3) * 0.45).clamp(0., 0.48),
        );
        let rim = (q.length() - 0.6).max(0.) * 0.24;
        mix(base, dark, rim)
    });
    // A softly curved cream belly, with a small peak below the mouth.
    let belly = contour(
        vec2(-0.86, 0.35),
        &[
            (vec2(-0.57, 0.70), vec2(-0.18, 0.21), vec2(0., 0.20)),
            (vec2(0.18, 0.21), vec2(0.57, 0.70), vec2(0.86, 0.35)),
            (vec2(0.76, 0.72), vec2(0.35, 0.89), vec2(0., 0.90)),
            (vec2(-0.35, 0.89), vec2(-0.76, 0.72), vec2(-0.86, 0.35)),
        ],
    );
    path(p, r, &belly, vec2(0., 0.69), |v| {
        mix(
            color_u8!(255, 246, 220, 255),
            color_u8!(255, 210, 156, 255),
            ((v.y - 0.35) * 0.65 + v.x.abs() * 0.1).clamp(0., 0.5),
        )
    });
    art.clothes(p, r, pet.clothes);
    // Small translucent highlights are geometry too, rather than baked pixels.
    gradient_ellipse(
        p + vec2(-0.41, -0.64) * r,
        vec2(r * 0.13, r * 0.055),
        |_| Color::new(1., 0.93, 0.68, 0.35),
    );
    gradient_ellipse(p + vec2(0.35, -0.66) * r, vec2(r * 0.14, r * 0.075), |_| {
        Color::new(1., 0.93, 0.68, 0.45)
    });
    gradient_ellipse(
        p + vec2(0.54, -0.54) * r,
        vec2(r * 0.060, r * 0.045),
        |_| Color::new(1., 0.93, 0.68, 0.3),
    );
    let blink = if pet.sleeping { 1. } else { pose.blink };
    let eye_height =
        ((1. - blink.clamp(0., 1.)) * (1. - pose.tired * 0.35 - pose.ill * 0.20)).max(0.05);
    for side in [-1., 1.] {
        let e = p + vec2(side * 0.49, -0.12) * r;
        if pet.sleeping || eye_height < 0.20 {
            stroke(
                e,
                r,
                [
                    vec2(-0.22, 0.),
                    vec2(-0.08, 0.12),
                    vec2(0.08, 0.12),
                    vec2(0.22, 0.),
                ],
                0.028,
                INK,
            );
            for dx in [-0.15, 0.15] {
                draw_line(
                    e.x + dx * r,
                    e.y + r * 0.04,
                    e.x + dx * r * 1.2,
                    e.y + r * 0.10,
                    r * 0.018,
                    INK,
                );
            }
            continue;
        }
        gradient_ellipse(
            e + vec2(0., -r * 0.02),
            vec2(r * 0.255, r * 0.335 * eye_height),
            |q| mix(dark, color_u8!(165, 65, 35, 255), (1. - q.y) * 0.10),
        );
        gradient_ellipse(e, vec2(r * 0.248, r * 0.307 * eye_height), |q| {
            mix(
                WHITE,
                color_u8!(233, 222, 216, 255),
                (q.y * 0.2 + q.length() * 0.1).clamp(0., 0.27),
            )
        });
        let iris = e + vec2(
            (pose.look.0 - side * 0.25) * r * 0.045,
            (0.01 + pose.look.1 * 0.04) * r * eye_height,
        );
        gradient_ellipse(iris, vec2(r * 0.185, r * 0.25 * eye_height), |q| {
            mix(
                color_u8!(47, 26, 22, 255),
                color_u8!(154, 84, 35, 255),
                ((q.y + 0.4) * 0.7).clamp(0., 0.9),
            )
        });
        gradient_ellipse(
            iris + vec2(0., -r * 0.012),
            vec2(r * 0.143, r * 0.21 * eye_height),
            |q| {
                mix(
                    color_u8!(26, 19, 22, 255),
                    color_u8!(49, 28, 24, 255),
                    q.y.max(0.) * 0.4,
                )
            },
        );
        gradient_ellipse(
            iris + vec2(-side * r * 0.061, -r * 0.115),
            vec2(r * 0.055, r * 0.056 * eye_height),
            |_| WHITE,
        );
        gradient_ellipse(
            iris + vec2(side * r * 0.055, r * 0.081),
            vec2(r * 0.021, r * 0.022 * eye_height),
            |_| color_u8!(255, 220, 177, 255),
        );
    }
    for side in [-1., 1.] {
        gradient_ellipse(
            p + vec2(side * 0.64, 0.27) * r,
            vec2(r * 0.12, r * 0.07),
            |q| Color::new(1., 0.40, 0.32, (1. - q.length()).max(0.) * 0.5),
        );
    }
    if pose.refuse > 0.1 {
        stroke(
            p,
            r,
            [
                vec2(-0.17, 0.23),
                vec2(-0.05, 0.23),
                vec2(0.05, 0.23),
                vec2(0.17, 0.23),
            ],
            0.03,
            INK,
        );
    } else if pose.hungry > 0.1 && pose.mouth < 0.1 && pose.delight < 0.1 {
        gradient_ellipse(
            p + vec2(0., 0.26) * r,
            vec2(r * 0.085, r * (0.055 + pose.hungry * 0.035)),
            |_| INK,
        );
    } else if pet.sleeping {
        stroke(
            p,
            r,
            [
                vec2(-0.14, 0.21),
                vec2(-0.05, 0.27),
                vec2(0.05, 0.27),
                vec2(0.14, 0.21),
            ],
            0.024,
            INK,
        );
    } else if (pose.sad > 0.2 || pose.ill > 0.2)
        && pose.mouth < 0.1
        && pose.chew.abs() < 0.05
        && pose.delight < 0.1
    {
        stroke(
            p,
            r,
            [
                vec2(-0.14, 0.26),
                vec2(-0.05, 0.18),
                vec2(0.05, 0.18),
                vec2(0.14, 0.26),
            ],
            0.024,
            INK,
        );
    } else {
        let mouth = contour(
            vec2(-0.20, 0.18),
            &[
                (vec2(-0.08, 0.20), vec2(0.08, 0.20), vec2(0.20, 0.18)),
                (vec2(0.23, 0.18), vec2(0.18, 0.36), vec2(0., 0.38)),
                (vec2(-0.18, 0.36), vec2(-0.23, 0.18), vec2(-0.20, 0.18)),
            ],
        );
        let stretch = 0.65 + pose.mouth * 0.9 + pose.chew.abs() * 0.30;
        let mouth: Vec<_> = mouth
            .into_iter()
            .map(|v| vec2(v.x, 0.18 + (v.y - 0.18) * stretch))
            .collect();
        path(p, r, &mouth, vec2(0., 0.18 + 0.07 * stretch), |v| {
            mix(
                color_u8!(103, 35, 30, 255),
                color_u8!(153, 45, 35, 255),
                (v.y - 0.18) * 3.,
            )
        });
        let tongue = contour(
            vec2(-0.13, 0.31),
            &[
                (vec2(-0.07, 0.26), vec2(0.07, 0.26), vec2(0.13, 0.31)),
                (vec2(0.06, 0.40), vec2(-0.06, 0.40), vec2(-0.13, 0.31)),
            ],
        );
        let tongue: Vec<_> = tongue
            .into_iter()
            .map(|v| vec2(v.x, 0.18 + (v.y - 0.18) * stretch))
            .collect();
        path(p, r, &tongue, vec2(0., 0.18 + 0.15 * stretch), |_| {
            mix(PINK, color_u8!(247, 101, 81, 255), 0.5)
        });
    }
    art.hat(p, r, pet.hat);
    if pet.clean < 65. {
        let dirt = ((65. - pet.clean) / 65.).clamp(0., 1.);
        for (i, v) in [
            vec2(-0.33, 0.43),
            vec2(0.37, 0.46),
            vec2(0.16, -0.62),
            vec2(-0.61, -0.45),
            vec2(0.64, 0.1),
            vec2(-0.16, 0.73),
            vec2(0.43, -0.51),
        ]
        .into_iter()
        .enumerate()
        {
            let size = 0.04 + dirt * 0.075;
            gradient_ellipse(p + v * r, vec2(r * size, r * size * 0.65), |_| {
                Color::new(0.39, 0.29, 0.18, 0.2 + dirt * 0.65)
            });
            if dirt > 0.6 && i % 2 == 0 {
                stroke(
                    p,
                    r,
                    [
                        v + vec2(-0.05, -0.025),
                        v,
                        v + vec2(0.04, 0.06),
                        v + vec2(0.07, 0.06),
                    ],
                    0.017,
                    Color::new(0.39, 0.29, 0.18, dirt * 0.6),
                );
            }
        }
    }
}
