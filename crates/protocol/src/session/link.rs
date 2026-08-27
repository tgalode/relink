//! Négociation des rôles, acquittement de connexion, menu du Cable Club.
//!
//! Ces trois phases ont une règle commune : le module renvoie ce qu'il
//! reçoit. Les sources ne s'accordent pas sur le nombre d'octets neutres
//! échangés, et c'est le joueur qui choisit dans le menu — le module ne
//! compte rien et ne décide rien.

use super::{Effect, MASTER, PREAMBLE, Phase, SLAVE, Session, Step};

impl Session {
    pub(crate) fn step_negotiating(&mut self, incoming: u8) -> Step {
        if incoming == MASTER {
            return self.plain(SLAVE);
        }
        if incoming == self.bytes.connected {
            self.phase = Phase::Menu;
            return self.with(incoming, Effect::LinkEstablished);
        }
        self.plain(incoming)
    }

    pub(crate) fn step_menu(&mut self, incoming: u8) -> Step {
        if incoming == MASTER {
            return self.restart();
        }
        if incoming == self.bytes.trade_centre {
            self.phase = Phase::Waiting;
            return self.plain(incoming);
        }
        if incoming == self.bytes.colosseum || incoming == self.bytes.break_link {
            self.phase = Phase::Broken;
            return self.with(self.bytes.break_link, Effect::LinkBroken);
        }
        self.plain(incoming)
    }

    pub(crate) fn step_waiting(&mut self, incoming: u8) -> Step {
        if incoming == MASTER {
            return self.restart();
        }
        if incoming == PREAMBLE {
            self.phase = Phase::Preamble;
            self.cursor = 1;
            return self.plain(incoming);
        }
        self.plain(incoming)
    }

    pub(crate) fn step_broken(&mut self, incoming: u8) -> Step {
        if incoming == MASTER {
            return self.restart();
        }
        self.plain(self.bytes.break_link)
    }
}
