//! Le cas d'usage de dépôt.

use pollster::block_on;
use relink_application::deposit::{Deposit, DepositError, DepositRequest};
use relink_application::domain::Timestamp;
use relink_application::ports::PoolRepository;
use relink_application::testing::{FixedClock, InMemoryPool, SequentialIds, StubLegality};

mod util;
use util::{some_name, some_pokemon};

fn request() -> DepositRequest {
    use relink_application::domain::TrainerId;
    DepositRequest {
        deposit: relink_application::domain::DepositId::from_u128(77),
        depositor: TrainerId {
            name: some_name(),
            number: 1234,
        },
        pokemon: some_pokemon(),
    }
}

#[test]
fn un_depot_valide_entre_dans_le_pool() {
    let pool = InMemoryPool::new();
    let uc = Deposit::new(
        &pool,
        StubLegality::accepting(),
        FixedClock::new(Timestamp::from_millis(7)),
        SequentialIds::new(),
    );

    let id = block_on(uc.execute(request())).expect("dépôt accepté");

    let stored = block_on(pool.get(id)).expect("lecture").expect("présente");
    assert!(stored.is_claimable());
    assert_eq!(stored.provenance.deposited_at, Timestamp::from_millis(7));
    assert_eq!(stored.provenance.depositor.number, 1234);
    assert_eq!(stored.pokemon.bytes, some_pokemon().bytes);
}

#[test]
fn un_pokemon_illegal_est_refuse_et_n_entre_pas() {
    let pool = InMemoryPool::new();
    let uc = Deposit::new(
        &pool,
        StubLegality::rejecting(),
        FixedClock::new(Timestamp::from_millis(0)),
        SequentialIds::new(),
    );

    assert_eq!(block_on(uc.execute(request())), Err(DepositError::Illegal));
    assert_eq!(pool.len(), 0, "rien ne doit entrer dans le pool");
}

#[test]
fn une_panne_du_stockage_remonte_sans_perdre_le_pokemon() {
    use relink_application::ports::PortError;
    let pool = InMemoryPool::new();
    pool.fail_next(PortError::new("base injoignable"));
    let uc = Deposit::new(
        &pool,
        StubLegality::accepting(),
        FixedClock::new(Timestamp::from_millis(0)),
        SequentialIds::new(),
    );

    match block_on(uc.execute(request())) {
        Err(DepositError::Port(_)) => {}
        other => panic!("attendu une erreur de port, obtenu {other:?}"),
    }
    assert_eq!(pool.len(), 0);
}

#[test]
fn rejouer_le_meme_depot_ne_cree_pas_de_doublon() {
    // Spec §7.4 : l'acquittement s'est perdu, le module rejoue son journal.
    // La cartouche n'a cédé qu'un seul Pokémon ; il ne doit y en avoir qu'un.
    let pool = InMemoryPool::new();
    let uc = Deposit::new(
        &pool,
        StubLegality::accepting(),
        FixedClock::new(Timestamp::from_millis(0)),
        SequentialIds::new(),
    );

    let first = block_on(uc.execute(request())).expect("premier");
    let replay = block_on(uc.execute(request())).expect("rejeu");

    assert_eq!(first, replay, "le rejeu doit rendre la même entrée");
    assert_eq!(pool.len(), 1, "un seul Pokémon physique, une seule entrée");
}

#[test]
fn deux_depots_recoivent_des_identifiants_distincts() {
    let pool = InMemoryPool::new();
    let uc = Deposit::new(
        &pool,
        StubLegality::accepting(),
        FixedClock::new(Timestamp::from_millis(0)),
        SequentialIds::new(),
    );

    let mut second = request();
    second.deposit = relink_application::domain::DepositId::from_u128(78);
    let a = block_on(uc.execute(request())).expect("premier");
    let b = block_on(uc.execute(second)).expect("second");
    assert_ne!(a, b);
    assert_eq!(pool.len(), 2);
}

#[test]
fn la_legalite_est_verifiee_avant_toute_ecriture() {
    // Le stockage échouerait s'il était touché ; il ne doit pas l'être.
    use relink_application::ports::PortError;
    let pool = InMemoryPool::new();
    pool.fail_next(PortError::new("ne devrait jamais être appelée"));
    let uc = Deposit::new(
        &pool,
        StubLegality::rejecting(),
        FixedClock::new(Timestamp::from_millis(0)),
        SequentialIds::new(),
    );

    assert_eq!(block_on(uc.execute(request())), Err(DepositError::Illegal));
}
