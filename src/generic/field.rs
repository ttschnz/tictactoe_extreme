use crate::Player;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Field {
    Vacant,
    Occupied { player: Player },
    Disabled,
}
