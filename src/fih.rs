//! Persistent pet rules. No rendering, device input, or background timers.
/// Each care interaction belongs to its own room.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Room {
    Kitchen,
    Bathroom,
    Bedroom,
    Playroom,
    Clinic,
}
impl Room {
    pub const ALL: [Self; 5] = [
        Self::Kitchen,
        Self::Bathroom,
        Self::Bedroom,
        Self::Playroom,
        Self::Clinic,
    ];
    pub fn index(self) -> usize {
        Self::ALL.iter().position(|&r| r == self).unwrap()
    }
    pub fn title(self) -> &'static str {
        match self {
            Self::Kitchen => "Kitchen",
            Self::Bathroom => "Bathroom",
            Self::Bedroom => "Bedroom",
            Self::Playroom => "Playroom",
            Self::Clinic => "Clinic",
        }
    }
    pub fn neighbor(self, direction: i32) -> Self {
        Self::ALL[(self.index() as i32 + direction).rem_euclid(5) as usize]
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Food {
    pub name: &'static str,
    pub category: usize,
    pub price: u32,
    pub nutrition: f32,
    pub joy: f32,
    pub health: f32,
}
pub const FOODS: [Food; 12] = [
    Food {
        name: "Flakes",
        category: 0,
        price: 0,
        nutrition: 18.,
        joy: 0.,
        health: 0.,
    },
    Food {
        name: "Apple",
        category: 0,
        price: 2,
        nutrition: 20.,
        joy: 3.,
        health: 2.,
    },
    Food {
        name: "Berries",
        category: 0,
        price: 3,
        nutrition: 16.,
        joy: 6.,
        health: 3.,
    },
    Food {
        name: "Seaweed",
        category: 0,
        price: 1,
        nutrition: 14.,
        joy: 1.,
        health: 4.,
    },
    Food {
        name: "Sushi",
        category: 1,
        price: 4,
        nutrition: 30.,
        joy: 5.,
        health: 2.,
    },
    Food {
        name: "Shrimp",
        category: 1,
        price: 4,
        nutrition: 28.,
        joy: 4.,
        health: 4.,
    },
    Food {
        name: "Sandwich",
        category: 1,
        price: 3,
        nutrition: 26.,
        joy: 3.,
        health: 0.,
    },
    Food {
        name: "Soup",
        category: 1,
        price: 3,
        nutrition: 24.,
        joy: 2.,
        health: 5.,
    },
    Food {
        name: "Cake",
        category: 2,
        price: 5,
        nutrition: 25.,
        joy: 12.,
        health: 0.,
    },
    Food {
        name: "Donut",
        category: 2,
        price: 3,
        nutrition: 20.,
        joy: 8.,
        health: 0.,
    },
    Food {
        name: "Ice cream",
        category: 2,
        price: 4,
        nutrition: 18.,
        joy: 10.,
        health: 0.,
    },
    Food {
        name: "Juice",
        category: 3,
        price: 2,
        nutrition: 12.,
        joy: 5.,
        health: 3.,
    },
];
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Care {
    Feed,
    Treat,
    Wash,
    Pet,
    Sleep,
    Heal,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    Color,
    Clothes,
    Hat,
    Background,
}
impl Style {
    pub fn count(self) -> u8 {
        match self {
            Self::Color => 8,
            Self::Clothes | Self::Hat => 12,
            Self::Background => 8,
        }
    }
    pub fn price(self, index: u8) -> u32 {
        if index == 0 || self == Self::Color {
            0
        } else {
            match self {
                Self::Clothes => {
                    [0, 30, 45, 60, 40, 25, 50, 80, 70, 35, 65, 95][usize::from(index.min(11))]
                }
                Self::Hat => {
                    [0, 20, 50, 35, 25, 40, 55, 30, 60, 25, 65, 20][usize::from(index.min(11))]
                }
                Self::Background => [0, 35, 45, 55, 30, 30, 30, 30][usize::from(index.min(7))],
                Self::Color => 0,
            }
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Changed,
    Full,
    Sleeping,
    Poor,
    Cooldown,
    Invalid,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Fih {
    pub food: f32,
    pub joy: f32,
    pub clean: f32,
    pub energy: f32,
    pub health: f32,
    pub coins: u32,
    pub xp: u32,
    pub sleeping: bool,
    pub color: u8,
    pub clothes: u8,
    pub hat: u8,
    pub background: u8,
    owned: [u16; 4],
    pub best: [u32; 5],
    pub pantry: [u16; 12],
    pub updated: f64,
    last_pet: f64,
}
impl Fih {
    pub fn new(now: f64) -> Self {
        Self {
            food: 78.,
            joy: 80.,
            clean: 82.,
            energy: 76.,
            health: 100.,
            coins: 80,
            xp: 0,
            sleeping: false,
            color: 0,
            clothes: 0,
            hat: 0,
            background: 0,
            owned: [255, 1, 1, 1],
            best: [0; 5],
            pantry: [0, 3, 2, 3, 2, 2, 2, 2, 1, 2, 1, 2],
            updated: now.max(0.),
            last_pet: 0.,
        }
    }
    pub fn level(&self) -> u32 {
        1 + self.xp / 100
    }
    /// Catch up only when visited. No work while the app is closed; no death.
    pub fn advance(&mut self, now: f64) {
        if !now.is_finite() || now <= self.updated {
            return;
        }
        let hours = ((now - self.updated) / 3600.).min(24. * 7.) as f32;
        let healthy_start = if self.sleeping {
            ((15. - self.energy) / 28.).max(0.)
        } else {
            0.
        };
        let mut healthy_end = hours
            .min(((self.food - 20.) / 4.).max(0.))
            .min(((self.clean - 20.) / 2.5).max(0.));
        if !self.sleeping {
            healthy_end = healthy_end.min(((self.energy - 15.) / 3.).max(0.));
        }
        let before = healthy_start.min(hours);
        let healthy = (healthy_end - before).max(0.);
        let after = (hours - before - healthy).max(0.);
        self.health = (self.health - before * 4.).clamp(10., 100.);
        self.health = (self.health + healthy * 3.).clamp(10., 100.);
        self.health = (self.health - after * 4.).clamp(10., 100.);
        self.food = (self.food - hours * 4.).max(0.);
        self.joy = (self.joy - hours * 2.).max(0.);
        self.clean = (self.clean - hours * 2.5).max(0.);
        self.energy = (self.energy + hours * if self.sleeping { 28. } else { -3. }).clamp(0., 100.);
        self.updated = now;
    }
    pub fn care(&mut self, action: Care, now: f64) -> Outcome {
        self.advance(now);
        if action == Care::Sleep {
            self.sleeping = !self.sleeping;
            return Outcome::Changed;
        }
        if self.sleeping {
            return Outcome::Sleeping;
        }
        let (value, amount, cost) = match action {
            Care::Feed => (&mut self.food, 22., 0),
            Care::Treat => (&mut self.food, 38., 5),
            Care::Wash => (&mut self.clean, 35., 0),
            Care::Pet => {
                if now - self.last_pet < 30. {
                    return Outcome::Cooldown;
                }
                (&mut self.joy, 12., 0)
            }
            Care::Heal => (&mut self.health, 30., 8),
            Care::Sleep => unreachable!(),
        };
        if *value >= 99. {
            return Outcome::Full;
        }
        if self.coins < cost {
            return Outcome::Poor;
        }
        *value = (*value + amount).min(100.);
        self.coins -= cost;
        self.xp = self.xp.saturating_add(3).min(1_000_000);
        if action == Care::Treat {
            self.joy = (self.joy + 8.).min(100.);
        }
        if action == Care::Feed || action == Care::Treat {
            self.clean = (self.clean - 2.).max(0.);
        }
        if action == Care::Pet {
            self.last_pet = now;
        }
        Outcome::Changed
    }
    fn slot(style: Style) -> usize {
        match style {
            Style::Color => 0,
            Style::Clothes => 1,
            Style::Hat => 2,
            Style::Background => 3,
        }
    }
    pub fn owns(&self, style: Style, index: u8) -> bool {
        index < style.count() && self.owned[Self::slot(style)] & (1 << index) != 0
    }
    pub fn selected(&self, style: Style) -> u8 {
        match style {
            Style::Color => self.color,
            Style::Clothes => self.clothes,
            Style::Hat => self.hat,
            Style::Background => self.background,
        }
    }
    pub fn customize(&mut self, style: Style, index: u8) -> Outcome {
        if index >= style.count() {
            return Outcome::Invalid;
        }
        if !self.owns(style, index) {
            let price = style.price(index);
            if self.coins < price {
                return Outcome::Poor;
            }
            self.coins -= price;
            self.owned[Self::slot(style)] |= 1 << index;
        }
        match style {
            Style::Color => self.color = index,
            Style::Clothes => self.clothes = index,
            Style::Hat => self.hat = index,
            Style::Background => self.background = index,
        }
        Outcome::Changed
    }
    pub fn reward(&mut self, game: usize, score: u32, now: f64) {
        self.advance(now);
        if game >= self.best.len() {
            return;
        }
        let score = score.min(100);
        self.best[game] = self.best[game].max(score);
        self.coins = self.coins.saturating_add(5 + score * 2).min(1_000_000);
        self.xp = self.xp.saturating_add(8 + score).min(1_000_000);
        self.joy = (self.joy + 15.).min(100.);
        self.energy = (self.energy - 5.).max(0.);
        self.clean = (self.clean - 3.).max(0.);
    }
    /// Buying adds stock; feeding never spends coins and never consumes on failure.
    pub fn buy_food(&mut self, index: usize) -> Outcome {
        let Some(food) = FOODS.get(index) else {
            return Outcome::Invalid;
        };
        if index == 0 || self.pantry[index] >= 99 {
            return Outcome::Full;
        }
        if self.coins < food.price {
            return Outcome::Poor;
        }
        self.coins -= food.price;
        self.pantry[index] += 1;
        Outcome::Changed
    }
    pub fn eat(&mut self, index: usize, now: f64) -> Outcome {
        self.advance(now);
        let Some(food) = FOODS.get(index) else {
            return Outcome::Invalid;
        };
        if self.sleeping {
            return Outcome::Sleeping;
        }
        if self.food >= 99. {
            return Outcome::Full;
        }
        if index != 0 && self.pantry[index] == 0 {
            return Outcome::Invalid;
        }
        if index != 0 {
            self.pantry[index] -= 1;
        }
        self.food = (self.food + food.nutrition).min(100.);
        self.joy = (self.joy + food.joy).min(100.);
        self.health = (self.health + food.health).min(100.);
        self.clean = (self.clean - 2.).max(0.);
        self.xp = (self.xp + 3).min(1_000_000);
        Outcome::Changed
    }
    pub fn encode(&self) -> String {
        let mut value = format!(
            "2 {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {}",
            self.food,
            self.joy,
            self.clean,
            self.energy,
            self.health,
            self.coins,
            self.xp,
            u8::from(self.sleeping),
            self.color,
            self.clothes,
            self.hat,
            self.background,
            self.owned[0],
            self.owned[1],
            self.owned[2],
            self.owned[3],
            self.best[0],
            self.best[1],
            self.updated,
            self.last_pet
        );
        for score in &self.best[2..] {
            value.push_str(&format!(" {score}"));
        }
        for stock in self.pantry {
            value.push_str(&format!(" {stock}"));
        }
        value
    }
    pub fn decode(value: &str, now: f64) -> Self {
        fn parse(value: &str) -> Option<Fih> {
            let f: Vec<_> = value.split_whitespace().collect();
            if !((f.len() == 21 && f[0] == "1") || (f.len() == 36 && f[0] == "2")) {
                return None;
            }
            let mut pet = Fih::new(0.);
            pet.food = f[1].parse().ok()?;
            pet.joy = f[2].parse().ok()?;
            pet.clean = f[3].parse().ok()?;
            pet.energy = f[4].parse().ok()?;
            pet.health = f[5].parse().ok()?;
            pet.coins = f[6].parse().ok()?;
            pet.xp = f[7].parse().ok()?;
            pet.sleeping = match f[8] {
                "0" => false,
                "1" => true,
                _ => return None,
            };
            pet.color = f[9].parse().ok()?;
            pet.clothes = f[10].parse().ok()?;
            pet.hat = f[11].parse().ok()?;
            pet.background = f[12].parse().ok()?;
            for (i, owned) in pet.owned.iter_mut().enumerate() {
                *owned = f[13 + i].parse().ok()?;
            }
            pet.best[0] = f[17].parse().ok()?;
            pet.best[1] = f[18].parse().ok()?;
            if f[0] == "2" {
                for i in 2..5 {
                    pet.best[i] = f[19 + i].parse().ok()?;
                }
                for i in 0..12 {
                    pet.pantry[i] = f[24 + i].parse().ok()?;
                }
                if pet.pantry.iter().any(|&n| n > 99) {
                    return None;
                }
            }
            pet.updated = f[19].parse().ok()?;
            pet.last_pet = f[20].parse().ok()?;
            if [pet.food, pet.joy, pet.clean, pet.energy, pet.health]
                .iter()
                .any(|v| !v.is_finite() || !(0. ..=100.).contains(v))
                || !pet.updated.is_finite()
                || pet.updated < 0.
                || !pet.last_pet.is_finite()
                || pet.last_pet < 0.
                || pet.coins > 1_000_000
                || pet.xp > 1_000_000
                || pet.best.iter().any(|&v| v > 100)
            {
                return None;
            }
            for style in [Style::Color, Style::Clothes, Style::Hat, Style::Background] {
                let owned = pet.owned[Fih::slot(style)];
                let count = if f[0] == "1" && style != Style::Color {
                    4
                } else {
                    style.count()
                };
                if owned & 1 == 0 || owned >= (1 << count) || !pet.owns(style, pet.selected(style))
                {
                    return None;
                }
            }
            Some(pet)
        }
        let mut pet = parse(value).unwrap_or_else(|| Self::new(now));
        // A clock change must not stall care for days.
        if pet.updated > now {
            pet.updated = now;
        }
        if pet.last_pet > now {
            pet.last_pet = 0.;
        }
        pet.advance(now);
        pet
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn care_is_bounded_and_sleep_blocks_feeding() {
        let mut f = Fih::new(100.);
        assert_eq!(f.care(Care::Feed, 100.), Outcome::Changed);
        assert_eq!(f.food, 100.);
        assert_eq!(f.care(Care::Feed, 100.), Outcome::Full);
        f.care(Care::Sleep, 100.);
        assert_eq!(f.care(Care::Wash, 100.), Outcome::Sleeping);
        f.advance(3700.);
        assert_eq!(f.energy, 100.);
        f.care(Care::Sleep, 3700.);
        assert!(!f.sleeping);
    }
    #[test]
    fn elapsed_time_is_independent_of_visits_and_clock_rollback_is_safe() {
        let mut a = Fih::new(100.);
        let mut b = a.clone();
        a.advance(3700.);
        for t in 1..=60 {
            b.advance(100. + f64::from(t) * 60.);
        }
        assert!((a.food - b.food).abs() < 0.001);
        let old = a.clone();
        a.advance(99.);
        assert_eq!(a, old);
        let restored = Fih::decode(&a.encode(), 50.);
        assert_eq!(restored.updated, 50.);
    }
    #[test]
    fn neglect_is_recoverable_and_pet_cooldown_prevents_farming() {
        let mut f = Fih::new(100.);
        f.advance(100. + 10. * 86400.);
        assert!(f.health >= 10.);
        assert_eq!(f.care(Care::Feed, f.updated), Outcome::Changed);
        f.joy = 20.;
        assert_eq!(f.care(Care::Pet, f.updated), Outcome::Changed);
        assert_eq!(f.care(Care::Pet, f.updated + 1.), Outcome::Cooldown);
    }
    #[test]
    fn purchases_charge_once_and_never_spend_if_unaffordable() {
        let mut f = Fih::new(100.);
        assert_eq!(f.customize(Style::Clothes, 1), Outcome::Changed);
        assert_eq!(f.coins, 50);
        f.customize(Style::Clothes, 0);
        f.customize(Style::Clothes, 1);
        assert_eq!(f.coins, 50);
        let before = f.clone();
        assert_eq!(f.customize(Style::Clothes, 3), Outcome::Poor);
        assert_eq!(f, before);
        assert_eq!(f.customize(Style::Color, 99), Outcome::Invalid);
    }
    #[test]
    fn saves_preserve_every_stat_purchase_and_selection() {
        let mut f = Fih::new(100.);
        f.customize(Style::Hat, 1);
        f.customize(Style::Color, 7);
        f.reward(0, 12, 100.);
        assert_eq!(Fih::decode(&f.encode(), 100.), f);
        for bad in ["", "2 10", "1 NaN", "1 inf"] {
            assert_eq!(Fih::decode(bad, 100.), Fih::new(100.));
        }
        for (index, value) in [
            (2, "NaN"),
            (6, "1000001"),
            (9, "255"),
            (13, "0"),
            (19, "inf"),
        ] {
            let mut fields: Vec<String> =
                f.encode().split_whitespace().map(str::to_owned).collect();
            fields[index] = value.into();
            assert_eq!(Fih::decode(&fields.join(" "), 100.), Fih::new(100.));
        }
    }
    #[test]
    fn rewards_update_progress_and_high_scores() {
        let mut f = Fih::new(100.);
        f.reward(1, 15, 100.);
        assert_eq!(f.coins, 115);
        assert_eq!(f.best[1], 15);
        assert_eq!(f.xp, 23);
        f.reward(1, 2, 100.);
        assert_eq!(f.best[1], 15);
    }

    #[test]
    fn pantry_and_food_transactions_preserve_stock_on_failure() {
        let mut f = Fih::new(100.);
        let n = f.pantry[4];
        f.sleeping = true;
        assert_eq!(f.eat(4, 100.), Outcome::Sleeping);
        assert_eq!(f.pantry[4], n);
        f.sleeping = false;
        f.food = 100.;
        assert_eq!(f.eat(4, 100.), Outcome::Full);
        assert_eq!(f.pantry[4], n);
        f.food = 50.;
        assert_eq!(f.eat(4, 100.), Outcome::Changed);
        assert_eq!(f.food, 80.);
        assert_eq!(f.pantry[4], n - 1);
        let coins = f.coins;
        assert_eq!(f.buy_food(4), Outcome::Changed);
        assert_eq!(f.coins, coins - 4);
        assert_eq!(f.pantry[4], n);
        f.coins = 0;
        let before = f.clone();
        assert_eq!(f.buy_food(4), Outcome::Poor);
        assert_eq!(f, before);
        f.food = 0.;
        assert_eq!(f.eat(0, 100.), Outcome::Changed);
        assert_eq!(f.coins, 0);
        assert_eq!(Fih::decode(&f.encode(), 100.), f);
    }
    #[test]
    fn old_saves_migrate_without_losing_purchases_or_scores() {
        let old = "1 70 60 50 40 90 123 200 0 2 1 3 0 255 3 9 1 12 8 100 90";
        let f = Fih::decode(old, 100.);
        assert_eq!(f.coins, 123);
        assert_eq!(f.best, [12, 8, 0, 0, 0]);
        assert!(f.owns(Style::Hat, 3));
        assert_eq!(f.clothes, 1);
        assert!(f.pantry[4] > 0);
        assert_eq!(Fih::decode(&f.encode(), 100.), f);
        for r in Room::ALL {
            assert_eq!(r.neighbor(1).neighbor(-1), r);
        }
    }
    #[test]
    fn offline_health_matches_frequent_visits_across_need_thresholds() {
        for sleeping in [false, true] {
            let mut a = Fih::new(100.);
            a.sleeping = sleeping;
            a.energy = 5.;
            a.health = 50.;
            let mut b = a.clone();
            a.advance(100. + 24. * 3600.);
            for minute in 1..=1440 {
                b.advance(100. + f64::from(minute) * 60.);
            }
            assert!(
                (a.health - b.health).abs() < 0.01,
                "{} {}",
                a.health,
                b.health
            );
            assert!((a.energy - b.energy).abs() < 0.01);
        }
    }
}
