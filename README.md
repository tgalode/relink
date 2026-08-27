# relink

Rendre leurs échanges aux joueurs des Pokémon Game Boy et Game Boy Color.

Un module se branche sur le port link de la console et se fait passer pour un
partenaire d'échange. Selon le parcours, ce partenaire rend le Pokémon évolué,
le dépose dans un pool en ligne, remet celui que le joueur a choisi à l'avance,
ou relaie l'échange d'un joueur situé à l'autre bout du monde.

Le jeu, lui, ne voit qu'un échange 1:1 parfaitement normal.

> **État : codecs Gen 1 livrés.** `crates/protocol` expose les codecs de
> première génération (jeu de caractères, Pokémon d'équipe, bloc d'échange,
> table d'espèces) et les règles de la Capsule Temporelle, testés. La machine
> à états de l'échange et les codecs Gen 2 restent à faire ; `crates/application`
> n'a encore aucun cas d'usage implémenté. Le dépôt est public dès le premier
> commit pour que l'hygiène de rétro-ingénierie soit vérifiable dans
> l'historique — voir [CONTRIBUTING.md](CONTRIBUTING.md).

## Ce que contient ce dépôt

Le **cœur métier**, et rien d'autre pour l'instant. Deux couches aux
contraintes disjointes :

| Crate | Contraintes | Rôle |
|---|---|---|
| `crates/protocol` | `no_std`, zéro I/O, O(1) par octet | Codecs Gen 1/2 et machine à états de l'échange. Partagé avec le firmware. |
| `crates/application` | `std` | Dépôt, réservation, provenance. Ports en traits, aucun adaptateur. |

Le firmware, l'API HTTP, l'application mobile et les adaptateurs concrets
viendront ensuite, chacun avec sa propre spécification.

La conception est écrite :
[docs/superpowers/specs/2026-08-27-relink-coeur-metier-design.md](docs/superpowers/specs/2026-08-27-relink-coeur-metier-design.md)

## Générations couvertes

Gen 1 (Rouge/Bleu/Jaune) et Gen 2 (Or/Argent/Cristal). Même couche physique,
même machine à états, deux formats de bloc d'échange.

La conversion entre les deux n'est pas inventée : c'est la Capsule Temporelle,
avec ses règles d'origine.

La Gen 3 utilise une couche physique entièrement différente. C'est un autre
projet, pas une extension de celui-ci.

## Licences

Le dépôt est délibérément sous deux régimes :

- **`crates/protocol` : MIT OU Apache-2.0.** Ce crate a vocation à être repris
  par des émulateurs et d'autres outils communautaires. Une licence permissive
  est ce qui lui donne une chance de devenir la référence du protocole
  d'échange Gen 1/2.
- **`crates/application` : AGPL-3.0-or-later.** Le service ne doit pas pouvoir
  être repris en hébergé fermé.

## Ce que ce projet n'est pas

Ce dépôt ne contient ni ROM, ni sauvegarde, ni code Nintendo, ni marque
déposée, et le service n'est pas monétisé. Voir
[CONTRIBUTING.md](CONTRIBUTING.md) — ces règles ne sont pas cosmétiques, ce
sont les conditions d'existence du projet.
