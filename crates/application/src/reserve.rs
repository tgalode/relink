//! Le cas d'usage de réservation : sortir un Pokémon du pool et le pousser
//! vers le module physique d'un joueur.
//!
//! L'ordre est imposé et n'est pas une préférence de style :
//!
//! 1. [`PoolRepository::claim`] d'abord, avec la [`ReservationId`] émise
//!    **avant** l'appel — c'est ce qui rend possible toute la déduplication
//!    du commit (spec §7.2).
//! 2. [`ClaimOutcome::AlreadyTaken`] et [`ClaimOutcome::NotFound`] rendent
//!    l'erreur correspondante sans que rien ne parte vers le module.
//! 3. [`ModuleTransport::push_reservation`] ensuite. Si elle échoue,
//!    l'erreur remonte mais **l'entrée reste réservée** : elle reviendra au
//!    pool par expiration, jamais par une annulation qui ouvrirait une
//!    fenêtre où deux joueurs pourraient tenir le même Pokémon.
//! 4. [`Notifier::entry_claimed`] en dernier, et son échec est **ignoré** :
//!    prévenir le déposant est accessoire, faire échouer une réservation
//!    valide parce qu'une notification push est tombée serait absurde.

use crate::domain::{EntryId, ReservationId, TrainerId};
use crate::ports::{
    ClaimOutcome, Clock, IdSource, ModuleId, ModuleTransport, Notifier, PoolRepository, PortError,
};

/// Ce qu'une réservation apporte : quelle entrée, vers quel module, pour
/// quel dresseur.
pub struct ReserveRequest {
    /// L'entrée que le joueur veut réserver.
    pub entry: EntryId,
    /// Le module physique vers lequel la pousser.
    pub module: ModuleId,
    /// Le dresseur qui réserve.
    pub claimant: TrainerId,
}

/// Ce qui peut empêcher une réservation d'aboutir.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum ReserveError {
    /// Quelqu'un d'autre a réservé cette entrée en premier.
    #[error("cette entrée est déjà réservée")]
    AlreadyTaken,
    /// Aucune entrée ne porte cet identifiant.
    #[error("cette entrée n'existe pas")]
    NotFound,
    /// Une panne du monde extérieur — voir [`PortError`].
    #[error(transparent)]
    Port(#[from] PortError),
}

/// Le cas d'usage de réservation : réclamer une entrée, puis la pousser vers
/// un module.
///
/// Le stockage, le transport et le notificateur sont détenus par référence :
/// les tests — et plus généralement tout appelant qui doit inspecter leur
/// état après l'appel — gardent la main dessus. L'horloge et la source
/// d'identifiants, jamais inspectées après coup, sont détenues par valeur.
pub struct Reserve<'p, R, T, N, C, I> {
    pool: &'p R,
    transport: &'p T,
    notifier: &'p N,
    clock: C,
    ids: I,
    ttl_millis: u64,
}

impl<'p, R, T, N, C, I> Reserve<'p, R, T, N, C, I>
where
    R: PoolRepository,
    T: ModuleTransport,
    N: Notifier,
    C: Clock,
    I: IdSource,
{
    /// Construit le cas d'usage à partir de ses ports et de la durée de vie
    /// d'une réservation, en millisecondes.
    #[must_use]
    pub fn new(
        pool: &'p R,
        transport: &'p T,
        notifier: &'p N,
        clock: C,
        ids: I,
        ttl_millis: u64,
    ) -> Self {
        Self {
            pool,
            transport,
            notifier,
            clock,
            ids,
            ttl_millis,
        }
    }

    /// Réserve une entrée du pool et la pousse vers le module donné.
    ///
    /// Ordre imposé : [`PoolRepository::claim`] d'abord, avec l'identifiant
    /// de réservation émis avant l'appel ; puis, seulement si l'entrée a été
    /// réclamée, [`ModuleTransport::push_reservation`] ; enfin
    /// [`Notifier::entry_claimed`], dont l'échec est ignoré. Voir la
    /// documentation du module.
    ///
    /// # Erreurs
    ///
    /// [`ReserveError::AlreadyTaken`] si l'entrée est déjà réservée,
    /// [`ReserveError::NotFound`] si elle n'existe pas — dans les deux cas
    /// avant que quoi que ce soit ne parte vers le module.
    /// [`ReserveError::Port`] si le stockage ou le transport échouent ; dans
    /// ce dernier cas, l'entrée reste réservée : voir la documentation du
    /// module.
    pub async fn execute(&self, request: ReserveRequest) -> Result<ReservationId, ReserveError> {
        let now = self.clock.now().await;
        let reservation = self.ids.next_reservation_id().await;
        let expires_at = now.saturating_add_millis(self.ttl_millis);

        match self
            .pool
            .claim(request.entry, reservation, expires_at)
            .await?
        {
            ClaimOutcome::Claimed => {}
            ClaimOutcome::AlreadyTaken => return Err(ReserveError::AlreadyTaken),
            ClaimOutcome::NotFound => return Err(ReserveError::NotFound),
        }

        let entry = self
            .pool
            .get(request.entry)
            .await?
            .expect("une entrée tout juste réclamée existe encore");

        self.transport
            .push_reservation(request.module, reservation, &entry.pokemon)
            .await?;

        // L'échec de la notification est ignoré à dessein : prévenir le
        // déposant est accessoire, la réservation reste valide.
        let _ = self
            .notifier
            .entry_claimed(&entry.provenance.depositor, entry.id)
            .await;

        Ok(reservation)
    }
}
