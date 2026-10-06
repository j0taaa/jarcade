//! Bounded, seeded generation. Every accepted grid is solved by line deductions.
use super::{Size, clues_for, solve_clues};
use rand_chacha::{
    ChaCha12Rng,
    rand_core::{RngCore, SeedableRng},
};

pub(super) struct Generated {
    pub size: Size,
    pub seed: u64,
    pub number: u64,
    pub name: String,
    pub solution: Vec<bool>,
    pub recent: Vec<u64>,
}
pub(super) fn id(size: Size) -> &'static str {
    match size {
        Size::Small => "endless-5",
        Size::Medium => "endless-10",
        Size::Large => "endless-15",
    }
}
fn fingerprint(solution: &[bool]) -> u64 {
    solution.iter().fold(0xcbf29ce484222325, |h, filled| {
        (h ^ u64::from(*filled)).wrapping_mul(0x100000001b3)
    })
}
pub(super) fn valid_solution(side: usize, solution: &[bool]) -> bool {
    if Size::from_side(side).is_none() || solution.len() != side * side {
        return false;
    }
    let count = solution.iter().filter(|c| **c).count();
    if count < side || count > side * side - side {
        return false;
    }
    let (rows, columns) = clues_for(side, |i| solution[i]);
    solve_clues(side, &rows, &columns)
}
impl Generated {
    pub fn restore(
        size: Size,
        seed: u64,
        number: u64,
        solution: Vec<bool>,
        mut recent: Vec<u64>,
    ) -> Self {
        recent.truncate(32);
        let hash = fingerprint(&solution);
        if !recent.contains(&hash) {
            recent.push(hash);
        }
        if recent.len() > 32 {
            recent.remove(0);
        }
        Self {
            size,
            seed,
            number,
            name: format!("Puzzle {number}"),
            solution,
            recent,
        }
    }
    pub fn new(size: Size, seed: u64, number: u64, recent: Vec<u64>) -> Self {
        let mut rng = ChaCha12Rng::seed_from_u64(seed);
        let side = size.side();
        // A hard attempt limit keeps clicking Play cheap on mobile. Most random
        // mosaics pass in the first few attempts; no work occurs during idle play.
        for attempt in 0..32 {
            let density = 55 + rng.next_u32() % 21;
            let mut solution = vec![false; side * side];
            if attempt % 3 == 0 {
                // Mirrored mosaics form more coherent abstract pixel patterns.
                for y in 0..side {
                    for x in 0..side.div_ceil(2) {
                        let filled = rng.next_u32() % 100 < density;
                        solution[y * side + x] = filled;
                        solution[y * side + side - x - 1] = filled;
                    }
                }
            } else {
                for cell in &mut solution {
                    *cell = rng.next_u32() % 100 < density;
                }
            }
            if !recent.contains(&fingerprint(&solution)) && valid_solution(side, &solution) {
                return Self::restore(size, seed, number, solution, recent);
            }
        }
        // Guaranteed logical fallback: full first/last columns anchor two runs
        // in every row, separated by a gap. Random run lengths still vary the
        // puzzle; the full columns force those runs to touch the corresponding
        // edge, proving both uniqueness and solvability without searching.
        let mut solution = vec![false; side * side];
        for y in 0..side {
            let left = 1 + rng.next_u32() as usize % (side - 2);
            let right = 1 + rng.next_u32() as usize % (side - left - 1);
            for x in 0..side {
                solution[y * side + x] = x < left || x >= side - right;
            }
        }
        // There are more fallback patterns than recent slots. If a collision
        // occurs, advance row lengths systematically until a new board is found.
        while recent.contains(&fingerprint(&solution)) {
            for row in solution.chunks_mut(side) {
                let left = row.iter().take_while(|c| **c).count();
                row.fill(false);
                let next = if left < side - 2 { left + 1 } else { 1 };
                row[..next].fill(true);
                row[side - 1] = true;
                if next != 1 {
                    break;
                }
            }
        }
        Self::restore(size, seed, number, solution, recent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hundreds_of_seeds_are_unique_logical_boards_at_every_size() {
        for size in Size::ALL {
            let mut hashes = std::collections::HashSet::new();
            for seed in 0..192 {
                let p = Generated::new(size, seed, 1, vec![]);
                assert!(
                    valid_solution(size.side(), &p.solution),
                    "{} seed {seed}",
                    size.side()
                );
                hashes.insert(fingerprint(&p.solution));
            }
            assert!(hashes.len() > 180, "generator diversity at {}", size.side());
        }
    }
    #[test]
    fn seeded_generation_is_repeatable_and_avoids_recent_boards() {
        let a = Generated::new(Size::Small, 77, 1, vec![]);
        let b = Generated::new(Size::Small, 77, 1, vec![]);
        assert_eq!(a.solution, b.solution);
        let mut recent = vec![];
        for n in 1..65 {
            let p = Generated::new(Size::Small, 77, n, recent.clone());
            assert!(!recent.contains(&fingerprint(&p.solution)));
            assert!(valid_solution(5, &p.solution));
            assert!(p.recent.len() <= 32);
            recent = p.recent;
        }
    }
}
