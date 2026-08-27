//! Tests des phases de table : sélection, verdict, échange, sortie.
//!
//! Valeurs recopiées de `docs/protocol/gen1-link-protocol.md`, sections
//! « Sélection du Pokémon », « Verdict » et « L'ambiguïté de 0x61 ».

mod util;

use relink_protocol::session::{Decision, Effect, Session};
use util::{bloc_fixture, effects, feed, jusqu_a_la_selection};

const BLANK: u8 = 0x00;
const SELECT_BASE: u8 = 0x60;
const TABLE_LEAVE: u8 = 0x6F;
const REJECT: u8 = 0x61;
const ACCEPT: u8 = 0x62;

fn a_la_selection() -> Session {
    let mut s = Session::gen1(bloc_fixture(0x10));
    jusqu_a_la_selection(&mut s, bloc_fixture(0x80));
    s
}

/// En entrant dans la phase, la session réclame une offre.
#[test]
fn reclame_une_offre_en_entrant() {
    let mut s = a_la_selection();
    assert_eq!(effects(&mut s, &[BLANK]), vec![Effect::OfferNeeded]);
}

/// Tant que l'offre n'est pas fournie, la session présente l'octet neutre —
/// indéfiniment. C'est ce qui rend l'échange direct possible.
#[test]
fn attend_sans_echeance_tant_que_l_offre_manque() {
    let mut s = a_la_selection();
    assert_eq!(feed(&mut s, &[BLANK; 500]), vec![BLANK; 500]);
}

/// L'offre fournie est annoncée par sa position.
#[test]
fn annonce_l_offre_fournie() {
    let mut s = a_la_selection();
    feed(&mut s, &[BLANK]);
    s.supply(Decision::Offer(2));
    assert_eq!(feed(&mut s, &[BLANK]), vec![SELECT_BASE + 2]);
}

/// L'offre du joueur est signalée avec sa position.
#[test]
fn signale_l_offre_du_joueur() {
    let mut s = a_la_selection();
    let sortis = effects(&mut s, &[BLANK, SELECT_BASE + 4]);
    assert!(sortis.contains(&Effect::PartnerOffered { index: 4 }));
}

/// Les deux offres connues, la session réclame un verdict.
#[test]
fn reclame_un_verdict_quand_les_deux_offres_sont_connues() {
    let mut s = a_la_selection();
    feed(&mut s, &[BLANK]);
    s.supply(Decision::Offer(0));
    let sortis = effects(&mut s, &[SELECT_BASE + 1, BLANK]);
    assert!(sortis.contains(&Effect::VerdictNeeded));
}

/// L'accord des deux côtés conclut l'échange, et dit lequel part et lequel
/// arrive.
#[test]
fn l_accord_des_deux_cotes_conclut_l_echange() {
    let mut s = a_la_selection();
    feed(&mut s, &[BLANK]);
    s.supply(Decision::Offer(0));
    feed(&mut s, &[SELECT_BASE + 3, BLANK]);
    s.supply(Decision::Accept);

    let sortis = effects(&mut s, &[ACCEPT]);
    assert_eq!(
        sortis,
        vec![Effect::TradeAgreed {
            offered: 0,
            received: 3
        }]
    );
}

/// L'accord se présente sur le fil, pas seulement en interne.
#[test]
fn l_accord_est_presente_sur_le_fil() {
    let mut s = a_la_selection();
    feed(&mut s, &[BLANK]);
    s.supply(Decision::Offer(0));
    feed(&mut s, &[SELECT_BASE + 3, BLANK]);
    s.supply(Decision::Accept);
    assert_eq!(feed(&mut s, &[BLANK]), vec![ACCEPT]);
}

/// Un refus du joueur ramène à la sélection : c'est là que 0x61 veut dire
/// « je refuse » et non « je propose le deuxième ».
#[test]
fn le_refus_du_joueur_ramene_a_la_selection() {
    let mut s = a_la_selection();
    feed(&mut s, &[BLANK]);
    s.supply(Decision::Offer(0));
    feed(&mut s, &[SELECT_BASE + 3, BLANK]);

    let sortis = effects(&mut s, &[REJECT, BLANK]);
    assert!(
        sortis.contains(&Effect::OfferNeeded),
        "on redemande une offre"
    );
}

/// En phase de sélection, le même octet veut dire « je propose le deuxième ».
#[test]
fn en_selection_le_meme_octet_designe_le_deuxieme_pokemon() {
    let mut s = a_la_selection();
    let sortis = effects(&mut s, &[BLANK, REJECT]);
    assert!(sortis.contains(&Effect::PartnerOffered { index: 1 }));
}

/// Le joueur qui quitte la table ramène la session dans la salle.
#[test]
fn quitter_la_table_ramene_dans_la_salle() {
    let mut s = a_la_selection();
    let sortis = effects(&mut s, &[BLANK, TABLE_LEAVE]);
    assert!(sortis.contains(&Effect::TableLeft));
}

/// Le module aussi peut quitter la table.
#[test]
fn le_module_peut_quitter_la_table() {
    let mut s = a_la_selection();
    feed(&mut s, &[BLANK]);
    s.supply(Decision::Leave);
    assert_eq!(feed(&mut s, &[BLANK]), vec![TABLE_LEAVE]);
}

/// Un index d'offre absurde est borné, jamais transmis tel quel.
#[test]
fn un_index_absurde_est_borne() {
    let mut s = a_la_selection();
    feed(&mut s, &[BLANK]);
    s.supply(Decision::Offer(200));
    assert_eq!(feed(&mut s, &[BLANK]), vec![SELECT_BASE + 5]);
}
