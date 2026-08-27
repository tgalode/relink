
# Règles de la Capsule Temporelle

Détermine si un Pokémon peut être remis à une cartouche de première
génération. C'est la fonction que la Capsule Temporelle remplit entre
Pokémon Or/Argent/Cristal et Pokémon Rouge/Bleu/Jaune : elle applique les
règles des jeux telles quelles, rien n'est inventé ici.

**Récapitulatif de confiance** (2 entrées « Confiance » distinctes dans ce
document) : **1 confirmée** (règle d'espèce, qui ne fait que rappeler une
valeur déjà établie et confirmée ailleurs), **1 probable** (dernière
capacité de première génération, source unique nommée). Détail
ci-dessous ; la section « Espèce inconnue » n'a volontairement pas
d'entrée de confiance propre, voir sa justification.

Source principale :

- Bulbapedia, [« Time Capsule »](https://bulbapedia.bulbagarden.net/wiki/Time_Capsule)
  — article de référence de la communauté décrivant le fonctionnement du
  transfert Gen 2 → Gen 1 et ses restrictions, citées ci-dessous.
- Bulbapedia, [« List of moves »](https://bulbapedia.bulbagarden.net/wiki/List_of_moves)
  — table complète des capacités par numéro d'index, toutes générations,
  utilisée pour situer la frontière entre capacités de première et de
  deuxième génération.

## Règle de capacité

- **Ce que c'est** — une capacité introduite en génération II rend un
  Pokémon inéligible à une cartouche de première génération.
- **Valeur / disposition** — dernier identifiant de capacité de première
  génération : `165` (Struggle/Lutte). Le premier identifiant de deuxième
  génération est `166` (Sketch/Capturécran) : la frontière est donc nette,
  sans identifiant partagé ni trou entre les deux générations.
- **Source** — Bulbapedia, page « Time Capsule » : « None of the Pokémon
  in the party can know any moves introduced in Generation II. » pour la
  règle elle-même ; Bulbapedia, page « List of moves », table indexée,
  pour la frontière numérique 165/166 entre les deux générations.
  Recoupement partiel : une recherche communautaire indépendante (liste
  des capacités de génération I sur PokémonDB, non indexée par numéro)
  dénombre également 165 capacités de première génération, sans toutefois
  donner les identifiants numériques eux-mêmes — ce deuxième point ne
  permet donc de corroborer que le compte total, pas la frontière exacte
  165/166.
- **Confiance** — probable (source unique nommée — Bulbapedia — pour
  l'identifiant numérique exact de la frontière ; le compte total de 165
  capacités est recoupé par une seconde source indépendante, mais celle-ci
  ne donne pas les numéros d'index).

## Règle d'espèce

- **Ce que c'est** — une espèce introduite en génération II rend un
  Pokémon inéligible à une cartouche de première génération.
- **Valeur / disposition** — dernier numéro national de première
  génération : `151` (Mew). Cette valeur est `gen1::LAST_GEN1_DEX_NUMBER`,
  déjà établie et sourcée dans `gen1-species-index.md` ; elle n'est pas
  re-sourcée ici, seulement rappelée pour le contexte de la règle.
- **Source** — Bulbapedia, page « Time Capsule » : « The player cannot
  have any Generation II Pokémon or Eggs in their party. » (le volet
  « Eggs » ne s'applique pas à ce domaine : un Pokémon d'équipe Gen 1 tel
  que représenté par `gen1::PartyPokemon` ne peut pas être un Œuf, ce
  concept n'existant pas en première génération) ; `gen1-species-index.md`
  pour la valeur numérique 151 elle-même.
- **Confiance** — confirmée (la valeur numérique reprend une entrée déjà
  confirmée ailleurs ; l'existence de la règle « pas d'espèce Gen 2 » est
  donnée explicitement par la source Bulbapedia ci-dessus).

## Espèce inconnue

Ce cas n'est pas une règle distincte trouvée dans les sources sur la
Capsule Temporelle : aucune des sources consultées ne décrit ce que fait
le jeu face à un octet d'espèce qui ne désigne aucune espèce valide
(un index « MissingNo. » ou glitch), parce que ce cas ne se produit
normalement jamais pour un Pokémon obtenu légitimement.

Le comportement retenu ici — refuser le transfert — est une conséquence
directe de la règle d'espèce ci-dessus, pas une règle ajoutée : si
`national_dex_number` ne peut pas établir que l'espèce a un numéro
national inférieur ou égal à 151 (parce que l'index ne désigne aucune
espèce connue de première génération), alors la condition « l'espèce est
une espèce de première génération » n'est pas remplie, au même titre que
si le numéro nationale dépassait 151. La distinction entre les deux cas
(`UnknownSpecies` contre `SpeciesTooRecent`) est une exigence de
l'interface produite par ce module, pas un comportement de jeu observé.

La confiance sur la table qui rend ce cas possible (les index inutilisés
de première génération) reste **probable**, comme déjà noté dans
`gen1-species-index.md` — elle n'est pas répétée ici en tant qu'entrée de
confiance propre à ce document.

## Règles additionnelles, hors périmètre de cette fonction

La page « Time Capsule » de Bulbapedia documente d'autres restrictions du
transfert Gen 2 → Gen 1, qui ne portent pas sur les données représentables
par `gen1::PartyPokemon` (un Pokémon déjà au format Gen 1) et ne sont donc
pas vérifiées par `eligible_for_gen1` :

- « The player cannot have any Pokémon holding Mail in their party. » —
  le courrier est un concept de deuxième génération, absent de la
  structure Gen 1.
- « If the Pokémon traded from the Generation I game changes its type
  during the trade (unless it's a Magnemite or Magneton), the trade is
  immediately cancelled. » — porte sur le sens Gen 1 → Gen 2, l'inverse du
  périmètre de cette fonction, et sur une comparaison de types plutôt que
  sur l'éligibilité de l'espèce ou des capacités.

Ces règles ne sont pas implémentées ici ; les mentionner sert uniquement à
documenter qu'elles ont été considérées et écartées du périmètre, pas
oubliées.
