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

/// Amène une session jusqu'à la phase de sélection, le bloc du partenaire
/// ayant été échangé.
pub fn jusqu_a_la_selection(session: &mut Session, partenaire: TradeBlock) {
    jusqu_a_la_table(session);
    let mut octets = vec![0xFD; 9];
    octets.extend_from_slice(&[0x2A; 10]);
    octets.extend_from_slice(&[0xFD; 9]);
    octets.extend_from_slice(partenaire.as_bytes());
    octets.extend_from_slice(&[0xDF, 0xFE, 0x15]);
    octets.extend_from_slice(&[0xFD; 6]);
    octets.extend_from_slice(&[0x00; 8]);
    octets.push(0xFF);
    octets.push(0xFF);
    // La section de patch list fait 195 octets comptés depuis son premier :
    // huit d'en-tête, les deux terminateurs, puis le remplissage. On s'arrête
    // pile à la frontière, pour laisser la phase de sélection intacte.
    octets.extend(core::iter::repeat_n(0x00, 185));
    feed(session, &octets);
}

/// Une cartouche simulée : elle joue le côté jeu à partir des valeurs
/// sourcées dans `docs/protocol/gen1-link-protocol.md`, et cadence l'échange
/// comme le ferait le matériel.
///
/// Elle vaut ce que vaut le sourçage et ne remplace pas une trace réelle.
/// Elle est là pour attraper les régressions de transition, pas pour prouver
/// l'accord avec une console.
pub struct Cartouche {
    programme: Vec<u8>,
    position: usize,
    equipe: TradeBlock,
}

impl Cartouche {
    /// Une cartouche qui va jusqu'au bord de la sélection, avec cette équipe.
    pub fn nouvelle(equipe: TradeBlock) -> Self {
        let mut cartouche = Self {
            programme: vec![0x01, 0x00, 0x00, 0x60, 0xD0, 0xD4, 0x60],
            position: 0,
            equipe,
        };
        cartouche.pousser_le_transfert();
        cartouche
    }

    /// Le joueur annonce le Pokémon qu'il propose. Une poignée d'octets
    /// neutres suit, comme sur le fil réel.
    pub fn choisit(&mut self, index: u8) {
        self.programme.push(0x60 + index);
        self.programme.extend_from_slice(&[0x00; 4]);
    }

    /// Le joueur accepte l'échange, suivi de la même poignée d'octets
    /// neutres.
    pub fn accepte(&mut self) {
        self.programme.push(0x62);
        self.programme.extend_from_slice(&[0x00; 4]);
    }

    /// Le joueur revient à la table pour un second échange : tout le
    /// transfert recommence.
    pub fn revient_a_la_table(&mut self) {
        self.programme.extend_from_slice(&[0x00; 4]);
        self.pousser_le_transfert();
    }

    /// L'octet suivant que la cartouche présente, ou `None` quand son
    /// programme est épuisé. L'octet reçu est ignoré : la cartouche déroule,
    /// c'est la session qui doit suivre.
    pub fn octet_suivant(&mut self, _recu: u8) -> Option<u8> {
        let octet = self.programme.get(self.position).copied();
        self.position += 1;
        octet
    }

    /// Préambule, graine, bloc, fin de bloc, patch list.
    fn pousser_le_transfert(&mut self) {
        self.programme.extend_from_slice(&[0xFD; 10]);
        self.programme.extend_from_slice(&[0x2A; 10]);
        self.programme.extend_from_slice(&[0xFD; 9]);
        let equipe = *self.equipe.as_bytes();
        self.programme.extend_from_slice(&equipe);
        self.programme.extend_from_slice(&[0xDF, 0xFE, 0x15]);
        self.programme.extend_from_slice(&[0xFD; 6]);
        self.programme.extend_from_slice(&[0x00; 8]);
        self.programme.push(0xFF);
        self.programme.push(0xFF);
        self.programme.extend(core::iter::repeat_n(0x00, 185));
    }
}
