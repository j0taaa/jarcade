//! Shareable pages, independent of the renderer and browser history.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route {
    SinglePlayer,
    Multiplayer,
    Settings,
    Snake,
    Mines,
    MinesPlay,
    Fih,
    Kitchen,
    Bathroom,
    Bedroom,
    Playroom,
    Clinic,
    Coupe,
    Dicksit,
    Wolvesville,
    Codenames,
    Wavelength,
    TableTennis,
    Nonograms,
    NonogramsPlay,
    Sudoku,
    SudokuPlay,
}
impl Route {
    pub const ALL: [Self; 22] = [
        Self::SinglePlayer,
        Self::Multiplayer,
        Self::Settings,
        Self::Snake,
        Self::Mines,
        Self::MinesPlay,
        Self::Fih,
        Self::Kitchen,
        Self::Bathroom,
        Self::Bedroom,
        Self::Playroom,
        Self::Clinic,
        Self::Coupe,
        Self::Dicksit,
        Self::Wolvesville,
        Self::Codenames,
        Self::Wavelength,
        Self::TableTennis,
        Self::Nonograms,
        Self::NonogramsPlay,
        Self::Sudoku,
        Self::SudokuPlay,
    ];
    pub fn path(self) -> &'static str {
        match self {
            Self::SinglePlayer => "/",
            Self::Multiplayer => "/multiplayer",
            Self::Settings => "/settings",
            Self::Snake => "/games/snake",
            Self::Mines => "/games/minesweeper",
            Self::MinesPlay => "/games/minesweeper/play",
            Self::Fih => "/games/fih",
            Self::Kitchen => "/games/fih/kitchen",
            Self::Bathroom => "/games/fih/bathroom",
            Self::Bedroom => "/games/fih/bedroom",
            Self::Playroom => "/games/fih/playroom",
            Self::Clinic => "/games/fih/clinic",
            Self::Coupe => "/games/coupe",
            Self::Dicksit => "/games/dicksit",
            Self::Wolvesville => "/games/wolvesville",
            Self::Codenames => "/games/codenames",
            Self::Wavelength => "/games/wavelength",
            Self::TableTennis => "/games/table-tennis",
            Self::Nonograms => "/games/nonograms",
            Self::NonogramsPlay => "/games/nonograms/play",
            Self::Sudoku => "/games/sudoku",
            Self::SudokuPlay => "/games/sudoku/play",
        }
    }
    pub fn parse(path: &str) -> Option<Self> {
        let path = path.split(['?', '#']).next()?.trim_end_matches('/');
        Self::ALL
            .into_iter()
            .find(|r| r.path().trim_end_matches('/') == path)
    }
    pub fn title(self) -> &'static str {
        match self {
            Self::SinglePlayer => "Single player",
            Self::Multiplayer => "Multiplayer",
            Self::Settings => "Settings",
            Self::Snake => "Snake",
            Self::Mines | Self::MinesPlay => "Minesweeper",
            Self::Fih
            | Self::Kitchen
            | Self::Bathroom
            | Self::Bedroom
            | Self::Playroom
            | Self::Clinic => "Fih",
            Self::Coupe => "Coupe",
            Self::Dicksit => "Dicksit",
            Self::Wolvesville => "Wolvesville",
            Self::Codenames => "Codenames",
            Self::Wavelength => "Wavelength",
            Self::TableTennis => "Table tennis",
            Self::Nonograms | Self::NonogramsPlay => "Nonograms",
            Self::Sudoku | Self::SudokuPlay => "Sudoku",
        }
    }
    pub fn is_menu(self) -> bool {
        matches!(
            self,
            Self::SinglePlayer | Self::Multiplayer | Self::Settings
        )
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn routes_round_trip_and_accept_links_with_queries() {
        for route in Route::ALL {
            assert_eq!(Route::parse(route.path()), Some(route));
            assert_eq!(
                Route::parse(&format!("{}/?room=ABC123", route.path())),
                Some(route)
            );
        }
        assert_eq!(Route::parse("/rooms.json"), None);
        assert_eq!(Route::parse("/games/missing"), None);
        assert_eq!(Route::parse("/games/sudoku/play/extra"), None);
    }
}
