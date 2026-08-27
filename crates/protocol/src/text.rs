//! Jeu de caractères Game Boy et chaînes de longueur fixe.
//!
//! Table sourcée dans `docs/protocol/gen1-charset.md`.

/// Octet qui termine une chaîne. Ce qui suit est du remplissage sans
/// signification, mais il est conservé tel quel.
pub const TERMINATOR: u8 = 0x50;

/// Traduit un octet en caractère Unicode, ou `None` s'il n'en représente pas
/// un — terminateur, code de contrôle, ou octet non attribué.
#[must_use]
pub fn decode_char(byte: u8) -> Option<char> {
    match byte {
        0x7F => Some(' '),
        0x80..=0x99 => Some((b'A' + (byte - 0x80)) as char),
        0xA0..=0xB9 => Some((b'a' + (byte - 0xA0)) as char),
        0xF6..=0xFF => Some((b'0' + (byte - 0xF6)) as char),
        _ => None,
    }
}

/// Chaîne de longueur fixe telle que la cartouche la stocke.
///
/// Les octets sont conservés à l'identique, terminateur et remplissage
/// compris : ce type sert à lire, jamais à reconstruire.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct GbString<const N: usize> {
    bytes: [u8; N],
}

impl<const N: usize> GbString<N> {
    /// Enveloppe des octets bruts. Ne valide rien : n'importe quelle suite
    /// d'octets est une chaîne acceptable.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; N]) -> Self {
        Self { bytes }
    }

    /// Les octets d'origine, inchangés.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; N] {
        &self.bytes
    }

    /// Nombre d'octets avant le terminateur, ou `N` s'il n'y en a pas.
    #[must_use]
    pub fn len(&self) -> usize {
        let mut i = 0;
        while i < N {
            if self.bytes[i] == TERMINATOR {
                return i;
            }
            i += 1;
        }
        N
    }

    /// Vrai si la chaîne commence par le terminateur.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Les caractères jusqu'au terminateur. `None` pour un octet qui ne
    /// correspond à aucun caractère connu — l'octet reste lisible via
    /// [`Self::as_bytes`].
    pub fn chars(&self) -> impl Iterator<Item = Option<char>> + '_ {
        self.bytes[..self.len()].iter().copied().map(decode_char)
    }
}
