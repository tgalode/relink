//! Sélection du Pokémon, verdict, échange, sortie de table.
//!
//! C'est ici que vivent les deux seuls points d'attente de la session. Tant
//! que la décision manque, on présente l'octet neutre : la cartouche en
//! envoie autant, et l'attend sans échéance. Le jeu y lit un dresseur qui
//! hésite dans ses menus.
//!
//! `0x61` vaut « je propose le Pokémon d'index 1 » en sélection et « je
//! refuse » en verdict. Seule la phase les distingue.

use super::{BLANK, Effect, LAST_INDEX, MASTER, PREAMBLE, Phase, Session, Step};

impl Session {
    pub(crate) fn step_select(&mut self, incoming: u8) -> Step {
        if incoming == MASTER {
            return self.restart();
        }

        if self.leaving {
            return self.leave_table();
        }

        if incoming == self.bytes.table_leave {
            return self.leave_table();
        }

        // Ce que le joueur annonce passe avant la demande d'offre : sinon
        // l'annonce avalerait l'octet, et son offre serait perdue. La demande
        // reste due, et sort au premier octet qui ne dit rien d'autre.
        if let Some(index) = self.partner_index(incoming) {
            self.partner_offer = Some(index);
            return self.with(self.select_outgoing(), Effect::PartnerOffered { index });
        }

        if !self.announced {
            self.announced = true;
            return self.with(self.select_outgoing(), Effect::OfferNeeded);
        }

        if incoming == BLANK && self.offer.is_some() && self.partner_offer.is_some() {
            self.phase = Phase::Verdict;
            return self.with(BLANK, Effect::VerdictNeeded);
        }

        self.plain(self.select_outgoing())
    }

    pub(crate) fn step_verdict(&mut self, incoming: u8) -> Step {
        if incoming == MASTER {
            return self.restart();
        }

        if incoming == self.bytes.trade_reject {
            self.phase = Phase::Select;
            self.reset_round();
            return self.plain(BLANK);
        }

        if incoming == self.bytes.trade_accept {
            self.partner_verdict = Some(true);
            if self.verdict == Some(true) {
                let offered = self.offer.unwrap_or(0);
                let received = self.partner_offer.unwrap_or(0);
                self.phase = Phase::Trading;
                return self.with(
                    self.bytes.trade_accept,
                    Effect::TradeAgreed { offered, received },
                );
            }
        }

        self.plain(self.verdict_outgoing())
    }

    pub(crate) fn step_trading(&mut self, incoming: u8) -> Step {
        if incoming == PREAMBLE {
            self.phase = Phase::Preamble;
            self.cursor = 1;
            self.partner_ready = false;
            self.reset_round();
            return self.plain(incoming);
        }
        self.plain(BLANK)
    }

    /// Quitte la table et regagne la salle d'échange.
    fn leave_table(&mut self) -> Step {
        self.phase = Phase::Waiting;
        self.reset_round();
        self.with(self.bytes.table_leave, Effect::TableLeft)
    }

    /// L'octet à présenter en sélection : l'offre si elle est connue, l'octet
    /// neutre sinon.
    fn select_outgoing(&self) -> u8 {
        match self.offer {
            Some(index) => self.bytes.select_base.wrapping_add(index),
            None => BLANK,
        }
    }

    /// L'octet à présenter en verdict.
    fn verdict_outgoing(&self) -> u8 {
        match self.verdict {
            Some(true) => self.bytes.trade_accept,
            Some(false) => self.bytes.trade_reject,
            None => BLANK,
        }
    }

    /// La position annoncée par le joueur, si l'octet en désigne une.
    fn partner_index(&self, incoming: u8) -> Option<u8> {
        let base = self.bytes.select_base;
        if incoming < base || incoming > base + LAST_INDEX {
            return None;
        }
        Some(incoming - base)
    }
}
