//! Cas d'usage du service relink : dépôt, réservation, relais d'échange direct.
//!
//! Ce crate déclare ses ports en traits et n'en implémente aucun. Il consomme
//! les valeurs de [`relink_protocol`] et lui fournit une stratégie de
//! partenaire ; il ne connaît rien du fil.
//!
//! Conception : `docs/superpowers/specs/2026-08-27-relink-coeur-metier-design.md`.
//!
//! Le module [`domain`] pose les types métier — instants, identifiants,
//! Pokémon et entrées de pool — sur lesquels s'appuieront les ports et les
//! cas d'usage des lots suivants.
//!
//! Le module [`testing`] fournit des doublures en mémoire des ports,
//! utilisées par les tests d'intégration de ce crate et des suivants. Il
//! n'est pas réservé au profil `test` : les tests d'intégration sont des
//! crates séparés et doivent pouvoir l'importer comme une dépendance
//! normale.

pub mod domain;
pub mod ports;
pub mod testing;
