//! Le cas d'usage d'accusé de réception : la porte d'entrée du domaine pour
//! [`crate::ports::PoolRepository::record_delivery`], le verrou du §7.2.
//!
//! C'était le seul port sans cas d'usage pour l'invoquer. Le risque concret
//! d'un adaptateur qui pousserait vers un module via
//! [`crate::ports::ModuleTransport`] sans jamais appeler celui-ci : `delivered`
//! reste faux pour toujours, et une entrée réellement remise à une cartouche
//! redevient réservable à l'échéance de son TTL — exactement la duplication
//! que tout le §7 existe pour empêcher.
//!
//! Comme [`crate::commit::Commit`], ce cas d'usage est **délibérément
//! mince** : il délègue au port, qui porte seul l'atomicité et
//! l'idempotence de [`crate::ports::PoolRepository::record_delivery`] — voir
//! sa documentation.

use crate::domain::ReservationId;
use crate::ports::{CommitOutcome, PoolRepository, PortError};

/// Ce que l'accusé de réception a donné.
///
/// **Une autorisation, pas une trace.** Ce verdict dit au module s'il a le
/// droit de laisser sa cartouche atteindre la phase de confirmation — voir
/// [`Self::authorizes_confirmation`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DeliveryVerdict {
    /// Premier accusé enregistré pour cette réservation.
    Acknowledged,
    /// Cette réservation avait déjà un accusé enregistré — le cas nominal
    /// d'un rejeu de message.
    AlreadyAcknowledged,
    /// Aucune réservation ne porte cet identifiant, ou elle ne tient plus
    /// d'entrée : elle a expiré avant que cet accusé n'arrive (MQTT QoS 1 ne
    /// garantit pas l'ordre).
    Unknown,
}

impl DeliveryVerdict {
    /// Vrai si ce verdict autorise le module à laisser sa cartouche
    /// atteindre la phase de confirmation.
    ///
    /// [`Self::Acknowledged`] et [`Self::AlreadyAcknowledged`] l'autorisent.
    /// [`Self::Unknown`] l'**interdit** : le module doit alors détruire ce
    /// qu'il détient sans rien donner à la cartouche. Le laisser remettre le
    /// Pokémon malgré un `Unknown` reproduirait exactement la duplication
    /// que le TTL existe pour empêcher.
    #[must_use]
    pub const fn authorizes_confirmation(self) -> bool {
        matches!(self, Self::Acknowledged | Self::AlreadyAcknowledged)
    }
}

/// Le cas d'usage d'accusé de réception : délègue au port.
///
/// Voir la documentation du module pour pourquoi ce cas d'usage est
/// délibérément mince, et pourquoi il doit le rester : toute logique de
/// décision qui s'y ajouterait romprait la garantie d'atomicité que porte
/// seul [`crate::ports::PoolRepository::record_delivery`].
///
/// Le stockage est détenu par référence, comme dans les autres cas d'usage
/// de ce crate. Contrairement à [`crate::commit::Commit`], aucune horloge
/// n'est nécessaire : [`crate::ports::PoolRepository::record_delivery`] ne
/// prend pas d'instant, l'accusé de réception n'a pas besoin d'être daté.
pub struct AcknowledgeDelivery<'pool, R> {
    pool: &'pool R,
}

impl<'pool, R> AcknowledgeDelivery<'pool, R>
where
    R: PoolRepository,
{
    /// Construit le cas d'usage à partir du pool.
    #[must_use]
    pub fn new(pool: &'pool R) -> Self {
        Self { pool }
    }

    /// Enregistre qu'un module a accusé réception d'une réservation, et rend
    /// le verdict qui l'autorise — ou non — à laisser sa cartouche atteindre
    /// la confirmation.
    ///
    /// Délègue entièrement à
    /// [`crate::ports::PoolRepository::record_delivery`], qui porte seul la
    /// garantie d'atomicité et d'idempotence — voir la documentation du
    /// module.
    ///
    /// # Erreurs
    ///
    /// [`PortError`] si le port échoue. Une `Err` ne dit pas si
    /// l'enregistrement a eu lieu ; voir « Reprise après échec » sur
    /// [`PoolRepository`] — cet appel peut être rejoué sans risque
    /// supplémentaire.
    pub async fn acknowledge(
        &self,
        reservation: ReservationId,
    ) -> Result<DeliveryVerdict, PortError> {
        let outcome = self.pool.record_delivery(reservation).await?;
        Ok(translate(outcome))
    }
}

/// Traduit un [`CommitOutcome`] du port en [`DeliveryVerdict`] du cas
/// d'usage. Traduction pure, sans écriture : la décision a déjà été prise,
/// atomiquement, par le port.
const fn translate(outcome: CommitOutcome) -> DeliveryVerdict {
    match outcome {
        CommitOutcome::Recorded => DeliveryVerdict::Acknowledged,
        CommitOutcome::AlreadyRecorded => DeliveryVerdict::AlreadyAcknowledged,
        CommitOutcome::Unknown => DeliveryVerdict::Unknown,
    }
}
