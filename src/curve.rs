use macroquad::prelude::{Vec2, vec2};

#[derive(Clone, Copy, Debug)]
pub struct CurvePoint {
    pub position: Vec2,
    pub tangent: Vec2,
}

/// Circular fillets keep both edges of a thick body curved. Tangents are
/// analytic, so the stroke keeps a constant width through every bend.
pub fn rounded_path(body: &[Vec2], unit: f32) -> Vec<CurvePoint> {
    let mut points = Vec::with_capacity(body.len());
    for &point in body {
        if points
            .last()
            .is_none_or(|p: &Vec2| p.distance_squared(point) > 0.000001)
        {
            points.push(point);
        }
    }
    let mut path = Vec::with_capacity(points.len() * 3);
    if points.len() < 2 {
        return path;
    }
    path.push(CurvePoint {
        position: points[0],
        tangent: (points[1] - points[0]).normalize(),
    });
    for triple in points.windows(3) {
        let incoming = triple[1] - triple[0];
        let outgoing = triple[2] - triple[1];
        let a = incoming.normalize();
        let b = outgoing.normalize();
        let angle = a.perp_dot(b).atan2(a.dot(b));
        if angle.abs() < 0.001 || angle.abs() > 3.13 {
            path.push(CurvePoint {
                position: triple[1],
                tangent: b,
            });
            continue;
        }
        let trim = (unit * 0.5)
            .min(incoming.length() * 0.5)
            .min(outgoing.length() * 0.5);
        let radius = trim / (angle.abs() * 0.5).tan();
        let entry = triple[1] - a * trim;
        let center = entry + vec2(-a.y, a.x) * radius * angle.signum();
        let offset = entry - center;
        // At most 7.5 degrees per segment, independent of screen DPI.
        let segments = (angle.abs() / (std::f32::consts::PI / 24.0)).ceil() as usize;
        for step in 0..=segments {
            let theta = angle * step as f32 / segments as f32;
            let (sin, cos) = theta.sin_cos();
            let rotate = |v: Vec2| vec2(v.x * cos - v.y * sin, v.x * sin + v.y * cos);
            path.push(CurvePoint {
                position: center + rotate(offset),
                tangent: rotate(a),
            });
        }
    }
    let last = points.len() - 1;
    path.push(CurvePoint {
        position: points[last],
        tangent: (points[last] - points[last - 1]).normalize(),
    });
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn right_angle_has_round_inner_and_outer_edges_and_tangent_joins() {
        let path = rounded_path(&[vec2(0., 0.), vec2(1., 0.), vec2(1., 1.)], 1.);
        let center = vec2(0.5, 0.5);
        for sample in &path[1..path.len() - 1] {
            assert!((sample.position.distance(center) - 0.5).abs() < 0.00001);
            let normal = vec2(-sample.tangent.y, sample.tangent.x);
            for (offset, radius) in [(0.385, 0.115), (-0.385, 0.885)] {
                assert!(
                    ((sample.position + normal * offset).distance(center) - radius).abs() < 0.00001
                );
            }
        }
        assert!(path[1].tangent.distance(vec2(1., 0.)) < 0.00001);
        assert!(path[path.len() - 2].tangent.distance(vec2(0., 1.)) < 0.00001);
    }

    #[test]
    fn interpolated_turns_remain_finite_bounded_and_continuous() {
        let before = [vec2(2., 0.), vec2(1., 0.), vec2(0., 0.), vec2(0., 1.)];
        let after = [vec2(2., 1.), vec2(2., 0.), vec2(1., 0.), vec2(0., 0.)];
        let mut last_heading = vec2(1., 0.);
        for frame in 0..=240 {
            let t = frame as f32 / 240.;
            let body: Vec<_> = before
                .iter()
                .zip(after)
                .map(|(&a, b)| a.lerp(b, t))
                .collect();
            let path = rounded_path(&body, 1.);
            assert_eq!(path[0].position, body[0]);
            assert_eq!(path.last().unwrap().position, body[3]);
            for p in &path {
                assert!(p.position.is_finite() && p.tangent.is_finite());
                assert!((p.tangent.length() - 1.).abs() < 0.00001);
                assert!(p.position.x >= -0.00001 && p.position.x <= 2.00001);
                assert!(p.position.y >= -0.00001 && p.position.y <= 1.00001);
            }
            let heading = -path[0].tangent;
            assert!(heading.distance(last_heading) < 0.01);
            last_heading = heading;
        }
    }

    #[test]
    fn straight_lines_and_duplicate_growth_points_do_not_create_nan() {
        assert!(rounded_path(&[], 1.).is_empty());
        assert!(rounded_path(&[Vec2::ZERO], 1.).is_empty());
        let path = rounded_path(&[Vec2::ZERO, Vec2::X, Vec2::X, Vec2::X * 2.], 1.);
        assert_eq!(path.len(), 3);
        assert!(
            path.iter()
                .all(|p| p.tangent == Vec2::X && p.position.y == 0.)
        );
    }
}
