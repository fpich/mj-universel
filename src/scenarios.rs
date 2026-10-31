//! Scénarios de démarrage inclus dans le programme, sans téléchargement ni fichier utilisateur.
//! Ils fournissent un contexte librement éditable avant création de la campagne.

#[derive(Debug, Clone, Copy)]
pub struct ScenarioPreset {
    pub title: &'static str,
    pub genre: &'static str,
    pub recommended_players: u8,
    pub summary: &'static str,
    pub context: &'static str,
}

pub const SCENARIOS: &[ScenarioPreset] = &[
    ScenarioPreset {
        title: r####"Sanctuaire — Les Cendres du Pèlerin"####,
        genre: r####"Dark fantasy gothique"####,
        recommended_players: 2,
        summary: r####"Une vallée corrompue, un bourg assiégé et deux héros séparés."####,
        context: r####"Sanctuaire se meurt. Il y a des mois, le Pèlerin Sombre a traversé Tristram ; depuis, récoltes, puits et sépultures se corrompent. Dans la vallée isolée de Sangvent, une influence appelée l'Éveil nourrit les trahisons et le retour des morts. Sa véritable nature demeure inconnue des personnages.

LIEUX ÉTABLIS : Pierre-Pluie, bourgade fortifiée et dernier refuge ; le couvent des Ravelées, tombé il y a trois semaines et dont les cloches résonnent sans sonneurs ; les mines de Fer-Noir, abandonnées après la découverte de créatures ; la Forêt-Sans-Nom au nord, où les arbres penchent dans la même direction ; les tertres des Anciens, cimetière druidique couvert de runes perdues. Les autres lieux et secrets peuvent être découverts progressivement.

PNJ ÉTABLIS : Gareth Branche-de-Fer, capitaine de la milice, réaliste et épuisé ; Maëdela Voss, dernière mage Vizjerei de la vallée, discrète et prudente ; le vieux Hirvo, fossoyeur affirmant converser avec les morts. Sept sœurs Ravelées auraient survécu au désastre ; leurs identités et leur état ne sont pas tous connus. Les Ferrés, les Fidèles de l'Écoute, les dépouilles levées et les Ravelées déchues menacent la région. Ce que les villageois disent d'eux n'est pas toujours confirmé.

TENSION : Pierre-Pluie manque de vivres ; la milice contrôle strictement les entrées. Les survivants se méfient des étrangers. L'Éveil peut influencer les décisions, mais son fonctionnement exact reste à découvrir. Le MJ ne révèle pas à un PJ un secret qu'il n'a pas appris.

CRÉATION : proposer des personnages de niveau 1 adaptés à Sanctuaire, parmi Barbare, Amazone, Nécromancien, Sorcière/Sorcier, Paladin, Assassin, Druide ou un autre rôle cohérent. Les capacités mécaniques doivent exister dans les règles Lite de Rust. Identité, âge, sexe/genre, taille, poids, apparence et équipement appropriés. Économie plausible : pièces d'or, d'argent, troc et monnaies étrangères possibles ; aucune monnaie unique obligatoire. Les dotations varient selon le rôle.

DÉPART À DEUX PJ : PJ1 attend sous la pluie devant la porte ouest de Pierre-Pluie, parmi les voyageurs ; PJ2 se trouve sur la place du vieux puits, à l'intérieur du bourg. Ils ne se connaissent pas nécessairement et ignorent automatiquement la position de l'autre.

AMORCE PJ1 : le garde sur les remparts demande : « La nuit dernière, as-tu entendu une voix t'appeler par ton nom ? » AMORCE PJ2 : deux habitants interrompent leur conversation devant le vieux puits ; l'un murmure : « Cette nuit, quelqu'un a encore entendu la voix. » Le MJ attend les décisions des joueurs ; il n'impose ni quête ni combat.

TON : gothique, sombre et humain ; espoir rare, conséquences durables. Aucune statistique ni jet narratif inventé ; les dés interviennent seulement si Rust les autorise. Le monde peut être dangereux, mais chaque PJ conserve le contrôle de ses actes."####,
    },
    ScenarioPreset {
        title: r####"Vigo 1800 — Les Voiles de Contrebande"####,
        genre: r####"Historique maritime"####,
        recommended_players: 2,
        summary: r####"Douanes, pêcheurs et sociétés secrètes dans une ville portuaire."####,
        context: r####"Vigo, en Espagne, année 1800. Le port accueille des pêcheurs, des négociants et des marins étrangers. Un navire est rentré sans son capitaine et un registre commercial a disparu des bureaux des douanes.

LIEUX : marché aux poissons, jetées, vieille ville, entrepôts et chemins menant aux villages alentour. PNJ : Inés Costa, aubergiste qui écoute tout ; Duarte Salgado, employé des douanes ; Tomás Lira, pêcheur en conflit avec un marchand. Les intrigues sont ouvertes : le registre peut avoir été volé, perdu ou dissimulé, à déterminer par le MJ sans imposer de vérité rétroactive.

ÉCONOMIE : réaux espagnols, devises étrangères et troc possibles ; aucune devise unique obligatoire. Équipement et professions compatibles avec la période. Départ : PJ1 sur les quais ; PJ2 devant l'auberge du quartier haut. Les deux joueurs peuvent se rejoindre ou s'ignorer. Pas de technologie ni de magie anachroniques. Les faits historiques servent de décor, sans simulation exhaustive."####,
    },
    ScenarioPreset {
        title: r####"Tharsis 2189 — Les Disparus du Secteur 4"####,
        genre: r####"Science-fiction enquête"####,
        recommended_players: 3,
        summary: r####"Une station orbitale, des mines martiennes et des signaux effacés."####,
        context: r####"En 2189, la station Atlas orbite autour de Mars. Des mineurs du secteur 4 ont disparu ; les systèmes de suivi indiquent pourtant des sorties de service ordinaires. Une navette a modifié son manifeste peu avant l'incident.

LIEUX : bar Le Sillage, docks sud, terminal des navettes, habitats de maintenance, mines de Tharsis. PNJ : Mara, tenancière pragmatique ; Jax, mécanicien prudent ; Nora Venn, technicienne dont le sort reste inconnu ; Dirk Voss, contremaître contesté, dont les liens avec les PJ ne sont pas présumés.

ÉCONOMIE : crédits. Matériel initial rationnel : badge d'accès selon fonction, rations, communicateur, outils. Départ : les trois PJ se trouvent dans trois secteurs distincts de la station. Les données privées de chacun ne sont pas connues des autres sans transmission. Mystère ouvert, aucune quête ni combat obligatoire."####,
    },
    ScenarioPreset {
        title: r####"Néo-Kyoto — Les Jardins de Verre"####,
        genre: r####"Cyberpunk urbain"####,
        recommended_players: 4,
        summary: r####"Une ville verticale, des implants et des mémoires volées."####,
        context: r####"Néo-Kyoto, 2097. Une pluie acide tombe sur une mégapole où les jardins des tours appartiennent à des multinationales. Des souvenirs personnels apparaissent à vendre sur un marché clandestin. Les propriétaires d'origine affirment ne rien avoir vendu.

LIEUX : marché souterrain d'Asagiri, jardins suspendus de la tour Hikari, métro interdit, clinique des implants. PNJ : Sumi, restauratrice de données ; Docteur Park, chirurgien indépendant ; inspecteur Arata, enquêteur à la loyauté ambiguë.

ÉCONOMIE : yens numériques. Les implants n'accordent pas automatiquement des avantages mécaniques ; Rust valide tout équipement. Quatre PJ issus de métiers différents, démarrant dans des quartiers distincts. Cyberpunk mélancolique, intrigues non linéaires et relations complexes."####,
    },
    ScenarioPreset {
        title: r####"Mer des Tempêtes — L’Île aux Sept Cartes"####,
        genre: r####"Pirates et exploration"####,
        recommended_players: 5,
        summary: r####"Cinq compagnons suivent une carte contradictoire à travers les récifs."####,
        context: r####"Dans un archipel imaginaire du XVIIe siècle, cinq fragments de cartes semblent indiquer cinq emplacements différents pour le même phare disparu. Les corsaires rivalisent avec une compagnie commerciale qui monopolise les routes.

LIEUX : port de Sable-Rouge, fortin du Gouverneur, lagon des Épaves, île des Sept Cartes. PNJ : capitaine Mirella, corsaire indépendante ; Orsen, cartographe borgne ; Yago, maître de port cupide.

MONNAIE : doublons. Navires et équipages ne sont pas automatiquement la propriété des personnages. Cinq PJ aux fonctions complémentaires peuvent débuter séparément sur les quais ou déjà au sein d'une équipe, selon leurs fiches. Les courants, les tempêtes et les ressources ont des conséquences vérifiées. Atmosphère d'aventure, pas de trésor garanti."####,
    },
    ScenarioPreset {
        title: r####"Poussière Rouge — Le Dernier Télégraphe"####,
        genre: r####"Western sans fantastique"####,
        recommended_players: 1,
        summary: r####"Un télégraphiste solitaire dans une ville au bord de la sécheresse."####,
        context: r####"Territoire fictif de l'Ouest nord-américain, 1881. La ligne télégraphique a cessé de fonctionner alors qu'un train de ravitaillement se rapproche d'un pont affaibli. La petite ville de Mesa Roja ne fait confiance ni au shérif ni aux prospecteurs.

LIEUX : bureau du télégraphe, gare, ranch Varela, ravin des Cendres. PNJ : shérif Nolan, figure lasse ; Ada Varela, propriétaire de ranch ; Elias Reed, ingénieur ferroviaire. Économie : dollars américains, monnaies étrangères ou troc selon les circonstances. Un seul PJ commence dans le bureau des télégraphes, avec équipement conforme à son métier. Le MJ n'impose pas que le PJ sauve le train ; il décrit les choix et leurs conséquences."####,
    },
    ScenarioPreset {
        title: r####"Paris 1933 — Les Ombres du Passage"####,
        genre: r####"Polar historique"####,
        recommended_players: 2,
        summary: r####"Un meurtre sans témoin dans un Paris pluvieux."####,
        context: r####"Paris, 1933. Un chanteur de cabaret disparaît le soir même où un industriel est retrouvé mort dans un hôtel. Un billet de train tamponné mais jamais utilisé relie les deux affaires.

LIEUX : passage couvert, hôtel de la Seine, club du Chat Bleu, commissariat. PNJ : inspectrice Delmas, attachée aux preuves ; Jeanne Mercier, chanteuse ; Léon Wexler, homme d'affaires. Monnaie : franc français. Deux PJ peuvent être détectives, journalistes ou habitants ordinaires et ne sont pas supposés travailler ensemble. Aucun coupable prédéfini dans les connaissances des joueurs ; l'enquête n'a pas de solution forcée."####,
    },
    ScenarioPreset {
        title: r####"Aube des Silex — Le Feu de l’Hiver"####,
        genre: r####"Préhistoire de fiction"####,
        recommended_players: 6,
        summary: r####"Une communauté nomade face aux migrations et aux glaciers."####,
        context: r####"Au temps d'un âge glaciaire imaginaire, six membres d'une communauté suivent des troupeaux qui disparaissent vers le nord. Un autre clan réclame le droit d'utiliser la même grotte abritée.

LIEUX : grotte des Échos, passage du Grand Mammouth, forêt gelée, rives d'un lac ancien. PNJ : Aroh, doyenne du groupe ; Sen, jeune pisteur du clan rival ; Mara, soigneuse expérimentée. ÉCONOMIE : aucun système monétaire ; troc d'objets et échanges de services. Les six PJ peuvent avoir des rôles différents. Aucune technologie moderne, aucun jugement simpliste sur les cultures. Tension de survie, diplomatie et découvertes sans chasse obligatoire."####,
    },
    ScenarioPreset {
        title: r####"Berlin 1962 — La Chambre des Échos"####,
        genre: r####"Espionnage guerre froide"####,
        recommended_players: 3,
        summary: r####"Une disparition et trois agents aux objectifs divergents."####,
        context: r####"Berlin, 1962. Un violoniste passe une frontière puis disparaît. Un carnet chiffré est retrouvé dans un étui qui n'appartient à aucun des services de renseignement connus.

LIEUX : poste de contrôle, café Neues Licht, salle de répétition, immeuble abandonné. PNJ : Eva Krüger, archiviste ; Anton Weiss, musicien ; lieutenant Falk, officier prudent. Économie : marks ou monnaies étrangères selon les circonstances, sans devise principale obligatoire. Trois PJ peuvent appartenir à des camps différents sans connaître leurs secrets réciproques. Le MJ ne révèle pas d'informations classifiées simplement parce que le lecteur les voit dans le contexte."####,
    },
    ScenarioPreset {
        title: r####"Éther et Rouages — Le Cœur de Cuivre"####,
        genre: r####"Steampunk"####,
        recommended_players: 4,
        summary: r####"Une ville à vapeur dont le soleil artificiel s’éteint."####,
        context: r####"Brumelune est une cité flottante tenue en l'air par de gigantesques turbines. Son soleil mécanique baisse en intensité et les quartiers inférieurs subissent déjà le gel.

LIEUX : atelier des Horlogers, docks d'aérostats, palais des Comptables, réservoirs d'éther. PNJ : ingénieure Lys Amber, prudente ; Lord Veil, financier influent ; Deko, réparateur clandestin. Monnaie : couronnes de cuivre. Quatre PJ provenant de milieux sociaux différents commencent dans des quartiers éloignés ; aucune machine puissante gratuite. Exploration technologique et alliances politiques."####,
    },
    ScenarioPreset {
        title: r####"Abysses — Le Chant de la Fosse"####,
        genre: r####"Science-fiction sous-marine"####,
        recommended_players: 2,
        summary: r####"Une station océanique coupée du monde après une transmission."####,
        context: r####"Année 2240. La base bathyscaphe Néréide se trouve à six kilomètres sous la surface d'un océan extraterrestre. Une transmission provenant des profondeurs répète la voix d'un membre d'équipage mort depuis des mois.

LIEUX : laboratoire humide, serre d'algues, sas extérieur, fossé tectonique. PNJ : commandante Rhee, disciplinée ; Olan, biologiste ; M-8, assistant robotique aux limites connues. Monnaie : crédits de station, sans circulation libre dans les services vitaux. Deux PJ peuvent avoir accès à des secteurs différents, sans oxygène ou matériel infinis. Horreur scientifique et exploration prudente."####,
    },
    ScenarioPreset {
        title: r####"Après la Poussière — La Route des Puits"####,
        genre: r####"Post-apocalyptique"####,
        recommended_players: 5,
        summary: r####"Un convoi cherche de l’eau dans des terres dévastées."####,
        context: r####"Soixante ans après une catastrophe climatique indéterminée, une coalition de cinq petites communautés partage une route ponctuée de puits. Trois d'entre eux sont désormais vides et des messagers disparaissent.

LIEUX : ferme solaire de Lierre, halte de Vaux, puits des Trois Pierres, ancien échangeur. PNJ : Berta, mécanicienne ; Solan, cartographe ; Tiz, responsable des réserves. ÉCONOMIE : jetons d'eau, utilisables uniquement quand les habitants les reconnaissent ; l'eau reste une ressource réelle séparée de la monnaie. Cinq PJ peuvent partir de localités différentes. Pénuries, conflits de valeurs et négociations, pas de violences forcées."####,
    },
    ScenarioPreset {
        title: r####"Université de Bracken — Les Notes Effacées"####,
        genre: r####"Mystère contemporain occulte"####,
        recommended_players: 3,
        summary: r####"Des pages se vident dans la bibliothèque pendant les examens."####,
        context: r####"Dans une petite ville universitaire actuelle, les notes d'un cours d'histoire locale disparaissent de tous les livres pendant une nuit. Un étudiant affirme qu'il a reçu un appel de son propre numéro la veille.

LIEUX : bibliothèque ancienne, dortoirs, station de radio étudiante, anciennes carrières. PNJ : professeure Vale, spécialiste de folklore ; Milo Chen, bibliothécaire ; Rosa, journaliste étudiante. Économie : euros ou autres moyens de paiement plausibles, sans monnaie imposée aux personnages. Trois PJ incarnent étudiants, membres du personnel ou habitants, libres de ne pas croire au surnaturel. Les phénomènes restent ambigus jusqu'à enquête."####,
    },
    ScenarioPreset {
        title: r####"Athènes des Brumes — La Dernière Offrande"####,
        genre: r####"Mythologie antique réinventée"####,
        recommended_players: 4,
        summary: r####"Des sanctuaires rivaux, des augures incomplets, une cité qui doute."####,
        context: r####"Dans une cité-État inspirée de la Grèce ancienne mais entièrement fictive, une procession rituelle a lieu sans que personne ne se rappelle l'avoir organisée. Des cultes se disputent le sens de cet événement.

LIEUX : agora, sanctuaire des Trois Feux, port des Oracles, colline des Potiers. PNJ : Philemon, prêtre indécis ; Doria, capitaine du port ; Kalé, potière et témoin. Monnaie : drachmes. Quatre PJ d'origines diverses commencent en ville. Les dieux ne doivent pas être traités comme des pouvoirs acquis ; interventions divines et miracles requièrent une proposition canonique validée."####,
    },
    ScenarioPreset {
        title: r####"Les Routes de la Soie — Le Parchemin Bleu"####,
        genre: r####"Aventure commerciale historique"####,
        recommended_players: 6,
        summary: r####"Caravane multiculturelle et manuscrit controversé."####,
        context: r####"Asie centrale imaginaire inspirée du XIIIe siècle. Une caravane de six voyageurs transporte des tissus, des épices et un parchemin qu'aucun de ses propriétaires ne reconnaît. Des étapes proches subissent une pénurie de fourrage.

LIEUX : caravansérail de Sary, défilé des Aigles, oasis du Puits Clair, marché de la Porte d'Est. PNJ : Nazira, négociante ; Temur, guide ; Bair, intendant. Monnaie : dirhams, avec troc et monnaies secondaires possibles. Six PJ de cultures ou professions différentes ; langues et accès à l'information comptent dans le rôleplay. Commerce, exploration et diplomatie, aucune caravane automatiquement possédée par un PJ."####,
    },
    ScenarioPreset {
        title: r####"Le Braquage du Musée Azur"####,
        genre: r####"Casse contemporain"####,
        recommended_players: 4,
        summary: r####"Un objet disparaît avant même le début du vol annoncé."####,
        context: r####"Métropole actuelle, musée d'art Azur. Quatre inconnus reçoivent la même invitation cryptique pour observer une vente aux enchères. Le soir venu, la vitrine censée contenir la pièce maîtresse est déjà vide.

LIEUX : galerie privée, musée Azur, café du personnel, sous-sol des archives. PNJ : conservatrice Salma, responsable de sécurité Vico, collectionneur Ravel. Monnaie : euro. Les quatre PJ ne sont pas nécessairement des criminels ni alliés. Le scénario peut devenir enquête, intrigue sociale ou infiltration ; le MJ ne décide pas à leur place d'organiser un cambriolage."####,
    },
    ScenarioPreset {
        title: r####"Cité des Nuages — Les Marches de Nacre"####,
        genre: r####"Fantasy aérienne"####,
        recommended_players: 1,
        summary: r####"Une messagère entre des îles flottantes qui perdent leur gravité."####,
        context: r####"Au-dessus d'un océan de nuages, des îles flottantes communiquent par des ponts de nacre. L'une d'elles descend de plusieurs mètres chaque nuit et ses habitants accusent les navigateurs d'avoir déplacé une balise.

LIEUX : port des Cerfs-Volants, archipel de Minuit, atelier des Ancriers, temple du Vent Muet. PNJ : Avel, cartographe ; mère Eryl, protectrice du temple ; Nox, navigateur en disgrâce. Monnaie : écailles d'argent. Un PJ choisit son rôle et démarre sur le pont d'une île flottante. Magie suggérée sans pouvoirs mécaniques gratuits ; périls liés au vide et aux traversées."####,
    },
    ScenarioPreset {
        title: r####"Éden Biotech — Les Héritiers de la Graine"####,
        genre: r####"Biopunk"####,
        recommended_players: 5,
        summary: r####"Une ville-jardin vivante se retourne contre ses jardiniers."####,
        context: r####"Au XXXIe siècle, la cité végétale Éden-9 est cultivée plutôt que construite. Après un dérèglement des pollens, plusieurs habitants se réveillent avec des souvenirs qui ne sont pas les leurs.

LIEUX : serres centrales, laboratoire symbiotique, faubourgs racinaires, réseau de canopées. PNJ : Dr Ayo Mensah, biologiste ; Lyra, jardinière communautaire ; commissaire Nel, chargé de quarantaine. Monnaie : unités de crédit organique, non assimilables aux ressources alimentaires. Cinq PJ d'origines et conditions diverses peuvent débuter séparés. Respecter la distinction entre soins, ressources biologiques et monnaie."####,
    },
    ScenarioPreset {
        title: r####"Bellombre — La Fête des Lanternes"####,
        genre: r####"Conte pastoral et social"####,
        recommended_players: 1,
        summary: r####"Un village paisible où les lanternes révèlent des souvenirs."####,
        context: r####"Bellombre est un petit village entouré de collines et de vergers. À chaque fête des Lanternes, une lanterne raconte normalement un souvenir heureux. Cette année, certaines montrent des scènes inconnues des familles qui les portent.

LIEUX : place de la fontaine, verger communal, moulin, maison des lanterniers. PNJ : Alma, boulangère ; Ren, instituteur ; Camille, gardienne des fêtes. Monnaie : sols. Un seul PJ commence sur la place avant la fête. Ambiance chaleureuse, mystère intime, aucun combat imposé. Le MJ peut créer des habitants ordinaires mais ne force pas les amitiés inventées par le joueur."####,
    },
    ScenarioPreset {
        title: r####"La Bibliothèque des Rêves"####,
        genre: r####"Fantastique onirique"####,
        recommended_players: 6,
        summary: r####"Six visiteurs piégés dans une bibliothèque aux histoires mouvantes."####,
        context: r####"Un lieu impossible contient toutes les histoires que personne n'a encore racontées. Six visiteurs s'éveillent dans des salles séparées tandis que certaines étagères se réorganisent selon leurs décisions. Ils ignorent pourquoi ils ont été réunis.

LIEUX : salle des Noms, escalier Sans-Haut, jardin des Livres, atelier des Relieurs. PNJ : l'Archiviste, figure aux règles strictes ; Sommeil, enfant qui prétend avoir cent ans ; la Relieuse, artisan discret. ÉCONOMIE : aucune monnaie obligatoire ; les échanges sont des objets et des services, et les souvenirs ne peuvent pas être retirés sans un consentement explicite du personnage concerné et une validation de Rust. Six PJ aux origines libres. Narration étrange mais cohérente ; les secrets des personnages restent individuels."####,
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn vingt_scenarios_et_toutes_tailles_de_groupes() {
        assert_eq!(SCENARIOS.len(), 20);
        assert_eq!(SCENARIOS.iter().map(|s| s.recommended_players).collect::<HashSet<_>>(),
            (1..=6).collect::<HashSet<_>>());
        let mut titles = HashSet::new();
        for scenario in SCENARIOS {
            assert!(titles.insert(scenario.title));
            assert!(!scenario.genre.is_empty());
            assert!(scenario.summary.chars().count() >= 30);
            assert!(scenario.context.chars().count() >= 300);
            assert!((1..=6).contains(&scenario.recommended_players));
            assert!(scenario.context.chars().count() <= 40000);
        }
    }

    #[test]
    fn sanctuaire_propose_deux_joueurs_et_positions_distinctes() {
        let s=&SCENARIOS[0];
        assert_eq!(s.recommended_players,2);
        assert!(s.context.contains("PJ1"));
        assert!(s.context.contains("PJ2"));
        assert!(s.context.contains("pièces d'or"));
    }
}
