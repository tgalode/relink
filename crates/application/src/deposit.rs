//! Le cas d'usage de dépôt : faire entrer un Pokémon dans le pool.
//!
//! Un dépôt est un transfert **physique et irréversible** : la cartouche a
//! déjà cédé le Pokémon quand le module en informe le serveur. C'est pourquoi
//! cette opération est idempotente par [`crate::domain::DepositId`] — voir la
//! documentation de [`crate::ports::PoolRepository::insert`] et le §7.4 de la
//! spec — et pourquoi elle ne fait rien avant d'avoir constaté que le
//! Pokémon est acceptable : rien n'est écrit tant qu'il ne l'est pas.

use crate::domain::{DepositId, EntryId, EntryState, Pokemon, PoolEntry, Provenance, TrainerId};
use crate::ports::{Clock, IdSource, LegalityChecker, PoolRepository, PortError};

/// Ce qu'un dépôt apporte : la clé d'idempotence frappée par le module, qui
/// l'a déposé, et le Pokémon lui-même.
pub struct DepositRequest {
    /// La clé d'idempotence du dépôt, frappée par le module qui l'a émise.
    pub deposit: DepositId,
    /// Le dresseur qui l'a déposé.
    pub depositor: TrainerId,
    /// Le Pokémon déposé.
    pub pokemon: Pokemon,
}

/// Ce qui peut empêcher un dépôt d'aboutir.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum DepositError {
    /// Le Pokémon n'aurait pas pu être obtenu par le jeu normal.
    #[error("le Pokémon n'est pas légal")]
    Illegal,
    /// Le Pokémon n'est pas éligible pour la cartouche de destination.
    ///
    /// Cette variante n'est **pas** produite par le dépôt : l'éligibilité
    /// dépend de la cartouche vers laquelle le Pokémon redescendrait, ce que
    /// le dépôt ignore. Elle appartient au cas d'usage de retrait.
    #[error("le Pokémon n'est pas éligible ({0})")]
    Ineligible(usize),
    /// Une offre d'échange direct se désigne elle-même comme destinataire.
    ///
    /// Distincte de [`Self::Illegal`] à dessein : un Pokémon parfaitement
    /// légal offert à soi-même n'est pas un problème de légalité, c'est un
    /// échange qui n'en est pas un — voir
    /// [`crate::pairing::OfferDirectTrade`].
    #[error("une offre directe ne peut pas se cibler elle-même")]
    SelfOffer,
    /// Une panne du monde extérieur — voir [`PortError`].
    #[error(transparent)]
    Port(#[from] PortError),
}

/// Le cas d'usage de dépôt : contrôle de légalité, puis écriture dans le
/// pool.
///
/// Le stockage est détenu par référence — plusieurs cas d'usage partagent
/// en général le même pool — tandis que le contrôle de légalité, l'horloge
/// et la source d'identifiants sont détenus par valeur.
pub struct Deposit<'pool, R, L, C, I> {
    pool: &'pool R,
    legality: L,
    clock: C,
    ids: I,
}

impl<'pool, R, L, C, I> Deposit<'pool, R, L, C, I>
where
    R: PoolRepository,
    L: LegalityChecker,
    C: Clock,
    I: IdSource,
{
    /// Construit le cas d'usage à partir de ses quatre ports.
    #[must_use]
    pub fn new(pool: &'pool R, legality: L, clock: C, ids: I) -> Self {
        Self {
            pool,
            legality,
            clock,
            ids,
        }
    }

    /// Dépose un Pokémon dans le pool.
    ///
    /// Ordre imposé : contrôle de légalité d'abord ; puis lecture de l'heure
    /// et des identifiants ; puis écriture. Rien n'est écrit tant que le
    /// Pokémon n'est pas accepté.
    ///
    /// Idempotent par [`DepositId`] : sur un rejeu, [`PoolRepository::insert`]
    /// rend l'identifiant déjà enregistré sans créer de doublon, et c'est cet
    /// identifiant — pas nécessairement celui frappé via [`IdSource`] — que
    /// cette méthode rend (spec §7.4).
    ///
    /// # Erreurs
    ///
    /// [`DepositError::Illegal`] si le contrôle de légalité rejette le
    /// Pokémon, avant toute écriture. [`DepositError::Port`] si un port
    /// échoue.
    pub async fn execute(&self, request: DepositRequest) -> Result<EntryId, DepositError> {
        self.execute_reserved(request, None).await
    }

    /// Le même chemin que [`Self::execute`], mais avec une réservation de
    /// destinataire optionnelle posée sur l'entrée créée.
    ///
    /// Partagé avec [`crate::pairing::OfferDirectTrade`] : la spec §7.3 fait
    /// de l'échange direct un dépôt ordinaire dont la seule différence est ce
    /// `reserved_for`, jamais un chemin de code séparé. `pub(crate)` : ce
    /// n'est pas une entrée publique du cas d'usage de dépôt, seulement le
    /// point de partage interne au crate.
    pub(crate) async fn execute_reserved(
        &self,
        request: DepositRequest,
        reserved_for: Option<TrainerId>,
    ) -> Result<EntryId, DepositError> {
        if !self.legality.is_legal(&request.pokemon).await? {
            return Err(DepositError::Illegal);
        }

        let deposited_at = self.clock.now().await;
        let id = self.ids.next_entry_id().await;

        let entry = PoolEntry {
            id,
            deposit: request.deposit,
            pokemon: request.pokemon,
            provenance: Provenance {
                depositor: request.depositor,
                deposited_at,
                previous: Vec::new(),
            },
            state: EntryState::Available,
            reserved_for,
        };

        let stored_id = self.pool.insert(entry).await?;
        Ok(stored_id)
    }
}
