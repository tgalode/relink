//! L'expiration des réservations : le seul chemin qui rend une entrée au pool.

use pollster::block_on;
use relink_application::commit::Commit;
use relink_application::domain::{EntryId, ReservationId, Timestamp};
use relink_application::expiry::ExpireReservations;
use relink_application::ports::PoolRepository;
use relink_application::testing::{FixedClock, InMemoryPool};

mod util;
use util::sample_entry;

fn at(ms: u64) -> Timestamp {
    Timestamp::from_millis(ms)
}

#[test]
fn une_reservation_echue_rend_l_entree_au_pool() {
    let pool = InMemoryPool::new();
    let id = EntryId::from_u128(1);
    block_on(pool.insert(sample_entry(id))).expect("insertion");
    block_on(pool.claim(id, ReservationId::from_u128(10), at(1_000))).expect("réservation");

    let clock = FixedClock::new(at(1_001));
    let released = block_on(ExpireReservations::new(&pool, clock).run()).expect("expiration");

    assert_eq!(released, vec![id]);
    assert!(
        block_on(pool.get(id))
            .expect("lecture")
            .expect("présente")
            .is_claimable()
    );
}

#[test]
fn une_reservation_encore_valide_n_est_pas_touchee() {
    let pool = InMemoryPool::new();
    let id = EntryId::from_u128(1);
    block_on(pool.insert(sample_entry(id))).expect("insertion");
    block_on(pool.claim(id, ReservationId::from_u128(10), at(1_000))).expect("réservation");

    let released = block_on(ExpireReservations::new(&pool, FixedClock::new(at(999))).run())
        .expect("expiration");

    assert!(released.is_empty());
    assert!(
        !block_on(pool.get(id))
            .expect("lecture")
            .expect("présente")
            .is_claimable()
    );
}

#[test]
fn l_echeance_exacte_n_expire_pas_encore() {
    let pool = InMemoryPool::new();
    let id = EntryId::from_u128(1);
    block_on(pool.insert(sample_entry(id))).expect("insertion");
    block_on(pool.claim(id, ReservationId::from_u128(10), at(1_000))).expect("réservation");

    // La convention du contrat de `expire_due` est `expires_at <= now`.
    let released = block_on(ExpireReservations::new(&pool, FixedClock::new(at(1_000))).run())
        .expect("expiration");
    assert_eq!(released, vec![id], "à l'échéance exacte, l'entrée revient");
}

#[test]
fn une_entree_commitee_n_expire_jamais_meme_tres_tard() {
    let pool = InMemoryPool::new();
    let id = EntryId::from_u128(1);
    let res = ReservationId::from_u128(10);
    block_on(pool.insert(sample_entry(id))).expect("insertion");
    block_on(pool.claim(id, res, at(1_000))).expect("réservation");
    block_on(Commit::new(&pool, FixedClock::new(at(500))).confirm(res)).expect("commit");

    let released = block_on(ExpireReservations::new(&pool, FixedClock::new(at(u64::MAX))).run())
        .expect("expiration");
    assert!(released.is_empty());
}

#[test]
fn une_entree_abandonnee_n_expire_jamais_non_plus() {
    let pool = InMemoryPool::new();
    let id = EntryId::from_u128(1);
    let res = ReservationId::from_u128(10);
    block_on(pool.insert(sample_entry(id))).expect("insertion");
    block_on(pool.claim(id, res, at(1_000))).expect("réservation");
    block_on(Commit::new(&pool, FixedClock::new(at(500))).abandon(res)).expect("abandon");

    let released = block_on(ExpireReservations::new(&pool, FixedClock::new(at(u64::MAX))).run())
        .expect("expiration");
    assert!(
        released.is_empty(),
        "on choisit la perte : elle ne revient jamais"
    );
}

#[test]
fn une_entree_remise_a_un_module_n_expire_jamais() {
    // Le scénario qui a fait corriger la spec §7.2 : le module a accusé
    // réception, puis a disparu. Il a peut-être déjà remis le Pokémon à la
    // cartouche — on ne le saura jamais. Rendre l'entrée au pool serait une
    // duplication.
    let pool = InMemoryPool::new();
    let id = EntryId::from_u128(1);
    let res = ReservationId::from_u128(10);
    block_on(pool.insert(sample_entry(id))).expect("insertion");
    block_on(pool.claim(id, res, at(1_000))).expect("réservation");
    block_on(pool.record_delivery(res)).expect("accusé de réception");

    let released = block_on(ExpireReservations::new(&pool, FixedClock::new(at(u64::MAX))).run())
        .expect("expiration");
    assert!(
        released.is_empty(),
        "on choisit la perte plutôt que la duplication"
    );
    assert!(
        !block_on(pool.get(id))
            .expect("lecture")
            .expect("présente")
            .is_claimable()
    );
}

#[test]
fn une_entree_jamais_parvenue_a_un_module_expire_normalement() {
    let pool = InMemoryPool::new();
    let id = EntryId::from_u128(1);
    block_on(pool.insert(sample_entry(id))).expect("insertion");
    block_on(pool.claim(id, ReservationId::from_u128(10), at(1_000))).expect("réservation");
    // Pas d'accusé de réception : rien n'a jamais atteint de cartouche.

    let released = block_on(ExpireReservations::new(&pool, FixedClock::new(at(1_001))).run())
        .expect("expiration");
    assert_eq!(released, vec![id]);
}

#[test]
fn une_entree_rendue_peut_etre_reservee_a_nouveau() {
    let pool = InMemoryPool::new();
    let id = EntryId::from_u128(1);
    block_on(pool.insert(sample_entry(id))).expect("insertion");
    block_on(pool.claim(id, ReservationId::from_u128(10), at(1_000))).expect("première");
    block_on(ExpireReservations::new(&pool, FixedClock::new(at(2_000))).run()).expect("expiration");

    let outcome =
        block_on(pool.claim(id, ReservationId::from_u128(11), at(3_000))).expect("seconde");
    assert_eq!(outcome, relink_application::ports::ClaimOutcome::Claimed);
}
