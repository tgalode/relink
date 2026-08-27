# Notes de rétro-ingénierie

Toute constante du protocole utilisée dans le code est documentée ici, avec sa
source. Une constante sans source est un bug, même si elle marche.

## Format attendu

Pour chaque valeur ou structure :

- **Ce que c'est** — le rôle dans le protocole.
- **Valeur / disposition** — l'octet, le décalage, la taille.
- **Source** — observation sur matériel (avec le montage), capture série (avec
  la trace jointe), ou documentation communautaire nommément citée.
- **Confiance** — confirmée, probable, hypothèse.

## Documents de ce dossier

- [`gen1-charset.md`](gen1-charset.md) — jeu de caractères Game Boy (Rouge/
  Bleu/Jaune) : terminateur, majuscules, minuscules, chiffres, ponctuation.
  Confirmé pour l'essentiel des octets, un seul (`0xF2`) reste probable.
- [`gen1-species-index.md`](gen1-species-index.md) — correspondance entre
  l'index interne d'espèce et le numéro national du Pokédex, pour les 151
  espèces de première génération. Table confirmée ; la liste des index
  inutilisés reste probable.
- [`gen1-trade-block.md`](gen1-trade-block.md) — disposition complète du
  bloc d'échange de 415 octets (nom du dresseur, liste et données d'équipe,
  noms de dresseur d'origine, surnoms) et des 44 octets d'un Pokémon
  d'équipe. Grande majorité des champs confirmés ; quelques conventions
  (dont « `0x00` capacité = emplacement vide », et la signification des
  bits de statut) restent probables, source unique.
- [`time-capsule-rules.md`](time-capsule-rules.md) — règles qui déterminent
  si un Pokémon peut redescendre vers une cartouche de première génération.
  La règle d'espèce est confirmée ; la frontière numérique des capacités
  (165/166) reste probable, source unique.

Ces documents sourcent les codecs Gen 1 livrés dans `crates/protocol` : jeu
de caractères, Pokémon d'équipe, bloc d'échange, table d'espèces, et les
règles de la Capsule Temporelle.

## Ce qui reste ouvert

Le design du cœur métier
(`docs/superpowers/specs/2026-08-27-relink-coeur-metier-design.md`) est
volontairement écrit au niveau des phases du protocole, pas des octets, pour
cette raison : deux inconnues conditionnent encore le design de l'échange et
de la Gen 2.

1. **Les valeurs du handshake et de la phase de sélection.** Elles déterminent
   s'il existe un octet de remplissage signifiant « le partenaire n'a pas
   encore choisi ». Tout le mécanisme d'attente — donc l'échange direct entre
   joueurs — en dépend.
2. **Le format exact du bloc Gen 2, courrier inclus.** Le courrier étant
   transporté opaque, seuls son décalage et sa taille importent.
