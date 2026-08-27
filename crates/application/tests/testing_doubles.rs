//! Les doublures doivent respecter les contrats des ports, sinon les tests
//! des tâches suivantes ne prouveraient rien.

use pollster::block_on;
use relink_application::domain::{EntryId, ReservationId, Timestamp};
use relink_application::ports::{ClaimOutcome, Clock, CommitOutcome, IdSource, PoolRepository};
use relink_application::testing::{FixedClock, InMemoryPool, SequentialIds};

mod util;
use util::sample_entry;

fn at(ms: u64) -> Timestamp {
    Timestamp::from_millis(ms)
}

#[test]
fn l_horloge_de_test_n_avance_que_quand_on_l_avance() {
    let clock = FixedClock::new(at(100));
    assert_eq!(block_on(clock.now()), at(100));
    assert_eq!(block_on(clock.now()), at(100));
    clock.advance(50);
    assert_eq!(block_on(clock.now()), at(150));
}

#[test]
fn les_identifiants_ne_se_repetent_jamais() {
    let ids = SequentialIds::new();
    let a = block_on(ids.next_entry_id());
    let b = block_on(ids.next_entry_id());
    let r = block_on(ids.next_reservation_id());
    assert_ne!(a, b);
    assert_ne!(r.as_u128(), 0);
}

#[test]
fn une_entree_ne_peut_etre_reservee_qu_une_fois() {
    let pool = InMemoryPool::new();
    let id = EntryId::from_u128(1);
    block_on(pool.insert(sample_entry(id))).expect("insertion");

    let first = block_on(pool.claim(id, ReservationId::from_u128(10), at(1000)));
    let second = block_on(pool.claim(id, ReservationId::from_u128(11), at(1000)));

    assert_eq!(first.expect("premier"), ClaimOutcome::Claimed);
    assert_eq!(second.expect("second"), ClaimOutcome::AlreadyTaken);
}

#[test]
fn reserver_une_entree_inexistante_le_dit() {
    let pool = InMemoryPool::new();
    let outcome = block_on(pool.claim(EntryId::from_u128(99), ReservationId::from_u128(1), at(1)));
    assert_eq!(outcome.expect("appel"), ClaimOutcome::NotFound);
}

#[test]
fn le_commit_est_idempotent() {
    let pool = InMemoryPool::new();
    let id = EntryId::from_u128(1);
    let res = ReservationId::from_u128(10);
    block_on(pool.insert(sample_entry(id))).expect("insertion");
    block_on(pool.claim(id, res, at(1000))).expect("réservation");

    assert_eq!(
        block_on(pool.record_commit(res, at(5))).expect("1"),
        CommitOutcome::Recorded
    );
    assert_eq!(
        block_on(pool.record_commit(res, at(9))).expect("2"),
        CommitOutcome::AlreadyRecorded
    );
}

#[test]
fn une_reservation_commitee_n_expire_jamais() {
    let pool = InMemoryPool::new();
    let id = EntryId::from_u128(1);
    let res = ReservationId::from_u128(10);
    block_on(pool.insert(sample_entry(id))).expect("insertion");
    block_on(pool.claim(id, res, at(1000))).expect("réservation");
    block_on(pool.record_commit(res, at(500))).expect("commit");

    let expired = block_on(pool.expire_due(at(9_999))).expect("expiration");
    assert!(
        expired.is_empty(),
        "une entrée commitée ne doit jamais revenir au pool"
    );
}

#[test]
fn une_reservation_abandonnee_n_expire_jamais_non_plus() {
    let pool = InMemoryPool::new();
    let id = EntryId::from_u128(1);
    let res = ReservationId::from_u128(10);
    block_on(pool.insert(sample_entry(id))).expect("insertion");
    block_on(pool.claim(id, res, at(1000))).expect("réservation");
    block_on(pool.record_abandon(res, at(500))).expect("abandon");

    let expired = block_on(pool.expire_due(at(9_999))).expect("expiration");
    assert!(
        expired.is_empty(),
        "l'arbitrage de la spec §7.1 choisit la perte"
    );
}

#[test]
fn une_reservation_echue_rend_l_entree_une_seule_fois() {
    let pool = InMemoryPool::new();
    let id = EntryId::from_u128(1);
    block_on(pool.insert(sample_entry(id))).expect("insertion");
    block_on(pool.claim(id, ReservationId::from_u128(10), at(1000))).expect("réservation");

    let first = block_on(pool.expire_due(at(1001))).expect("1");
    let second = block_on(pool.expire_due(at(1002))).expect("2");
    assert_eq!(first, vec![id]);
    assert!(
        second.is_empty(),
        "une expiration ne doit être réclamée qu'une fois"
    );
}

#[test]
fn inserer_deux_fois_la_meme_cle_de_depot_ne_cree_qu_une_entree() {
    // Spec §7.4. Sans ce test, la doublure serait écrite sans cette garantie
    // et le manque se découvrirait à la tâche 5, une tâche trop tard.
    let pool = InMemoryPool::new();
    let first = sample_entry(EntryId::from_u128(1));
    let mut second = sample_entry(EntryId::from_u128(2));
    second.deposit = first.deposit;

    let a = block_on(pool.insert(first)).expect("premier");
    let b = block_on(pool.insert(second)).expect("rejeu");

    assert_eq!(a, b, "le rejeu doit rendre l'identifiant déjà enregistré");
    assert_eq!(pool.len(), 1);
}

#[test]
fn deux_cles_de_depot_distinctes_creent_deux_entrees() {
    let pool = InMemoryPool::new();
    block_on(pool.insert(sample_entry(EntryId::from_u128(1)))).expect("premier");
    block_on(pool.insert(sample_entry(EntryId::from_u128(2)))).expect("second");
    assert_eq!(pool.len(), 2);
}

#[test]
fn la_panne_injectee_ne_frappe_qu_une_fois() {
    use relink_application::ports::PortError;
    let pool = InMemoryPool::new();
    pool.fail_next(PortError::new("base injoignable"));
    assert!(block_on(pool.insert(sample_entry(EntryId::from_u128(1)))).is_err());
    assert!(block_on(pool.insert(sample_entry(EntryId::from_u128(2)))).is_ok());
}
