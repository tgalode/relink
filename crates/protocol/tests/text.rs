//! Tests d'intégration pour le jeu de caractères Game Boy et les chaînes de
//! longueur fixe.

use relink_protocol::text::{GbString, TERMINATOR, decode_char};

#[test]
fn decode_les_majuscules() {
    assert_eq!(decode_char(0x80), Some('A'));
    assert_eq!(decode_char(0x99), Some('Z'));
}

#[test]
fn decode_les_minuscules() {
    assert_eq!(decode_char(0xA0), Some('a'));
    assert_eq!(decode_char(0xB9), Some('z'));
}

#[test]
fn decode_les_chiffres_et_l_espace() {
    assert_eq!(decode_char(0xF6), Some('0'));
    assert_eq!(decode_char(0xFF), Some('9'));
    assert_eq!(decode_char(0x7F), Some(' '));
}

#[test]
fn le_terminateur_n_est_pas_un_caractere() {
    assert_eq!(decode_char(TERMINATOR), None);
}

#[test]
fn la_longueur_s_arrete_au_terminateur() {
    // "RED" puis terminateur, puis du remplissage qui doit être ignoré.
    let raw = [
        0x91, 0xA4, 0xA3, TERMINATOR, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];
    let name = GbString::<11>::from_bytes(raw);
    assert_eq!(name.len(), 3);
    assert!(!name.is_empty());
}

#[test]
fn un_nom_sans_terminateur_occupe_tout_le_champ() {
    let raw = [0x80; 11];
    assert_eq!(GbString::<11>::from_bytes(raw).len(), 11);
}

#[test]
fn un_nom_vide_commence_par_le_terminateur() {
    let mut raw = [0x00; 11];
    raw[0] = TERMINATOR;
    assert!(GbString::<11>::from_bytes(raw).is_empty());
}

#[test]
fn les_octets_sont_conserves_a_l_identique() {
    let raw = [
        0x91, 0xA4, 0xA3, TERMINATOR, 0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE,
    ];
    assert_eq!(GbString::<11>::from_bytes(raw).as_bytes(), &raw);
}

#[test]
fn chars_rend_les_caracteres_jusqu_au_terminateur() {
    let raw = [
        0x91, 0xA4, 0xA3, TERMINATOR, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];
    let name = GbString::<11>::from_bytes(raw);
    let decoded: Vec<Option<char>> = name.chars().collect();
    assert_eq!(decoded, vec![Some('R'), Some('e'), Some('d')]);
}
