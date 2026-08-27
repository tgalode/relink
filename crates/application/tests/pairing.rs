//! L'échange direct : un dépôt et un retrait appariés, rien d'autre.

use pollster::block_on;
use relink_application::domain::Timestamp;
use relink_application::domain::TrainerId;
use relink_application::pairing::{DirectTradeRequest, OfferDirectTrade};
use relink_application::ports::PoolRepository;
use relink_application::testing::{FixedClock, InMemoryPool, SequentialIds, StubLegality};

mod util;
use util::{some_name, some_pokemon};

fn trainer(number: u16) -> TrainerId {
    TrainerId {
        name: some_name(),
        number,
    }
}

fn offer(to: u16) -> DirectTradeRequest {
    DirectTradeRequest {
        // Spec §7.4 : une offre directe est un dépôt, la cartouche y cède le
        // Pokémon de la même façon. Elle porte donc la même clé d'idempotence,
        // émise par le module — jamais frappée côté serveur.
        deposit: relink_application::domain::DepositId::from_u128(88),
        depositor: trainer(1),
        pokemon: some_pokemon(),
        reserved_for: trainer(to),
    }
}

fn use_case(
    pool: &InMemoryPool,
) -> OfferDirectTrade<'_, InMemoryPool, StubLegality, FixedClock, SequentialIds> {
    OfferDirectTrade::new(
        pool,
        StubLegality::accepting(),
        FixedClock::new(Timestamp::from_millis(0)),
        SequentialIds::new(),
    )
}

#[test]
fn une_offre_directe_entre_dans_le_pool_reservee_a_son_destinataire() {
    let pool = InMemoryPool::new();
    let id = block_on(use_case(&pool).execute(offer(2))).expect("offre");

    let stored = block_on(pool.get(id)).expect("lecture").expect("présente");
    assert!(
        stored.is_claimable(),
        "elle reste prenable, mais par une seule personne"
    );
    assert!(stored.is_offered_to(&trainer(2)));
    assert!(!stored.is_offered_to(&trainer(3)));
}

#[test]
fn un_depot_ordinaire_est_offert_a_tout_le_monde() {
    use relink_application::deposit::{Deposit, DepositRequest};
    let pool = InMemoryPool::new();
    let uc = Deposit::new(
        &pool,
        StubLegality::accepting(),
        FixedClock::new(Timestamp::from_millis(0)),
        SequentialIds::new(),
    );
    let id = block_on(uc.execute(DepositRequest {
        deposit: relink_application::domain::DepositId::from_u128(89),
        depositor: trainer(1),
        pokemon: some_pokemon(),
    }))
    .expect("dépôt");

    let stored = block_on(pool.get(id)).expect("lecture").expect("présente");
    assert!(stored.is_offered_to(&trainer(2)));
    assert!(stored.is_offered_to(&trainer(3)));
}

#[test]
fn une_offre_directe_n_apparait_pas_dans_le_pool_ouvert_des_autres() {
    let pool = InMemoryPool::new();
    block_on(use_case(&pool).execute(offer(2))).expect("offre");

    let visible: Vec<_> = block_on(pool.list_claimable())
        .expect("liste")
        .into_iter()
        .filter(|e| e.is_offered_to(&trainer(3)))
        .collect();
    assert!(visible.is_empty());
}

#[test]
fn un_pokemon_illegal_est_refuse_meme_en_offre_directe() {
    use relink_application::deposit::DepositError;
    let pool = InMemoryPool::new();
    let uc = OfferDirectTrade::new(
        &pool,
        StubLegality::rejecting(),
        FixedClock::new(Timestamp::from_millis(0)),
        SequentialIds::new(),
    );
    assert_eq!(block_on(uc.execute(offer(2))), Err(DepositError::Illegal));
    assert_eq!(pool.len(), 0);
}

#[test]
fn on_ne_peut_pas_s_offrir_un_pokemon_a_soi_meme() {
    use relink_application::deposit::DepositError;
    let pool = InMemoryPool::new();
    let request = DirectTradeRequest {
        deposit: relink_application::domain::DepositId::from_u128(90),
        depositor: trainer(1),
        pokemon: some_pokemon(),
        reserved_for: trainer(1),
    };
    assert_eq!(
        block_on(use_case(&pool).execute(request)),
        Err(DepositError::SelfOffer),
        "un échange avec soi-même n'en est pas un"
    );
    assert_eq!(pool.len(), 0);
}

#[test]
fn rejouer_la_meme_offre_directe_ne_cree_pas_de_doublon() {
    // Spec §7.4, porte d'à côté du dépôt ordinaire : une offre directe est
    // un dépôt, la cartouche y cède le Pokémon de la même façon, et la clé
    // d'idempotence traverse le même chemin. Elle paraît couverte par
    // `rejouer_le_meme_depot_ne_cree_pas_de_doublon` (tâche 5) alors qu'elle
    // ne l'est que si `execute_reserved` passe bien la clé du module, et pas
    // une nouvelle frappée en interne.
    let pool = InMemoryPool::new();
    let uc = use_case(&pool);

    let first = block_on(uc.execute(offer(2))).expect("première offre");
    let replay = block_on(uc.execute(offer(2))).expect("rejeu");

    assert_eq!(first, replay, "le rejeu doit rendre la même entrée");
    assert_eq!(pool.len(), 1, "un seul Pokémon physique, une seule entrée");

    let stored = block_on(pool.get(first))
        .expect("lecture")
        .expect("présente");
    assert!(
        stored.is_offered_to(&trainer(2)),
        "le rejeu ne doit pas perdre la réservation d'origine"
    );
}
