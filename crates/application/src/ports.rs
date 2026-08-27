//! Les ports du service : ce que le domaine attend du monde extérieur.
//!
//! Aucun n'a d'implémentation dans ce crate, et c'est le principe. Mais un
//! port n'est pas qu'une signature : plusieurs portent des **garanties**
//! d'atomicité sans lesquelles le commit idempotent du domaine serait faux.
//! Ces garanties sont documentées trait par trait et lient tout adaptateur.
//!
//! Tous les traits ci-dessous utilisent `async fn`, ce que clippy signale
//! (`async_fn_in_trait`) parce que la `Future` rendue ne porte aucune borne
//! `Send`. On l'accepte, mais pas parce qu'un appelant générique pourrait
//! l'exiger lui-même sur son propre paramètre : il ne le peut pas. Un
//! paramètre `R: PoolRepository + Send` ne borne que le type `R`, pas la
//! `Future` que rendent ses méthodes — le compilateur le confirme (« future
//! cannot be sent between threads safely ») dès qu'on essaie de `spawn` un tel
//! appel depuis une fonction générique. Le seul mécanisme qui permettrait
//! d'exprimer cette borne, la *return type notation*
//! (`R: PoolRepository<claim(..): Send>`), est encore instable (`E0658` sur
//! stable, y compris sous la MSRV 1.85 de ce projet) : tant qu'elle ne l'est
//! pas, aucune couche générique ne peut exiger `Send` sur ces `Future`, quoi
//! qu'elle fasse.
//!
//! Ce qui rend l'`allow` sûr malgré ça, c'est que ce n'est jamais une couche
//! générique qui a besoin de cette borne : c'est le point d'assemblage, où
//! l'adaptateur est un type **concret** (une structure Postgres, MQTT, etc.)
//! branché sur un exécuteur concret, qui décide si la `Future` traverse des
//! tâches. Sur un type concret, l'analyse d'auto-traits du compilateur
//! détermine `Send` de façon structurelle, à partir de ce que la `Future`
//! capture, sans qu'aucune annotation ne soit nécessaire ni possible : c'est
//! la fuite d'auto-traits (*auto trait leakage*). Un exécuteur multi-thread
//! qui assemble un adaptateur Postgres concret verra donc ses `Future`
//! `Send` sans que ce module ait rien à écrire.
//!
//! Limite à connaître : cela interdit d'intercaler, entre le point
//! d'assemblage et l'adaptateur concret, une couche **générique** — un
//! middleware de retry ou d'instrumentation générique sur `R: PoolRepository`,
//! par exemple — qui aurait elle-même besoin d'exiger `Send` sur ces
//! `Future`. Ce ne sera possible qu'une fois RTN stabilisée.
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

/// Ce qu'un enregistrement de commit, d'abandon, ou de livraison a donné.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CommitOutcome {
    /// Premier enregistrement pour cette réservation.
    Recorded,
    /// Cette réservation était déjà tranchée — un commit, un abandon, ou
    /// (pour [`PoolRepository::record_delivery`]) un accusé de réception,
    /// avait déjà été enregistré. **Ce n'est pas une erreur** : c'est le cas
    /// nominal d'un rejeu de message, et l'opération n'a rien modifié.
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
/// Cinq opérations doivent être **atomiques**, c'est-à-dire indivisibles
/// vis-à-vis de tout appel concurrent. Sans elles, deux joueurs peuvent
/// réserver la même entrée, ou un rejeu de message peut commiter deux fois :
///
/// - [`Self::claim`] doit vérifier la disponibilité **et** poser la réservation
///   en une seule opération. Un adaptateur qui lirait puis écrirait
///   séparément serait faux.
/// - [`Self::record_commit`] et [`Self::record_abandon`] doivent distinguer
///   le premier enregistrement d'un rejeu, en une seule opération, sans
///   jamais changer de verdict une fois la réservation tranchée — voir
///   « Trancher une réservation » plus bas.
/// - [`Self::record_delivery`] doit constater atomiquement si la réservation
///   tient encore une entrée, et ne rien modifier si ce n'est pas le cas.
/// - [`Self::expire_due`] doit réclamer chaque réservation échue une seule
///   fois, même appelée en parallèle depuis plusieurs processus.
///
/// # Invariant : qui peut rendre une entrée disponible
///
/// **[`Self::expire_due`] est la seule opération autorisée à ramener une
/// entrée à [`crate::domain::EntryState::Available`], et seulement depuis
/// [`crate::domain::EntryState::Reserved`] avec `delivered: false`.** Aucune
/// opération présente ou future ne ramène à `Available` une entrée
/// `Committed`, `Abandoned`, ou `Reserved` avec `delivered: true`. C'est cet
/// invariant qu'une expiration paresseuse évaluée dans [`Self::claim`]
/// violerait — voir sa documentation — et il lie tout adaptateur, y compris
/// ceux qui n'ont pas de balayeur dédié pour [`Self::expire_due`].
///
/// # Trancher une réservation, une fois pour toutes
///
/// [`Self::record_commit`] et [`Self::record_abandon`] tranchent ensemble la
/// même question — que devient une réservation remise à un module — et ne la
/// tranchent qu'**une fois** à eux deux : le premier des deux appels, quel
/// qu'il soit, rend [`CommitOutcome::Recorded`] et fixe l'issue ; tout appel
/// ultérieur de **l'un ou l'autre** rend [`CommitOutcome::AlreadyRecorded`]
/// sans rien modifier. Un adaptateur qui tiendrait deux registres séparés
/// pour le commit et l'abandon écraserait silencieusement `Committed` par
/// `Abandoned` (ou l'inverse) sur un rejeu croisé, détruisant la trace sur
/// laquelle s'appuie le traitement de litige de la spec §7.1.
///
/// # Reprise après échec
///
/// Une `Err` ne dit pas si l'écriture a eu lieu : le domaine ne peut pas
/// distinguer une panne survenue avant l'effet d'une panne survenue après.
/// Toute opération de ce trait doit donc pouvoir être **rejouée telle quelle
/// sans effet supplémentaire**, et ne jamais laisser d'état intermédiaire
/// observable entre deux appels.
///
/// Le domaine ne peut compenser l'absence d'aucune de ces garanties, et les
/// doublures de test de la tâche 4 vérifient qu'il ne le tente pas.
pub trait PoolRepository {
    /// Ajoute une entrée disponible au pool.
    ///
    /// **Idempotente par [`crate::domain::DepositId`].** Un dépôt est un
    /// transfert physique irréversible, et le module le rejoue comme il rejoue
    /// un commit. Si une entrée portant déjà cette clé de dépôt existe, cette
    /// opération **ne crée pas de doublon**, ne modifie rien, et rend
    /// l'identifiant de l'entrée déjà présente. Sans cela, un acquittement
    /// perdu suffit à créer deux entrées réservables pour un seul Pokémon
    /// physique (spec §7.4).
    async fn insert(&self, entry: PoolEntry) -> Result<EntryId, PortError>;

    /// L'entrée portant cet identifiant, si elle existe.
    async fn get(&self, id: EntryId) -> Result<Option<PoolEntry>, PortError>;

    /// Les entrées dont l'état est exactement
    /// [`crate::domain::EntryState::Available`] — les mêmes, et seulement les
    /// mêmes, que [`Self::claim`] accepterait de réserver à cet instant.
    async fn list_claimable(&self) -> Result<Vec<PoolEntry>, PortError>;

    /// **Atomique.** Réserve l'entrée si et seulement si son état est
    /// **exactement** [`crate::domain::EntryState::Available`] au moment de
    /// l'appel, et pose alors `Reserved { reservation, expires_at, delivered:
    /// false }`.
    ///
    /// « Disponible » ne veut rien dire d'autre : une entrée `Reserved` dont
    /// l'échéance est déjà passée n'est **pas** disponible tant que
    /// [`Self::expire_due`] ne l'a pas effectivement rendue au pool — voir
    /// l'invariant de ce trait. Un adaptateur qui évaluerait `expires_at` ici
    /// au lieu de vérifier l'état — par exemple `WHERE state = 'available' OR
    /// (state = 'reserved' AND expires_at <= now())`, la forme la plus
    /// idiomatique en SQL pour une expiration paresseuse — contournerait la
    /// garde de la spec §7.2 sur `delivered` : une réservation que le module a
    /// déjà remise à une cartouche, mais dont le serveur n'a pas encore
    /// entendu parler, redeviendrait réservable, et donnerait deux cartouches
    /// pour un seul Pokémon.
    async fn claim(
        &self,
        id: EntryId,
        reservation: ReservationId,
        expires_at: Timestamp,
    ) -> Result<ClaimOutcome, PortError>;

    /// **Atomique et idempotente.** Enregistre qu'une réservation a été
    /// commitée sur une cartouche.
    ///
    /// Tranche la réservation avec [`Self::record_abandon`] : voir « Trancher
    /// une réservation, une fois pour toutes » sur ce trait. Rend
    /// [`CommitOutcome::Unknown`] si aucune réservation ne porte cet
    /// identifiant.
    async fn record_commit(
        &self,
        reservation: ReservationId,
        at: Timestamp,
    ) -> Result<CommitOutcome, PortError>;

    /// **Atomique et idempotente.** Marque une réservation comme abandonnée :
    /// un commit a été tenté sans qu'on sache s'il a abouti, et l'entrée reste
    /// consommée.
    ///
    /// Tranche la réservation avec [`Self::record_commit`] : voir « Trancher
    /// une réservation, une fois pour toutes » sur ce trait.
    async fn record_abandon(
        &self,
        reservation: ReservationId,
        at: Timestamp,
    ) -> Result<CommitOutcome, PortError>;

    /// **Atomique et idempotente. Une autorisation, pas une trace.**
    /// Enregistre qu'un module a accusé réception d'une réservation, et rend
    /// le verdict qui dit au module s'il a le droit de laisser sa cartouche
    /// atteindre la phase de confirmation.
    ///
    /// Seuls [`CommitOutcome::Recorded`] et [`CommitOutcome::AlreadyRecorded`]
    /// l'autorisent. Sur `Recorded` — le premier accusé pour cette
    /// réservation — l'entrée passe à `delivered: true` (voir le champ
    /// `delivered` de [`crate::domain::EntryState::Reserved`]) et ne peut
    /// plus expirer, conformément à l'invariant de ce trait.
    /// [`CommitOutcome::Unknown`] **interdit** la remise : soit aucune
    /// réservation ne porte cet identifiant, soit elle ne tient plus d'entrée
    /// parce qu'elle a expiré entre-temps — MQTT QoS 1 ne garantit pas
    /// l'ordre, l'accusé peut arriver après l'expiration. Le module doit alors
    /// détruire ce qu'il détient sans rien donner à la cartouche : le laisser
    /// remettre le Pokémon après un `Unknown` reproduit exactement la
    /// duplication que le TTL existe pour empêcher.
    ///
    /// Si la réservation est déjà tranchée ([`Self::record_commit`] ou
    /// [`Self::record_abandon`] a déjà été enregistré — l'ordre MQTT ne
    /// garantit pas non plus que l'accusé précède la confirmation), cette
    /// opération ne modifie rien et rend [`CommitOutcome::AlreadyRecorded`] :
    /// un accusé tardif ne doit jamais dé-commiter une entrée déjà remise.
    ///
    /// Ne touche que l'entrée que cette réservation tient **à cet instant** ;
    /// n'en modifie aucune si elle n'en tient plus.
    ///
    /// L'accusé vient du module, jamais du courtier de messages : qu'un
    /// courtier ait accepté un message ne dit rien de ce que le module en a
    /// fait.
    async fn record_delivery(&self, reservation: ReservationId)
    -> Result<CommitOutcome, PortError>;

    /// **Atomique.** Rend au pool les entrées dont la réservation a expiré à
    /// cet instant — `expires_at <= now`, borne incluse (fixé par la tâche 8)
    /// — et rend leurs identifiants.
    ///
    /// N'en font **jamais** partie : une entrée dont un commit ou un abandon a
    /// été enregistré, ni une entrée dont un module a accusé réception
    /// ([`Self::record_delivery`]). Seule une réservation qui n'est jamais
    /// parvenue à un module peut expirer — voir l'invariant de ce trait.
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
    ///
    /// Une poussée acceptée par le courtier de messages **ne vaut rien** :
    /// avant de laisser sa cartouche atteindre la phase de confirmation, le
    /// module doit obtenir l'autorisation de
    /// [`PoolRepository::record_delivery`]. C'est ce verdict, jamais
    /// l'acceptation du courtier, qui dit si la réservation tient encore
    /// l'entrée (spec §7.2).
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
