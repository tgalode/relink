# Diagrammes

Fichiers HTML autonomes : aucun build, aucune image externe, ouvrables
directement dans un navigateur. Produits avec le plugin
[diagram-design](https://github.com/cathrynlavery/diagram-design) ; la charte
retenue pour le dépôt est celle livrée par défaut, fixée par le marqueur
`.diagram-design` à la racine.

## Structure

- [`architecture.html`](architecture.html) — la chaîne complète, de la
  cartouche au pool. Trait plein pour ce qui est livré, pointillés pour ce
  qui reste à faire. L'accent est sur `crates/protocol`, le seul code
  partagé entre le module et le service.

- [`materiel.html`](materiel.html) — les briques du module. **Cadrage, pas
  décision** : aucun choix matériel n'est arrêté dans ce dépôt. Le schéma
  distingue ce que la console impose de ce qui reste à trancher, et nomme
  chaque case ouverte. Il sert d'entrée au futur lot firmware.

## Schémas électriques

Deux schémas, pas de principe mais d'électronique : symboles de composants,
nets à angles vifs, masse, cartouche. Ils suivent les conventions de
l'électricité plutôt que celles des autres diagrammes de ce dossier, parce
qu'un schéma qui se câble ne se lit pas comme un schéma qui s'explique.

- [`banc-esp32.html`](banc-esp32.html) — **le banc de mesure.** Un seul strap
  entre `D25` et `D18`, tout en 3,3 V, aucun composant : la carte se parle à
  elle-même, une moitié jouant la cartouche. C'est le montage des mesures de
  [`../firmware/latence-decodage.md`](../firmware/latence-decodage.md).

- [`interface-link.html`](interface-link.html) — **l'interface réelle,
  proposée.** L'étage d'adaptation entre le port link en 5 V et l'ESP32 en
  3,3 V : un `74LVC245` pour les deux signaux entrants, un `74AHCT245` pour le
  sortant, découplage, et la broche d'alimentation de la console laissée non
  connectée. **Proposition, pas décision** — comme `materiel.html`. Le
  brochage vient de
  [`../protocol/link-couche-physique.md`](../protocol/link-couche-physique.md),
  qui porte aussi la justification des deux boîtiers.

## Les quatre parcours, dans le temps

Un diagramme de séquence par parcours produit. Ils partagent la même
grammaire : trait plein pour un appel, pointillés pour une réponse, tête de
flèche creuse pour un message asynchrone, et un seul accent, sur l'octet qui
conclut l'échange.

- [`sequence-evolution.html`](sequence-evolution.html) — **évolution par
  échange.** Deux participants, aucun réseau : le module fabrique le
  partenaire sur place. C'est le parcours qui marche sans compte et sans
  serveur.
- [`sequence-depot.html`](sequence-depot.html) — **dépôt dans le pool.** Le
  module présente un leurre, la cartouche cède son Pokémon, le service
  commite. Le fragment `opt` montre ce qui se passe quand l'acquittement se
  perd : le rejeu retombe sur le même commit et ne duplique rien.
- [`sequence-retrait.html`](sequence-retrait.html) — **retrait du pool.** Le
  joueur réserve depuis l'application mobile, l'échange ne fait plus que
  livrer.
- [`sequence-echange-direct.html`](sequence-echange-direct.html) — **échange
  direct entre deux joueurs.** Le fragment `loop` porte le mécanisme du §5.2
  de la conception : tant que le joueur distant n'a pas tranché, le module
  présente l'octet neutre et la cartouche patiente sans échéance.

## Ce qui n'est pas dessiné

- **L'expiration d'une réservation.** C'est une branche du retrait, pas un
  parcours : la dessiner demanderait un second fragment dans un diagramme qui
  en a déjà un.
- **Le choix des composants.** Le schéma matériel cadre les briques ; il ne
  dit ni quel microcontrôleur, ni quelle connectivité, ni quelle
  alimentation. Ces décisions appartiennent au lot firmware et devront être
  écrites avant d'être dessinées. `interface-link.html` dessine un étage
  d'adaptation sourcé, ce qui ferme la case « tensions à confirmer » — il ne
  ferme pas les autres.
- **L'implantation des broches sur la carte.** Un schéma montre des nets, pas
  des emplacements, et l'ordre des broches varie d'une révision de carte à
  l'autre. `D25` et `D18` se repèrent à la sérigraphie.
- **La machine à états de l'échange.** Ses douze phases et leurs transitions
  relèvent d'un diagramme d'états, pas d'une séquence ; le tableau de
  [la conception](../superpowers/specs/2026-08-27-gen1-machine-a-etats-design.md)
  en tient lieu pour l'instant.

## Exporter

Les diagrammes sont en HTML pour rester relisibles en diff. GitHub ne les
rend pas dans la page : pour un visuel affiché directement dans le README, il
faut un export PNG ou SVG, que le plugin produit à la demande.
