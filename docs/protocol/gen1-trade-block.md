# Bloc d'échange Gen 1

Disposition du bloc de 415 octets échangé par câble link lors d'un échange
Pokémon Rouge/Bleu/Jaune. Ce bloc est envoyé après la graine aléatoire du
combat et contient le nom du dresseur ainsi que le détail complet de son
équipe : le partenaire peut donc consulter chaque Pokémon avant de choisir,
sans échange supplémentaire.

Sources principales, recoupées entre elles :

- Bulbapedia, [« Pokémon data structure (Generation I) »](https://bulbapedia.bulbagarden.net/wiki/Pok%C3%A9mon_data_structure_(Generation_I))
  — article de référence de la communauté, décrit les 44 octets d'un Pokémon
  et la structure d'équipe de 6 Pokémon en RAM (offsets relatifs au début de
  l'équipe, sans le nom du dresseur).
- nitwhiz, [« Spoofing a Pokémon (Red) Trade »](https://blog.nitwhiz.dev/posts/002-pokemon-red-trade/)
  — écriture d'un spoofer de câble link, capture le trafic réel via
  l'émulateur BGB (protocole série TCP) et documente la structure complète
  du bloc de 415 octets, nom du dresseur inclus. L'auteur dit avoir
  recoupé son observation avec le projet de désassemblage `pret/pokered` et
  Emulicious pour vérifier son interprétation ; seule sa description est
  reprise ici, aucun extrait de code désassemblé n'est cité.
- GBPlay, [« Emulating a Pokemon Trade with Generated Link Cable Data »](https://blog.gbplay.io/2021/05/11/Emulating-a-Pokemon-Trade-with-Generated-Link-Cable-Data.html)
  — confirme le déroulé (graine aléatoire puis bloc contenant le nom du
  dresseur et le détail de l'équipe) mais annonce « 424 octets » pour le
  bloc, sans détailler le calcul. Ce chiffre ne recoupe pas Bulbapedia et
  nitwhiz, ni la somme des sous-parties ci-dessous (qui fait exactement
  415) ; il n'est pas retenu. Cité ici uniquement pour le déroulé général,
  pas pour la taille.

## Taille totale du bloc

- **Ce que c'est** — taille totale du bloc d'échange transmis par câble.
- **Valeur / disposition** — 415 octets. Vérifiée par somme des cinq
  sections ci-dessous : 11 (nom du dresseur) + 8 (liste d'équipe) + 264
  (6 × 44, données d'équipe) + 66 (6 × 11, noms OT) + 66 (6 × 11,
  surnoms) = 415.
- **Source** — nitwhiz (annonce explicitement « 415 bytes » pour ce bloc,
  capturé sur trafic réel) ; recoupé par addition des offsets donnés par
  Bulbapedia (voir sections suivantes).
- **Confiance** — confirmée (deux sources indépendantes convergent, la
  troisième source — GBPlay, « 424 octets » — diverge et est écartée).

## Nom du dresseur

- **Ce que c'est** — nom du joueur propriétaire du bloc.
- **Valeur / disposition** — décalage 0, longueur 11 octets. Chaîne au
  format Game Boy (voir `gen1-charset.md`) : caractères puis terminateur
  0x50, le reste du champ étant du remplissage sans signification.
- **Source** — nitwhiz, structure Go `TradeBlock{ TrainerName [11]uint8; ... }`
  (le nom du dresseur précède la liste d'équipe, absente de la structure
  RAM documentée isolément par Bulbapedia). Bulbapedia confirme que la
  taille d'un champ de nom est 11 octets par la différence entre offsets
  successifs de sa table RAM (0x110 puis 0x11B pour l'OT name du premier et
  du second Pokémon, soit 0x0B = 11 octets d'écart), bien que le libellé de
  colonne « Size » de cette même table dise à tort « 10 Bytes » — la
  longueur maximale affichable d'un nom est 10 caractères, le onzième octet
  étant réservé au terminateur (confirmé par Bulbapedia, [« Nickname »](https://bulbapedia.bulbagarden.net/wiki/Nickname) :
  « In Generation I to V, nicknames have a maximum length of 10 characters
  in Western languages »).
- **Confiance** — confirmée pour le décalage 0 et la longueur 11 (recoupées
  par les deux sources) ; la longueur utile de 10 caractères + 1 terminateur
  est confirmée séparément par la page Bulbapedia sur les surnoms.

## Liste d'équipe

- **Ce que c'est** — nombre de Pokémon dans l'équipe suivi de la liste de
  leurs index d'espèce internes (voir `gen1-species-index.md`), utilisée par
  l'écran de sélection avant que les 44 octets détaillés ne soient
  consultés.
- **Valeur / disposition** — décalage 11. Forme : 1 octet de compteur, puis
  jusqu'à 6 octets d'index d'espèce, puis un octet terminateur 0xFF ; les
  emplacements de la liste au-delà de l'équipe réelle mais avant le
  terminateur ne sont pas spécifiés par les sources consultées. Taille
  réservée totale : 8 octets (décalages 11 à 18 inclus).
- **Source** — Bulbapedia, table RAM « 6-Pokémon Party Structure » :
  `0x00 Number of Pokémon in party (1 Byte)`, `0x01 List of Party Pokémon
  Species Index values (7 Bytes)` — soit 1 + 7 = 8 octets, le terminateur
  0xFF étant inclus dans les 7 octets de liste. nitwhiz, structure Go
  `PartySize uint8; PartyMembers [7]uint8 // terminated with 0xFF` — même
  découpage, avec confirmation explicite du terminateur.
- **Confiance** — confirmée (offset relatif et taille recoupés par les deux
  sources ; le décalage absolu de 11 découle du nom du dresseur précédent,
  lui-même confirmé).

## Données d'équipe

- **Ce que c'est** — les six emplacements de Pokémon d'équipe, structure
  détaillée en 44 octets chacun (voir plus bas).
- **Valeur / disposition** — décalage 19, 6 entrées de 44 octets chacune
  (264 octets au total, décalages 19 à 282 inclus). Les emplacements
  au-delà de la taille annoncée de l'équipe contiennent des octets sans
  signification garantie.
- **Source** — Bulbapedia, table RAM : structure d'équipe à l'offset relatif
  `0x08` (8 en décimal), après le compteur (1) et la liste d'espèces
  (7) = 8. Décalage absolu = 8 + 11 (nom du dresseur, qui précède dans le
  bloc complet mais n'apparaît pas dans la table RAM isolée de Bulbapedia)
  = 19. nitwhiz confirme cette valeur : struct `TradeBlock` place `Party
  [6]PartyData` directement après `PartySize` et `PartyMembers [7]uint8`
  (11 + 1 + 7 = 19).
- **Confiance** — confirmée.

## Noms de dresseur d'origine

- **Ce que c'est** — le nom du dresseur d'origine (OT) de chacun des 6
  emplacements d'équipe, dans le même ordre que la liste d'équipe.
- **Valeur / disposition** — décalage 283, 6 entrées de 11 octets chacune
  (66 octets, décalages 283 à 348 inclus). Même format de chaîne que le nom
  du dresseur.
- **Source** — Bulbapedia, table RAM : offset relatif `0x110` = 272
  décimal ; décalage absolu = 272 + 11 (nom du dresseur) = 283. nitwhiz,
  struct `TradeBlock`, champ `OriginalTrainerNames [6]Name` placé juste
  après `Party [6]PartyData` (19 + 6×44 = 283).
- **Confiance** — confirmée.

## Surnoms

- **Ce que c'est** — le surnom de chacun des 6 emplacements d'équipe, dans
  le même ordre que la liste d'équipe.
- **Valeur / disposition** — décalage 349, 6 entrées de 11 octets chacune
  (66 octets, décalages 349 à 414 inclus — dernier octet du bloc de 415).
- **Source** — Bulbapedia, table RAM : offset relatif `0x152` = 338
  décimal ; décalage absolu = 338 + 11 = 349. nitwhiz, struct `TradeBlock`,
  champ `Nicknames [6]Name` placé juste après `OriginalTrainerNames [6]Name`
  (283 + 6×11 = 349).
- **Confiance** — confirmée.

---

## Structure interne des 44 octets d'un Pokémon d'équipe

Champ par champ, décalages relatifs au début des 44 octets de ce Pokémon.
Sauf mention contraire, source : Bulbapedia, « Pokémon data structure
(Generation I) », table « The structure ». Confiance : confirmée pour tous
les champs de cette section (table explicite, nommant chaque décalage).

| Décalage | Champ | Taille |
|---|---|---|
| 0x00 | Index interne de l'espèce | 1 octet |
| 0x01 | PV courants | 2 octets |
| 0x03 | Niveau (tel que vu depuis la Boîte PC) | 1 octet |
| 0x04 | Statut | 1 octet |
| 0x05 | Type 1 | 1 octet |
| 0x06 | Type 2 | 1 octet |
| 0x07 | Taux de capture (recyclé en objet tenu lors d'un échange avec Gen 2) | 1 octet |
| 0x08 | Capacité 1 | 1 octet |
| 0x09 | Capacité 2 | 1 octet |
| 0x0A | Capacité 3 | 1 octet |
| 0x0B | Capacité 4 | 1 octet |
| 0x0C | Identifiant du dresseur d'origine | 2 octets |
| 0x0E | Expérience | 3 octets |
| 0x11 | Points d'effort — PV | 2 octets |
| 0x13 | Points d'effort — Attaque | 2 octets |
| 0x15 | Points d'effort — Défense | 2 octets |
| 0x17 | Points d'effort — Vitesse | 2 octets |
| 0x19 | Points d'effort — Spécial | 2 octets |
| 0x1B | DV (Attaque/Défense/Vitesse/Spécial) | 2 octets |
| 0x1D | PP de la capacité 1 | 1 octet |
| 0x1E | PP de la capacité 2 | 1 octet |
| 0x1F | PP de la capacité 3 | 1 octet |
| 0x20 | PP de la capacité 4 | 1 octet |
| 0x21 | Niveau (recalculé) | 1 octet |
| 0x22 | PV maximum | 2 octets |
| 0x24 | Attaque | 2 octets |
| 0x26 | Défense | 2 octets |
| 0x28 | Vitesse | 2 octets |
| 0x2A | Spécial | 2 octets |

Total : 0x2A + 2 = 0x2C = **44 octets**, confirmant la taille annoncée.

### Notes complémentaires par champ

- **Espèce (0x00)** — index interne, distinct du numéro national. Voir
  `gen1-species-index.md` pour la table de correspondance.
- **Ordre des octets des champs multi-octets** (PV, identifiant dresseur,
  expérience, points d'effort, statistiques) — poids fort en premier
  (« big endian »). Source : nitwhiz, qui l'affirme explicitement à propos
  de ce même bloc (« The data received by the Game Boy uses big endian »),
  observation faite sur trafic série réel plutôt que déduite. Bulbapedia ne
  précise pas l'ordre des octets dans cet article. Confiance : confirmée
  pour ce point précis, sourcée uniquement par nitwhiz — à surveiller si une
  seconde source indépendante devait un jour le contredire.
- **Statut (0x04)** — champ de bits : bit 3 (0x04) endormi, bit 4 (0x08)
  empoisonné, bit 5 (0x10) brûlé, bit 6 (0x20) gelé, bit 7 (0x40) paralysé.
  Source : Bulbapedia, section « Status conditions ». Confiance : confirmée.
- **Capacités (0x08 à 0x0B)** — quatre index de capacité d'un octet chacun,
  `0x00` signifiant emplacement vide. Source : Bulbapedia. Confiance :
  confirmée.
- **Identifiant du dresseur (0x0C, 2 octets)** — poids fort d'abord (voir
  note d'ordre des octets ci-dessus).
- **Expérience (0x0E, 3 octets)** — poids fort d'abord (voir note d'ordre
  des octets ci-dessus). La fixture du plan (`0x00, 0x4E, 0x20` →
  `0x004E20`) est cohérente avec cet ordre.
- **DV (0x1B, 2 octets)** — un quartet par statistique parmi Attaque,
  Défense, Vitesse, Spécial (le PV n'a pas de quartet propre). Répartition
  exacte :
  - premier octet : quartet haut = DV d'Attaque, quartet bas = DV de
    Défense ;
  - second octet : quartet haut = DV de Vitesse, quartet bas = DV de
    Spécial.

  Source : archive de [Pokémon Speedruns Wiki, « Pokémon Red/Blue/Wild
  DVs »](https://web.archive.org/web/20170227062531/https://wiki.pokemonspeedruns.com/index.php?title=Pok%C3%A9mon_Red/Blue_Wild_DVs)
  (citée en référence par Bulbapedia sur la page « Individual values ») :
  la génération des DV d'un Pokémon sauvage encode explicitement
  `16*Attack DV + Defense DV` dans le premier octet aléatoire consommé et
  `16*Speed DV + Special DV` dans le second — ce qui décrit directement
  l'agencement en octets utilisé par le jeu. Confiance : confirmée.

  Le DV de PV n'est pas stocké : il se déduit du bit de poids faible des
  quatre DV ci-dessus, dans l'ordre Attaque (poids 8), Défense (poids 4),
  Vitesse (poids 2), Spécial (poids 1). Source : Bulbapedia, « Individual
  values » : « The HP IV is calculated by taking the least significant bit
  […] of the Attack, Defense, Speed, and Special IVs […] a Pokémon with an
  odd-number Attack IV has 8 added to its HP IV, an odd-number Defense IV
  has 4 added, an odd-number Speed IV has 2 added, and an odd-number
  Special IV has 1 added. » Confiance : confirmée.
- **PP (0x1D à 0x20)** — par capacité, 1 octet : les 6 bits de poids faible
  sont le PP courant, les 2 bits de poids fort le nombre de PP Up
  appliqués. Source : Bulbapedia, section « PP ». Confiance : confirmée.
- **Niveau recalculé (0x21)** — c'est ce champ, et non celui de 0x03, qui
  sert de niveau courant en combat et en RAM d'équipe ; celui de 0x03 n'est
  significatif que pour un Pokémon stocké en Boîte PC. Source : Bulbapedia,
  section « Level ». Confiance : confirmée.
- **Statistiques calculées (0x22 à 0x2B)** — PV max, Attaque, Défense,
  Vitesse, Spécial : valeurs déjà calculées à partir des statistiques de
  base, DV et points d'effort, pas recalculées à la lecture. Source :
  Bulbapedia, sections « Maximum HP » et « Attack, Defense, Speed, and
  Special ». Confiance : confirmée.
