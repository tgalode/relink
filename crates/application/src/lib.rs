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
//! Le module [`acknowledge`] porte le cas d'usage d'accusé de réception : le
//! verrou du §7.2, sans lequel une entrée réellement remise à une cartouche
//! redeviendrait réservable à l'échéance de son TTL — voir la documentation
//! du module avant d'y toucher.
//!
//! Le module [`commit`] porte le cas d'usage de commit : trancher une
//! réservation, une fois pour toutes. C'est l'endroit le plus dangereux du
//! service pour détruire des données irremplaçables — pas le seul : le
//! dépôt aussi perd le Pokémon de façon tout aussi irréversible (spec §7.4)
//! — voir la documentation du module avant d'y toucher.
//!
//! Le module [`expiry`] porte le cas d'usage d'expiration : le seul chemin
//! du service qui rend une entrée au pool, et seulement pour une
//! réservation jamais parvenue à un module — voir la documentation du
//! module avant d'y toucher.
//!
//! Le module [`pairing`] porte le cas d'usage d'échange direct : un dépôt
//! réservé à un destinataire, apparié à un retrait ordinaire. Ce n'est pas
//! un protocole séparé — voir la documentation du module avant d'y toucher.

pub mod acknowledge;
pub mod commit;
pub mod deposit;
pub mod domain;
pub mod expiry;
pub mod pairing;
pub mod ports;
pub mod reserve;
pub mod testing;
