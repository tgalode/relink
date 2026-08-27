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

## État

Rien n'est encore établi. Le design du cœur métier
(`docs/superpowers/specs/2026-08-27-relink-coeur-metier-design.md`) est
volontairement écrit au niveau des phases du protocole, pas des octets, pour
cette raison.

Les deux inconnues qui conditionnent le design :

1. **Les valeurs du handshake et de la phase de sélection.** Elles déterminent
   s'il existe un octet de remplissage signifiant « le partenaire n'a pas
   encore choisi ». Tout le mécanisme d'attente — donc l'échange direct entre
   joueurs — en dépend.
2. **Le format exact du bloc Gen 2, courrier inclus.** Le courrier étant
   transporté opaque, seuls son décalage et sa taille importent.
