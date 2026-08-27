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

use relink_protocol::gen1::{PARTY_CAPACITY, TRADE_BLOCK_LEN, TradeBlock};
use relink_protocol::time_capsule::first_ineligible_in_party;

/// Les décalages viennent de `docs/protocol/gen1-trade-block.md`.
const OFF_PARTY_LIST: usize = 11;
const OFF_PARTY_DATA: usize = 19;
const PARTY_POKEMON: usize = 44;

/// Un bloc dont l'équipe compte `count` Pokémon, tous d'espèce `species`,
/// et dont le Pokémon à `bad_slot` connaît une capacité trop récente.
fn party(count: u8, species: u8, bad_slot: Option<usize>) -> TradeBlock {
    let mut raw = [0u8; TRADE_BLOCK_LEN];
    raw[OFF_PARTY_LIST] = count;
    for i in 0..count as usize {
        let base = OFF_PARTY_DATA + i * PARTY_POKEMON;
        raw[base] = species;
        raw[base + 0x08] = 1;
        if bad_slot == Some(i) {
            raw[base + 0x09] = 200; // postérieure à la Gen 1
        }
    }
    TradeBlock::from_bytes(raw)
}

/// Espèce valide en Gen 1 selon `docs/protocol/gen1-species-index.md`.
const MEW: u8 = 0x15;

#[test]
fn une_equipe_entierement_eligible_ne_rend_rien() {
    assert_eq!(first_ineligible_in_party(&party(3, MEW, None)), None);
}

#[test]
fn une_equipe_vide_ne_rend_rien() {
    assert_eq!(first_ineligible_in_party(&party(0, MEW, None)), None);
}

#[test]
fn rend_la_position_du_premier_fautif() {
    let (slot, _) = first_ineligible_in_party(&party(4, MEW, Some(2))).expect("un fautif");
    assert_eq!(slot, 2);
}

#[test]
fn rend_le_premier_fautif_et_pas_un_suivant() {
    let block = party(4, MEW, Some(3));
    let mut raw = *block.as_bytes();
    raw[OFF_PARTY_DATA + PARTY_POKEMON + 0x09] = 200;
    let (slot, _) = first_ineligible_in_party(&TradeBlock::from_bytes(raw)).expect("un fautif");
    assert_eq!(slot, 1, "c'est le premier fautif qui doit être rendu");
}

#[test]
fn n_examine_jamais_au_dela_de_l_equipe_annoncee() {
    // Une équipe de 1, mais des octets fautifs dans les emplacements suivants.
    let block = party(1, MEW, None);
    let mut raw = *block.as_bytes();
    for i in 1..PARTY_CAPACITY {
        raw[OFF_PARTY_DATA + i * PARTY_POKEMON + 0x09] = 200;
    }
    assert_eq!(
        first_ineligible_in_party(&TradeBlock::from_bytes(raw)),
        None
    );
}
