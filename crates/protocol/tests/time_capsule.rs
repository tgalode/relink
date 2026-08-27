//! Tests d'intégration pour l'éligibilité Capsule Temporelle.
//!
//! Règles sourcées dans `docs/protocol/time-capsule-rules.md`.

use relink_protocol::gen1::{PARTY_POKEMON_LEN, PartyPokemon};
use relink_protocol::time_capsule::{Ineligible, LAST_GEN1_MOVE_ID, eligible_for_gen1};

fn pokemon(species_index: u8, moves: [u8; 4]) -> PartyPokemon {
    let mut raw = [0u8; PARTY_POKEMON_LEN];
    raw[0x00] = species_index;
    raw[0x08..0x0C].copy_from_slice(&moves);
    PartyPokemon::from_bytes(raw)
}

#[test]
fn un_pokemon_de_gen1_avec_des_capacites_de_gen1_passe() {
    // 0x15 est Mew selon `docs/protocol/gen1-species-index.md` (le plan
    // original l'attribuait par erreur à Bulbizarre) — une espèce Gen 1
    // valide comme une autre pour ce test.
    assert_eq!(eligible_for_gen1(&pokemon(0x15, [1, 33, 0, 0])), Ok(()));
}

#[test]
fn les_emplacements_de_capacite_vides_sont_ignores() {
    assert_eq!(eligible_for_gen1(&pokemon(0x15, [0, 0, 0, 0])), Ok(()));
}

#[test]
fn une_capacite_posterieure_a_la_gen1_bloque() {
    let refus = eligible_for_gen1(&pokemon(0x15, [1, LAST_GEN1_MOVE_ID + 1, 0, 0]));
    assert_eq!(
        refus,
        Err(Ineligible::MoveTooRecent {
            slot: 1,
            move_id: LAST_GEN1_MOVE_ID + 1
        })
    );
}

#[test]
fn la_derniere_capacite_de_gen1_passe_encore() {
    assert_eq!(
        eligible_for_gen1(&pokemon(0x15, [LAST_GEN1_MOVE_ID, 0, 0, 0])),
        Ok(())
    );
}

#[test]
fn une_espece_inconnue_bloque() {
    assert_eq!(
        eligible_for_gen1(&pokemon(0x1F, [1, 0, 0, 0])),
        Err(Ineligible::UnknownSpecies)
    );
}

#[test]
fn l_espece_est_verifiee_avant_les_capacites() {
    // Les deux sont fautifs : le refus doit porter sur l'espèce.
    let refus = eligible_for_gen1(&pokemon(0x1F, [LAST_GEN1_MOVE_ID + 1, 0, 0, 0]));
    assert_eq!(refus, Err(Ineligible::UnknownSpecies));
}
