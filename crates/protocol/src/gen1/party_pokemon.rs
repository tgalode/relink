//! Vue sur les 44 octets d'un Pokémon d'équipe Gen 1.
//!
//! Disposition sourcée dans `docs/protocol/gen1-trade-block.md`.

/// Taille, en octets, d'un Pokémon dans les données d'équipe.
pub const PARTY_POKEMON_LEN: usize = 44;

const OFF_SPECIES: usize = 0x00;
const OFF_MOVES: usize = 0x08;
const OFF_TRAINER_ID: usize = 0x0C;
const OFF_EXPERIENCE: usize = 0x0E;
const OFF_DVS: usize = 0x1B;
const OFF_LEVEL: usize = 0x21;

/// Les valeurs déterminantes d'un Pokémon Gen 1, quatre quartets stockés sur
/// deux octets.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Dvs {
    /// DV d'Attaque, de 0 à 15.
    pub attack: u8,
    /// DV de Défense, de 0 à 15.
    pub defense: u8,
    /// DV de Vitesse, de 0 à 15.
    pub speed: u8,
    /// DV de Spécial, de 0 à 15.
    pub special: u8,
}

impl Dvs {
    /// Le DV de PV n'est pas stocké : il se reconstitue à partir du bit de
    /// poids faible des quatre autres.
    #[must_use]
    pub const fn hp(&self) -> u8 {
        ((self.attack & 1) << 3)
            | ((self.defense & 1) << 2)
            | ((self.speed & 1) << 1)
            | (self.special & 1)
    }
}

/// Vue sur un Pokémon d'équipe. Les octets sont conservés à l'identique :
/// ce type sert à lire, jamais à reconstruire.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PartyPokemon {
    bytes: [u8; PARTY_POKEMON_LEN],
}

impl PartyPokemon {
    /// Enveloppe des octets bruts. Ne valide rien.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; PARTY_POKEMON_LEN]) -> Self {
        Self { bytes }
    }

    /// Les octets d'origine, inchangés.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; PARTY_POKEMON_LEN] {
        &self.bytes
    }

    /// Index interne de l'espèce. Ce n'est **pas** le numéro national ; la
    /// conversion est en tâche 5.
    #[must_use]
    pub const fn species_index(&self) -> u8 {
        self.bytes[OFF_SPECIES]
    }

    /// Niveau courant.
    #[must_use]
    pub const fn level(&self) -> u8 {
        self.bytes[OFF_LEVEL]
    }

    /// Points d'expérience, stockés sur trois octets de poids fort au début.
    #[must_use]
    pub const fn experience(&self) -> u32 {
        (self.bytes[OFF_EXPERIENCE] as u32) << 16
            | (self.bytes[OFF_EXPERIENCE + 1] as u32) << 8
            | (self.bytes[OFF_EXPERIENCE + 2] as u32)
    }

    /// Identifiant du dresseur d'origine.
    #[must_use]
    pub const fn trainer_id(&self) -> u16 {
        (self.bytes[OFF_TRAINER_ID] as u16) << 8 | (self.bytes[OFF_TRAINER_ID + 1] as u16)
    }

    /// Les quatre emplacements de capacité. `0` signifie « vide ».
    #[must_use]
    pub const fn moves(&self) -> [u8; 4] {
        [
            self.bytes[OFF_MOVES],
            self.bytes[OFF_MOVES + 1],
            self.bytes[OFF_MOVES + 2],
            self.bytes[OFF_MOVES + 3],
        ]
    }

    /// Les DV, éclatés en quartets.
    #[must_use]
    pub const fn dvs(&self) -> Dvs {
        let hi = self.bytes[OFF_DVS];
        let lo = self.bytes[OFF_DVS + 1];
        Dvs {
            attack: hi >> 4,
            defense: hi & 0x0F,
            speed: lo >> 4,
            special: lo & 0x0F,
        }
    }
}
