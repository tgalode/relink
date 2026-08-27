//! Tests de la vue sur le bloc d'échange Gen 1 complet (415 octets).

use relink_protocol::gen1::{
    NAME_LEN, PARTY_CAPACITY, PARTY_POKEMON_LEN, TRADE_BLOCK_LEN, TradeBlock,
};
use relink_protocol::text::TERMINATOR;

const OFF_TRAINER_NAME: usize = 0;
const OFF_PARTY_LIST: usize = 11;
const OFF_PARTY_DATA: usize = 19;
const OFF_OT_NAMES: usize = 283;
const OFF_NICKNAMES: usize = 349;

/// Un bloc contenant deux Pokémon, chacun reconnaissable.
/// Les décalages viennent de `docs/protocol/gen1-trade-block.md`.
fn fixture() -> [u8; TRADE_BLOCK_LEN] {
    let mut raw = [0u8; TRADE_BLOCK_LEN];

    // Dresseur : « Red »
    raw[OFF_TRAINER_NAME] = 0x91;
    raw[OFF_TRAINER_NAME + 1] = 0xA4;
    raw[OFF_TRAINER_NAME + 2] = 0xA3;
    raw[OFF_TRAINER_NAME + 3] = TERMINATOR;

    // Liste d'équipe : 2 Pokémon, puis le terminateur de liste.
    raw[OFF_PARTY_LIST] = 2;
    raw[OFF_PARTY_LIST + 1] = 0x15;
    raw[OFF_PARTY_LIST + 2] = 0x99;
    raw[OFF_PARTY_LIST + 3] = 0xFF;

    // Données d'équipe : espèce et niveau de chacun.
    raw[OFF_PARTY_DATA] = 0x15;
    raw[OFF_PARTY_DATA + 0x21] = 30;
    raw[OFF_PARTY_DATA + PARTY_POKEMON_LEN] = 0x99;
    raw[OFF_PARTY_DATA + PARTY_POKEMON_LEN + 0x21] = 42;

    // Dresseur d'origine du premier : « Red »
    raw[OFF_OT_NAMES] = 0x91;
    raw[OFF_OT_NAMES + 1] = 0xA4;
    raw[OFF_OT_NAMES + 2] = 0xA3;
    raw[OFF_OT_NAMES + 3] = TERMINATOR;

    // Surnom du second, terminateur immédiat : pas de surnom.
    raw[OFF_NICKNAMES + NAME_LEN] = TERMINATOR;

    raw
}

#[test]
fn les_constantes_sont_coherentes() {
    assert_eq!(TRADE_BLOCK_LEN, OFF_NICKNAMES + PARTY_CAPACITY * NAME_LEN);
}

#[test]
fn lit_le_nom_du_dresseur() {
    let b = TradeBlock::from_bytes(fixture());
    let name = b.trainer_name();
    assert_eq!(name.len(), 3);
    assert_eq!(
        name.chars().collect::<Vec<_>>(),
        vec![Some('R'), Some('e'), Some('d')]
    );
}

#[test]
fn lit_la_taille_de_l_equipe() {
    assert_eq!(TradeBlock::from_bytes(fixture()).party_len(), 2);
}

#[test]
fn une_equipe_annoncee_trop_grande_est_ramenee_a_la_capacite() {
    let mut raw = fixture();
    raw[OFF_PARTY_LIST] = 200;
    assert_eq!(TradeBlock::from_bytes(raw).party_len(), PARTY_CAPACITY);
}

#[test]
fn lit_chaque_pokemon_de_l_equipe() {
    let b = TradeBlock::from_bytes(fixture());
    let premier = b.pokemon(0).expect("le premier existe");
    assert_eq!(premier.species_index(), 0x15);
    assert_eq!(premier.level(), 30);

    let second = b.pokemon(1).expect("le second existe");
    assert_eq!(second.species_index(), 0x99);
    assert_eq!(second.level(), 42);
}

#[test]
fn au_dela_de_l_equipe_il_n_y_a_rien() {
    let b = TradeBlock::from_bytes(fixture());
    assert!(b.pokemon(2).is_none());
    assert!(b.pokemon(PARTY_CAPACITY).is_none());
    assert!(b.pokemon(usize::MAX).is_none());
    assert!(b.original_trainer(2).is_none());
    assert!(b.nickname(usize::MAX).is_none());
}

#[test]
fn lit_le_dresseur_d_origine_et_le_surnom() {
    let b = TradeBlock::from_bytes(fixture());
    assert_eq!(b.original_trainer(0).expect("présent").len(), 3);
    assert!(b.nickname(1).expect("présent").is_empty());
}

#[test]
fn les_octets_sont_conserves_a_l_identique() {
    let raw = fixture();
    assert_eq!(TradeBlock::from_bytes(raw).as_bytes(), &raw);
}
