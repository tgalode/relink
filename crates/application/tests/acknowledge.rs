//! Le cas d'usage d'accusé de réception : le verrou du §7.2.

use pollster::block_on;
use relink_application::acknowledge::{AcknowledgeDelivery, DeliveryVerdict};
use relink_application::domain::{EntryId, ReservationId, Timestamp};
use relink_application::expiry::ExpireReservations;
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
fn le_premier_accuse_autorise_la_confirmation() {
    let (id, res) = (EntryId::from_u128(1), ReservationId::from_u128(10));
    let pool = reserved_pool(id, res);
    let uc = AcknowledgeDelivery::new(&pool);

    let verdict = block_on(uc.acknowledge(res)).expect("accusé");

    assert_eq!(verdict, DeliveryVerdict::Acknowledged);
    assert!(verdict.authorizes_confirmation());
}

#[test]
fn un_accuse_rejoue_reste_une_autorisation() {
    let (id, res) = (EntryId::from_u128(1), ReservationId::from_u128(10));
    let pool = reserved_pool(id, res);
    let uc = AcknowledgeDelivery::new(&pool);

    block_on(uc.acknowledge(res)).expect("premier accusé");
    let verdict = block_on(uc.acknowledge(res)).expect("accusé rejoué");

    assert_eq!(verdict, DeliveryVerdict::AlreadyAcknowledged);
    assert!(
        verdict.authorizes_confirmation(),
        "un rejeu ne doit jamais interdire ce qu'un premier accusé a déjà autorisé"
    );
}

#[test]
fn un_accuse_sur_une_reservation_inconnue_interdit_la_remise() {
    let pool = InMemoryPool::new();
    let uc = AcknowledgeDelivery::new(&pool);

    let verdict = block_on(uc.acknowledge(ReservationId::from_u128(404))).expect("appel");

    assert_eq!(verdict, DeliveryVerdict::Unknown);
    assert!(
        !verdict.authorizes_confirmation(),
        "un module qui ne trouve pas de réservation ne doit jamais remettre le Pokémon"
    );
}

#[test]
fn un_accuse_arrive_apres_expiration_interdit_la_remise() {
    // MQTT QoS 1 ne garantit pas l'ordre : l'accusé peut arriver après que
    // le TTL a rendu l'entrée au pool.
    let (id, res) = (EntryId::from_u128(1), ReservationId::from_u128(10));
    let pool = InMemoryPool::new();
    block_on(pool.insert(sample_entry(id))).expect("insertion");
    block_on(pool.claim(id, res, Timestamp::from_millis(1_000))).expect("réservation");
    block_on(
        ExpireReservations::new(&pool, FixedClock::new(Timestamp::from_millis(2_000))).execute(),
    )
    .expect("expiration");

    let uc = AcknowledgeDelivery::new(&pool);
    let verdict = block_on(uc.acknowledge(res)).expect("accusé tardif");

    assert_eq!(verdict, DeliveryVerdict::Unknown);
    assert!(!verdict.authorizes_confirmation());
}

#[test]
fn un_accuse_reussi_empeche_l_expiration() {
    let (id, res) = (EntryId::from_u128(1), ReservationId::from_u128(10));
    let pool = reserved_pool(id, res);
    let uc = AcknowledgeDelivery::new(&pool);
    block_on(uc.acknowledge(res)).expect("accusé");

    let released = block_on(
        ExpireReservations::new(&pool, FixedClock::new(Timestamp::from_millis(u64::MAX))).execute(),
    )
    .expect("expiration bien après l'échéance");

    assert!(
        released.is_empty(),
        "une entrée remise à un module ne doit jamais revenir au pool"
    );
}
