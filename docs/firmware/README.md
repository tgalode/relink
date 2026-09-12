# Notes du lot firmware

Ce que la mesure sur matériel apprend au futur firmware. Même exigence que
`docs/protocol/` : chaque valeur cite sa source, et une observation sur
matériel décrit son montage — sans quoi elle n'est pas reproductible, donc pas
une source.

La différence avec `docs/protocol/` tient à ce qui est décrit. Là-bas, le
protocole : des faits sur les jeux, que la rétro-ingénierie établit et que
personne ne peut changer. Ici, le comportement d'une puce que nous avons
choisie : des faits sur *notre* montage, qui changeraient si nous changions de
cible.

Aucune constante du crate `protocol` n'est sourcée ici. Ce crate ne connaît ni
le temps ni les tensions — il ne voit que des octets.

## Documents de ce dossier

- [`latence-decodage.md`](latence-decodage.md) — le lien se décode-t-il par
  interruption sur `SCK` ? Latence mesurée au repos et radio allumée, coût de
  l'ordonnanceur, reprise à froid du cache de flash, et la marge que tout cela
  laisse à chaque cadence. Cinq entrées confirmées, deux conséquences de
  conception, et une méthode invalide signalée en fin de document.

- [`echange-sur-le-fil.md`](echange-sur-le-fil.md) — un échange Gen 1 complet
  joué sur matériel, `Session` tournant dans l'ISR, comparé au même échange
  joué en mémoire sur la même puce. Le flux sortant est identique. Le fil
  révèle en revanche que la réponse de la machine à états sort avec un créneau
  de retard — inhérent au registre à décalage, et pas encore vérifié sur
  console.

## Le banc

Les mesures viennent de [`tools/banc-esp32`](../../tools/banc-esp32), qui se
téléverse sur une carte ESP32 et n'exige que des straps entre broches — un
seul pour la latence, trois pour les échanges. Il est versionné pour que les
chiffres puissent être refaits, contestés, ou repris sur une autre carte.

Le binaire `echange-gen1` dépend de `relink-protocol` et fait tourner la
machine à états du dépôt telle quelle. C'est voulu : un banc qui réimplémenterait
le protocole ne testerait que lui-même.

## Ce qui reste ouvert

**La cadence réelle de l'échange Gen 2.** Les mesures montrent que le décodage
par interruption tient à 8192 Hz et s'effondre à l'horloge rapide du Game Boy
Color. Aucune source consultée ne dit laquelle l'échange Gen 2 emploie (cf.
`docs/protocol/link-couche-physique.md`, « Horloge rapide CGB »). Tant que
cette question n'est pas tranchée, la stratégie de décodage du firmware ne
peut pas être figée.
