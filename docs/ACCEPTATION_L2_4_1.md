> **Archive historique** : ce protocole concerne la version L2.4.1. Pour L2.4.2, suivre `ACCEPTATION_L2_4_2.md`. La devise principale obligatoire a ete retiree.

# L2.4.1 — Bibliothèque de scénarios et création sans modes

## Critères principaux

1. Lancer une **nouvelle campagne** : les mots « Libre », « Partiel » et « Complet » ne doivent pas figurer dans les formulaires de création ou d'édition. Seules les informations **titre, joueurs et contexte** sont nécessaires (le contexte peut être court ou vide).
2. Vérifier le catalogue de **20 scénarios** : genres différents, avec des propositions adaptées de **1 à 6 joueurs**. Tester la recherche « Vigo », « Sanctuaire », « cyberpunk » et le filtre « 6 joueurs ».
3. Choisir « Sanctuaire — Les Cendres du Pèlerin ». Confirmer que le titre, le **nombre de joueurs 2** et le contexte complet sont préremplis. Le contexte doit inclure **Gareth**, **Maëdela**, **Hirvo**, la **monnaie principale en pièces d'or**, ainsi que des départs différents pour **PJ1** et **PJ2**.
4. Modifier une phrase du contexte : le changement doit rester en place jusqu'à la création. Changer le nombre de joueurs si nécessaire.
5. Avec la fenêtre réduite à la taille minimale autorisée (760×540), vérifier que **« Créer et préparer les personnages » et « Annuler » restent visibles**. Dérouler les 20 scénarios et le contexte long avec souris/touchpad et barre de défilement.
6. Cliquer sur **Créer et préparer les personnages** ; la campagne doit s'ouvrir. Revenir aux campagnes : le titre et le contexte doivent être enregistrés en SQLite.
7. Dans la préparation, changer un contexte long et enregistrer ; la zone défile et la création des fiches reste disponible. Générer trois propositions pour chaque joueur, en sélectionner une, et vérifier les monnaies et inventaires comme dans L2.4.
8. Vérifier le **mode sans scénario** : ne pas sélectionner de modèle, saisir « Vigo en 1800 », un titre, 2 joueurs et créer la campagne. Aucune question sur le degré de détail ne doit apparaître.
9. Ne pas régresser : `cargo check && cargo test && cargo run`, puis scénario de jeu pour les contrôles mécaniques de L2.4.

## Rétrocompatibilité et données

- Le champ `mode` est **conservé uniquement pour compatibilité interne** avec le schéma et les anciennes campagnes ; les nouvelles campagnes sont créées sous la valeur technique `free` sans exposer de modes dans l'interface.
- Le contexte existant n'est jamais écrasé sans clic explicite sur **« Utiliser ce scénario »**.
- L2.4.1 ouvre une nouvelle base `mj-universel-l2-4-1.sqlite3` (schéma 25), sans toucher à la base L2.4.
- Le nombre de joueurs conseillé est celui du modèle ; l'utilisateur peut le modifier avant la création.
- **La compilation Rust et l'essai graphique restent à effectuer sur Linux** : Cargo n'est pas disponible dans cet environnement.
