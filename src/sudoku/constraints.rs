use super::{ALL_DIGITS, Variant, bit};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineKind {
    Arrow,
    Renban,
    Whispers,
    RegionSum,
    Palindrome,
    Between,
    Entropic,
}
impl LineKind {
    pub const ALL: [Self; 7] = [
        Self::Arrow,
        Self::Renban,
        Self::Whispers,
        Self::RegionSum,
        Self::Palindrome,
        Self::Between,
        Self::Entropic,
    ];
    pub fn variant(self) -> Variant {
        match self {
            Self::Arrow => Variant::Arrow,
            Self::Renban => Variant::Renban,
            Self::Whispers => Variant::Whispers,
            Self::RegionSum => Variant::RegionSum,
            Self::Palindrome => Variant::Palindrome,
            Self::Between => Variant::Between,
            Self::Entropic => Variant::Entropic,
        }
    }
    pub fn reason(self) -> &'static str {
        match self {
            Self::Arrow => "The arrow sum equals its circle",
            Self::Renban => "Renban digits are distinct and form a consecutive set",
            Self::Whispers => "Whispers neighbours differ by at least five",
            Self::RegionSum => "Each box segment has the same sum",
            Self::Palindrome => "Mirrored palindrome cells contain equal digits",
            Self::Between => "Middle digits lie strictly between the endpoints",
            Self::Entropic => "Each triple has one low, one middle and one high digit",
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Line {
    pub kind: LineKind,
    pub cells: Vec<usize>,
}
impl Line {
    pub fn valid_shape(&self) -> bool {
        (3..=8).contains(&self.cells.len())
            && self
                .cells
                .iter()
                .enumerate()
                .all(|(j, &i)| i < 81 && !self.cells[..j].contains(&i))
            && self
                .cells
                .windows(2)
                .all(|p| (p[0] / 9).abs_diff(p[1] / 9) <= 1 && (p[0] % 9).abs_diff(p[1] % 9) <= 1)
    }
    pub fn segments(&self) -> Vec<&[usize]> {
        let mut segments = Vec::new();
        let mut start = 0;
        for j in 1..=self.cells.len() {
            if j == self.cells.len() || box_id(self.cells[j]) != box_id(self.cells[j - 1]) {
                segments.push(&self.cells[start..j]);
                start = j;
            }
        }
        segments
    }
    pub fn valid(&self, values: &[u8]) -> bool {
        let v: Vec<_> = self.cells.iter().map(|&i| values[i]).collect();
        match self.kind {
            LineKind::Arrow => v[1..].iter().map(|&d| u16::from(d)).sum::<u16>() == u16::from(v[0]),
            LineKind::Renban => {
                let m = v.iter().fold(0, |m, &d| m | bit(d));
                m.count_ones() == v.len() as u32
                    && v.iter().max().unwrap() - v.iter().min().unwrap() + 1 == v.len() as u8
            }
            LineKind::Whispers => v.windows(2).all(|p| p[0].abs_diff(p[1]) >= 5),
            LineKind::RegionSum => {
                let sums: Vec<_> = self
                    .segments()
                    .iter()
                    .map(|segment| segment.iter().map(|&i| u16::from(values[i])).sum::<u16>())
                    .collect();
                sums.len() >= 2 && sums.iter().all(|s| *s == sums[0])
            }
            LineKind::Palindrome => v.iter().eq(v.iter().rev()),
            LineKind::Between => v[1..v.len() - 1]
                .iter()
                .all(|&d| d > v[0].min(*v.last().unwrap()) && d < v[0].max(*v.last().unwrap())),
            LineKind::Entropic => v.windows(3).all(|p| {
                (p[0] - 1) / 3 != (p[1] - 1) / 3
                    && (p[0] - 1) / 3 != (p[2] - 1) / 3
                    && (p[1] - 1) / 3 != (p[2] - 1) / 3
            }),
        }
    }
}
pub(super) fn box_id(i: usize) -> usize {
    i / 27 * 3 + i % 9 / 3
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sandwich {
    pub unit: usize,
    pub sum: u8,
}
impl Sandwich {
    pub fn cells(&self) -> Vec<usize> {
        if self.unit < 9 {
            (0..9).map(|x| self.unit * 9 + x).collect()
        } else {
            (0..9).map(|y| y * 9 + self.unit - 9).collect()
        }
    }
    pub fn valid(&self, values: &[u8]) -> bool {
        let cells = self.cells();
        let a = cells.iter().position(|&i| values[i] == 1);
        let b = cells.iter().position(|&i| values[i] == 9);
        if let (Some(a), Some(b)) = (a, b) {
            cells[a.min(b) + 1..a.max(b)]
                .iter()
                .map(|&i| u16::from(values[i]))
                .sum::<u16>()
                == u16::from(self.sum)
        } else {
            false
        }
    }
}
fn bits(mut mask: u16) -> impl Iterator<Item = u16> {
    std::iter::from_fn(move || {
        if mask == 0 {
            None
        } else {
            let b = 1 << mask.trailing_zeros();
            mask ^= b;
            Some(b)
        }
    })
}
fn reachable(masks: &[u16]) -> Vec<[bool; 512]> {
    let mut p = vec![[false; 512]; masks.len() + 1];
    p[0][0] = true;
    for (i, &mask) in masks.iter().enumerate() {
        for used in 0..512 {
            if p[i][used] {
                for b in bits(mask & !(used as u16)) {
                    p[i + 1][used | b as usize] = true;
                }
            }
        }
    }
    p
}
fn matching(masks: &[u16], finals: &[bool; 512]) -> Vec<u16> {
    let p = reachable(masks);
    let mut next = *finals;
    let mut support = vec![0; masks.len()];
    for i in (0..masks.len()).rev() {
        let mut current = [false; 512];
        for used in 0..512 {
            if p[i][used] {
                for b in bits(masks[i] & !(used as u16)) {
                    if next[used | b as usize] {
                        support[i] |= b;
                        current[used] = true;
                    }
                }
            }
        }
        next = current;
    }
    if !next[0] {
        support.fill(0);
    }
    support
}
fn digit_sum(mask: u16) -> u8 {
    (1..=9).filter(|&d| mask & bit(d) != 0).sum()
}
/// Exact positional sum support. Repeats are allowed: normal units handle them.
fn sum_support(masks: &[u16], total: usize) -> Vec<u16> {
    let mut prefix = vec![vec![false; total + 1]; masks.len() + 1];
    prefix[0][0] = true;
    for (i, &mask) in masks.iter().enumerate() {
        for sum in 0..=total {
            if prefix[i][sum] {
                for d in 1..=9 {
                    if mask & bit(d) != 0 && sum + d as usize <= total {
                        prefix[i + 1][sum + d as usize] = true;
                    }
                }
            }
        }
    }
    let mut next = vec![false; total + 1];
    next[total] = true;
    let mut support = vec![0; masks.len()];
    for i in (0..masks.len()).rev() {
        let mut current = vec![false; total + 1];
        for sum in 0..=total {
            if prefix[i][sum] {
                for d in 1..=9 {
                    let end = sum + d as usize;
                    if masks[i] & bit(d) != 0 && end <= total && next[end] {
                        support[i] |= bit(d);
                        current[sum] = true;
                    }
                }
            }
        }
        next = current;
    }
    if !next[0] {
        support.fill(0);
    }
    support
}
pub(super) fn line_support(line: &Line, masks: &[u16]) -> Vec<u16> {
    let m: Vec<_> = line.cells.iter().map(|&i| masks[i]).collect();
    let n = m.len();
    let mut s = vec![0; n];
    match line.kind {
        LineKind::Arrow => {
            for d in 1..=9 {
                if m[0] & bit(d) == 0 {
                    continue;
                }
                let shaft = sum_support(&m[1..], d as usize);
                if !shaft.contains(&0) {
                    s[0] |= bit(d);
                    for (s, &v) in s[1..].iter_mut().zip(&shaft) {
                        *s |= v;
                    }
                }
            }
        }
        LineKind::Renban => {
            let mut finals = [false; 512];
            for low in 1..=10 - n {
                let mask = (0..n).fold(0, |m, j| m | bit((low + j) as u8));
                finals[mask as usize] = true;
            }
            s = matching(&m, &finals);
        }
        LineKind::Whispers => {
            s = m.clone();
            for i in 0..n - 1 {
                let mut a = 0;
                let mut b = 0;
                for x in 1..=9 {
                    for y in 1..=9 {
                        if m[i] & bit(x) != 0 && m[i + 1] & bit(y) != 0 && x.abs_diff(y) >= 5 {
                            a |= bit(x);
                            b |= bit(y);
                        }
                    }
                }
                s[i] &= a;
                s[i + 1] &= b;
            }
        }
        LineKind::Palindrome => {
            for i in 0..n {
                s[i] = m[i] & m[n - 1 - i];
            }
        }
        LineKind::Between => {
            for x in 1..=9 {
                for y in 1..=9 {
                    if m[0] & bit(x) == 0 || m[n - 1] & bit(y) == 0 {
                        continue;
                    }
                    let range = (x.min(y) + 1..x.max(y)).fold(0, |m, d| m | bit(d));
                    if m[1..n - 1].iter().all(|&m| m & range != 0) {
                        s[0] |= bit(x);
                        s[n - 1] |= bit(y);
                        for i in 1..n - 1 {
                            s[i] |= m[i] & range;
                        }
                    }
                }
            }
        }
        LineKind::Entropic => {
            s = m.clone();
            for i in 0..n - 2 {
                let mut support = [0; 3];
                for x in 1..=9 {
                    for y in 1..=9 {
                        for z in 1..=9 {
                            if m[i] & bit(x) != 0
                                && m[i + 1] & bit(y) != 0
                                && m[i + 2] & bit(z) != 0
                                && (x - 1) / 3 != (y - 1) / 3
                                && (x - 1) / 3 != (z - 1) / 3
                                && (y - 1) / 3 != (z - 1) / 3
                            {
                                support[0] |= bit(x);
                                support[1] |= bit(y);
                                support[2] |= bit(z);
                            }
                        }
                    }
                }
                for j in 0..3 {
                    s[i + j] &= support[j];
                }
            }
        }
        LineKind::RegionSum => {
            let segments = line.segments();
            let bounds: Vec<_> = segments
                .iter()
                .map(|segment| {
                    let low: usize = segment
                        .iter()
                        .map(|&i| {
                            if masks[i] == 0 {
                                10
                            } else {
                                masks[i].trailing_zeros() as usize + 1
                            }
                        })
                        .sum();
                    let high: usize = segment
                        .iter()
                        .map(|&i| {
                            if masks[i] == 0 {
                                0
                            } else {
                                16 - masks[i].leading_zeros() as usize
                            }
                        })
                        .sum();
                    (low, high)
                })
                .collect();
            let low = bounds.iter().map(|b| b.0).max().unwrap_or(1);
            let high = bounds.iter().map(|b| b.1).min().unwrap_or(0);
            for total in low..=high {
                let supports: Vec<_> = segments
                    .iter()
                    .map(|segment| {
                        sum_support(
                            &segment.iter().map(|&i| masks[i]).collect::<Vec<_>>(),
                            total,
                        )
                    })
                    .collect();
                if supports.iter().all(|s| !s.contains(&0)) {
                    let mut j = 0;
                    for support in supports {
                        for v in support {
                            s[j] |= v;
                            j += 1;
                        }
                    }
                }
            }
        }
    }
    s
}
pub(super) fn sandwich_support(clue: &Sandwich, masks: &[u16]) -> Vec<u16> {
    let cells = clue.cells();
    let m: Vec<_> = cells.iter().map(|&i| masks[i]).collect();
    let mut support = vec![0; 9];
    let middle = ALL_DIGITS & !bit(1) & !bit(9);
    for a in 0..9 {
        if m[a] & bit(1) == 0 {
            continue;
        }
        for b in 0..9 {
            if a == b || m[b] & bit(9) == 0 {
                continue;
            }
            let inside: Vec<_> = (a.min(b) + 1..a.max(b)).collect();
            let outside: Vec<_> = (0..9)
                .filter(|i| *i != a && *i != b && !inside.contains(i))
                .collect();
            let im: Vec<_> = inside.iter().map(|&i| m[i] & middle).collect();
            let om: Vec<_> = outside.iter().map(|&i| m[i] & middle).collect();
            let ip = reachable(&im);
            let op = reachable(&om);
            let mut inf = [false; 512];
            let mut outf = [false; 512];
            for used in 0..512 {
                let rest = middle as usize ^ used;
                if ip[im.len()][used] && digit_sum(used as u16) == clue.sum && op[om.len()][rest] {
                    inf[used] = true;
                    outf[rest] = true;
                }
            }
            if !inf.contains(&true) {
                continue;
            }
            support[a] |= bit(1);
            support[b] |= bit(9);
            for (indices, domains, finals) in [(&inside, &im, &inf), (&outside, &om, &outf)] {
                for (&i, s) in indices.iter().zip(matching(domains, finals)) {
                    support[i] |= s;
                }
            }
        }
    }
    support
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check_line(kind: LineKind, cells: Vec<usize>, digits: &[u8], expected: bool) {
        let line = Line { kind, cells };
        assert!(line.valid_shape());
        let mut values = vec![1; 81];
        for (&i, &d) in line.cells.iter().zip(digits) {
            values[i] = d;
        }
        assert_eq!(line.valid(&values), expected, "{kind:?} {digits:?}");
        let masks: Vec<_> = values.into_iter().map(bit).collect();
        let support = line_support(&line, &masks);
        if expected {
            assert_eq!(support, digits.iter().copied().map(bit).collect::<Vec<_>>());
        } else {
            assert!(support.contains(&0));
        }
    }

    #[test]
    fn line_definitions_cover_repeats_boundaries_and_box_reentry() {
        for (kind, yes, no) in [
            (LineKind::Arrow, [6, 3, 3], [6, 3, 4]),
            (LineKind::Renban, [5, 3, 4], [3, 3, 4]),
            (LineKind::Whispers, [1, 6, 1], [1, 5, 1]),
            (LineKind::Palindrome, [3, 5, 3], [3, 5, 4]),
            (LineKind::Between, [2, 5, 8], [2, 2, 8]),
            (LineKind::Entropic, [3, 4, 9], [3, 4, 6]),
        ] {
            check_line(kind, vec![0, 1, 2], &yes, true);
            check_line(kind, vec![0, 1, 2], &no, false);
        }
        // Box A, box B, box A again are three separate segments.
        check_line(LineKind::RegionSum, vec![2, 3, 11], &[4, 4, 4], true);
        check_line(LineKind::RegionSum, vec![2, 3, 11], &[4, 8, 4], false);
        check_line(LineKind::Entropic, vec![0, 1, 2, 3], &[1, 4, 7, 2], true);
        check_line(LineKind::Entropic, vec![0, 1, 2, 3], &[1, 4, 7, 5], false);
    }

    #[test]
    fn line_support_never_removes_a_brute_force_completion() {
        for kind in LineKind::ALL {
            let cells = if kind == LineKind::RegionSum {
                vec![2, 3, 11]
            } else {
                vec![0, 1, 2]
            };
            let line = Line { kind, cells };
            for masks in [
                [bit(1) | bit(6), bit(2) | bit(7), bit(3) | bit(8)],
                [ALL_DIGITS; 3],
            ] {
                let mut domains = vec![ALL_DIGITS; 81];
                for (&i, &m) in line.cells.iter().zip(&masks) {
                    domains[i] = m;
                }
                let support = line_support(&line, &domains);
                let mut values = vec![1; 81];
                let mut exact = [0; 3];
                for a in 1..=9 {
                    for b in 1..=9 {
                        for c in 1..=9 {
                            let digits = [a, b, c];
                            if digits.iter().zip(masks).any(|(&d, m)| m & bit(d) == 0) {
                                continue;
                            }
                            for (&i, &d) in line.cells.iter().zip(&digits) {
                                values[i] = d;
                            }
                            if line.valid(&values) {
                                for j in 0..3 {
                                    exact[j] |= bit(digits[j]);
                                }
                            }
                        }
                    }
                }
                for (s, e) in support.into_iter().zip(exact) {
                    assert_eq!(s & e, e, "{kind:?}");
                }
            }
        }
    }

    #[test]
    fn sandwich_support_matches_independent_permutation_search() {
        fn enumerate(
            pos: usize,
            values: &mut [u8],
            cells: &[usize],
            masks: &[u16],
            used: u16,
            clue: &Sandwich,
            support: &mut [u16],
        ) {
            if pos == 9 {
                if clue.valid(values) {
                    for (j, &i) in cells.iter().enumerate() {
                        support[j] |= bit(values[i]);
                    }
                }
                return;
            }
            for d in 1..=9 {
                if masks[cells[pos]] & bit(d) != 0 && used & bit(d) == 0 {
                    values[cells[pos]] = d;
                    enumerate(pos + 1, values, cells, masks, used | bit(d), clue, support);
                }
            }
        }
        for unit in [0, 13] {
            for sum in [0, 5, 15, 35] {
                let clue = Sandwich { unit, sum };
                let cells = clue.cells();
                let mut masks = vec![ALL_DIGITS; 81];
                for (j, &i) in cells.iter().enumerate() {
                    masks[i] = bit((j + 1) as u8) | bit((9 - j) as u8) | bit(1) | bit(9);
                }
                let mut expected = vec![0; 9];
                enumerate(0, &mut [0; 81], &cells, &masks, 0, &clue, &mut expected);
                assert_eq!(sandwich_support(&clue, &masks), expected, "{unit} {sum}");
            }
        }
    }
}
