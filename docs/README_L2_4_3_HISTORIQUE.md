> Correctif v0.3.16 : protections des autres PJ contre les réactions/paroles inventées par le MJ ; contrôle de proximité et mouvements narrés sans effet Rust. La base SQLite reste en schéma 27. Voir `docs/ACCEPTATION_L2_4_4.md`.

# MJ Universel — L2.4.3 (v0.3.15) — Cohérence multijoueur et narrative

**Candidate de correctif** construite sur L2.4.2, avec les créations IA de niveau 1, les 20 scénarios préchargés, les monnaies libres et le verrouillage des fiches conservés.

## Nouveautés

1. **Position distincte pour chaque PJ**, mémorisée dans `actor_scenes` avec une clé de scène normalisée ; les déplacements sont enregistrés atomiquement avec l'action. Deux formulations telles que « sur la place du vieux puits, à Pierre-Pluie » et « place du vieux puits » correspondent à la même scène. En revanche « devant la porte » ne se confond pas avec « porte ».
2. **Contexte multijoueur** : l'IA reçoit les autres PJ se trouvant dans la **même scène canonique**, sans révéler la position de ceux qui sont ailleurs. Leur biographie privée n'est pas divulguée.
3. **Correction narrative ciblée** : pour une question sur les personnes présentes, ou l'affirmation qu'un PJ présent serait absent, une réponse omettant ce PJ doit être révisée une fois par le MJ IA. Rust interdit que cette correction modifie l'effet mécanique, le jet, le DD ou les autres données. Aucune réponse incohérente n'est enregistrée en cas d'échec de réparation.
4. **PNJ improvisés** : le MJ peut proposer `introduce_npc` ou `introduce_npcs` (1 à 4) pour nommer plusieurs habitants dans une seule interaction. Rust vérifie l'absence de collision avec des PNJ ou PJ existants et persiste les introductions dans une transaction.
5. **Continuité du dialogue** : le dernier tour du PJ agissant reste indiqué même si plusieurs autres PJ ont pris la parole entre-temps.
6. **Déplacements et obstacles** : les instructions du MJ interdisent de traverser narrativement une porte bloquée sans résolution préalable. **Limite importante :** le schéma L2 ne dispose pas encore d'une topologie complète et de toutes les conditions d'accès structurées ; Rust ne peut donc pas encore vérifier à lui seul chaque obstacle déclaré en langage naturel. Ce point reste à tester en partie et à développer pour une garantie mécanique exhaustive.

## Base de données isolée

Nouvelle base : `mj-universel-l2-4-3.sqlite3` ; **schéma SQLite 27**, sans migration automatique. Les bases des versions précédentes ne sont pas modifiées.

## Démarrer

```bash
cargo check && cargo test && cargo run
```

La compilation Cargo nécessite ton poste Linux (Rust indisponible dans l'environnement de préparation). Les contrôles SQL du schéma ont été effectués.

## Vérification de stabilisation

Voir `docs/ACCEPTATION_L2_4_3.md`. Cette version n'est pas déclarée L2 figée avant les tests utilisateurs.
