use crate::Player;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Vacant,
    Occupied { player: Player },
    Disabled,
}
