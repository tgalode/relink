//! Tests d'intégration pour la vue sur les 44 octets d'un Pokémon d'équipe
//! Gen 1.

use relink_protocol::gen1::{PARTY_POKEMON_LEN, PartyPokemon};

/// Construit 44 octets où chaque champ porte une valeur reconnaissable.
/// Les décalages viennent de `docs/protocol/gen1-trade-block.md`.
fn fixture() -> [u8; PARTY_POKEMON_LEN] {
    let mut raw = [0u8; PARTY_POKEMON_LEN];
    raw[0x00] = 0x15; // espèce, index interne
    raw[0x03] = 25; // niveau du champ « box »
    raw[0x08] = 0x21; // capacité 1
    raw[0x09] = 0x2D; // capacité 2
    raw[0x0A] = 0x00; // capacité 3, absente
    raw[0x0B] = 0x00; // capacité 4, absente
    raw[0x0C] = 0x30; // identifiant du dresseur, octet de poids fort
    raw[0x0D] = 0x39; // identifiant du dresseur, octet de poids faible
    raw[0x0E] = 0x00; // expérience, 3 octets de poids fort au début
    raw[0x0F] = 0x4E;
    raw[0x10] = 0x20;
    raw[0x1B] = 0x9A; // DV : attaque 9, défense 10
    raw[0x1C] = 0xBC; // DV : vitesse 11, spécial 12
    raw[0x21] = 30; // niveau, second emplacement
    raw
}

#[test]
fn lit_l_espece_et_le_niveau() {
    let p = PartyPokemon::from_bytes(fixture());
    assert_eq!(p.species_index(), 0x15);
    assert_eq!(p.level(), 30);
}

#[test]
fn lit_l_experience_sur_trois_octets() {
    let p = PartyPokemon::from_bytes(fixture());
    assert_eq!(p.experience(), 0x004E20);
}

#[test]
fn lit_l_identifiant_du_dresseur() {
    let p = PartyPokemon::from_bytes(fixture());
    assert_eq!(p.trainer_id(), 0x3039);
}

#[test]
fn lit_les_quatre_capacites() {
    let p = PartyPokemon::from_bytes(fixture());
    assert_eq!(p.moves(), [0x21, 0x2D, 0x00, 0x00]);
}

#[test]
fn separe_les_dv_en_quartets() {
    let dvs = PartyPokemon::from_bytes(fixture()).dvs();
    assert_eq!(dvs.attack, 9);
    assert_eq!(dvs.defense, 10);
    assert_eq!(dvs.speed, 11);
    assert_eq!(dvs.special, 12);
}

#[test]
fn le_dv_de_pv_vient_des_bits_de_poids_faible_des_quatre_autres() {
    let dvs = PartyPokemon::from_bytes(fixture()).dvs();
    // attaque 9 (impair) -> 8, défense 10 (pair) -> 0,
    // vitesse 11 (impair) -> 2, spécial 12 (pair) -> 0
    assert_eq!(dvs.hp(), 0b1010);
}

#[test]
fn les_octets_sont_conserves_a_l_identique() {
    let raw = fixture();
    assert_eq!(PartyPokemon::from_bytes(raw).as_bytes(), &raw);
}
