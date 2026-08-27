//! Types du domaine du service relink.

use relink_application::domain::{EntryId, EntryState, PoolEntry, ReservationId, Timestamp};
use relink_protocol::gen1::{NAME_LEN, PARTY_POKEMON_LEN};

fn at(ms: u64) -> Timestamp {
    Timestamp::from_millis(ms)
}

fn entry(state: EntryState) -> PoolEntry {
    use relink_application::domain::{DepositId, Pokemon, Provenance, TrainerId};
    use relink_protocol::gen1::Name;
    let name = Name::from_bytes([0x50; NAME_LEN]);
    PoolEntry {
        id: EntryId::from_u128(1),
        deposit: DepositId::from_u128(1),
        pokemon: Pokemon {
            bytes: [0u8; PARTY_POKEMON_LEN],
            nickname: name,
            original_trainer: name,
        },
        provenance: Provenance {
            depositor: TrainerId { name, number: 42 },
            deposited_at: at(0),
            previous: Vec::new(),
        },
        state,
        reserved_for: None,
    }
}

#[test]
fn le_temps_se_compare_et_s_avance() {
    assert!(at(10) < at(20));
    assert_eq!(at(10).saturating_add_millis(5), at(15));
    assert_eq!(at(10).as_millis(), 10);
}

#[test]
fn l_avance_du_temps_sature_au_lieu_de_deborder() {
    assert_eq!(
        Timestamp::from_millis(u64::MAX)
            .saturating_add_millis(1)
            .as_millis(),
        u64::MAX
    );
}

#[test]
fn seule_une_entree_disponible_est_prenable() {
    let r = ReservationId::from_u128(7);
    assert!(entry(EntryState::Available).is_claimable());
    assert!(
        !entry(EntryState::Reserved {
            reservation: r,
            expires_at: at(1),
            delivered: false
        })
        .is_claimable()
    );
    assert!(
        !entry(EntryState::Reserved {
            reservation: r,
            expires_at: at(1),
            delivered: true
        })
        .is_claimable()
    );
    assert!(
        !entry(EntryState::Committed {
            reservation: r,
            at: at(1)
        })
        .is_claimable()
    );
    assert!(
        !entry(EntryState::Abandoned {
            reservation: r,
            at: at(1)
        })
        .is_claimable()
    );
}

#[test]
fn l_etat_rend_la_reservation_qui_le_gouverne() {
    let r = ReservationId::from_u128(7);
    assert_eq!(EntryState::Available.reservation(), None);
    assert_eq!(
        EntryState::Reserved {
            reservation: r,
            expires_at: at(1),
            delivered: false
        }
        .reservation(),
        Some(r)
    );
    assert_eq!(
        EntryState::Committed {
            reservation: r,
            at: at(1)
        }
        .reservation(),
        Some(r)
    );
    assert_eq!(
        EntryState::Abandoned {
            reservation: r,
            at: at(1)
        }
        .reservation(),
        Some(r)
    );
}
