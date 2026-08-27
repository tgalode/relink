//! Les doublures doivent respecter les contrats des ports, sinon les tests
//! des tâches suivantes ne prouveraient rien.

use pollster::block_on;
use relink_application::domain::{
    DepositId, EntryId, EntryState, ReservationId, Timestamp, TrainerId,
};
use relink_application::ports::{
    ClaimOutcome, Clock, CommitOutcome, IdSource, LegalityChecker, ModuleId, ModuleTransport,
    Notifier, PoolRepository, PortError,
};
use relink_application::testing::{
    FixedClock, InMemoryPool, RecordingNotifier, RecordingTransport, SequentialIds, StubLegality,
};

mod util;
use util::{sample_entry, some_name, some_pokemon};

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

// --- Round 1 : les cinq trous Important -------------------------------

#[test]
fn record_delivery_est_recorded_puis_already_recorded() {
    let pool = InMemoryPool::new();
    let id = EntryId::from_u128(1);
    let res = ReservationId::from_u128(10);
    block_on(pool.insert(sample_entry(id))).expect("insertion");
    block_on(pool.claim(id, res, at(1000))).expect("réservation");

    assert_eq!(
        block_on(pool.record_delivery(res)).expect("premier accusé"),
        CommitOutcome::Recorded
    );
    assert_eq!(
        block_on(pool.record_delivery(res)).expect("accusé rejoué"),
        CommitOutcome::AlreadyRecorded
    );
}

#[test]
fn une_entree_livree_n_expire_jamais() {
    let pool = InMemoryPool::new();
    let id = EntryId::from_u128(1);
    let res = ReservationId::from_u128(10);
    block_on(pool.insert(sample_entry(id))).expect("insertion");
    block_on(pool.claim(id, res, at(1000))).expect("réservation");
    block_on(pool.record_delivery(res)).expect("accusé");

    let expired = block_on(pool.expire_due(at(9_999))).expect("expiration");
    assert!(
        expired.is_empty(),
        "une entrée dont un module a accusé réception ne doit jamais revenir au pool"
    );
}

#[test]
fn record_delivery_apres_tranchage_ne_dement_pas_le_commit() {
    let pool = InMemoryPool::new();
    let id = EntryId::from_u128(1);
    let res = ReservationId::from_u128(10);
    block_on(pool.insert(sample_entry(id))).expect("insertion");
    block_on(pool.claim(id, res, at(1000))).expect("réservation");
    block_on(pool.record_commit(res, at(500))).expect("commit");

    let outcome = block_on(pool.record_delivery(res)).expect("accusé tardif");
    assert_eq!(outcome, CommitOutcome::AlreadyRecorded);

    let entry = block_on(pool.get(id))
        .expect("get")
        .expect("l'entrée existe");
    assert_eq!(
        entry.state,
        EntryState::Committed {
            reservation: res,
            at: at(500)
        },
        "un accusé tardif ne doit jamais dé-commiter une entrée déjà remise"
    );
}

#[test]
fn record_delivery_sur_une_reservation_qui_ne_tient_plus_rien_rend_unknown() {
    let pool = InMemoryPool::new();
    let id = EntryId::from_u128(1);
    let res = ReservationId::from_u128(10);
    block_on(pool.insert(sample_entry(id))).expect("insertion");
    block_on(pool.claim(id, res, at(1000))).expect("réservation");
    block_on(pool.expire_due(at(1000))).expect("expiration");

    let outcome = block_on(pool.record_delivery(res)).expect("accusé après expiration");
    assert_eq!(outcome, CommitOutcome::Unknown);
}

#[test]
fn le_tranchage_croise_commit_puis_abandon_ne_bouge_pas_l_etat() {
    let pool = InMemoryPool::new();
    let id = EntryId::from_u128(1);
    let res = ReservationId::from_u128(10);
    block_on(pool.insert(sample_entry(id))).expect("insertion");
    block_on(pool.claim(id, res, at(1000))).expect("réservation");

    assert_eq!(
        block_on(pool.record_commit(res, at(5))).expect("commit"),
        CommitOutcome::Recorded
    );
    assert_eq!(
        block_on(pool.record_abandon(res, at(9))).expect("abandon croisé"),
        CommitOutcome::AlreadyRecorded
    );

    let entry = block_on(pool.get(id))
        .expect("get")
        .expect("l'entrée existe");
    assert_eq!(
        entry.state,
        EntryState::Committed {
            reservation: res,
            at: at(5)
        },
        "un abandon rejoué après un commit ne doit pas écraser le commit"
    );
}

#[test]
fn le_tranchage_croise_abandon_puis_commit_ne_bouge_pas_l_etat() {
    let pool = InMemoryPool::new();
    let id = EntryId::from_u128(1);
    let res = ReservationId::from_u128(10);
    block_on(pool.insert(sample_entry(id))).expect("insertion");
    block_on(pool.claim(id, res, at(1000))).expect("réservation");

    assert_eq!(
        block_on(pool.record_abandon(res, at(5))).expect("abandon"),
        CommitOutcome::Recorded
    );
    assert_eq!(
        block_on(pool.record_commit(res, at(9))).expect("commit croisé"),
        CommitOutcome::AlreadyRecorded
    );

    let entry = block_on(pool.get(id))
        .expect("get")
        .expect("l'entrée existe");
    assert_eq!(
        entry.state,
        EntryState::Abandoned {
            reservation: res,
            at: at(5)
        },
        "un commit rejoué après un abandon ne doit pas écraser l'abandon"
    );
}

#[test]
fn rejouer_claim_avec_la_meme_reservation_rend_claimed_sans_prolonger_l_echeance() {
    let pool = InMemoryPool::new();
    let id = EntryId::from_u128(1);
    let res = ReservationId::from_u128(10);
    block_on(pool.insert(sample_entry(id))).expect("insertion");

    let first = block_on(pool.claim(id, res, at(1000))).expect("premier claim");
    let replay = block_on(pool.claim(id, res, at(5000))).expect("rejeu");

    assert_eq!(first, ClaimOutcome::Claimed);
    assert_eq!(
        replay,
        ClaimOutcome::Claimed,
        "un rejeu à l'identique doit rendre Claimed, c'est bien sa réservation"
    );

    let entry = block_on(pool.get(id))
        .expect("get")
        .expect("l'entrée existe");
    assert_eq!(
        entry.state,
        EntryState::Reserved {
            reservation: res,
            expires_at: at(1000),
            delivered: false
        },
        "le rejeu ne doit pas prolonger l'échéance initiale"
    );
}

#[test]
fn le_clone_de_l_horloge_partage_le_meme_etat() {
    let clock = FixedClock::new(at(100));
    let clone = clock.clone();

    clock.advance(50);
    assert_eq!(
        block_on(clone.now()),
        at(150),
        "un `advance` sur l'original doit être visible depuis le clone"
    );

    clone.set(at(500));
    assert_eq!(
        block_on(clock.now()),
        at(500),
        "un `set` sur le clone doit être visible depuis l'original"
    );
}

#[test]
fn reutiliser_un_entry_id_sous_une_autre_cle_de_depot_est_rejete() {
    let pool = InMemoryPool::new();
    let id = EntryId::from_u128(1);
    block_on(pool.insert(sample_entry(id))).expect("premier dépôt");

    let mut collision = sample_entry(id);
    collision.deposit = DepositId::from_u128(999);

    let result = block_on(pool.insert(collision));
    assert!(
        result.is_err(),
        "un identifiant d'entrée déjà enregistré sous une autre clé de dépôt doit être rejeté"
    );
    assert_eq!(
        pool.len(),
        1,
        "la collision ne doit pas créer ou écraser d'entrée"
    );
}

// --- Round 1 : les quatre Mineurs qui valent le détour -----------------

#[test]
fn deux_fils_qui_reservent_la_meme_entree_n_en_obtiennent_qu_un_claimed() {
    use std::sync::Arc;
    use std::thread;

    let pool = Arc::new(InMemoryPool::new());
    let id = EntryId::from_u128(1);
    block_on(pool.insert(sample_entry(id))).expect("insertion");

    let pool_a = Arc::clone(&pool);
    let pool_b = Arc::clone(&pool);
    let a =
        thread::spawn(move || block_on(pool_a.claim(id, ReservationId::from_u128(10), at(1000))));
    let b =
        thread::spawn(move || block_on(pool_b.claim(id, ReservationId::from_u128(11), at(1000))));

    let outcomes = [
        a.join().unwrap().expect("fil a"),
        b.join().unwrap().expect("fil b"),
    ];
    let claimed = outcomes
        .iter()
        .filter(|o| **o == ClaimOutcome::Claimed)
        .count();
    let taken = outcomes
        .iter()
        .filter(|o| **o == ClaimOutcome::AlreadyTaken)
        .count();

    assert_eq!(claimed, 1, "un seul des deux fils doit obtenir Claimed");
    assert_eq!(
        taken, 1,
        "l'autre doit obtenir AlreadyTaken, jamais les deux Claimed"
    );
}

#[test]
fn l_expiration_inclut_l_instant_exact() {
    let pool = InMemoryPool::new();
    let id = EntryId::from_u128(1);
    block_on(pool.insert(sample_entry(id))).expect("insertion");
    block_on(pool.claim(id, ReservationId::from_u128(10), at(1000))).expect("réservation");

    let expired = block_on(pool.expire_due(at(1000))).expect("expiration à l'instant pile");
    assert_eq!(expired, vec![id], "expires_at <= now, borne incluse");
}

#[test]
fn get_rend_l_entree_ou_rien() {
    let pool = InMemoryPool::new();
    let id = EntryId::from_u128(1);
    assert_eq!(block_on(pool.get(id)).expect("get sur pool vide"), None);

    block_on(pool.insert(sample_entry(id))).expect("insertion");
    let found = block_on(pool.get(id)).expect("get après insertion");
    assert_eq!(found.map(|entry| entry.id), Some(id));
}

#[test]
fn list_claimable_ne_rend_que_les_entrees_disponibles() {
    let pool = InMemoryPool::new();
    let available = EntryId::from_u128(1);
    let reserved = EntryId::from_u128(2);
    block_on(pool.insert(sample_entry(available))).expect("premier dépôt");
    block_on(pool.insert(sample_entry(reserved))).expect("second dépôt");
    block_on(pool.claim(reserved, ReservationId::from_u128(10), at(1000))).expect("réservation");

    let claimable = block_on(pool.list_claimable()).expect("liste");
    let ids: Vec<EntryId> = claimable.iter().map(|entry| entry.id).collect();
    assert_eq!(ids, vec![available]);
}

#[test]
fn stub_legality_rend_le_verdict_fixe_a_la_construction() {
    let pokemon = some_pokemon();
    let accepting = StubLegality::accepting();
    let rejecting = StubLegality::rejecting();

    assert_eq!(block_on(accepting.is_legal(&pokemon)), Ok(true));
    assert_eq!(block_on(rejecting.is_legal(&pokemon)), Ok(false));
}

#[test]
fn recording_transport_enregistre_les_poussees_dans_l_ordre() {
    let transport = RecordingTransport::new();
    let pokemon = some_pokemon();
    let module = ModuleId::from_u128(1);

    block_on(transport.push_reservation(module, ReservationId::from_u128(1), &pokemon))
        .expect("première poussée");
    block_on(transport.push_reservation(module, ReservationId::from_u128(2), &pokemon))
        .expect("seconde poussée");

    let pushed = transport.pushed();
    assert_eq!(pushed.len(), 2);
    assert_eq!(pushed[0].reservation, ReservationId::from_u128(1));
    assert_eq!(pushed[1].reservation, ReservationId::from_u128(2));
}

#[test]
fn recording_transport_fail_next_ne_frappe_qu_une_fois() {
    let transport = RecordingTransport::new();
    let pokemon = some_pokemon();
    let module = ModuleId::from_u128(1);

    transport.fail_next(PortError::new("module hors service"));
    assert!(
        block_on(transport.push_reservation(module, ReservationId::from_u128(1), &pokemon))
            .is_err()
    );
    assert!(
        block_on(transport.push_reservation(module, ReservationId::from_u128(2), &pokemon)).is_ok()
    );
    assert_eq!(
        transport.pushed().len(),
        1,
        "la poussée en échec ne doit pas être enregistrée"
    );
}

#[test]
fn recording_notifier_enregistre_les_notifications_dans_l_ordre() {
    let notifier = RecordingNotifier::new();
    let trainer = TrainerId {
        name: some_name(),
        number: 1234,
    };

    block_on(notifier.entry_claimed(&trainer, EntryId::from_u128(1))).expect("première notif");
    block_on(notifier.entry_claimed(&trainer, EntryId::from_u128(2))).expect("seconde notif");

    let notified = notifier.notified();
    assert_eq!(notified.len(), 2);
    assert_eq!(notified[0].entry, EntryId::from_u128(1));
    assert_eq!(notified[1].entry, EntryId::from_u128(2));
}

#[test]
fn recording_notifier_fail_next_ne_frappe_qu_une_fois() {
    let notifier = RecordingNotifier::new();
    let trainer = TrainerId {
        name: some_name(),
        number: 1234,
    };

    notifier.fail_next(PortError::new("joueur injoignable"));
    assert!(block_on(notifier.entry_claimed(&trainer, EntryId::from_u128(1))).is_err());
    assert!(block_on(notifier.entry_claimed(&trainer, EntryId::from_u128(2))).is_ok());
    assert_eq!(
        notifier.notified().len(),
        1,
        "la notification en échec ne doit pas être enregistrée"
    );
}

#[test]
fn les_identifiants_suivent_la_sequence_et_les_familles_sont_independantes() {
    let ids = SequentialIds::new();

    assert_eq!(block_on(ids.next_entry_id()), EntryId::from_u128(1));
    assert_eq!(block_on(ids.next_entry_id()), EntryId::from_u128(2));
    assert_eq!(block_on(ids.next_entry_id()), EntryId::from_u128(3));

    // La famille des réservations a son propre compteur : elle repart de 1
    // alors que celui des entrées est déjà à 3.
    assert_eq!(
        block_on(ids.next_reservation_id()),
        ReservationId::from_u128(1)
    );
    assert_eq!(
        block_on(ids.next_reservation_id()),
        ReservationId::from_u128(2)
    );
}
