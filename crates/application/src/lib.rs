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
//!
//! Le module [`deposit`] porte le cas d'usage de dépôt : faire entrer un
//! Pokémon dans le pool.
//!
//! Le module [`reserve`] porte le cas d'usage de réservation : sortir un
//! Pokémon du pool et le pousser vers le module physique d'un joueur.
//!
//! Le module [`commit`] porte le cas d'usage de commit : trancher une
//! réservation, une fois pour toutes. C'est le seul endroit du service où
//! l'on peut détruire des données irremplaçables — voir la documentation du
//! module avant d'y toucher.

pub mod commit;
pub mod deposit;
pub mod domain;
pub mod ports;
pub mod reserve;
pub mod testing;
