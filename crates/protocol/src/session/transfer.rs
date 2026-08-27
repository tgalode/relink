//! Préambule, graine aléatoire, bloc d'échange, patch list.
//!
//! Deux principes tirés du sourçage :
//!
//! - **On ne compte que là où les sources s'accordent.** Le nombre d'octets
//!   neutres n'est pas fixé ; le nombre d'octets de préambule l'est.
//! - **On ne sort pas de la patch list trop tôt.** Après ses deux
//!   terminateurs viennent des octets de remplissage à zéro, identiques à
//!   ceux de la phase de sélection : rien ne les distingue. Sortir en avance
//!   ferait présenter une offre pendant que la cartouche lit encore sa liste,
//!   et cette offre serait prise pour une position à corriger — le bloc
//!   présenté arriverait faux. On suit donc le compte de la seule
//!   implémentation vérifiée sur matériel, et sortir en retard est sans
//!   conséquence : on y présente l'octet neutre, celui-là même que la
//!   cartouche attend.
//!
//! L'octet de leader n'est pas traité ici, et c'est délibéré : ces phases
//! transportent des octets arbitraires, où `0x01` est une donnée et non une
//! demande de renégociation. Une cartouche qui redémarre en plein transfert
//! laisse donc la session bloquée ; `protocol` n'a pas d'horloge et ne peut
//! pas s'en sortir seul. C'est au firmware de la détruire et d'en ouvrir une
//! neuve.

use super::{BLANK, Effect, OFF_PARTY_DATA, PREAMBLE, Phase, Session, Step};
use crate::gen1::TRADE_BLOCK_LEN;
use crate::gen1::patch_list::{self, PARTY_DATA_LEN, PATCH_LIST_LEN};

/// Octets de préambule qui ouvrent la graine.
const SEED_PREAMBLE: u16 = 10;

/// Octets d'aléa, puis les 9 octets de préambule qui ferment la section.
const SEED_LEN: u16 = 19;

/// Octets de préambule entre le bloc et la patch list.
const PATCH_PREAMBLE: u16 = 6;

/// Octets d'en-tête neutres avant les données de liste. L'octet qui a
/// complété le sixième préambule est déjà consommé par la phase précédente :
/// il en reste sept.
const PATCH_HEADER_LEN: u16 = 7;

/// Longueur de la section, comptée depuis son premier octet. Décalée de deux
/// par rapport au compte de la source (196), qui démarre le sien un octet
/// plus tôt et compte à partir de un.
const PATCH_SECTION_LEN: u16 = 195;

impl Session {
    pub(crate) fn step_preamble(&mut self, incoming: u8) -> Step {
        if incoming == PREAMBLE {
            self.cursor = self.cursor.saturating_add(1);
            if self.cursor >= SEED_PREAMBLE {
                self.phase = Phase::Seed;
                self.cursor = 0;
            }
        }
        self.plain(incoming)
    }

    pub(crate) fn step_seed(&mut self, incoming: u8) -> Step {
        self.cursor = self.cursor.saturating_add(1);
        if self.cursor >= SEED_LEN {
            self.phase = Phase::Block;
            self.cursor = 0;
        }
        self.plain(incoming)
    }

    pub(crate) fn step_block(&mut self, incoming: u8) -> Step {
        let position = self.cursor as usize;
        let outgoing = if position < TRADE_BLOCK_LEN {
            self.incoming[position] = incoming;
            self.outgoing[position]
        } else {
            BLANK
        };

        self.cursor = self.cursor.saturating_add(1);
        if self.cursor as usize >= TRADE_BLOCK_LEN {
            self.phase = Phase::PatchHeader;
            self.cursor = 0;
        }
        self.plain(outgoing)
    }

    pub(crate) fn step_patch_header(&mut self, incoming: u8) -> Step {
        if incoming == PREAMBLE {
            self.cursor = self.cursor.saturating_add(1);
            if self.cursor >= PATCH_PREAMBLE {
                self.phase = Phase::PatchList;
                self.cursor = 0;
                self.incoming_patch = [0u8; PATCH_LIST_LEN];
            }
        }
        self.plain(incoming)
    }

    pub(crate) fn step_patch_list(&mut self, incoming: u8) -> Step {
        let position = self.cursor;
        self.cursor = self.cursor.saturating_add(1);

        let outgoing = if position < PATCH_HEADER_LEN {
            incoming
        } else {
            let index = (position - PATCH_HEADER_LEN) as usize;
            if index < PATCH_LIST_LEN {
                self.incoming_patch[index] = incoming;
                self.outgoing_patch[index]
            } else {
                BLANK
            }
        };

        if self.cursor >= PATCH_SECTION_LEN {
            self.finish_transfer();
            return self.with(outgoing, Effect::PartnerBlockReceived);
        }
        self.plain(outgoing)
    }

    /// Applique la patch list reçue à l'équipe entrante et passe la main à la
    /// phase de sélection.
    fn finish_transfer(&mut self) {
        let mut party = [0u8; PARTY_DATA_LEN];
        party.copy_from_slice(&self.incoming[OFF_PARTY_DATA..OFF_PARTY_DATA + PARTY_DATA_LEN]);
        patch_list::apply(&mut party, &self.incoming_patch);
        self.incoming[OFF_PARTY_DATA..OFF_PARTY_DATA + PARTY_DATA_LEN].copy_from_slice(&party);

        self.partner_ready = true;
        self.phase = Phase::Select;
        self.cursor = 0;
        self.reset_round();
    }
}
