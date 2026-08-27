//! Le cas d'usage de commit : trancher une réservation, une fois pour
//! toutes.
//!
//! C'est l'endroit le plus dangereux du service où l'on peut détruire des
//! données irremplaçables (spec §7) — pas le seul : au dépôt aussi la
//! cartouche perd le Pokémon tout aussi irréversiblement (spec §7.4). Un
//! échange se conclut **physiquement** sur la cartouche : une fois
//! l'animation passée, le Pokémon vit dans une
//! sauvegarde vieille de trente ans, et il n'y a pas de rollback. Un module
//! rejoue son journal d'intention à la reconnexion, et MQTT QoS 1 peut
//! livrer un message plusieurs fois : le même message reçu dix fois doit
//! produire exactement le même effet qu'une fois.
//!
//! Ce cas d'usage est **délibérément mince** — il lit l'heure et délègue au
//! port — et ce n'est pas un manque d'ambition. C'est le **seul endroit où
//! la garantie d'idempotence peut vivre**. Un domaine qui lirait l'état
//! d'une réservation puis l'écrirait serait faux dès qu'un second processus
//! tournerait en parallèle : la vérification et l'écriture doivent être une
//! seule opération indivisible, ce qu'un cas d'usage ne peut pas offrir
//! depuis l'extérieur du stockage. C'est pourquoi
//! [`crate::ports::PoolRepository::record_commit`] et
//! [`crate::ports::PoolRepository::record_abandon`] portent
//! **contractuellement** l'atomicité et l'idempotence — voir « Trancher une
//! réservation, une fois pour toutes » sur [`crate::ports::PoolRepository`].
//! Un futur relecteur ne doit pas être tenté d'« enrichir » ce module : toute
//! logique de décision qui s'y ajouterait romprait la garantie qu'il existe
//! pour préserver.
//!
//! # L'interdit absolu
//!
//! **Aucun chemin de ce module ne rend une entrée au pool.** Seule
//! [`crate::ports::PoolRepository::expire_due`] le peut, et seulement pour
//! une réservation jamais parvenue à un module (spec §7.2) — ce que ce cas
//! d'usage ne traite pas. [`Commit::confirm`] et [`Commit::abandon`]
//! délèguent l'un à [`crate::ports::PoolRepository::record_commit`], l'autre
//! à [`crate::ports::PoolRepository::record_abandon`], et ces deux
//! opérations ne connaissent que trois issues :
//! [`crate::ports::CommitOutcome::Recorded`],
//! [`crate::ports::CommitOutcome::AlreadyRecorded`] et
//! [`crate::ports::CommitOutcome::Unknown`] — aucune ne repose sur
//! [`crate::domain::EntryState::Available`].
//!
//! # En cas d'ambiguïté, on choisit la perte
//!
//! [`Commit::abandon`] existe pour le cas où un commit a été tenté sans
//! qu'on sache s'il a abouti (module détruit, flash perdue en plein
//! échange — spec §7.1). Il consomme l'entrée exactement comme
//! [`Commit::confirm`] : la seule différence entre
//! [`crate::domain::EntryState::Committed`] et
//! [`crate::domain::EntryState::Abandoned`] est la trace qu'elle laisse pour
//! le traitement de litige manuel, hors périmètre v1. Aucun des deux ne
//! rend l'entrée disponible.

use crate::domain::ReservationId;
use crate::ports::{Clock, CommitOutcome, PoolRepository, PortError};

/// Ce que la tentative de trancher une réservation a donné.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CommitVerdict {
    /// Premier enregistrement pour cette réservation : l'appel l'a
    /// effectivement tranchée.
    Applied,
    /// Cette réservation était déjà tranchée — par un commit, ou par un
    /// abandon — et cet appel n'a rien modifié. C'est le cas nominal d'un
    /// rejeu de message.
    AlreadySettled,
    /// Aucune réservation ne porte cet identifiant, ou elle ne tient plus
    /// d'entrée.
    Unknown,
}

/// Le cas d'usage de commit : lit l'heure, délègue au port.
///
/// Voir la documentation du module pour pourquoi ce cas d'usage est
/// délibérément mince, et pourquoi il doit le rester.
///
/// Le stockage est détenu par référence — comme dans [`crate::deposit`] et
/// [`crate::reserve`], plusieurs cas d'usage partagent en général le même
/// pool — tandis que l'horloge est détenue par valeur.
pub struct Commit<'pool, R, C> {
    pool: &'pool R,
    clock: C,
}

impl<'pool, R, C> Commit<'pool, R, C>
where
    R: PoolRepository,
    C: Clock,
{
    /// Construit le cas d'usage à partir du pool et de l'horloge.
    #[must_use]
    pub fn new(pool: &'pool R, clock: C) -> Self {
        Self { pool, clock }
    }

    /// Confirme qu'une réservation a été commitée sur une cartouche.
    ///
    /// Lit l'heure, puis délègue à
    /// [`PoolRepository::record_commit`], qui porte seul la garantie
    /// d'atomicité et d'idempotence — voir la documentation du module.
    ///
    /// # Erreurs
    ///
    /// [`PortError`] si le port échoue. Une `Err` ne dit pas si
    /// l'enregistrement a eu lieu ; voir « Reprise après échec » sur
    /// [`PoolRepository`] — cet appel peut être rejoué sans risque
    /// supplémentaire.
    pub async fn confirm(&self, reservation: ReservationId) -> Result<CommitVerdict, PortError> {
        let at = self.clock.now().await;
        let outcome = self.pool.record_commit(reservation, at).await?;
        Ok(settle(outcome))
    }

    /// Signale qu'un commit a été tenté sans qu'on sache s'il a abouti :
    /// l'entrée reste consommée.
    ///
    /// Lit l'heure, puis délègue à
    /// [`PoolRepository::record_abandon`], qui porte seul la garantie
    /// d'atomicité et d'idempotence — voir la documentation du module.
    /// **N'ouvre aucun chemin vers [`crate::domain::EntryState::Available`]** :
    /// c'est l'arbitrage de la spec §7.1, on choisit la perte plutôt que le
    /// risque de duplication.
    ///
    /// # Erreurs
    ///
    /// [`PortError`] si le port échoue. Une `Err` ne dit pas si
    /// l'enregistrement a eu lieu ; voir « Reprise après échec » sur
    /// [`PoolRepository`] — cet appel peut être rejoué sans risque
    /// supplémentaire.
    pub async fn abandon(&self, reservation: ReservationId) -> Result<CommitVerdict, PortError> {
        let at = self.clock.now().await;
        let outcome = self.pool.record_abandon(reservation, at).await?;
        Ok(settle(outcome))
    }
}

/// Traduit un [`CommitOutcome`] du port en [`CommitVerdict`] du cas d'usage.
/// Traduction pure, sans écriture : la décision a déjà été prise,
/// atomiquement, par le port.
const fn settle(outcome: CommitOutcome) -> CommitVerdict {
    match outcome {
        CommitOutcome::Recorded => CommitVerdict::Applied,
        CommitOutcome::AlreadyRecorded => CommitVerdict::AlreadySettled,
        CommitOutcome::Unknown => CommitVerdict::Unknown,
    }
}
