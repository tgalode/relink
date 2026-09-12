//! Une cartouche simulée, pour exercer [`crate::session`] sans matériel.
//!
//! Ce module n'est **pas** derrière `#[cfg(test)]`, pour la même raison que
//! son homologue de `relink-application` : ses consommateurs vivent dans des
//! crates séparés. Ici il y en a deux, et c'est le second qui a décidé de
//! l'emplacement — les tests d'intégration de `crates/protocol/tests/`, et le
//! banc de mesure `tools/banc-esp32`, qui fait tourner la même cartouche sur
//! un ESP32 pour comparer un échange passé par le fil à un échange joué en
//! mémoire.
//!
//! Une seconde définition recopiée dans le banc aurait été plus simple à
//! écrire et impossible à maintenir : le jour où une valeur sourcée change,
//! les tests suivraient et le banc mentirait en silence.
//!
//! ## Ce que cette doublure vaut
//!
//! Elle joue le côté jeu à partir des valeurs sourcées dans
//! `docs/protocol/gen1-link-protocol.md`. **Elle vaut donc ce que vaut ce
//! sourçage, et ne remplace pas une trace réelle.** Elle est là pour attraper
//! les régressions de transition, pas pour prouver l'accord avec une console.
//!
//! ## Pourquoi une capacité en paramètre
//!
//! Le crate est sans allocateur : le programme de la cartouche vit dans un
//! tableau de taille fixe, et l'appelant choisit cette taille. Un échange
//! complet consomme [`OCTETS_PAR_TRANSFERT`] octets plus la poignée d'octets
//! de sélection ; deux échanges dans la même session en consomment le double.
//! Dépasser la capacité déclenche une panique immédiate plutôt qu'une
//! troncature silencieuse, qui produirait un échange incomplet ressemblant à
//! un bug de la machine à états.

use crate::gen1::TradeBlock;

/// Ce que coûte un transfert complet : préambule, graine, bloc d'échange,
/// fin de bloc et patch list.
pub const OCTETS_PAR_TRANSFERT: usize =
    10 + 10 + 9 + crate::gen1::TRADE_BLOCK_LEN + 3 + 6 + 8 + 2 + 185;

/// Ce que coûte une annonce du joueur — sélection ou acceptation — suivie de
/// ses octets neutres.
pub const OCTETS_PAR_ANNONCE: usize = 5;

/// Une cartouche simulée, qui déroule son programme sans attendre personne.
///
/// `N` est la capacité du programme en octets. Voir la documentation du module
/// pour la choisir.
pub struct Cartouche<const N: usize> {
    programme: [u8; N],
    longueur: usize,
    position: usize,
    equipe: TradeBlock,
}

impl<const N: usize> Cartouche<N> {
    /// Une cartouche qui va jusqu'au bord de la sélection, avec cette équipe.
    pub fn nouvelle(equipe: TradeBlock) -> Self {
        let mut cartouche = Self {
            programme: [0; N],
            longueur: 0,
            position: 0,
            equipe,
        };
        cartouche.pousser(&[0x01, 0x00, 0x00, 0x60, 0xD0, 0xD4, 0x60]);
        cartouche.pousser_le_transfert();
        cartouche
    }

    /// Le joueur annonce le Pokémon qu'il propose. Une poignée d'octets
    /// neutres suit, comme sur le fil réel.
    pub fn choisit(&mut self, index: u8) {
        self.pousser(&[0x60 + index]);
        self.pousser(&[0x00; 4]);
    }

    /// Le joueur accepte l'échange, suivi de la même poignée d'octets neutres.
    pub fn accepte(&mut self) {
        self.pousser(&[0x62]);
        self.pousser(&[0x00; 4]);
    }

    /// Le joueur revient à la table pour un second échange : tout le transfert
    /// recommence.
    pub fn revient_a_la_table(&mut self) {
        self.pousser(&[0x00; 4]);
        self.pousser_le_transfert();
    }

    /// L'octet suivant que la cartouche présente, ou `None` quand son
    /// programme est épuisé.
    ///
    /// L'octet reçu est ignoré : la cartouche déroule, c'est la session qui
    /// doit suivre. C'est le point de la doublure — un jeu réel ne ralentit
    /// pas pour le module.
    pub fn octet_suivant(&mut self, _recu: u8) -> Option<u8> {
        let octet = self.programme.get(self.position).copied();
        if self.position < self.longueur {
            self.position += 1;
            octet
        } else {
            None
        }
    }

    /// Le programme complet, pour un consommateur qui cadence lui-même —
    /// typiquement un banc qui le pousse sur un vrai fil.
    pub fn programme(&self) -> &[u8] {
        &self.programme[..self.longueur]
    }

    /// Préambule, graine, bloc, fin de bloc, patch list.
    fn pousser_le_transfert(&mut self) {
        self.pousser(&[0xFD; 10]);
        self.pousser(&[0x2A; 10]);
        self.pousser(&[0xFD; 9]);
        let equipe = *self.equipe.as_bytes();
        self.pousser(&equipe);
        self.pousser(&[0xDF, 0xFE, 0x15]);
        self.pousser(&[0xFD; 6]);
        self.pousser(&[0x00; 8]);
        self.pousser(&[0xFF, 0xFF]);
        self.pousser(&[0x00; 185]);
    }

    /// Ajoute des octets au programme.
    ///
    /// Panique si la capacité est dépassée. Une doublure de test qui tronque
    /// en silence produirait un échange incomplet qu'on irait chercher dans la
    /// machine à états.
    fn pousser(&mut self, octets: &[u8]) {
        assert!(
            self.longueur + octets.len() <= N,
            "programme de cartouche trop long pour la capacité déclarée"
        );
        self.programme[self.longueur..self.longueur + octets.len()].copy_from_slice(octets);
        self.longueur += octets.len();
    }
}

/// Un bloc d'échange reconnaissable : chaque octet dérive du marqueur, ce qui
/// rend une confusion entre bloc sortant et bloc entrant visible en test.
pub fn bloc_fixture(marqueur: u8) -> TradeBlock {
    let mut raw = [0u8; crate::gen1::TRADE_BLOCK_LEN];
    for (i, b) in raw.iter_mut().enumerate() {
        *b = marqueur.wrapping_add((i % 97) as u8);
    }
    raw[11] = 1; // un Pokémon dans l'équipe
    TradeBlock::from_bytes(raw)
}
