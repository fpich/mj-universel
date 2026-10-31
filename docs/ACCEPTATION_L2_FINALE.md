# Recette de gel — L2 finale (v0.3.17)

**Périmètre :** consolider les lots L0, L1, L2 déjà validés en jeu, sans anticiper L3. Référence normative : `../specifications/specification_mj_universel_rust_v9_1_stable.md` (§13.7 et annexe C).

## Conditions minimales de réception

1. Sur Linux, avec Rust/Cargo disponibles, exécuter `cargo check`, `cargo test --lib`, puis `cargo test` ; conserver les sorties exactes.
2. Créer une nouvelle base SQLite : vérifier `PRAGMA user_version=27`, `PRAGMA quick_check=ok`, l’activation des clés étrangères et la création/reprise d’une campagne.
3. Réouvrir une base du schéma 27 avec une campagne existante ; vérifier la conservation des participants, inventaires, événements et positions.
4. Refuser une ancienne base `user_version=21` **sans en changer le contenu** ni lancer de migration. Refuser également une base non versionnée mais non vide ; vérifier les deux nouveaux tests unitaires.
5. Réexécuter les parcours d’idempotence : même clé d’opération, aucun deuxième jet après reprise/annulation, aucune double dépense ou création d’objet.
6. Rejouer une scène de 20–30 tours non combattants avec trois PNJ, échanges, promesse, mouvements et sauvegarde/reprise selon `ACCEPTATION_L2_4_3.md`.
7. Avec un fournisseur LLM réel, exécuter le scénario Takeshi / Rin / Akira de `ACCEPTATION_L2_4_4.md` ; en particulier, ne jamais inventer paroles, décisions ou déplacements d’un autre PJ.
8. Examiner les traces : rejet = aucune mutation ; état/positions valides après redémarrage ; aucune fuite de lieu privé d’un autre PJ.

## Décision

La v0.3.16 a été déclarée validée pour L0–L2 par le porteur du projet. **La v0.3.17 exige l'exécution des contrôles ci-dessus** avant que ce livrable puisse être qualifié de « gel final vérifié » sur Linux. Le rapport de préparation ne prétend pas remplacer ces résultats.

**Exclus de cette recette :** migration des anciens schémas (explicitement interdite), mécaniques L3, longues campagnes 300–500 tours, catalogue D&D Lite complet.
