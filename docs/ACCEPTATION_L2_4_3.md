# Acceptation L2.4.3 — cohérence des scènes et PNJ

## Préparation

- `cargo check && cargo test && cargo run`.
- Nouvelle campagne 2 PJ, Sanctuaire (contexte chargé dans les 20 scénarios), personnages **Elias Valtierra** et **Sœur Elara de la Voie**.
- Positions initiales **Elias : devant la porte ouest de Pierre-Pluie** ; **Elara : sur la place du vieux puits, à Pierre-Pluie**.
- Ne pas importer une base L2.4.2 dans le nouveau schéma 27.

## Tests de récit (ordre)

| N° | Action | Résultat attendu |
|---|---|---|
| 1 | Elias : « Où suis-je ? » | Porte ouest, et pas la place. |
| 2 | Elara : « Où suis-je ? » | Place du vieux puits ; pas de déplacement implicite. |
| 3 | Elias : « Comment rejoindre Elara ? » | Pas de distance précise inventée, ni de secrets biographiques privés divulgués. |
| 4 | Elias : « Je vais sur la place du vieux puits. » | Si la porte est fermée, le MJ doit d'abord présenter l'obstacle et ne pas proposer `move`. Pour tester la co-présence, résoudre l'entrée avec les gardes ou commencer à l'intérieur. |
| 5 | Elias entre réellement sur la place après résolution | `✓ Rust · Déplacement validé`, position canonique sauvegardée. |
| 6 | Elias : « Qui est présent ? » | Le MJ mentionne Elara, sauf dissimulation ou obstacle de perception explicitement établi. |
| 7 | Elara : « Il y a qui autour de moi ? » | Le MJ mentionne Elias si sa position est identique et sa présence perceptible. |
| 8 | Elara : « Elias n'est pas là. » | Le MJ traite **Elias** et nuance la perception, sans répondre à un ancien sujet sans rapport. |
| 9 | Elara : « Je demande aux deux hommes leurs noms. » | Introductions de Brondan/Jorik possibles, enregistrées dans les contacts ; pas de PNJ inventé par le joueur comme vérité imposée. |
| 10 | Reposer la question sur Brondan et Jorik | Pas de duplication, noms cohérents. |
| 11 | Rejouer une autre action de déplacement d'Elias | Changement canonique propre à Elias seulement. |
| 12 | Quitter puis rouvrir l'application | Positions indépendantes et contacts conservés. |

## Cas négatifs

- Un PNJ ne doit pas avoir le même nom qu'un PJ, y compris avec variation de casse.
- Un `introduce_npcs` avec noms en double doit échouer entièrement (zéro introduction).
- Une réparation narrative doit être refusée si elle change les ressources, les effets ou les paramètres des dés.
- Un texte indiquant une rue ou une ville générale ne permet pas d'inférer automatiquement une distance précise.
- Le texte de départ ne remplace pas une confirmation structurée de passage à travers une porte gardée : **constater ce comportement séparément**.

## Conditions de gel

Tests Cargo réussis, journal de jeu cohérent sur 20–30 tours, puis redémarrage et vérification de l'état ; toute anomalie reproductible entraîne un correctif supplémentaire. Les restrictions de passage purement narratives restent à qualifier.
