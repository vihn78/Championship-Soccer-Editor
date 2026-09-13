/// Stato delle finestre che modificano la struttura dei campionati.
#[derive(Default)]
pub struct Creation {
    pub kind: CreationKind,
    pub country: String,
    pub division_name: String,
    pub clubs: String,
}

#[derive(Default, PartialEq, Eq)]
pub enum CreationKind {
    #[default]
    League,
    Division {
        league_index: usize,
    },
}

pub enum Removal {
    Division {
        league_index: usize,
        division_index: usize,
        delete_clubs: bool,
    },
    League {
        league_index: usize,
        delete_clubs: bool,
    },
    Club {
        league_index: usize,
        division_index: usize,
        club_index: usize,
        delete_club: bool,
    },
}

pub struct ClubAddition {
    pub league_index: usize,
    pub division_index: usize,
    pub source: ClubSource,
    pub club_name: String,
}

#[derive(Default, PartialEq, Eq)]
pub enum ClubSource {
    #[default]
    Neutral,
    New,
}
