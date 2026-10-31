# L2.4.2 — Protocole monétaire

1. `cargo check && cargo test` doivent réussir.
2. Créer une campagne de troc, générer des personnages sans devises de départ ; aucune monnaie n'est exigée.
3. Créer un personnage avec 10 pièces d'or ET 20 pièces d'argent : deux soldes séparés sont visibles.
4. Créer un second personnage avec uniquement des pièces de cuivre (ou aucun argent) : le premier PJ ne force pas sa devise.
5. Vérifier l'échec de la validation d'un JSON proposant deux fois la même devise sous une casse différente ou une somme excessive (> 10 000 par devise).
6. Tester une dépense au-delà d'un solde : Rust refuse ; aucune conversion entre devises implicite.
7. Fermer et reprendre : les trois portefeuilles et les ressources sont persistants.
8. Vérifier la création et les boutons de validation visibles en bas malgré un contexte long.
