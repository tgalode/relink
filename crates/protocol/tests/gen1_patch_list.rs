//! Tests du codec de patch list Gen 1.
//!
//! Valeurs et découpage sourcés dans `docs/protocol/gen1-link-protocol.md`,
//! sections « Patch list : principe » et « Patch list : zone couverte ».

use relink_protocol::gen1::patch_list::{
    self, NO_DATA, PART_TERMINATOR, PARTY_DATA_LEN, PATCH_LIST_LEN,
};

/// Sans aucun octet à corriger, la liste n'est que ses deux terminateurs.
#[test]
fn une_equipe_sans_octet_special_donne_une_liste_vide() {
    let mut party = [0u8; PARTY_DATA_LEN];
    let list = patch_list::build(&mut party);

    assert_eq!(list[0], PART_TERMINATOR, "fin de la première partie");
    assert_eq!(list[1], PART_TERMINATOR, "fin de la seconde partie");
    assert!(
        list[2..].iter().all(|&b| b == 0),
        "le reste est du remplissage"
    );
    assert_eq!(
        party, [0u8; PARTY_DATA_LEN],
        "rien à corriger, rien de touché"
    );
}

/// Une position de la première partie est notée incrémentée de un, et
/// l'octet part sur le fil en 0xFF.
#[test]
fn la_premiere_partie_note_la_position_incrementee() {
    let mut party = [0u8; PARTY_DATA_LEN];
    party[0] = NO_DATA;
    party[0x0A] = NO_DATA;
    let list = patch_list::build(&mut party);

    assert_eq!(list[0], 0x01, "position 0 notée 1");
    assert_eq!(list[1], 0x0B, "position 0x0A notée 0x0B");
    assert_eq!(list[2], PART_TERMINATOR);
    assert_eq!(list[3], PART_TERMINATOR);
    assert_eq!(party[0], 0xFF, "l'octet corrigé part en 0xFF");
    assert_eq!(party[0x0A], 0xFF);
}

/// La frontière entre les deux parties : 0xFB est la dernière position de la
/// première, 0xFC la première de la seconde, notée par rapport à la base.
#[test]
fn la_frontiere_entre_les_deux_parties_est_a_la_bonne_position() {
    let mut party = [0u8; PARTY_DATA_LEN];
    party[0xFB] = NO_DATA;
    party[0xFC] = NO_DATA;
    let list = patch_list::build(&mut party);

    assert_eq!(
        list[0], 0xFC,
        "0xFB est la dernière position de la partie 1"
    );
    assert_eq!(list[1], PART_TERMINATOR);
    assert_eq!(
        list[2], 0x01,
        "0xFC est la première de la partie 2, notée 1"
    );
    assert_eq!(list[3], PART_TERMINATOR);
}

/// La dernière position couvrable, 0x107, est bien dans la seconde partie.
#[test]
fn la_derniere_position_de_l_equipe_est_couverte() {
    let mut party = [0u8; PARTY_DATA_LEN];
    party[PARTY_DATA_LEN - 1] = NO_DATA;
    let list = patch_list::build(&mut party);

    assert_eq!(list[0], PART_TERMINATOR, "rien dans la partie 1");
    assert_eq!(list[1], 0x0C, "0x107 notée 0x107 - 0xFB");
    assert_eq!(list[2], PART_TERMINATOR);
}

/// Aucune valeur écrite dans la liste ne peut valoir 0xFE : ce serait
/// indistinguable de l'octet « pas de câble ». C'est la raison d'être du
/// découpage en deux parties.
#[test]
fn aucune_valeur_de_liste_ne_vaut_l_octet_pas_de_cable() {
    let mut party = [NO_DATA; PARTY_DATA_LEN];
    let list = patch_list::build(&mut party);

    assert!(list.iter().all(|&b| b != NO_DATA));
}

/// L'aller-retour rend les octets d'origine à l'identique.
#[test]
fn l_aller_retour_rend_les_octets_d_origine() {
    let mut party = [0u8; PARTY_DATA_LEN];
    for (i, b) in party.iter_mut().enumerate() {
        *b = (i % 251) as u8;
    }
    party[3] = NO_DATA;
    party[0xFB] = NO_DATA;
    party[0xFC] = NO_DATA;
    party[PARTY_DATA_LEN - 1] = NO_DATA;
    let origine = party;

    let list = patch_list::build(&mut party);
    assert_ne!(party, origine, "les octets spéciaux ont été corrigés");

    patch_list::apply(&mut party, &list);
    assert_eq!(party, origine, "l'aller-retour est sans perte");
}

/// Une liste reçue absurde ne doit rien casser : les valeurs hors zone sont
/// ignorées, pas appliquées de travers.
#[test]
fn une_liste_recue_absurde_ne_casse_rien() {
    let mut party = [0u8; PARTY_DATA_LEN];
    let mut list = [0u8; PATCH_LIST_LEN];
    list[0] = 0x00; // remplissage prématuré, sans effet
    list[1] = PART_TERMINATOR;
    list[2] = 0xFD; // hors de la zone couverte par la seconde partie
    list[3] = PART_TERMINATOR;

    patch_list::apply(&mut party, &list);

    assert_eq!(
        party, [0u8; PARTY_DATA_LEN],
        "aucune position valide, rien de touché"
    );
}
