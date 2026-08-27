//! Le commit, l'endroit le plus dangereux du service pour détruire des
//! données — pas le seul, voir la spec §7.4 sur le dépôt.

use pollster::block_on;
use relink_application::commit::{Commit, CommitVerdict};
use relink_application::domain::{EntryId, EntryState, ReservationId, Timestamp};
use relink_application::ports::PoolRepository;
use relink_application::testing::{FixedClock, InMemoryPool};

mod util;
use util::sample_entry;

fn reserved_pool(id: EntryId, res: ReservationId) -> InMemoryPool {
    let pool = InMemoryPool::new();
    block_on(pool.insert(sample_entry(id))).expect("insertion");
    block_on(pool.claim(id, res, Timestamp::from_millis(10_000))).expect("réservation");
    pool
}

#[test]
fn confirmer_consomme_l_entree() {
    let (id, res) = (EntryId::from_u128(1), ReservationId::from_u128(10));
    let pool = reserved_pool(id, res);
    let uc = Commit::new(&pool, FixedClock::new(Timestamp::from_millis(50)));

    assert_eq!(
        block_on(uc.confirm(res)).expect("commit"),
        CommitVerdict::Applied
    );
    let stored = block_on(pool.get(id)).expect("lecture").expect("présente");
    assert_eq!(
        stored.state,
        EntryState::Committed {
            reservation: res,
            at: Timestamp::from_millis(50)
        }
    );
}

#[test]
fn rejouer_le_meme_commit_ne_change_rien() {
    let (id, res) = (EntryId::from_u128(1), ReservationId::from_u128(10));
    let pool = reserved_pool(id, res);
    let clock = FixedClock::new(Timestamp::from_millis(50));
    let uc = Commit::new(&pool, clock.clone());

    assert_eq!(
        block_on(uc.confirm(res)).expect("1"),
        CommitVerdict::Applied
    );
    clock.set(Timestamp::from_millis(9_999));
    assert_eq!(
        block_on(uc.confirm(res)).expect("2"),
        CommitVerdict::AlreadySettled
    );
    assert_eq!(
        block_on(uc.confirm(res)).expect("3"),
        CommitVerdict::AlreadySettled
    );

    let stored = block_on(pool.get(id)).expect("lecture").expect("présente");
    assert_eq!(
        stored.state,
        EntryState::Committed {
            reservation: res,
            at: Timestamp::from_millis(50)
        },
        "l'instant du premier commit ne doit pas bouger"
    );
}

#[test]
fn abandonner_laisse_l_entree_consommee() {
    let (id, res) = (EntryId::from_u128(1), ReservationId::from_u128(10));
    let pool = reserved_pool(id, res);
    let uc = Commit::new(&pool, FixedClock::new(Timestamp::from_millis(50)));

    assert_eq!(
        block_on(uc.abandon(res)).expect("abandon"),
        CommitVerdict::Applied
    );
    let stored = block_on(pool.get(id)).expect("lecture").expect("présente");
    assert_eq!(
        stored.state,
        EntryState::Abandoned {
            reservation: res,
            at: Timestamp::from_millis(50)
        }
    );
    assert!(
        !stored.is_claimable(),
        "on choisit la perte, pas la duplication"
    );
}

#[test]
fn on_ne_peut_pas_abandonner_ce_qui_est_deja_confirme() {
    let (id, res) = (EntryId::from_u128(1), ReservationId::from_u128(10));
    let pool = reserved_pool(id, res);
    let uc = Commit::new(&pool, FixedClock::new(Timestamp::from_millis(50)));

    block_on(uc.confirm(res)).expect("commit");
    assert_eq!(
        block_on(uc.abandon(res)).expect("abandon"),
        CommitVerdict::AlreadySettled
    );

    let stored = block_on(pool.get(id)).expect("lecture").expect("présente");
    assert!(
        matches!(stored.state, EntryState::Committed { .. }),
        "une réservation ne se tranche qu'une fois"
    );
}

#[test]
fn on_ne_peut_pas_confirmer_ce_qui_est_deja_abandonne() {
    let (id, res) = (EntryId::from_u128(1), ReservationId::from_u128(10));
    let pool = reserved_pool(id, res);
    let uc = Commit::new(&pool, FixedClock::new(Timestamp::from_millis(50)));

    block_on(uc.abandon(res)).expect("abandon");
    assert_eq!(
        block_on(uc.confirm(res)).expect("commit"),
        CommitVerdict::AlreadySettled
    );
}

#[test]
fn une_reservation_inconnue_le_dit_sans_rien_casser() {
    let pool = InMemoryPool::new();
    let uc = Commit::new(&pool, FixedClock::new(Timestamp::from_millis(0)));
    assert_eq!(
        block_on(uc.confirm(ReservationId::from_u128(404))).expect("appel"),
        CommitVerdict::Unknown
    );
}

#[test]
fn une_entree_commitee_ne_revient_jamais_au_pool() {
    let (id, res) = (EntryId::from_u128(1), ReservationId::from_u128(10));
    let pool = reserved_pool(id, res);
    let uc = Commit::new(&pool, FixedClock::new(Timestamp::from_millis(50)));
    block_on(uc.confirm(res)).expect("commit");

    // Bien après l'échéance de la réservation.
    let expired = block_on(pool.expire_due(Timestamp::from_millis(u64::MAX))).expect("expiration");
    assert!(expired.is_empty());
    assert!(
        !block_on(pool.get(id))
            .expect("lecture")
            .expect("présente")
            .is_claimable()
    );
}
