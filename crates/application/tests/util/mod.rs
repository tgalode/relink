//! Constructeurs communs aux tests d'intégration.

use relink_application::domain::{
    DepositId, EntryId, EntryState, Pokemon, PoolEntry, Provenance, Timestamp, TrainerId,
};
use relink_protocol::gen1::{NAME_LEN, Name, PARTY_POKEMON_LEN};

/// Un nom de dresseur quelconque mais valide.
#[must_use]
pub fn some_name() -> Name {
    let mut raw = [0x50u8; NAME_LEN];
    raw[0] = 0x91;
    raw[1] = 0xA4;
    raw[2] = 0xA3;
    Name::from_bytes(raw)
}

/// Un Pokémon d'espèce Mew, éligible en Gen 1.
#[must_use]
pub fn some_pokemon() -> Pokemon {
    let mut bytes = [0u8; PARTY_POKEMON_LEN];
    bytes[0x00] = 0x15;
    bytes[0x08] = 1;
    Pokemon {
        bytes,
        nickname: some_name(),
        original_trainer: some_name(),
    }
}

/// Une entrée disponible, portant l'identifiant donné.
///
/// `#[allow(dead_code)]` : ce module est compilé séparément pour chaque
/// binaire de test (`mod util;`), et tous ne s'en servent pas.
#[allow(dead_code)]
#[must_use]
pub fn sample_entry(id: EntryId) -> PoolEntry {
    PoolEntry {
        id,
        deposit: DepositId::from_u128(id.as_u128()),
        pokemon: some_pokemon(),
        provenance: Provenance {
            depositor: TrainerId {
                name: some_name(),
                number: 1234,
            },
            deposited_at: Timestamp::from_millis(0),
            previous: Vec::new(),
        },
        state: EntryState::Available,
        reserved_for: None,
    }
}
