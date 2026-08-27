# Contribuer à relink

## Hygiène de rétro-ingénierie

Ces règles ne sont pas négociables. Ce sont les conditions d'existence du
projet, et la raison pour laquelle le dépôt est public depuis son premier
commit : l'historique doit pouvoir montrer que l'hygiène a été tenue depuis le
début, parce qu'elle ne se reconstitue pas après coup.

### Ce qui n'entre jamais dans le dépôt

- une ROM, un extrait de ROM, un désassemblage ;
- une sauvegarde de cartouche, la vôtre comprise ;
- du code provenant de Nintendo, de Game Freak ou de The Pokémon Company ;
- une marque déposée dans un nom de crate, de domaine ou de ressource ;
- des ressources graphiques ou sonores tirées des jeux.

### Ce qu'on exige à la place

**Toute constante du protocole cite sa source.** Chaque champ du bloc
d'échange, chaque valeur d'octet du handshake, chaque décalage est documenté
dans `docs/protocol/` avec l'origine de l'information : observation sur
matériel, capture de trafic série, ou documentation communautaire nommément
citée.

Une constante sans source est un bug, même si elle marche.

**Les fixtures de test sont synthétiques.** Elles sont construites par le code,
ou écrites à la main et sourcées. Aucune donnée capturée sur une cartouche
tierce.

## Style

`protocol` est du cœur fonctionnel pur : pas d'I/O, pas d'allocation dans le
chemin critique, pas de `panic!` dans `step()`. Si une fonction de ce crate
peut échouer à l'exécution, c'est un problème de conception, pas un cas à
gérer.

`application` déclare des ports en traits et n'en implémente aucun.

## Tests

Le test d'invariant du commit — *jamais de duplication* — précède le code qu'il
protège. Ce n'est pas une préférence de méthode : c'est le seul endroit du
projet où un bug détruit des données irremplaçables.
