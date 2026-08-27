//! Tests des phases de transfert : préambule, graine, bloc, patch list.
//!
//! Longueurs et valeurs recopiées de `docs/protocol/gen1-link-protocol.md`,
//! sections « Préambule et graine aléatoire » à « Patch list : longueur
//! transmise ».

mod util;

use relink_protocol::gen1::patch_list::{NO_DATA, PART_TERMINATOR};
use relink_protocol::gen1::{TRADE_BLOCK_LEN, TradeBlock};
use relink_protocol::session::{Effect, Session};
use util::{bloc_fixture, effects, feed, jusqu_a_la_table};

const PREAMBLE: u8 = 0xFD;
const BLANK: u8 = 0x00;
const OFF_PARTY_DATA: usize = 19;
const PARTY_DATA_LEN: usize = 264;

/// Le préambule complet : 10 octets, puis 10 d'aléa, puis 9 de préambule.
fn en_tete() -> Vec<u8> {
    let mut v = vec![PREAMBLE; 10];
    v.extend_from_slice(&[0x2A; 10]);
    v.extend_from_slice(&[PREAMBLE; 9]);
    v
}

/// Le module ne présente son bloc qu'après l'en-tête complet, pas avant.
#[test]
fn le_bloc_ne_part_qu_apres_l_en_tete() {
    let mut s = Session::gen1(bloc_fixture(0x10));
    jusqu_a_la_table(&mut s);

    // L'octet de préambule initial est déjà consommé : il en reste neuf,
    // puis l'aléa, puis les neuf derniers.
    let mut reste = vec![PREAMBLE; 9];
    reste.extend_from_slice(&[0x2A; 10]);
    reste.extend_from_slice(&[PREAMBLE; 9]);
    let sortis = feed(&mut s, &reste);
    assert!(
        sortis.iter().all(|&b| b == PREAMBLE || b == 0x2A),
        "pendant l'en-tête, le module renvoie ce qu'il reçoit"
    );

    let premier = feed(&mut s, &[BLANK])[0];
    assert_eq!(
        premier,
        bloc_fixture(0x10).as_bytes()[0],
        "le bloc commence ici"
    );
}

/// Le bloc entrant est reçu en entier et rendu par `partner_block`.
#[test]
fn le_bloc_du_partenaire_est_recu_en_entier() {
    let mut s = Session::gen1(bloc_fixture(0x10));
    jusqu_a_la_table(&mut s);

    let mut octets = en_tete();
    octets.remove(0); // le premier préambule est déjà consommé
    let partenaire = bloc_fixture(0x80);
    octets.extend_from_slice(partenaire.as_bytes());
    feed(&mut s, &octets);

    assert!(
        s.partner_block().is_none(),
        "rien tant que la patch list n'est pas passée"
    );

    // Fin de bloc et en-tête de patch list : six octets de préambule.
    feed(&mut s, &[0xDF, 0xFE, 0x15]);
    feed(&mut s, &[PREAMBLE; 6]);
    // Sept octets neutres d'en-tête, puis la liste vide et son remplissage.
    let mut liste = vec![BLANK; 8];
    liste.push(PART_TERMINATOR);
    liste.push(PART_TERMINATOR);
    liste.extend(std::iter::repeat_n(BLANK, 200));
    let sortis = effects(&mut s, &liste);

    assert!(sortis.contains(&Effect::PartnerBlockReceived));
    assert_eq!(s.partner_block(), Some(partenaire));
}

/// Le module présente son propre bloc, corrigé : aucun octet « pas de
/// câble » ne part sur le fil.
#[test]
fn aucun_octet_pas_de_cable_ne_part_sur_le_fil() {
    let mut raw = [0u8; TRADE_BLOCK_LEN];
    raw[11] = 1;
    raw[OFF_PARTY_DATA] = NO_DATA;
    raw[OFF_PARTY_DATA + PARTY_DATA_LEN - 1] = NO_DATA;
    let mut s = Session::gen1(TradeBlock::from_bytes(raw));
    jusqu_a_la_table(&mut s);

    let mut octets = en_tete();
    octets.remove(0);
    octets.extend_from_slice(&[BLANK; TRADE_BLOCK_LEN]);
    let sortis = feed(&mut s, &octets);

    assert!(!sortis.contains(&NO_DATA), "le fil ne porte jamais 0xFE");
}

/// La patch list reçue est appliquée : l'octet « pas de câble » est remis en
/// place dans l'équipe du partenaire.
#[test]
fn la_patch_list_recue_est_appliquee() {
    let mut s = Session::gen1(bloc_fixture(0x10));
    jusqu_a_la_table(&mut s);

    let mut attendu = [0u8; TRADE_BLOCK_LEN];
    attendu[11] = 1;
    attendu[OFF_PARTY_DATA + 5] = NO_DATA;

    // Sur le fil, cette position porte 0xFF et sa correction est annoncée.
    let mut sur_le_fil = attendu;
    sur_le_fil[OFF_PARTY_DATA + 5] = PART_TERMINATOR;

    let mut octets = en_tete();
    octets.remove(0);
    octets.extend_from_slice(&sur_le_fil);
    octets.extend_from_slice(&[0xDF, 0xFE, 0x15]);
    octets.extend_from_slice(&[PREAMBLE; 6]);
    octets.extend_from_slice(&[BLANK; 8]);
    octets.push(0x06); // position 5, notée incrémentée de un
    octets.push(PART_TERMINATOR);
    octets.push(PART_TERMINATOR);
    octets.extend(std::iter::repeat_n(BLANK, 200));
    feed(&mut s, &octets);

    let recu = s.partner_block().expect("le bloc doit être reçu");
    assert_eq!(
        recu.as_bytes()[OFF_PARTY_DATA + 5],
        NO_DATA,
        "0xFE remis en place"
    );
}

/// Limitation documentée : une cartouche qui redémarre sa négociation en
/// plein transfert n'est pas suivie. Les phases de données transportent des
/// octets arbitraires — 0x01 y est une donnée, pas un signal — et `protocol`
/// n'a pas d'horloge pour trancher. C'est au firmware de détruire la session
/// et d'en ouvrir une neuve.
#[test]
fn une_renegociation_en_plein_transfert_n_est_pas_suivie() {
    let mut s = Session::gen1(bloc_fixture(0x10));
    jusqu_a_la_table(&mut s);
    let mut octets = en_tete();
    octets.remove(0);
    octets.extend_from_slice(&[BLANK; TRADE_BLOCK_LEN]);
    feed(&mut s, &octets);

    // En attente des six octets de préambule de la patch list : l'octet de
    // leader y est renvoyé comme n'importe quel autre.
    assert_eq!(feed(&mut s, &[0x01]), vec![0x01]);
}

/// Un second échange réutilise le même chemin : après la table, un nouveau
/// préambule relance le transfert.
#[test]
fn un_nouveau_preambule_relance_le_transfert() {
    let mut s = Session::gen1(bloc_fixture(0x10));
    jusqu_a_la_table(&mut s);
    let mut octets = en_tete();
    octets.remove(0);
    octets.extend_from_slice(bloc_fixture(0x80).as_bytes());
    octets.extend_from_slice(&[0xDF, 0xFE, 0x15]);
    octets.extend_from_slice(&[PREAMBLE; 6]);
    octets.extend_from_slice(&[BLANK; 8]);
    octets.push(PART_TERMINATOR);
    octets.push(PART_TERMINATOR);
    octets.extend(std::iter::repeat_n(BLANK, 200));
    feed(&mut s, &octets);

    // On est en phase de sélection : le module présente l'octet neutre.
    assert_eq!(feed(&mut s, &[BLANK]), vec![BLANK]);
}
