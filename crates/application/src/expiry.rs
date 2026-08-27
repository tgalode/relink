//! L'expiration des réservations : le seul chemin du service qui rend une
//! entrée au pool (spec §7.2).
//!
//! Sans TTL, un joueur qui ne branche jamais son module gèlerait une entrée
//! du pool indéfiniment. Mais le TTL a une portée volontairement étroite,
//! fixée par [`crate::ports::PoolRepository::expire_due`] : il ne protège
//! **que** les réservations qui ne sont jamais parvenues à un module.
//!
//! Le serveur ne sait pas qu'aucun commit n'a été tenté ; il sait seulement
//! qu'il n'en a pas entendu parler. Un module qui a reçu la réservation, remis
//! le Pokémon à la cartouche, puis perdu le réseau est **indiscernable** d'un
//! module hors ligne qui ne l'a pas encore remis. Rendre l'entrée au pool
//! dans ce cas produirait une duplication — deux cartouches détenant le même
//! Pokémon, de façon permanente et irrattrapable.
//!
//! D'où la règle en deux temps, portée par le champ `delivered` de
//! [`crate::domain::EntryState::Reserved`] :
//!
//! - **Réservée, non remise** (`delivered: false`) — à l'échéance, l'entrée
//!   revient au pool. Rien n'a pu atteindre une cartouche.
//! - **Réservée et remise** (`delivered: true`) — l'entrée ne revient
//!   **jamais** automatiquement. Seul le module peut la trancher, via
//!   [`crate::commit::Commit`]. Un module détruit avant d'avoir parlé laisse
//!   l'entrée bloquée, ce qui relève d'un traitement de litige manuel, hors
//!   périmètre v1.
//!
//! Ce cas d'usage est, comme [`crate::commit`], **délibérément mince** : il
//! lit l'heure et délègue au port. La difficulté n'est pas ici — un domaine
//! qui lirait les réservations échues puis les écrirait une à une serait faux
//! dès qu'un second processus balaierait en parallèle, une réservation
//! pourrait être rendue deux fois. Elle vit tout entière dans le contrat de
//! [`crate::ports::PoolRepository::expire_due`], **la seule opération
//! autorisée à ramener une entrée à [`crate::domain::EntryState::Available`]**
//! — voir l'invariant documenté sur [`crate::ports::PoolRepository`].

use crate::domain::EntryId;
use crate::ports::{Clock, PoolRepository, PortError};

/// Le cas d'usage d'expiration : lit l'heure, délègue au port.
///
/// Voir la documentation du module pour pourquoi ce cas d'usage est
/// délibérément mince, et pourquoi il doit le rester.
///
/// Le stockage est détenu par référence — comme dans [`crate::deposit`],
/// [`crate::reserve`] et [`crate::commit`] — tandis que l'horloge est
/// détenue par valeur.
pub struct ExpireReservations<'pool, R, C> {
    pool: &'pool R,
    clock: C,
}

impl<'pool, R, C> ExpireReservations<'pool, R, C>
where
    R: PoolRepository,
    C: Clock,
{
    /// Construit le cas d'usage à partir du pool et de l'horloge.
    #[must_use]
    pub fn new(pool: &'pool R, clock: C) -> Self {
        Self { pool, clock }
    }

    /// Rend au pool les entrées dont la réservation a expiré, et rend leurs
    /// identifiants.
    ///
    /// Lit l'heure, puis délègue à
    /// [`PoolRepository::expire_due`], qui porte seul la garantie
    /// d'atomicité et la garde sur `delivered` — voir la documentation du
    /// module. N'inclut jamais une entrée commitée, abandonnée, ou remise à
    /// un module.
    ///
    /// # Erreurs
    ///
    /// [`PortError`] si le port échoue.
    pub async fn execute(&self) -> Result<Vec<EntryId>, PortError> {
        let now = self.clock.now().await;
        self.pool.expire_due(now).await
    }
}
