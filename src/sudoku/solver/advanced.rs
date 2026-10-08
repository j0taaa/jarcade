use super::{Logic, bit};

impl Logic<'_> {
    pub(super) fn patterns(&mut self, units: &[Vec<usize>]) {
        // Naked and hidden triples/quads. Apply one pattern, then return to
        // simpler deductions; scores count necessary pattern uses, not blanks.
        for size in 2..=4 {
            for unit in units {
                for selection in combinations(unit.len(), size) {
                    let cells: Vec<_> = selection.iter().map(|&j| unit[j]).collect();
                    if cells.iter().all(|&i| self.masks[i].count_ones() > 1) {
                        let mask = cells.iter().fold(0, |mask, &i| mask | self.masks[i]);
                        if mask.count_ones() as usize == size {
                            let before = self.masks.clone();
                            for &i in unit {
                                if !cells.contains(&i) {
                                    self.restrict(i, !mask, "Naked subset", 16);
                                }
                            }
                            if before != self.masks {
                                self.advanced_steps += 1;
                                self.context = Some(format!(
                                    "A naked subset of {size} cells removes competing digits."
                                ));
                                return;
                            }
                        }
                    }
                }
                for digits in combinations(9, size) {
                    let mask = digits.iter().fold(0, |mask, &d| mask | bit(d as u8 + 1));
                    let cells: Vec<_> = unit
                        .iter()
                        .copied()
                        .filter(|&i| self.masks[i] & mask != 0)
                        .collect();
                    if cells.len() == size
                        && digits
                            .iter()
                            .all(|&d| cells.iter().any(|&i| self.masks[i] & bit(d as u8 + 1) != 0))
                    {
                        let before = self.masks.clone();
                        for i in cells {
                            self.restrict(i, mask, "Hidden subset", 16);
                        }
                        if before != self.masks {
                            self.advanced_steps += 1;
                            self.context = Some(format!(
                                "{size} digits are confined to {size} cells in one unit."
                            ));
                            return;
                        }
                    }
                }
            }
        }
        // Basic fish: N rows confined to N columns, and the transposed case.
        for size in 2..=3 {
            for transpose in [false, true] {
                for d in 1..=9 {
                    let rows: Vec<u16> = (0..9)
                        .map(|row| {
                            (0..9)
                                .filter(|&col| {
                                    let i = if transpose {
                                        col * 9 + row
                                    } else {
                                        row * 9 + col
                                    };
                                    self.masks[i].count_ones() > 1 && self.masks[i] & bit(d) != 0
                                })
                                .fold(0, |mask, col| mask | (1 << col))
                        })
                        .collect();
                    for bases in combinations(9, size) {
                        if bases.iter().any(|&row| {
                            rows[row].count_ones() < 2 || rows[row].count_ones() as usize > size
                        }) {
                            continue;
                        }
                        let covers = bases.iter().fold(0u16, |mask, &row| mask | rows[row]);
                        if covers.count_ones() as usize != size {
                            continue;
                        }
                        let before = self.masks.clone();
                        for row in 0..9 {
                            if bases.contains(&row) {
                                continue;
                            }
                            for col in 0..9 {
                                if covers & (1 << col) != 0 {
                                    let i = if transpose {
                                        col * 9 + row
                                    } else {
                                        row * 9 + col
                                    };
                                    self.restrict(
                                        i,
                                        !bit(d),
                                        if size == 2 { "X-Wing" } else { "Swordfish" },
                                        24,
                                    );
                                }
                            }
                        }
                        if before != self.masks {
                            self.advanced_steps += 1;
                            self.context = Some(format!(
                                "{} eliminates {d} outside its {} base units.",
                                if size == 2 {
                                    "An X-Wing"
                                } else {
                                    "A Swordfish"
                                },
                                size
                            ));
                            return;
                        }
                    }
                }
            }
        }
    }
}

fn combinations(n: usize, size: usize) -> &'static [Vec<usize>] {
    use std::sync::OnceLock;
    static COMBINATIONS: OnceLock<[Vec<Vec<usize>>; 3]> = OnceLock::new();
    fn visit(size: usize, start: usize, selected: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
        if selected.len() == size {
            out.push(selected.clone());
            return;
        }
        for i in start..9 - (size - selected.len() - 1) {
            selected.push(i);
            visit(size, i + 1, selected, out);
            selected.pop();
        }
    }
    debug_assert_eq!(n, 9);
    &COMBINATIONS.get_or_init(|| {
        std::array::from_fn(|i| {
            let mut out = Vec::new();
            visit(i + 2, 0, &mut Vec::new(), &mut out);
            out
        })
    })[size - 2]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sudoku::{ALL_DIGITS, Difficulty, Generator, Variant};

    #[test]
    fn subset_combinations_are_complete_unique_and_ordered() {
        for (size, count) in [(2, 36), (3, 84), (4, 126)] {
            let items = combinations(9, size);
            assert_eq!(items.len(), count);
            for (i, item) in items.iter().enumerate() {
                assert_eq!(item.len(), size);
                assert!(item.windows(2).all(|p| p[0] < p[1]));
                assert!(item.iter().all(|&j| j < 9));
                assert!(!items[..i].contains(item));
            }
        }
    }
    fn logic(puzzle: &crate::sudoku::Puzzle, masks: Vec<u16>) -> Logic<'_> {
        Logic {
            puzzle,
            masks,
            steps: Vec::new(),
            effort: 0,
            valid: true,
            advanced_steps: 0,
            context: None,
        }
    }
    #[test]
    fn naked_and_hidden_triples_keep_other_units_and_remaining_candidates() {
        let p = Generator::new(3, Variant::Classic, Difficulty::Easy).finish();
        let mut masks = vec![ALL_DIGITS; 81];
        masks[0] = bit(1) | bit(2);
        masks[1] = bit(2) | bit(3);
        masks[2] = bit(1) | bit(3);
        let mut l = logic(&p, masks);
        l.patterns(&p.units());
        for i in 3..9 {
            assert_eq!(l.masks[i], ALL_DIGITS & !(bit(1) | bit(2) | bit(3)));
        }
        assert_eq!(l.masks[40], ALL_DIGITS);
        assert_eq!(l.advanced_steps, 1);
        let mut masks = vec![ALL_DIGITS; 81];
        for mask in &mut masks[3..9] {
            *mask &= !(bit(1) | bit(2) | bit(3));
        }
        let mut l = logic(&p, masks);
        l.patterns(&p.units());
        assert_eq!(&l.masks[..3], &[bit(1) | bit(2) | bit(3); 3]);
        assert_eq!(l.masks[40], ALL_DIGITS);
        assert_eq!(l.advanced_steps, 1);
    }
    #[test]
    fn x_wing_and_swordfish_work_in_both_orientations() {
        let p = Generator::new(3, Variant::Classic, Difficulty::Easy).finish();
        for size in 2..=3 {
            for transpose in [false, true] {
                let bases = &([1, 4, 7])[..size];
                let covers = &([2, 5, 8])[..size];
                let index = |row: usize, col: usize| {
                    if transpose {
                        col * 9 + row
                    } else {
                        row * 9 + col
                    }
                };
                let mut masks = vec![ALL_DIGITS; 81];
                for &row in bases {
                    for col in 0..9 {
                        if !covers.contains(&col) {
                            masks[index(row, col)] &= !bit(6);
                        }
                    }
                }
                let mut l = logic(&p, masks);
                l.patterns(&p.units());
                for row in 0..9 {
                    for col in 0..9 {
                        let expected = bases.contains(&row) && covers.contains(&col)
                            || !bases.contains(&row) && !covers.contains(&col);
                        assert_eq!(
                            l.masks[index(row, col)] & bit(6) != 0,
                            expected,
                            "size {size}, transpose {transpose}, {row}/{col}"
                        );
                        assert_eq!(l.masks[index(row, col)] & !bit(6), ALL_DIGITS & !bit(6));
                    }
                }
                assert_eq!(l.advanced_steps, 1);
                assert!(l.valid);
            }
        }
    }
}
