# Ce que le fil ajoute à la machine à états

Un échange Gen 1 complet a été joué sur du matériel, `Session` tournant dans
l'ISR. Ce document consigne ce que cette exécution apprend, et qui n'était pas
visible depuis les tests en mémoire.

**Récapitulatif de confiance** (2 entrées) : **1 confirmée**, **1 conséquence
de conception**.

## Le montage

- **Carte et câblage** — la même qu'en
  [`latence-decodage.md`](latence-decodage.md), avec deux straps de plus :
  `D26 → D19` pour les données de la cartouche vers le module, `D21 → D27`
  pour celles du module vers la cartouche. Trois lignes, tout en 3,3 V.
- **Banc** — [`tools/banc-esp32`](../../tools/banc-esp32), binaire
  `echange-gen1`.
- **Scénario** — l'échange de `crates/protocol/tests/session_echange.rs`, à
  l'octet près : la cartouche de `relink_protocol::testing` propose son
  Pokémon d'indice 3, le module offre le sien d'indice 0, les deux acceptent.
  665 octets, cadencés à 8192 Hz par le rôle cartouche, qui n'attend personne.
- **Témoin** — le même échange joué en mémoire, sur la même puce, juste avant.
  Aucune valeur attendue n'est figée dans le banc : la référence est produite
  par la session elle-même. Un écart ne peut donc venir que de ce qui sépare
  les deux exécutions, c'est-à-dire du temps réel.
- **Date** — 2026-09-12.

## L'échange passe

- **Ce que c'est** — la machine à états du crate, alimentée par un vrai fil.
- **Valeur** — flux sortant **identique** à la référence sur les 664 octets
  comparables, à chaque passe. Tous les jalons atteints : lien établi, bloc du
  partenaire reçu et intact, offre demandée, proposition du partenaire lue à
  l'indice attendu, verdict demandé, accord conclu.
- **Source** — mesure directe, montage ci-dessus. `Session::step()` est appelé
  depuis l'ISR, pas depuis une boucle de test : c'est la configuration du
  futur firmware, pas une approximation.
- **Confiance** — confirmée. **Portée** : cela prouve l'accord entre la
  machine à états et *la cartouche simulée*, pas avec une console. Cette
  doublure vaut ce que vaut le sourçage de
  [`../protocol/gen1-link-protocol.md`](../protocol/gen1-link-protocol.md).

## L'octet sortant a un créneau de retard

- **Ce que c'est** — un écart entre le modèle du crate et ce que le matériel
  peut faire, que seule une exécution sur le fil pouvait rendre visible.
- **Valeur** — `Session::step(entrant) -> sortant` modélise un échange
  **simultané** : les deux registres à décalage se croisent, et l'octet rendu
  est traité comme parti en même temps que l'octet reçu. Le matériel ne peut
  pas faire ça. Pendant le créneau `i`, le module décale un octet qu'il devait
  avoir **déjà chargé** avant que le créneau ne commence ; sa réponse à
  l'octet `i` ne peut donc sortir qu'au créneau `i+1`.

  Le banc en tient compte explicitement — il compare `observé[i+1]` à
  `référence[i]` — plutôt que de le masquer, et le créneau 0 transporte
  l'octet préchargé, qui ne répond à rien.

- **Source** — conséquence directe du fonctionnement d'un registre à
  décalage, rendue visible par la comparaison entre l'exécution sur le fil et
  celle en mémoire.
- **Confiance** — **conséquence de conception.** Ce n'est pas un défaut de la
  machine à états : le décalage est inhérent au lien, et le §5.1 de la
  conception le dit déjà à sa manière — « l'octet sortant doit être prêt avant
  le front ». Ce que la mesure ajoute, c'est la taille de la marge : **un
  créneau entier, soit 122 µs à 8192 Hz**, pour préparer l'octet suivant. Le
  décodage consomme 2,7 µs de ce budget (cf.
  [`latence-decodage.md`](latence-decodage.md)).

  **Ce qui reste à vérifier sur console** : que le jeu tolère ce décalage,
  c'est-à-dire qu'aucune phase du protocole n'attende une réponse dans le
  créneau même où elle a posé sa question. Rien dans les sources consultées ne
  suggère le contraire — le protocole est une suite d'octets de remplissage
  ponctuée de valeurs signifiantes, ce qui laisse de la place — mais la
  cartouche simulée ne peut pas trancher ce point, puisqu'elle déroule un
  programme fixe sans jamais réagir à ce qu'elle reçoit.
