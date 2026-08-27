//! Tests de la correspondance entre index interne d'espèce et numéro
//! national du Pokédex, première génération.

use relink_protocol::gen1::{LAST_GEN1_DEX_NUMBER, national_dex_number};

#[test]
fn traduit_des_index_connus() {
    // Valeurs sourcées dans `docs/protocol/gen1-species-index.md`. Corrigé
    // en tâche 1 : le plan original inversait les deux (0x15 n'est pas
    // Bulbizarre, et 0x14 n'est pas Mew).
    assert_eq!(national_dex_number(0x99), Some(1)); // Bulbizarre
    assert_eq!(national_dex_number(0x15), Some(151)); // Mew
}

#[test]
fn un_index_inutilise_ne_traduit_rien() {
    assert_eq!(national_dex_number(0x1F), None);
}

#[test]
fn toute_traduction_reste_dans_le_pokedex_gen1() {
    for index in 0..=u8::MAX {
        if let Some(dex) = national_dex_number(index) {
            assert!(
                (1..=LAST_GEN1_DEX_NUMBER).contains(&dex),
                "index {index:#04X} traduit en {dex}, hors du Pokédex Gen 1"
            );
        }
    }
}

#[test]
fn la_table_couvre_les_151_especes_sans_doublon() {
    let mut vus = [false; LAST_GEN1_DEX_NUMBER as usize + 1];
    for index in 0..=u8::MAX {
        if let Some(dex) = national_dex_number(index) {
            assert!(
                !vus[dex as usize],
                "numéro national {dex} produit deux fois"
            );
            vus[dex as usize] = true;
        }
    }
    for dex in 1..=LAST_GEN1_DEX_NUMBER {
        assert!(
            vus[dex as usize],
            "numéro national {dex} absent de la table"
        );
    }
}
