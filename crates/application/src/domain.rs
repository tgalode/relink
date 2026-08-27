//! Les types que le service manipule, indépendants de tout stockage.

use relink_protocol::gen1::{self, PARTY_POKEMON_LEN};

/// Un instant, en millisecondes depuis l'époque Unix.
///
/// Le domaine ne lit jamais l'horloge système : le temps entre par le port
/// [`crate::ports::Clock`]. C'est ce qui rend les scénarios reproductibles.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct Timestamp(u64);

impl Timestamp {
    /// Construit un instant à partir de millisecondes depuis l'époque Unix.
    #[must_use]
    pub const fn from_millis(millis: u64) -> Self {
        Self(millis)
    }

    /// Les millisecondes depuis l'époque Unix.
    #[must_use]
    pub const fn as_millis(self) -> u64 {
        self.0
    }

    /// Avance l'instant, en saturant plutôt qu'en débordant.
    #[must_use]
    pub const fn saturating_add_millis(self, millis: u64) -> Self {
        Self(self.0.saturating_add(millis))
    }
}

/// Le dresseur tel que la cartouche le connaît : son nom et son identifiant.
///
/// C'est la seule identité que le domaine manipule. Les comptes utilisateurs
/// sont un problème d'adaptateur et ne remontent pas jusqu'ici.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TrainerId {
    /// Nom du dresseur, tel qu'il est stocké sur la cartouche.
    pub name: gen1::Name,
    /// Identifiant de dresseur.
    pub number: u16,
}

/// Identifiant d'une entrée du pool.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, PartialOrd, Ord)]
pub struct EntryId(u128);

impl EntryId {
    /// Construit un identifiant.
    #[must_use]
    pub const fn from_u128(value: u128) -> Self {
        Self(value)
    }
    /// La valeur sous-jacente.
    #[must_use]
    pub const fn as_u128(self) -> u128 {
        self.0
    }
}

/// Identifiant d'une réservation.
///
/// Émis **avant** que quoi que ce soit ne parte vers le module : c'est la clé
/// sur laquelle repose toute la déduplication du commit.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, PartialOrd, Ord)]
pub struct ReservationId(u128);

impl ReservationId {
    /// Construit un identifiant.
    #[must_use]
    pub const fn from_u128(value: u128) -> Self {
        Self(value)
    }
    /// La valeur sous-jacente.
    #[must_use]
    pub const fn as_u128(self) -> u128 {
        self.0
    }
}

/// D'où vient un Pokémon et par quelles mains il est passé.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Provenance {
    /// Le dresseur qui l'a déposé dans le pool.
    pub depositor: TrainerId,
    /// Quand il a été déposé.
    pub deposited_at: Timestamp,
    /// Les dresseurs qui l'ont possédé avant, du plus ancien au plus récent.
    pub previous: Vec<TrainerId>,
}

/// Un Pokémon tel que le pool le conserve.
///
/// Les octets sont gardés à l'identique, comme dans `relink-protocol` : on
/// stocke ce que la cartouche a envoyé, jamais une reconstruction.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Pokemon {
    /// Les octets bruts du Pokémon d'équipe.
    pub bytes: [u8; PARTY_POKEMON_LEN],
    /// Son surnom, tel que stocké sur la cartouche d'origine.
    pub nickname: gen1::Name,
    /// Le nom de son dresseur d'origine.
    pub original_trainer: gen1::Name,
}

/// Où en est une entrée du pool.
///
/// Une entrée quitte l'état [`EntryState::Available`] à la **réservation**, pas
/// au commit : sinon deux joueurs pourraient réserver la même. Elle n'y revient
/// que par expiration, et jamais après qu'un commit a été tenté.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EntryState {
    /// Dans le pool, personne ne l'a réservée.
    Available,
    /// Réservée, en attente d'être remise à une cartouche.
    Reserved {
        /// La réservation qui la tient.
        reservation: ReservationId,
        /// Au-delà de cet instant, l'entrée retourne au pool — **mais seulement
        /// si aucun module n'en a accusé réception**.
        expires_at: Timestamp,
        /// Un module a-t-il accusé réception de cette réservation ?
        ///
        /// Tant que c'est faux, rien n'a pu atteindre une cartouche et
        /// l'expiration est sûre. Dès que c'est vrai, l'entrée ne revient plus
        /// jamais au pool toute seule : un module hors ligne ayant déjà remis
        /// le Pokémon est indiscernable d'un module qui n'a rien reçu.
        delivered: bool,
    },
    /// Remise à une cartouche, confirmée par le module.
    Committed {
        /// La réservation qui l'a consommée.
        reservation: ReservationId,
        /// Quand le commit a été enregistré.
        at: Timestamp,
    },
    /// Un commit a été tenté sans qu'on sache s'il a abouti.
    ///
    /// L'entrée reste consommée : c'est l'arbitrage de la spec §7.1, où l'on
    /// choisit la perte plutôt que le risque de duplication. Un traitement de
    /// litige manuel, hors périmètre, s'appuiera sur cet état.
    Abandoned {
        /// La réservation concernée.
        reservation: ReservationId,
        /// Quand l'ambiguïté a été constatée.
        at: Timestamp,
    },
}

impl EntryState {
    /// La réservation qui gouverne cet état, s'il y en a une.
    #[must_use]
    pub const fn reservation(&self) -> Option<ReservationId> {
        match self {
            Self::Available => None,
            Self::Reserved { reservation, .. }
            | Self::Committed { reservation, .. }
            | Self::Abandoned { reservation, .. } => Some(*reservation),
        }
    }
}

/// Un Pokémon déposé dans le pool, avec son état et sa provenance.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PoolEntry {
    /// Son identifiant.
    pub id: EntryId,
    /// Le Pokémon lui-même.
    pub pokemon: Pokemon,
    /// D'où il vient.
    pub provenance: Provenance,
    /// Où il en est.
    pub state: EntryState,
}

impl PoolEntry {
    /// Vrai si cette entrée peut encore être réservée par quelqu'un.
    #[must_use]
    pub const fn is_claimable(&self) -> bool {
        matches!(self.state, EntryState::Available)
    }
}
