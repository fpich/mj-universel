# Lot L2.1 — Critères de validation

## Tests automatisés

- `cargo check` et `cargo test` passent, y compris les tests L0, L1 et L2.
- `llm::tests::mundane_dialogue_needs_no_roll` : le filtre protège une question simple et ne supprime pas la persuasion explicite.
- `llm::http_tests::an_unjustified_skill_check_is_corrected_once` : une proposition de jet pour une question reçoit une seule correction ; le résultat retenu est `auto_success`, aucun dé lancé.
- `storage::l2_tests::recurring_topic_recovers_old_dialogue` : une conversation ancienne, toujours dans les 500 événements consultés, revient dans le contexte si le sujet réapparaît.
- `storage::l2_tests::ai_atomic_and_facts` : les faits proposés sont étiquetés « pending » avant approbation et restent persistants.
- Un refus de l'API, JSON invalide ou correction invalide ne produit ni événement `action_resolved` ni lancer de dés.

## Scénarios manuels

1. Question ordinaire à un tavernier → dialogue sans jet ; le PNJ n'est pas forcé à donner une information.
2. Persuasion explicite avec obstacle et enjeu → jet possible après arbitrage.
3. Le joueur renoue une discussion avec un PNJ après plusieurs actions → le MJ tient compte des événements pertinents encore consultables.
4. Les déclarations des PNJ restent distinctes des vérités canoniques ; les nouvelles propositions passent par l'écran « Mémoire ».
5. Fermeture et réouverture → les campagnes et événements L2 sont conservés.

## Limites de validation

Tests de réseau réels, performance et exactitude des récits selon chaque modèle restent à vérifier localement. Le moteur ne garantit pas la qualité des arbitrages pour toutes les intentions naturelles possibles.
