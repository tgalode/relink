//! Codecs et machine à états de l'échange par câble link, Pokémon Gen 1 et Gen 2.
//!
//! Ce crate est un cœur fonctionnel pur : aucune I/O, aucune allocation dans le
//! chemin critique, aucune faillibilité dans le pas d'exécution. C'est ce qui le
//! rend `no_std`, partageable avec le firmware, et rejouable en test.
//!
//! La contrainte qui dicte tout : en série synchrone Gen 1/2 la cartouche
//! fournit l'horloge, et l'octet sortant doit être prêt avant le front.
//!
//! ## Principe de conception
//!
//! Chaque type de ce crate est une vue sur des octets bruts, pas un analyseur
//! qui reconstruit une structure : les accesseurs lisent, ils ne valident ni
//! ne normalisent, et le ré-encodage reste identique à l'octet près. Le crate
//! est `no_std`, sans allocateur.
//!
//! ## Ce qui est livré
//!
//! Les codecs de première génération (Rouge/Bleu/Jaune), les règles de la
//! Capsule Temporelle et la machine à états de l'échange, testés :
//!
//! - [`text`] — jeu de caractères Game Boy et chaînes de longueur fixe.
//! - [`gen1`] — Pokémon d'équipe, bloc d'échange, et table de correspondance
//!   index interne → numéro national.
//! - [`time_capsule`] — règles d'éligibilité au transfert vers une cartouche
//!   de première génération.
//! - [`session`] — la machine à états de l'échange Trade Center : le module
//!   joue le suiveur, la cartouche cadence, et les décisions que le module
//!   n'a pas encore n'arrêtent rien.
//!
//! Ce qui manque encore : les codecs de deuxième génération
//! (Or/Argent/Cristal).
//!
//! Conception : `docs/superpowers/specs/2026-08-27-relink-coeur-metier-design.md`.

#![no_std]

pub mod gen1;
pub mod session;
pub mod text;
pub mod time_capsule;
