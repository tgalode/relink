//! Le cas d'usage de réservation : sortir un Pokémon du pool et le pousser
//! vers le module physique d'un joueur.
//!
//! L'ordre est imposé et n'est pas une préférence de style :
//!
//! 1. [`PoolRepository::get`] d'abord, pour vérifier que cette offre est
//!    bien faite à ce dresseur — [`ReserveError::NotOffered`] sinon, avant
//!    que quoi que ce soit ne parte vers le module ou même vers `claim`. Ce
//!    contrôle est sûr en lecture seule, malgré la fenêtre qui le sépare de
//!    `claim` : [`crate::domain::PoolEntry::reserved_for`] est immuable une
//!    fois l'entrée créée (voir sa documentation), donc rien ne peut le
//!    faire mentir entre la lecture et l'action.
//! 2. [`PoolRepository::claim`] ensuite, avec la [`ReservationId`] émise
//!    **avant** l'appel — c'est ce qui rend possible toute la déduplication
//!    du commit (spec §7.2). `claim` reste seul juge de l'**état** de
//!    l'entrée à cet instant ; le contrôle d'exclusivité ci-dessus ne s'est
//!    prononcé que sur son destinataire, jamais sur sa disponibilité.
//! 3. [`ClaimOutcome::AlreadyTaken`] et [`ClaimOutcome::NotFound`] rendent
//!    l'erreur correspondante sans que rien ne parte vers le module.
//! 4. [`ModuleTransport::push_reservation`] ensuite. Si elle échoue,
//!    l'erreur remonte mais **l'entrée reste réservée** : elle reviendra au
//!    pool par expiration, jamais par une annulation qui ouvrirait une
//!    fenêtre où deux joueurs pourraient tenir le même Pokémon.
//! 5. [`Notifier::entry_claimed`] en dernier, et son échec est **ignoré** :
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
    /// Cette entrée est une offre nominative (spec §7.3) faite à un autre
    /// dresseur — voir [`crate::domain::PoolEntry::is_offered_to`].
    #[error("cette entrée est réservée à quelqu'un d'autre")]
    NotOffered,
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
pub struct Reserve<'pool, R, T, N, C, I> {
    pool: &'pool R,
    transport: &'pool T,
    notifier: &'pool N,
    clock: C,
    ids: I,
    ttl_millis: u64,
}

impl<'pool, R, T, N, C, I> Reserve<'pool, R, T, N, C, I>
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
        pool: &'pool R,
        transport: &'pool T,
        notifier: &'pool N,
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
    /// Ordre imposé : [`PoolRepository::get`] d'abord, pour vérifier
    /// l'exclusivité d'une offre nominative ; puis [`PoolRepository::claim`],
    /// avec l'identifiant de réservation émis avant l'appel ; puis, seulement
    /// si l'entrée a été réclamée, [`ModuleTransport::push_reservation`] ;
    /// enfin [`Notifier::entry_claimed`], dont l'échec est ignoré. Voir la
    /// documentation du module.
    ///
    /// # Erreurs
    ///
    /// [`ReserveError::NotFound`] si l'entrée n'existe pas.
    /// [`ReserveError::NotOffered`] si elle est réservée nommément à un
    /// autre dresseur. [`ReserveError::AlreadyTaken`] si elle est déjà
    /// réservée. Ces trois cas sont tranchés avant que quoi que ce soit ne
    /// parte vers le module.
    ///
    /// [`ReserveError::Port`] si la lecture initiale, le stockage ou le
    /// transport échouent. Les trois ne se valent pas :
    ///
    /// - une panne de la **lecture initiale** laisse l'appelant sans savoir
    ///   dans quel état se trouve l'entrée — libre, déjà réservée par un
    ///   autre, ou nominative — mais elle ne peut pas, elle, avoir réservé
    ///   l'entrée : elle précède toujours `claim` dans le même appel ;
    /// - une panne de **`claim`** ne dit pas si son effet a eu lieu (voir
    ///   « Reprise après échec » sur [`PoolRepository`]) : l'entrée peut être
    ///   réservée sans que l'appelant le sache ;
    /// - une panne du **transport** survient nécessairement après un
    ///   [`ClaimOutcome::Claimed`] déjà acquis : l'entrée est réservée, et
    ///   elle le restera jusqu'à son échéance.
    ///
    /// Dans aucun de ces cas l'appelant ne doit supposer l'entrée libre : voir
    /// la documentation du module.
    pub async fn execute(&self, request: ReserveRequest) -> Result<ReservationId, ReserveError> {
        let entry = self
            .pool
            .get(request.entry)
            .await?
            .ok_or(ReserveError::NotFound)?;

        if !entry.is_offered_to(&request.claimant) {
            return Err(ReserveError::NotOffered);
        }

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
