//! Le cas d'usage de réservation.

use pollster::block_on;
use relink_application::domain::{EntryId, EntryState, Timestamp, TrainerId};
use relink_application::ports::{ModuleId, PoolRepository, PortError};
use relink_application::reserve::{Reserve, ReserveError, ReserveRequest};
use relink_application::testing::{
    FixedClock, InMemoryPool, NotifiedClaim, PushedReservation, RecordingNotifier,
    RecordingTransport, SequentialIds,
};

mod util;
use util::{sample_entry, some_name, some_pokemon};

const TTL: u64 = 3_600_000;

fn request(entry: EntryId) -> ReserveRequest {
    ReserveRequest {
        entry,
        module: ModuleId::from_u128(7),
        claimant: TrainerId {
            name: some_name(),
            number: 999,
        },
    }
}

/// Le dépositaire posé par [`sample_entry`] : c'est lui que le notificateur
/// doit prévenir.
fn sample_depositor() -> TrainerId {
    TrainerId {
        name: some_name(),
        number: 1234,
    }
}

#[test]
fn une_reservation_sort_l_entree_du_pool_et_pousse_vers_le_module() {
    let pool = InMemoryPool::new();
    let transport = RecordingTransport::new();
    let notifier = RecordingNotifier::new();
    let id = EntryId::from_u128(1);
    block_on(pool.insert(sample_entry(id))).expect("insertion");

    let uc = Reserve::new(
        &pool,
        &transport,
        &notifier,
        FixedClock::new(Timestamp::from_millis(1_000)),
        SequentialIds::new(),
        TTL,
    );
    let reservation = block_on(uc.execute(request(id))).expect("réservation");

    let stored = block_on(pool.get(id)).expect("lecture").expect("présente");
    assert!(
        !stored.is_claimable(),
        "l'entrée quitte le pool à la réservation"
    );
    // `delivered` reste faux : pousser vers le courtier n'est pas un accusé de
    // réception du module. Tant qu'il est faux, l'expiration reste sûre.
    assert_eq!(
        stored.state,
        EntryState::Reserved {
            reservation,
            expires_at: Timestamp::from_millis(1_000 + TTL),
            delivered: false,
        }
    );
    assert_eq!(
        transport.pushed(),
        vec![PushedReservation {
            module: ModuleId::from_u128(7),
            reservation,
            pokemon: some_pokemon(),
        }]
    );
}

#[test]
fn le_deposant_est_prevenu_que_son_pokemon_a_ete_pris() {
    let pool = InMemoryPool::new();
    let transport = RecordingTransport::new();
    let notifier = RecordingNotifier::new();
    let id = EntryId::from_u128(1);
    block_on(pool.insert(sample_entry(id))).expect("insertion");

    let uc = Reserve::new(
        &pool,
        &transport,
        &notifier,
        FixedClock::new(Timestamp::from_millis(0)),
        SequentialIds::new(),
        TTL,
    );
    block_on(uc.execute(request(id))).expect("réservation");

    assert_eq!(
        notifier.notified(),
        vec![NotifiedClaim {
            depositor: sample_depositor(),
            entry: id,
        }]
    );
}

#[test]
fn deux_joueurs_ne_peuvent_pas_reserver_la_meme_entree() {
    let pool = InMemoryPool::new();
    let transport = RecordingTransport::new();
    let notifier = RecordingNotifier::new();
    let id = EntryId::from_u128(1);
    block_on(pool.insert(sample_entry(id))).expect("insertion");

    let uc = Reserve::new(
        &pool,
        &transport,
        &notifier,
        FixedClock::new(Timestamp::from_millis(0)),
        SequentialIds::new(),
        TTL,
    );
    block_on(uc.execute(request(id))).expect("première");

    assert_eq!(
        block_on(uc.execute(request(id))),
        Err(ReserveError::AlreadyTaken)
    );
    assert_eq!(
        transport.pushed().len(),
        1,
        "rien ne doit partir vers le module la seconde fois"
    );
}

#[test]
fn reserver_une_entree_inexistante_le_dit() {
    let pool = InMemoryPool::new();
    let transport = RecordingTransport::new();
    let notifier = RecordingNotifier::new();
    let uc = Reserve::new(
        &pool,
        &transport,
        &notifier,
        FixedClock::new(Timestamp::from_millis(0)),
        SequentialIds::new(),
        TTL,
    );
    assert_eq!(
        block_on(uc.execute(request(EntryId::from_u128(42)))),
        Err(ReserveError::NotFound)
    );
}

#[test]
fn l_entree_est_reservee_avant_que_quoi_que_ce_soit_ne_parte_vers_le_module() {
    // Si le module recevait le Pokémon avant que l'entrée ne soit sortie du
    // pool, un second joueur pourrait la réserver dans l'intervalle.
    let pool = InMemoryPool::new();
    let transport = RecordingTransport::new();
    let id = EntryId::from_u128(1);
    block_on(pool.insert(sample_entry(id))).expect("insertion");
    transport.fail_next(PortError::new("courtier hors service"));
    let notifier = RecordingNotifier::new();

    let uc = Reserve::new(
        &pool,
        &transport,
        &notifier,
        FixedClock::new(Timestamp::from_millis(0)),
        SequentialIds::new(),
        TTL,
    );
    let outcome = block_on(uc.execute(request(id)));

    assert!(matches!(outcome, Err(ReserveError::Port(_))));
    let stored = block_on(pool.get(id)).expect("lecture").expect("présente");
    assert!(
        !stored.is_claimable(),
        "l'entrée reste réservée : elle reviendra par expiration, jamais par annulation"
    );
}

#[test]
fn une_notification_en_panne_n_annule_pas_la_reservation() {
    let pool = InMemoryPool::new();
    let transport = RecordingTransport::new();
    let notifier = RecordingNotifier::new();
    let id = EntryId::from_u128(1);
    block_on(pool.insert(sample_entry(id))).expect("insertion");
    notifier.fail_next(PortError::new("service de push hors ligne"));

    let uc = Reserve::new(
        &pool,
        &transport,
        &notifier,
        FixedClock::new(Timestamp::from_millis(0)),
        SequentialIds::new(),
        TTL,
    );
    let reservation = block_on(uc.execute(request(id)));

    assert!(
        reservation.is_ok(),
        "prévenir le déposant est accessoire, pas critique"
    );
}
