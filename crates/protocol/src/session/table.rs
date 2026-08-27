//! Phases de table. Corps écrits à la tâche 4.

use super::{BLANK, Session, Step};

impl Session {
    pub(crate) fn step_select(&mut self, _incoming: u8) -> Step {
        self.plain(BLANK)
    }
    pub(crate) fn step_verdict(&mut self, _incoming: u8) -> Step {
        self.plain(BLANK)
    }
    pub(crate) fn step_trading(&mut self, _incoming: u8) -> Step {
        self.plain(BLANK)
    }
}
