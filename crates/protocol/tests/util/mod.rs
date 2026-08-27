//! Outils partagés par les tests de session.
//!
//! `#![allow(dead_code)]` : ce module est compilé séparément pour chaque
//! binaire de test, et chacun n'en utilise qu'une partie.
#![allow(dead_code)]

use relink_protocol::gen1::{TRADE_BLOCK_LEN, TradeBlock};
use relink_protocol::session::{Effect, Session};

/// Fait consommer une suite d'octets à la session et rend ce qu'elle a
/// présenté en retour, octet pour octet.
pub fn feed(session: &mut Session, bytes: &[u8]) -> Vec<u8> {
    bytes.iter().map(|&b| session.step(b).outgoing).collect()
}

/// Fait consommer une suite d'octets et rend les effets émis en chemin.
pub fn effects(session: &mut Session, bytes: &[u8]) -> Vec<Effect> {
    bytes
        .iter()
        .filter_map(|&b| session.step(b).effect)
        .collect()
}

/// Un bloc d'échange reconnaissable : chaque octet dérive du marqueur, ce qui
/// rend une confusion entre bloc sortant et bloc entrant visible en test.
pub fn bloc_fixture(marqueur: u8) -> TradeBlock {
    let mut raw = [0u8; TRADE_BLOCK_LEN];
    for (i, b) in raw.iter_mut().enumerate() {
        *b = marqueur.wrapping_add((i % 97) as u8);
    }
    raw[11] = 1; // un Pokémon dans l'équipe
    TradeBlock::from_bytes(raw)
}

/// Amène une session fraîche au bord du transfert : lien établi, Trade
/// Center choisi, table utilisée. Le premier octet de préambule est consommé.
pub fn jusqu_a_la_table(session: &mut Session) {
    feed(session, &[0x01, 0x00, 0x60, 0xD4, 0x60, 0xFD]);
}
