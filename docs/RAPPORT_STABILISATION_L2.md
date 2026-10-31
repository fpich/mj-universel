# Rapport de préparation — L2 finale v0.3.17

## Objectif et base

Archive source v0.3.16 (`mj-universel-coherence-multijoueur-v0.3.16.zip`), schéma SQLite 27. Les lots L0–L2 étaient déjà validés selon le retour utilisateur ; ce rapport ne requalifie pas leurs fonctionnalités.

## Changements ciblés

- `src/storage.rs` : contrôle de `PRAGMA user_version` et de l’absence d’objets utilisateur **avant** l’activation du mode `journal_mode=WAL` ou l’exécution du schéma. Une base étrangère ou non versionnée mais non vide est refusée sans transformation.
- `src/storage.rs` : deux tests de non-régression : schéma 21 inchangé (comparaison des octets, absence de fichiers WAL/SHM) et base non versionnée non vide inchangée.
- `Cargo.toml`, `src/main.rs`, `README.md` : identification claire de la version candidate de gel ; chemin SQLite conservé, schéma inchangé.
- `specifications/` : ajout de la V9.1 amendée et du plan de démarrage harmonisé ; **aucune migration d’anciennes bases**.
- `docs/` : recette L2 finale et conservation de toutes les recettes historiques.

## Vérifications effectuées dans l’environnement de préparation

- Intégrité de l’archive, présence des spécifications, comparaison des fichiers source inchangés hors ceux cités ;
- Exécution du SQL embarqué avec SQLite Python : version 27, 18 tables, `PRAGMA quick_check=ok` ;
- Contrôle de présence des nouveaux tests et des gardes de validation ;
- **Non exécutés :** `cargo check`, `cargo test`, validation de l'interface et test contre le fournisseur IA réel : Cargo/Rust indisponibles.

Le nouveau contrôle d’ouverture SQLite ne modifie ni les règles de jeu, ni les décisions MJ, ni les effets UARE, ni les informations LLM.

## Limites conservées et orientation L3

- Le contrôle du récit des PJ est lexical, pas une preuve sémantique exhaustive du français.
- La co-présence dépend d’une clé de scène ; pas de graphe spatial complet.
- La mémoire de croyances et déclarations, les agendas des PNJ, l’horloge événementielle, le compactage long terme et les branches sont réservés à L3 conformément à l’annexe C.4.
- `Cargo.lock` n’est pas fourni, car il n’a pas pu être généré ni vérifié sans Cargo ; un verrouillage de dépendances reste souhaitable sur la machine de développement.

## État

**Archive préparée et vérifiée statiquement ; gel final de la v0.3.17 conditionné au retour des vérifications Cargo et de la recette terrain.**
