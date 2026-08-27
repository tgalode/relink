//! Tests des phases de lien : négociation des rôles, acquittement, menu du
//! Cable Club.
//!
//! Valeurs d'octets recopiées de `docs/protocol/gen1-link-protocol.md`.

mod util;

use relink_protocol::session::{Effect, Session};
use util::{bloc_fixture, effects, feed};

const MASTER: u8 = 0x01;
const SLAVE: u8 = 0x02;
const BLANK: u8 = 0x00;
const CONNECTED: u8 = 0x60;
const TRADE_CENTRE: u8 = 0xD4;
const COLOSSEUM: u8 = 0xD5;
const BREAK_LINK: u8 = 0xD6;
const HIGHLIGHT_FIRST: u8 = 0xD0;

fn session() -> Session {
    Session::gen1(bloc_fixture(0x10))
}

/// Le module est suiveur : il répond 0x02 à l'octet de leader, toujours.
#[test]
fn repond_suiveur_a_l_octet_de_leader() {
    let mut s = session();
    assert_eq!(feed(&mut s, &[MASTER, MASTER]), vec![SLAVE, SLAVE]);
}

/// Les octets neutres de la négociation sont renvoyés tels quels : les
/// sources ne s'accordent pas sur leur nombre, on ne les compte pas.
#[test]
fn renvoie_les_octets_neutres_sans_les_compter() {
    let mut s = session();
    assert_eq!(
        feed(&mut s, &[MASTER, BLANK, BLANK, BLANK]),
        vec![SLAVE, BLANK, BLANK, BLANK]
    );
}

/// L'octet de connexion établit le lien.
#[test]
fn l_octet_de_connexion_etablit_le_lien() {
    let mut s = session();
    let sortis = effects(&mut s, &[MASTER, BLANK, CONNECTED]);
    assert_eq!(sortis, vec![Effect::LinkEstablished]);
}

/// Dans le menu, le module renvoie ce qu'il reçoit : c'est le joueur qui
/// choisit, pas le module.
#[test]
fn le_menu_laisse_choisir_le_joueur() {
    let mut s = session();
    feed(&mut s, &[MASTER, BLANK, CONNECTED]);
    assert_eq!(feed(&mut s, &[HIGHLIGHT_FIRST]), vec![HIGHLIGHT_FIRST]);
}

/// Le Trade Center est le seul parcours implémenté : il fait avancer sans
/// rompre le lien.
#[test]
fn le_trade_center_ne_rompt_pas_le_lien() {
    let mut s = session();
    feed(&mut s, &[MASTER, BLANK, CONNECTED]);
    let sortis = effects(&mut s, &[TRADE_CENTRE]);
    assert!(
        sortis.is_empty(),
        "aucun effet, on entre simplement dans la salle"
    );
}

/// Le Colosseum est reconnu et refusé proprement : les combats ne sont pas
/// dans le projet.
#[test]
fn le_colosseum_est_refuse_proprement() {
    let mut s = session();
    feed(&mut s, &[MASTER, BLANK, CONNECTED]);
    let mut s2 = Session::gen1(bloc_fixture(0x10));
    feed(&mut s2, &[MASTER, BLANK, CONNECTED]);

    assert_eq!(effects(&mut s, &[COLOSSEUM]), vec![Effect::LinkBroken]);
    assert_eq!(feed(&mut s2, &[COLOSSEUM]), vec![BREAK_LINK]);
}

/// L'annulation depuis le menu rompt le lien elle aussi.
#[test]
fn l_annulation_rompt_le_lien() {
    let mut s = session();
    feed(&mut s, &[MASTER, BLANK, CONNECTED]);
    assert_eq!(effects(&mut s, &[BREAK_LINK]), vec![Effect::LinkBroken]);
}

/// Une cartouche qui redémarre sa négociation retrouve un module qui la
/// suit, tant qu'on est dans une phase de synchronisation.
#[test]
fn une_negociation_qui_repart_est_suivie() {
    let mut s = session();
    feed(&mut s, &[MASTER, BLANK, CONNECTED, TRADE_CENTRE]);
    assert_eq!(feed(&mut s, &[MASTER]), vec![SLAVE]);
    assert_eq!(effects(&mut s, &[CONNECTED]), vec![Effect::LinkEstablished]);
}
