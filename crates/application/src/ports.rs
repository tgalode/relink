//! Les ports du service : ce que le domaine attend du monde extérieur.
//!
//! Aucun n'a d'implémentation dans ce crate, et c'est le principe. Mais un
//! port n'est pas qu'une signature : plusieurs portent des **garanties**
//! d'atomicité sans lesquelles le commit idempotent du domaine serait faux.
//! Ces garanties sont documentées trait par trait et lient tout adaptateur.
//!
//! Tous les traits ci-dessous utilisent `async fn`, ce que clippy signale
//! (`async_fn_in_trait`) parce que la `Future` rendue ne porte aucune borne
//! `Send`. On l'accepte délibérément : la borne n'a de sens que pour un
//! appelant qui envoie la `Future` entre tâches, et ces ports ne sont jamais
//! utilisés à travers un `dyn Trait` mais toujours en paramètre générique
//! d'un cas d'usage. C'est donc l'appelant — pas ce module — qui est en
//! position de décider s'il lui faut `Send`, en l'exigeant lui-même sur son
//! propre générique s'il en a besoin. Désucrer ces signatures en
//! `-> impl Future<Output = _> + Send` figerait ce choix ici et l'imposerait
//! à tout adaptateur, y compris ceux qui n'en ont pas l'usage.
#![allow(async_fn_in_trait)]

use crate::domain::{EntryId, Pokemon, PoolEntry, ReservationId, Timestamp, TrainerId};

/// Une panne du monde extérieur : base injoignable, courtier hors service,
/// service de légalité en erreur.
///
/// Le domaine ne cherche jamais à l'interpréter. Il ne distingue que deux
/// choses : l'opération a eu lieu, ou on ne sait pas.
#[derive(Clone, PartialEq, Eq, Debug, thiserror::Error)]
#[error("le port a échoué : {message}")]
pub struct PortError {
    /// Ce que l'adaptateur a à en dire, pour les journaux.
    pub message: String,
}

impl PortError {
    /// Construit une erreur de port.
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// Identifiant d'un module physique appairé à un compte.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct ModuleId(u128);

impl ModuleId {
    /// Construit un identifiant de module.
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

/// Ce qu'une tentative de réservation a donné.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ClaimOutcome {
    /// L'entrée était disponible, elle est maintenant réservée.
    Claimed,
    /// Quelqu'un d'autre l'a prise en premier.
    AlreadyTaken,
    /// Aucune entrée ne porte cet identifiant.
    NotFound,
}

/// Ce qu'un enregistrement de commit a donné.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CommitOutcome {
    /// Premier enregistrement pour cette réservation.
    Recorded,
    /// Cette réservation avait déjà été commitée. **Ce n'est pas une erreur** :
    /// c'est le cas nominal d'un rejeu de message.
    AlreadyRecorded,
    /// Aucune réservation ne porte cet identifiant, ou elle ne tient plus
    /// d'entrée.
    Unknown,
}

/// L'horloge. Le domaine ne lit jamais l'heure autrement.
pub trait Clock {
    /// L'instant courant.
    async fn now(&self) -> Timestamp;
}

/// La source des identifiants. Le domaine ne tire jamais d'aléa lui-même.
pub trait IdSource {
    /// Un identifiant d'entrée qui n'a jamais été rendu.
    async fn next_entry_id(&self) -> EntryId;
    /// Un identifiant de réservation qui n'a jamais été rendu.
    async fn next_reservation_id(&self) -> ReservationId;
}

/// Le stockage du pool.
///
/// # Garanties exigées de tout adaptateur
///
/// Trois opérations doivent être **atomiques**, c'est-à-dire indivisibles vis-à-vis
/// de tout appel concurrent. Sans elles, deux joueurs peuvent réserver la même
/// entrée, ou un rejeu de message peut commiter deux fois :
///
/// - [`Self::claim`] doit vérifier la disponibilité **et** poser la réservation
///   en une seule opération. Un adaptateur qui lirait puis écrirait
///   séparément serait faux.
/// - [`Self::record_commit`] doit distinguer le premier enregistrement d'un
///   rejeu, en une seule opération, et rendre le même verdict pour toujours.
/// - [`Self::expire_due`] doit réclamer chaque réservation échue une seule
///   fois, même appelée en parallèle depuis plusieurs processus.
///
/// Le domaine ne peut pas compenser leur absence, et les doublures de test de
/// la tâche 4 vérifient qu'il ne le tente pas.
pub trait PoolRepository {
    /// Ajoute une entrée disponible au pool.
    async fn insert(&self, entry: PoolEntry) -> Result<(), PortError>;

    /// L'entrée portant cet identifiant, si elle existe.
    async fn get(&self, id: EntryId) -> Result<Option<PoolEntry>, PortError>;

    /// Les entrées actuellement réservables.
    async fn list_claimable(&self) -> Result<Vec<PoolEntry>, PortError>;

    /// **Atomique.** Réserve l'entrée si et seulement si elle est disponible.
    async fn claim(
        &self,
        id: EntryId,
        reservation: ReservationId,
        expires_at: Timestamp,
    ) -> Result<ClaimOutcome, PortError>;

    /// **Atomique et idempotente.** Enregistre qu'une réservation a été
    /// commitée sur une cartouche.
    async fn record_commit(
        &self,
        reservation: ReservationId,
        at: Timestamp,
    ) -> Result<CommitOutcome, PortError>;

    /// **Atomique et idempotente.** Marque une réservation comme abandonnée :
    /// un commit a été tenté sans qu'on sache s'il a abouti, et l'entrée reste
    /// consommée.
    async fn record_abandon(
        &self,
        reservation: ReservationId,
        at: Timestamp,
    ) -> Result<CommitOutcome, PortError>;

    /// **Atomique et idempotente.** Enregistre qu'un module a accusé réception
    /// d'une réservation. À partir de là, l'entrée ne peut plus expirer.
    ///
    /// L'accusé vient du module, jamais du courtier de messages : qu'un
    /// courtier ait accepté un message ne dit rien de ce que le module en a
    /// fait.
    async fn record_delivery(&self, reservation: ReservationId)
    -> Result<CommitOutcome, PortError>;

    /// **Atomique.** Rend au pool les entrées dont la réservation a expiré à
    /// cet instant, et rend leurs identifiants.
    ///
    /// N'en font **jamais** partie : une entrée dont un commit ou un abandon a
    /// été enregistré, ni une entrée dont un module a accusé réception. Seule
    /// une réservation qui n'est jamais parvenue à un module peut expirer.
    async fn expire_due(&self, now: Timestamp) -> Result<Vec<EntryId>, PortError>;
}

/// Le contrôle de légalité, derrière lequel vivra PKHeX.Core.
pub trait LegalityChecker {
    /// Vrai si ce Pokémon aurait pu être obtenu par le jeu normal.
    async fn is_legal(&self, pokemon: &Pokemon) -> Result<bool, PortError>;
}

/// Le lien vers un module physique.
pub trait ModuleTransport {
    /// Pousse vers un module le Pokémon qu'il devra remettre à la cartouche.
    async fn push_reservation(
        &self,
        module: ModuleId,
        reservation: ReservationId,
        pokemon: &Pokemon,
    ) -> Result<(), PortError>;
}

/// Les notifications vers les joueurs.
pub trait Notifier {
    /// Prévient un déposant que son Pokémon a été pris.
    async fn entry_claimed(&self, depositor: &TrainerId, entry: EntryId) -> Result<(), PortError>;
}
