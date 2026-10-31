# MJ Universel Rust — L2 finale (v0.3.17)

**Base de travail :** v0.3.16, validée pour L0–L2 selon les essais du porteur du projet. Cette livraison ajoute uniquement un durcissement de l’ouverture SQLite et de la documentation de gel. **Elle n'ajoute aucune fonctionnalité L3.**

## Repères dans l'archive

- `specifications/specification_mj_universel_rust_v9_1_stable.md` — **référence normative V9.1 mise à jour**, avec la décision explicite « zéro migration des anciennes bases SQLite » (§13.7, annexe C).
- `specifications/plan_demarrage_mj_rust.md` — plan initial harmonisé ; son statut historique « aucune fonctionnalité implémentée » ne décrit **pas** la version actuelle.
- `docs/ACCEPTATION_L2_FINALE.md` — recette de vérification à exécuter sur la machine de développement.
- `docs/RAPPORT_STABILISATION_L2.md` — modifications, vérifications et limites.
- `docs/ACCEPTATION_L2_4_4.md` — régressions sur Takeshi/Rin/Akira.
- `docs/README_L2_4_3_HISTORIQUE.md` — notes de la version antérieure conservées pour traçabilité.

## Exécuter sur Linux

```bash
cargo check
cargo test --lib
cargo test
cargo run
```

**Important :** la compilation Rust n’a pas été possible dans l’environnement de préparation (Cargo/Rust absents). Les deux nouveaux tests ont été écrits mais **pas exécutés** ; la validation précédente de la v0.3.16 ne constitue pas une validation automatique de la v0.3.17.

## Bases SQLite et compatibilité

Schéma embarqué : **27**. Les bases compatibles sont réutilisées sans migration. Le chemin par défaut reste `mj-universel-l2-4-3.sqlite3` pour préserver les campagnes **du même schéma** déjà créées. Un schéma incompatible, même non versionné mais non vide, est refusé avant l’activation du mode WAL ; aucun effacement, conversion ou modification volontaire. Ne pas utiliser une ancienne base d’un autre schéma comme base de cette version.

Les historiques de recettes L1/L2 restent sous `docs/`. Les fonctions avancées de L3 (croyances/claims, temporalité, directeur narratif, compactage, branches) ne sont pas ajoutées ni déclarées terminées.
