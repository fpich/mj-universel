# L2.2 — Critères d'acceptation (par rapport à V9.1)

## Jets autorisés par le moteur

- Une simple question au tavernier ne demande aucun jet et apparaît dans le récit.
- Une persuasion justifiée produit un arbitrage DD/aptitude/enjeux **persisté avant le d20**. La réponse du MJ ne doit pas prétendre que le jet a déjà été effectué.
- Si aucun jet n'est en attente, il est impossible d'accéder à un bouton de lancer de dés dans l'UI RP.
- Une action en attente empêche une autre action, mais laisse lire l'historique et les fiches.
- Un joueur peut renoncer avant le lancer avec confirmation ; le journal consigne cette décision.
- Après clic sur le bouton de lancer : calcul Rust, d20 brut, modificateur, comparaison DD, résultat et narration conditionnelle ; un seul événement de résolution durable.
- Fermeture avant lancer : attente conservée, aucun dé tiré ni résultat inventé.
- Fermeture après lancer : résultat conservé et aucun nouveau lancer lors de la reprise.

## Chat global RP

- Une chronologie unique contient toutes les actions des personnages et les narrations du MJ.
- Au cours de la séance, les tours des joueurs et la réponse du MJ restent lisibles sans quitter la page de jeu.
- L'historique peut être remonté par tranches de 120 messages.
- Les événements techniques ne polluent pas la narration. Le détail des dés est disponible en petit sous le récit ; les JSON complets sont dans le journal séparé.
- Les issues alternatives non réalisées ne sont pas révélées au joueur dans le journal technique.

## Persistance et sécurité

- L2.1 SQLite v21 → L2.2 SQLite v22 migrée sans perte de personnage, inventaire, secret chiffré, événement ou campagne.
- En attente de d20, les mutations de fiche/inventaire et l'approbation de faits sont refusées côté Rust/SQLite.
- Les données de campagne sont isolées par identifiant de campagne ; aucun accès SQL fourni au modèle IA.
- Les clés API restent chiffrées comme dans L2.1.

## Limites reportées à L2.3 / L3

- Dépense de ressource libre et transfert d'objet (ex. crédits) **non implémentés automatiquement**.
- Transferts et conséquences mécaniques typées pour les actions composées : L2.3.
- Simulation des PNJ autonomes, croyances et mémoire longue durée : L3 et après.

## Vérification de compilation

Lancer `cargo check && cargo test && cargo run` et rapporter les erreurs si besoin : ces commandes n'ont pas pu être exécutées dans l'environnement de création.
