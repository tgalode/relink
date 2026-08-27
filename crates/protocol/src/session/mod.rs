//! Machine à états de l'échange par câble link.
//!
//! Le module joue toujours le suiveur : la cartouche fournit l'horloge et
//! dicte le rythme. `step()` est donc appelée à chaque octet reçu et doit
//! présenter l'octet sortant sans allouer, sans attendre et sans faillir.
//!
//! Déroulé et valeurs sourcés dans `docs/protocol/gen1-link-protocol.md`.

mod link;
mod table;
mod transfer;

use crate::gen1::patch_list::{self, PARTY_DATA_LEN, PATCH_LIST_LEN};
use crate::gen1::{TRADE_BLOCK_LEN, TradeBlock};

/// Décalage des données d'équipe dans le bloc d'échange : la zone que la
/// patch list couvre.
pub(crate) const OFF_PARTY_DATA: usize = 19;

/// Octet neutre : « rien à dire pour l'instant ». C'est lui que la session
/// présente tant qu'une décision manque.
pub(crate) const BLANK: u8 = 0x00;

/// L'octet qu'émet la cartouche qui fournit l'horloge.
pub(crate) const MASTER: u8 = 0x01;

/// La réponse du suiveur, que le module est toujours.
pub(crate) const SLAVE: u8 = 0x02;

/// Marque les frontières de section du transfert.
pub(crate) const PREAMBLE: u8 = 0xFD;

/// Les valeurs qui changent d'une génération à l'autre. La Gen 2 ajoutera sa
/// table sans rien déplacer.
#[derive(Clone, Copy)]
pub(crate) struct LinkBytes {
    pub connected: u8,
    pub trade_centre: u8,
    pub colosseum: u8,
    pub break_link: u8,
    pub select_base: u8,
    pub table_leave: u8,
    pub trade_reject: u8,
    pub trade_accept: u8,
}

/// Les valeurs de première génération.
pub(crate) const GEN1: LinkBytes = LinkBytes {
    connected: 0x60,
    trade_centre: 0xD4,
    colosseum: 0xD5,
    break_link: 0xD6,
    select_base: 0x60,
    table_leave: 0x6F,
    trade_reject: 0x61,
    trade_accept: 0x62,
};

/// Ce que la session présente en réponse à un octet, et ce qu'elle a à dire
/// au passage.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Step {
    /// L'octet à présenter avant le prochain front d'horloge.
    pub outgoing: u8,
    /// Au plus un événement par octet.
    pub effect: Option<Effect>,
}

/// Ce que la session a à signaler à l'application.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Effect {
    /// Le lien est établi, le menu du Cable Club s'affiche.
    LinkEstablished,
    /// L'équipe du partenaire est reçue et lisible par
    /// [`Session::partner_block`].
    PartnerBlockReceived,
    /// Il faut annoncer quel Pokémon le module propose.
    OfferNeeded,
    /// Le joueur a annoncé le sien.
    PartnerOffered {
        /// Sa position dans l'équipe du joueur.
        index: u8,
    },
    /// Il faut accepter ou refuser.
    VerdictNeeded,
    /// Les deux côtés ont accepté : l'échange a lieu.
    TradeAgreed {
        /// La position du Pokémon que le module cède.
        offered: u8,
        /// La position, dans l'équipe du joueur, de celui qu'il reçoit.
        received: u8,
    },
    /// Le joueur a quitté la table et regagné la salle.
    TableLeft,
    /// Le lien est rompu.
    LinkBroken,
}

/// Ce que l'application fournit à une session qui attend.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Decision {
    /// Le Pokémon proposé, par sa position dans l'équipe.
    Offer(u8),
    /// Accepter l'échange annoncé.
    Accept,
    /// Le refuser, et retourner à la sélection.
    Reject,
    /// Quitter la table.
    Leave,
    /// Réarmer la session avec une nouvelle équipe. À fournir entre deux
    /// échanges — après [`Effect::TradeAgreed`] ou [`Effect::TableLeft`] —
    /// jamais pendant le transfert d'un bloc.
    Party(TradeBlock),
}

/// Où en est la session. Voir le tableau des phases dans
/// `docs/superpowers/specs/2026-08-27-gen1-machine-a-etats-design.md`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Phase {
    Negotiating,
    Menu,
    Waiting,
    Preamble,
    Seed,
    Block,
    PatchHeader,
    PatchList,
    Select,
    Verdict,
    Trading,
    Broken,
}

/// Une session d'échange. Environ un kilo-octet, immobile, sans allocation.
pub struct Session {
    pub(crate) phase: Phase,
    pub(crate) bytes: LinkBytes,
    pub(crate) outgoing: [u8; TRADE_BLOCK_LEN],
    pub(crate) outgoing_patch: [u8; PATCH_LIST_LEN],
    pub(crate) incoming: [u8; TRADE_BLOCK_LEN],
    pub(crate) incoming_patch: [u8; PATCH_LIST_LEN],
    pub(crate) partner_ready: bool,
    pub(crate) cursor: u16,
    pub(crate) offer: Option<u8>,
    pub(crate) partner_offer: Option<u8>,
    pub(crate) verdict: Option<bool>,
    pub(crate) partner_verdict: Option<bool>,
    pub(crate) leaving: bool,
    pub(crate) announced: bool,
}

impl Session {
    /// Ouvre une session de première génération, avec l'équipe que le module
    /// présentera au joueur.
    #[must_use]
    pub fn gen1(offered: TradeBlock) -> Self {
        let mut session = Self {
            phase: Phase::Negotiating,
            bytes: GEN1,
            outgoing: [0u8; TRADE_BLOCK_LEN],
            outgoing_patch: [0u8; PATCH_LIST_LEN],
            incoming: [0u8; TRADE_BLOCK_LEN],
            incoming_patch: [0u8; PATCH_LIST_LEN],
            partner_ready: false,
            cursor: 0,
            offer: None,
            partner_offer: None,
            verdict: None,
            partner_verdict: None,
            leaving: false,
            announced: false,
        };
        session.arm(offered);
        session
    }

    /// Consomme un octet et présente le suivant. O(1), sans allocation,
    /// infaillible : un octet inattendu est une transition, jamais une faute.
    pub fn step(&mut self, incoming: u8) -> Step {
        match self.phase {
            Phase::Negotiating => self.step_negotiating(incoming),
            Phase::Menu => self.step_menu(incoming),
            Phase::Waiting => self.step_waiting(incoming),
            Phase::Preamble => self.step_preamble(incoming),
            Phase::Seed => self.step_seed(incoming),
            Phase::Block => self.step_block(incoming),
            Phase::PatchHeader => self.step_patch_header(incoming),
            Phase::PatchList => self.step_patch_list(incoming),
            Phase::Select => self.step_select(incoming),
            Phase::Verdict => self.step_verdict(incoming),
            Phase::Trading => self.step_trading(incoming),
            Phase::Broken => self.step_broken(incoming),
        }
    }

    /// Fournit une décision à une session qui attend.
    pub fn supply(&mut self, decision: Decision) {
        match decision {
            Decision::Offer(index) => self.offer = Some(index.min(LAST_INDEX)),
            Decision::Accept => self.verdict = Some(true),
            Decision::Reject => self.verdict = Some(false),
            Decision::Leave => self.leaving = true,
            Decision::Party(block) => self.arm(block),
        }
    }

    /// L'équipe du partenaire, dès que [`Effect::PartnerBlockReceived`] a été
    /// émis.
    ///
    /// Rend une copie : 415 octets, hors du chemin critique de l'octet.
    #[must_use]
    pub fn partner_block(&self) -> Option<TradeBlock> {
        self.partner_ready
            .then(|| TradeBlock::from_bytes(self.incoming))
    }

    /// Charge l'équipe à présenter : corrige les octets « pas de câble » des
    /// données d'équipe et construit la patch list correspondante.
    fn arm(&mut self, block: TradeBlock) {
        self.outgoing = *block.as_bytes();
        let mut party = [0u8; PARTY_DATA_LEN];
        party.copy_from_slice(&self.outgoing[OFF_PARTY_DATA..OFF_PARTY_DATA + PARTY_DATA_LEN]);
        self.outgoing_patch = patch_list::build(&mut party);
        self.outgoing[OFF_PARTY_DATA..OFF_PARTY_DATA + PARTY_DATA_LEN].copy_from_slice(&party);
    }

    /// Présente un octet sans rien signaler.
    pub(crate) fn plain(&self, outgoing: u8) -> Step {
        Step {
            outgoing,
            effect: None,
        }
    }

    /// Présente un octet et signale un événement.
    pub(crate) fn with(&self, outgoing: u8, effect: Effect) -> Step {
        Step {
            outgoing,
            effect: Some(effect),
        }
    }

    /// Repart d'une négociation : la cartouche a redémarré la sienne.
    pub(crate) fn restart(&mut self) -> Step {
        self.phase = Phase::Negotiating;
        self.cursor = 0;
        self.partner_ready = false;
        self.reset_round();
        self.plain(SLAVE)
    }

    /// Oublie tout ce qui appartenait à l'échange en cours.
    pub(crate) fn reset_round(&mut self) {
        self.offer = None;
        self.partner_offer = None;
        self.verdict = None;
        self.partner_verdict = None;
        self.leaving = false;
        self.announced = false;
    }
}

/// Position maximale dans une équipe.
pub(crate) const LAST_INDEX: u8 = 5;
