use crate::snake::{Cell, Direction};
use macroquad::prelude::{Vec2, vec2};
use std::collections::VecDeque;

/// Ease the face around a corner while the head stays aligned to the rules grid.
pub fn head_heading(previous: &[Cell], direction: Direction, fraction: f32) -> Vec2 {
    let (dx, dy) = direction.delta();
    let forward = vec2(f32::from(dx), f32::from(dy));
    if previous.len() < 2 {
        return forward;
    }
    let old = vec2(
        f32::from(previous[0].x - previous[1].x),
        f32::from(previous[0].y - previous[1].y),
    )
    .normalize_or_zero();
    if old == Vec2::ZERO {
        return forward;
    }
    let t = (fraction * 2.).clamp(0., 1.);
    let angle = old.perp_dot(forward).atan2(old.dot(forward)) * t * t * (3. - 2. * t);
    let (sin, cos) = angle.sin_cos();
    vec2(old.x * cos - old.y * sin, old.x * sin + old.y * cos)
}

/// Slide the endpoints along the actual trail. Interior corners stay fixed;
/// interpolating every body cell instead makes bends drift diagonally and shrink.
pub fn body_path(previous: &[Cell], current: &VecDeque<Cell>, fraction: f32, out: &mut Vec<Vec2>) {
    let point = |cell: Cell| vec2(f32::from(cell.x), f32::from(cell.y));
    out.clear();
    if previous.is_empty() || current.is_empty() {
        out.extend(current.iter().copied().map(point));
        return;
    }
    let t = fraction.clamp(0., 1.);
    out.push(point(previous[0]).lerp(point(current[0]), t));
    out.extend(current.iter().skip(1).copied().map(point));
    if previous.len() == current.len() {
        out.push(point(*previous.last().unwrap()).lerp(point(*current.back().unwrap()), t));
    }
    out.dedup_by(|a, b| a.distance_squared(*b) < 0.000001);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn head_rotation_is_continuous_through_a_turn() {
        let before = [Cell { x: 2, y: 0 }, Cell { x: 1, y: 0 }];
        let mut last = Vec2::X;
        for frame in 0..=240 {
            let heading = head_heading(&before, Direction::Down, frame as f32 / 240.);
            assert!((heading.length() - 1.).abs() < 0.00001);
            assert!(heading.distance(last) < 0.02);
            last = heading;
        }
        assert!(last.distance(Vec2::Y) < 0.00001);
    }

    #[test]
    fn bends_stay_on_the_trail_instead_of_cutting_across_cells() {
        let before = [
            Cell { x: 2, y: 0 },
            Cell { x: 1, y: 0 },
            Cell { x: 0, y: 0 },
            Cell { x: 0, y: 1 },
        ];
        let after = VecDeque::from([Cell { x: 2, y: 1 }, before[0], before[1], before[2]]);
        let mut path = Vec::new();
        for frame in 0..=240 {
            let t = frame as f32 / 240.;
            body_path(&before, &after, t, &mut path);
            assert_eq!(path[0], vec2(2., t));
            assert_eq!(*path.last().unwrap(), vec2(0., 1. - t));
            assert!(path.windows(2).all(|p| (p[1].x-p[0].x).abs() < 0.00001 || (p[1].y-p[0].y).abs() < 0.00001));
        }
    }

    #[test]
    fn tick_boundaries_and_growth_are_continuous() {
        let before = [
            Cell { x: 2, y: 0 },
            Cell { x: 1, y: 0 },
            Cell { x: 0, y: 0 },
        ];
        for growing in [false, true] {
            let mut after = VecDeque::from([Cell { x: 2, y: 1 }, before[0], before[1]]);
            if growing {
                after.push_back(before[2]);
            }
            let mut start = Vec::new();
            body_path(&before, &after, 0., &mut start);
            assert_eq!(start, before.map(|c| vec2(f32::from(c.x), f32::from(c.y))));
            let mut end = Vec::new();
            body_path(&before, &after, 1., &mut end);
            let prev: Vec<_> = after.iter().copied().collect();
            after.push_front(Cell { x: 3, y: 1 });
            after.pop_back();
            body_path(&prev, &after, 0., &mut start);
            assert_eq!(start, end);
        }
    }
}
