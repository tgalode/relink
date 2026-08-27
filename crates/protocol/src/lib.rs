//! Codecs et machine à états de l'échange par câble link, Pokémon Gen 1 et Gen 2.
//!
//! Ce crate est un cœur fonctionnel pur : aucune I/O, aucune allocation dans le
//! chemin critique, aucune faillibilité dans le pas d'exécution. C'est ce qui le
//! rend `no_std`, partageable avec le firmware, et rejouable en test.
//!
//! La contrainte qui dicte tout : en série synchrone Gen 1/2 la cartouche
//! fournit l'horloge, et l'octet sortant doit être prêt avant le front.
//!
//! Conception : `docs/superpowers/specs/2026-08-27-relink-coeur-metier-design.md`.
//!
//! Rien n'est encore implémenté.

#![no_std]
