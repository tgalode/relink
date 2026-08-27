//! Codec de la patch list Gen 1.
//!
//! `0xFE` est l'octet « pas de câble » du port série : une cartouche qui le
//! reçoit peut le prendre pour une déconnexion. Le jeu ne l'envoie donc
//! jamais tel quel. Il le remplace par `0xFF` dans les données d'équipe et
//! transmet, juste après le bloc, la liste des positions ainsi corrigées.
//!
//! Sourcé dans `docs/protocol/gen1-link-protocol.md`, sections « Pourquoi une
//! patch list », « Patch list : principe » et « Patch list : zone couverte ».

use crate::gen1::{PARTY_CAPACITY, PARTY_POKEMON_LEN};

/// Taille de la zone couverte : les six emplacements de données d'équipe.
pub const PARTY_DATA_LEN: usize = PARTY_CAPACITY * PARTY_POKEMON_LEN;

/// Nombre d'octets de liste que le module présente sur le fil. La cadence est
/// donnée par la cartouche : au-delà de ce que contient la liste, le module
/// présente du remplissage, et ce qu'il n'a pas le temps d'envoyer n'est
/// jamais réclamé. Les sources ne s'accordent pas à l'octet près sur cette
/// longueur — voir « Patch list : longueur transmise ».
pub const PATCH_LIST_LEN: usize = 189;

/// L'octet « pas de câble ».
pub const NO_DATA: u8 = 0xFE;

/// Marque la fin de chacune des deux parties de la liste.
pub const PART_TERMINATOR: u8 = 0xFF;

/// Dernière position couverte par la première partie.
const PART_ONE_LAST: usize = 0xFB;

/// Construit la patch list des données d'équipe et corrige celles-ci sur
/// place : chaque `0xFE` devient `0xFF` et sa position rejoint la liste.
///
/// Les positions sont notées incrémentées de un dans la première partie, et
/// relativement à `0xFB` dans la seconde : aucune valeur écrite ne peut ainsi
/// valoir `0xFE`.
///
/// Une équipe pathologique — plus de positions à corriger que la liste ne
/// peut en porter — voit les positions surnuméraires corrigées sans être
/// notées : le fil reste sain, ces octets-là arrivent en `0xFF`. Le cas ne se
/// produit pas sur des données réelles ; il est borné plutôt que faillible,
/// parce que `step()` ne peut pas échouer.
#[must_use]
pub fn build(party: &mut [u8; PARTY_DATA_LEN]) -> [u8; PATCH_LIST_LEN] {
    let mut list = [0u8; PATCH_LIST_LEN];
    let mut written = 0usize;

    // Deux emplacements sont réservés aux deux terminateurs.
    let capacity = PATCH_LIST_LEN - 2;

    for (position, byte) in party.iter_mut().enumerate().take(PART_ONE_LAST + 1) {
        if *byte == NO_DATA {
            *byte = PART_TERMINATOR;
            if written < capacity {
                list[written] = (position + 1) as u8;
                written += 1;
            }
        }
    }
    list[written] = PART_TERMINATOR;
    written += 1;

    for (position, byte) in party.iter_mut().enumerate().skip(PART_ONE_LAST + 1) {
        if *byte == NO_DATA {
            *byte = PART_TERMINATOR;
            if written < capacity + 1 {
                list[written] = (position - PART_ONE_LAST) as u8;
                written += 1;
            }
        }
    }
    list[written] = PART_TERMINATOR;

    list
}

/// Applique une patch list reçue : remet `0xFE` aux positions qu'elle
/// désigne. Toute valeur hors de la zone couverte est ignorée.
pub fn apply(party: &mut [u8; PARTY_DATA_LEN], list: &[u8; PATCH_LIST_LEN]) {
    let mut second_part = false;

    for &value in list {
        match value {
            PART_TERMINATOR if !second_part => second_part = true,
            PART_TERMINATOR => return,
            0 => {}
            _ => {
                let position = if second_part {
                    PART_ONE_LAST + value as usize
                } else {
                    value as usize - 1
                };
                if position < PARTY_DATA_LEN {
                    party[position] = NO_DATA;
                }
            }
        }
    }
}
