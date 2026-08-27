//! Phases de transfert. Corps écrits à la tâche 3.

use super::{BLANK, Session, Step};

impl Session {
    pub(crate) fn step_preamble(&mut self, _incoming: u8) -> Step {
        self.plain(BLANK)
    }
    pub(crate) fn step_seed(&mut self, _incoming: u8) -> Step {
        self.plain(BLANK)
    }
    pub(crate) fn step_block(&mut self, _incoming: u8) -> Step {
        self.plain(BLANK)
    }
    pub(crate) fn step_patch_header(&mut self, _incoming: u8) -> Step {
        self.plain(BLANK)
    }
    pub(crate) fn step_patch_list(&mut self, _incoming: u8) -> Step {
        self.plain(BLANK)
    }
}
