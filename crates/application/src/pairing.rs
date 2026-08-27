//! Le cas d'usage d'échange direct : un dépôt et un retrait appariés.
//!
//! Spec §7.3, décision B. L'échange direct **n'est pas un protocole
//! distinct** : un commit à deux cartouches ajouterait le seul mécanisme du
//! projet capable de produire une duplication non arbitrable, pour une
//! simultanéité dont personne n'a besoin. Il se réduit à deux commits
//! indépendants, chacun couvert par l'arbitrage déjà validé du §7.1 : un
//! dépôt ordinaire côté offreur, un retrait ordinaire côté destinataire.
//!
//! Ce module ne porte que la moitié dépôt de cet appariement — [`Deposit`]
//! réservé à un destinataire nommé plutôt qu'ouvert à tous. Le retrait
//! n'a besoin d'aucun code nouveau : le cas d'usage de réservation de
//! [`crate::reserve`] fonctionne déjà sur une entrée dont
//! [`crate::domain::PoolEntry::reserved_for`] est posé, à condition que
//! l'appelant ne présente au destinataire que les entrées dont
//! [`crate::domain::PoolEntry::is_offered_to`] est vrai — une affaire de
//! requête, donc d'adaptateur, hors du domaine.
//!
//! Spec §7.4 : une offre directe est un dépôt, la cartouche y cède le
//! Pokémon exactement de la même façon. Elle porte donc la même clé
//! d'idempotence [`crate::domain::DepositId`], émise par le module et jamais
//! frappée côté serveur — sinon un rejeu recréerait une seconde entrée pour
//! un seul Pokémon physique, réservée au même destinataire, qui la
//! retirerait deux fois.

use crate::deposit::{Deposit, DepositError, DepositRequest};
use crate::domain::{DepositId, EntryId, Pokemon, TrainerId};
use crate::ports::{Clock, IdSource, LegalityChecker, PoolRepository};

/// Ce qu'une offre d'échange direct apporte : la clé d'idempotence frappée
/// par le module, qui offre, le Pokémon lui-même, et à qui il est réservé.
pub struct DirectTradeRequest {
    /// La clé d'idempotence du dépôt, frappée par le module qui l'a émise —
    /// voir [`crate::domain::DepositId`] et la doc de ce module (spec §7.4).
    pub deposit: DepositId,
    /// Le dresseur qui offre le Pokémon.
    pub depositor: TrainerId,
    /// Le Pokémon offert.
    pub pokemon: Pokemon,
    /// Le dresseur à qui cette offre est réservée.
    pub reserved_for: TrainerId,
}

/// Le cas d'usage d'offre directe : un dépôt réservé à un destinataire
/// unique plutôt qu'ouvert à tout le monde.
///
/// Réutilise le chemin de [`Deposit`] en entier — même contrôle de légalité,
/// même horloge, même source d'identifiants, même écriture — et ne s'en
/// distingue que par deux choses : l'entrée créée porte
/// [`crate::domain::PoolEntry::reserved_for`], et l'auto-offre est refusée
/// avant toute autre vérification.
pub struct OfferDirectTrade<'pool, R, L, C, I> {
    deposit: Deposit<'pool, R, L, C, I>,
}

impl<'pool, R, L, C, I> OfferDirectTrade<'pool, R, L, C, I>
where
    R: PoolRepository,
    L: LegalityChecker,
    C: Clock,
    I: IdSource,
{
    /// Construit le cas d'usage à partir de ses quatre ports, les mêmes que
    /// [`Deposit`].
    #[must_use]
    pub fn new(pool: &'pool R, legality: L, clock: C, ids: I) -> Self {
        Self {
            deposit: Deposit::new(pool, legality, clock, ids),
        }
    }

    /// Dépose un Pokémon dans le pool, réservé au destinataire de l'offre.
    ///
    /// Ordre imposé, comme pour [`Deposit::execute`] : l'auto-offre est
    /// rejetée d'abord — une vérification pure, qui n'a besoin d'aucun port
    /// — puis vient le contrôle de légalité, puis l'écriture. Rien n'est
    /// écrit tant que l'offre n'est pas acceptée.
    ///
    /// Idempotent par [`DepositId`], exactement comme [`Deposit::execute`] :
    /// sur un rejeu, l'identifiant déjà enregistré pour cette clé est rendu
    /// sans créer de doublon (spec §7.4).
    ///
    /// # Erreurs
    ///
    /// [`DepositError::SelfOffer`] si `depositor` et `reserved_for`
    /// désignent le même dresseur. [`DepositError::Illegal`] si le contrôle
    /// de légalité rejette le Pokémon. [`DepositError::Port`] si un port
    /// échoue.
    pub async fn execute(&self, request: DirectTradeRequest) -> Result<EntryId, DepositError> {
        if request.depositor == request.reserved_for {
            return Err(DepositError::SelfOffer);
        }

        self.deposit
            .execute_reserved(
                DepositRequest {
                    deposit: request.deposit,
                    depositor: request.depositor,
                    pokemon: request.pokemon,
                },
                Some(request.reserved_for),
            )
            .await
    }
}
