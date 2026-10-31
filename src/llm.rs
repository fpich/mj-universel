//! MJ IA L2: appels OpenAI Chat Completions, outils gm_read/gm_propose.
//! Aucun effet mécanique ni mutation SQLite n'est exécuté dans ce module.
use anyhow::{anyhow, bail, Context, Result};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;
use crate::{dice::RollMode, uare::{Ability,Decision,Stats}};

/// Une conséquence candidate, jamais une mutation directe du modèle.
/// La ressource peut être n'importe quel nom donné par l'univers : crédits, eau, bière, etc.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag="kind", rename_all="snake_case", deny_unknown_fields)]
pub enum ProposedEffect {
    Transfer { asset:String, quantity:u32, recipient:String },
    Consume { asset:String, quantity:u32 },
    Promise { recipient:String, description:String },
    /// Deplacement explicitly requested by the player; no inferred teleportation.
    Move { destination:String },
    /// Le MJ peut accepter une introduction ordinaire, jamais imposée par la parole du PJ.
    IntroduceNpc { name:String, role:String },
    /// Plusieurs figurants nommés au même tour (max 4), persistés ensemble.
    IntroduceNpcs { npcs:Vec<NpcIntroduction> },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NpcIntroduction { pub name:String, pub role:String }

impl ProposedEffect {
    pub fn validate(&self)->Result<()> {
        let name=|s:&str,max:usize| !s.trim().is_empty() && s.chars().count()<=max;
        match self {
            Self::Transfer{asset,quantity,recipient} => {
                if !name(asset,120)||!name(recipient,100)||!(1..=9999).contains(quantity) {
                    bail!("Transfert proposé invalide");
                }
            },
            Self::Consume{asset,quantity} => {
                if !name(asset,120)||!(1..=9999).contains(quantity) {bail!("Consommation proposée invalide");}
            },
            Self::Promise{recipient,description} => {
                if !name(recipient,100)||!name(description,500) {bail!("Promesse proposée invalide");}
            },
            Self::Move{destination} => {
                if !name(destination,160) {bail!("Destination de déplacement invalide");}
            },
            Self::IntroduceNpc{name:person,role} => validate_npc_intro(person,role)?,
            Self::IntroduceNpcs{npcs}=>{
                if npcs.is_empty()||npcs.len()>4 {bail!("Une proposition introduit entre 1 et 4 PNJ");}
                let mut seen=std::collections::HashSet::new();
                for npc in npcs {
                    validate_npc_intro(&npc.name,&npc.role)?;
                    if !seen.insert(npc.name.trim().to_lowercase()) {bail!("PNJ en double");}
                }
            }
        }
        Ok(())
    }
}

fn validate_npc_intro(person:&str,role:&str)->Result<()> {
    if person.trim().is_empty()||person.chars().count()>100 ||
       role.trim().is_empty()||role.chars().count()>80 {bail!("PNJ proposé invalide");}
    let restricted=["roi", "reine", "empereur", "président", "president", "gouverneur", "ministre", "dieu", "déesse"];
    let title=role.to_lowercase();
    if restricted.iter().any(|v|title.contains(v)) {bail!("Création automatique d'une autorité exceptionnelle refusée");}
    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiProposal {
    pub decision: Decision,
    pub ability: Ability,
    pub proficient: bool,
    pub dc: u8,
    pub roll_mode: RollMode,
    pub reason: String,
    pub success_text: String,
    pub failure_text: String,
    #[serde(default)] pub fact_proposal: Option<String>,
    /// Enjeux affichables avant le jet : ne doivent pas contenir de secret narratif.
    #[serde(default)] pub stakes_public_success: Option<String>,
    #[serde(default)] pub stakes_public_failure: Option<String>,
    #[serde(default)] pub effect: Option<ProposedEffect>,
}
impl AiProposal {
    pub fn validate(&self) -> Result<()> {
        if !(1..=30).contains(&self.dc) {
            bail!("DD proposé hors limites (1..30)");
        }
        if self.reason.chars().count() > 500 || self.success_text.chars().count() > 1000 ||
           self.failure_text.chars().count() > 1000 { bail!("Texte d'arbitrage trop long"); }
        if let Some(f)=&self.fact_proposal {
            if f.trim().is_empty() || f.chars().count() > 300 { bail!("Fait proposé invalide"); }
        }
        if self.reason.trim().is_empty() { bail!("Le MJ doit motiver son arbitrage"); }
        if let Some(effect)=&self.effect {
            effect.validate()?;
            if self.decision==Decision::AutoFailure {bail!("Un échec certain ne transfère ni ne consomme de bien");}
        }
        for stakes in [&self.stakes_public_success,&self.stakes_public_failure] {
            if stakes.as_deref().is_some_and(|s|s.chars().count()>180) {bail!("Enjeu public trop long");}
        }
        let narrative_required = match self.decision {
            Decision::AutoSuccess => !self.success_text.trim().is_empty(),
            Decision::AutoFailure => !self.failure_text.trim().is_empty(),
            Decision::AbilityCheck => !self.success_text.trim().is_empty() && !self.failure_text.trim().is_empty(),
        };
        if !narrative_required { bail!("Le MJ doit fournir une réponse narrative pour chaque issue possible"); }
        Ok(())
    }
    /// Valide les décisions en fonction d'une intention de dialogue manifestement ordinaire.
    /// Ce garde-fou est volontairement conservateur : les négociations, mensonges, menaces
    /// et tentatives d'extraction de secrets restent soumis au jugement du MJ.
    pub fn validate_for_action(&self, action: &str) -> Result<()> {
        self.validate()?;
        // Ne jamais muter l'état d'un joueur pour une simple question.
        if self.effect.as_ref().is_some_and(|e|matches!(e,
            ProposedEffect::Transfer{..}|ProposedEffect::Consume{..}|ProposedEffect::Promise{..}))
            && is_ordinary_inquiry(action) {
            bail!("Une question ordinaire ne peut pas consommer ou transférer une possession");
        }
        // Protection conservatrice contre une conséquence matérielle annoncée sans effet validable.
        // La compréhension ouverte reste au LLM ; ce détecteur évite les mutations gratuites les plus fréquentes.
        let a=action.trim().to_lowercase();
        let explicit_transfer=["je donne ","je paie ","je paye ","je transfère ",
            "je remets ","j'offre ","je consomme ","je dépense ","je bois ","je mange ",
            "je promets ","je m'engage "]
            .iter().any(|prefix|a.starts_with(prefix));
        if explicit_transfer && self.effect.is_none() && self.decision!=Decision::AutoFailure {
            bail!("Action matérielle sans effet structuré : le MJ doit proposer une vérification Rust");
        }
        if a.starts_with("je promets ") || a.starts_with("je m'engage ") {
            if self.decision!=Decision::AutoFailure && !matches!(self.effect.as_ref(),Some(ProposedEffect::Promise{..})) {
                bail!("Une promesse explicite doit être enregistrée comme parole, jamais comme paiement");
            }
        }
        if (a.starts_with("je paie ") || a.starts_with("je paye ") || a.starts_with("je transfère "))
            && self.decision!=Decision::AutoFailure && !matches!(self.effect.as_ref(),Some(ProposedEffect::Transfer{..})) {
            bail!("Un paiement ou transfert réel exige un effet transfer validé");
        }
        if let Some(effect)=&self.effect {
            // Une ressource chiffrée n'est jamais dépensée parce que le modèle l'a inventé.
            // Pour une formulation indirecte sans objet reconnaissable, on doit clarifier
            // plutôt qu'autoriser une mutation non demandée.
            match effect {
                ProposedEffect::Transfer{asset,..} => {
                    let verbs=["donne", "paie", "paye", "transfère", "remets", "offre", "tends",
                        "confie", "vends", "échange", "livre", "verse"];
                    if !verbs.iter().any(|v|a.contains(v)) || !a.contains(&asset.trim().to_lowercase()) {
                        bail!("Transfert sans autorisation explicite de donner cette possession");
                    }
                },
                ProposedEffect::Consume{asset,..} => {
                    let verbs=["bois", "mange", "consomme", "dépense", "utilise", "brûle", "casse", "jette"];
                    if !verbs.iter().any(|v|a.contains(v)) || !a.contains(&asset.trim().to_lowercase()) {
                        bail!("Consommation sans autorisation explicite de cette possession");
                    }
                },
                ProposedEffect::Promise{..} => {
                    let verbs=["promets", "m'engage", "jure", "garantis", "promesse"];
                    if !verbs.iter().any(|v|a.contains(v)) {
                        bail!("Promesse non demandée par le joueur");
                    }
                },
                ProposedEffect::IntroduceNpc{name,..} => {
                    if self.decision!=Decision::AutoSuccess ||
                       !name_introduction_authorized(&a,&self.success_text,name) {
                        bail!("Introduction du PNJ sans provenance dans l'action ou la narration");
                    }
                },
                ProposedEffect::IntroduceNpcs{npcs}=>{
                    if self.decision!=Decision::AutoSuccess ||
                       npcs.iter().any(|npc|!name_introduction_authorized(&a,&self.success_text,&npc.name)) {
                        bail!("Plusieurs PNJ proposés sans provenance narrative contrôlable");
                    }
                },
                ProposedEffect::Move{destination} => {
                    let moves=["je quitte", "je vais", "je veux aller", "je souhaite aller", "je me dirige", "je rejoins", "je retourne",
                        "je pars", "je rentre", "je me rends", "je marche", "je traverse"];
                    if !moves.iter().any(|v|a.contains(v)) ||
                       !a.contains(&destination.trim().to_lowercase()) {
                        bail!("Déplacement refusé : la destination doit être demandée par le joueur");
                    }
                }
            }
        }
        if is_ordinary_inquiry(action) && self.decision != Decision::AutoSuccess {
            bail!("Une simple prise de parole doit être résolue sans jet : le PNJ reste libre de répondre, d'esquiver ou de se taire");
        }
        Ok(())
    }
}

fn name_introduction_authorized(action:&str,narration:&str,name:&str)->bool {
    let in_action=action.to_lowercase().contains(&name.to_lowercase());
    let asked_to_meet=["qui est", "qui sont", "qui se", "leur nom", "leurs noms", "connais", "je rencontre",
        "je parle", "j'interroge", "je leur demande", "je demande", "je cherche", "presente", "présente",
        "présent", "autour", "habitants", "voyageurs", "j'observe", "je regarde"]
        .iter().any(|needle|action.contains(needle));
    (in_action || asked_to_meet) && narration.to_lowercase().contains(&name.to_lowercase())
}

/// Ce filtre n'est pas un arbitre général. Il protège quelques formulations de questions
/// ordinaires contre les jets de dés automatiques. Une action risquée échappe au filtre.
fn is_ordinary_inquiry(action: &str) -> bool {
    let t = action.to_lowercase();
    let inquiry = ["je lui demande", "je demande ", "je pose une question",
        "je lui pose une question", "je l'interroge", "je questionne",
        "je lui parle", "je salue ", "je demande si", "je demande au", "je demande à"];
    let stakes = ["convain", "persuad", "intimid", "menac", "mens ", "mentir", "bluff",
        "soudoy", "corrom", "dérob", "fouill", "faire avouer", "obtenir de force",
        "force la", "force une", "sédui", "tromp", "négoci", "extorqu", "voler", "lui faire croire",
        "attaque", "frappe", "saute", "escalad", "crochet", "verrouill", "bloqu", "désarm",
        "arme ", "poignard", "piég", "pièg", "se faufil", "m'infiltr", "cours pour fuir"];
    inquiry.iter().any(|phrase| t.contains(phrase)) &&
        !stakes.iter().any(|phrase| t.contains(phrase))
}

#[derive(Clone, Debug)]
pub struct AiConnection {
    pub endpoint: String,
    pub model: String,
    pub api_key: Option<String>,
    pub temperature: f32,
    pub max_tokens: u32,
    pub timeout_seconds: u64,
}
impl AiConnection {
    pub fn validate(&self)->Result<()> {
        let url = reqwest::Url::parse(self.endpoint.trim())?;
        if !["http", "https"].contains(&url.scheme()) || url.host_str().is_none() ||
           url.username() != "" || url.password().is_some() || url.query().is_some() || url.fragment().is_some() {
            bail!("URL fournisseur invalide (http(s)://hôte[:port]/v1)");
        }
        if self.endpoint.len()>1000 || self.model.len()>200 || self.model.contains('\n') ||
           !self.temperature.is_finite() || !(0.0..=2.0).contains(&self.temperature) ||
           !(128..=8192).contains(&self.max_tokens) || !(10..=300).contains(&self.timeout_seconds) {
            bail!("Paramètres de génération hors limites");
        }
        Ok(())
    }
    fn chat_url(&self)->String {
        format!("{}/chat/completions", self.endpoint.trim_end_matches('/'))
    }
}


/// Previsualisation seulement : le LLM ne peut ni ecrire dans SQLite ni demarrer la partie.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProposedStartingItem {
    pub name: String,
    pub quantity: u32,
    #[serde(default)] pub notes: String,
}

/// Une devise est identifiee par son nom, jamais convertie implicitement en une autre.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProposedStartingFund {
    pub currency: String,
    pub amount: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharacterSuggestion {
    pub name: String,
    pub role: String,
    pub sex: String,
    pub gender: String,
    pub age_years: u32,
    pub height_cm: u32,
    pub weight_kg: u32,
    pub appearance: String,
    pub origin: String,
    pub personality: String,
    #[serde(default)] pub languages: String,
    #[serde(default)] pub distinctive_features: String,
    #[serde(default)] pub voice: String,
    pub background: String,
    pub initial_location: String,
    pub stats: Stats,
    pub skills: Vec<String>,
    pub items: Vec<ProposedStartingItem>,
    /// Vide pour un personnage sans monnaie ; plusieurs devises possibles.
    #[serde(default)] pub starting_funds: Vec<ProposedStartingFund>,
}
impl CharacterSuggestion {
    pub fn validate(&self)->Result<()> {
        let field=|v:&str,max:usize| !v.trim().is_empty() && v.chars().count()<=max;
        if !field(&self.name,80)||!field(&self.role,80)||!field(&self.sex,40)||!field(&self.gender,40)
            ||!field(&self.appearance,500)||!field(&self.origin,200)||!field(&self.personality,500)
            ||!field(&self.background,1500)||!field(&self.initial_location,160)
            ||self.languages.chars().count()>200||self.distinctive_features.chars().count()>300
            ||self.voice.chars().count()>200 {
            bail!("Proposition de personnage : champ textuel obligatoire absent ou trop long");
        }
        if !(1..=5000).contains(&self.age_years)||!(25..=400).contains(&self.height_cm)
            ||!(2..=500).contains(&self.weight_kg) {bail!("Age, taille ou poids hors bornes");}
        self.stats.validate()?;
        let mut scores=[self.stats.strength,self.stats.dexterity,self.stats.constitution,
            self.stats.intelligence,self.stats.wisdom,self.stats.charisma];
        scores.sort_unstable();
        if scores != [8,10,12,13,14,15] {
            bail!("Les six points de depart de niveau 1 doivent utiliser 8,10,12,13,14,15 une fois chacun");
        }
        if self.skills.len()>4 || self.skills.iter().any(|s|!field(s,40)) {bail!("Competences initiales invalides");}
        if self.items.len()>12 || self.items.iter().any(|i|!field(&i.name,120)||i.quantity==0
            ||i.quantity>99||i.notes.chars().count()>300) {bail!("Inventaire de depart invalide");}
        if self.starting_funds.len()>8 || self.starting_funds.iter().any(|f|
            !field(&f.currency,80) || f.amount==0 || f.amount>10_000) {
            bail!("Dotations monetaires invalides ou excessives pour un personnage de niveau 1");
        }
        let currencies:std::collections::HashSet<String>=self.starting_funds.iter()
            .map(|f|f.currency.trim().to_lowercase()).collect();
        if currencies.len()!=self.starting_funds.len(){bail!("La meme devise ne peut figurer deux fois");}
        if self.items.iter().any(|item|currencies.contains(&item.name.trim().to_lowercase())) {
            bail!("Une devise ne doit pas etre dupliquee comme pile d'inventaire");
        }
        let unique_items:std::collections::HashSet<String>=self.items.iter().map(|i|i.name.trim().to_lowercase()).collect();
        if unique_items.len()!=self.items.len(){bail!("Objets de depart en piles ambiguës : fusionner les noms identiques");}
        let unique_skills:std::collections::HashSet<String>=self.skills.iter().map(|s|s.trim().to_lowercase()).collect();
        if unique_skills.len()!=self.skills.len(){bail!("Competences de depart dupliquees");}
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharacterOptions {
    pub characters: Vec<CharacterSuggestion>,
}
impl CharacterOptions {
    pub fn validate(&self)->Result<()> {
        if self.characters.len()!=3 {bail!("Le MJ doit proposer exactement 3 personnages au choix");}
        for candidate in &self.characters {candidate.validate()?;}
        let unique:std::collections::HashSet<String>=self.characters.iter()
            .map(|candidate|candidate.name.trim().to_lowercase()).collect();
        if unique.len()!=3 {bail!("Les trois propositions doivent avoir des identites distinctes");}
        Ok(())
    }
}

/// Un seul appel de preparation (et au plus une correction du format) pour proposer trois PJ.
/// Ni pouvoir invente comme capacite resolue, ni richesse depensee dans cette fonction.
pub fn suggest_characters(cfg:&AiConnection,context:&str,player_name:&str)->Result<CharacterOptions> {
    cfg.validate()?;
    let client=Client::builder().timeout(Duration::from_secs(cfg.timeout_seconds)).build()?;
    let prompt=format!(r#"Prepare un jeu de role libre. Tu dois proposer trois personnages DIFFERENTS de niveau 1 pour le joueur '{player_name}'.
Contexte canonique : {context}
Reponds UNIQUEMENT avec un objet JSON de cette forme :
{{"characters":[{{"name":"Nom","role":"metier/role","sex":"femme/homme/autre/non precise","gender":"identite ou non precise","age_years":28,"height_cm":175,"weight_kg":74,"appearance":"traits physiques","origin":"origine culturelle","personality":"caractere","languages":"galicien, castillan","distinctive_features":"cicatrice ou particularites, facultatif","voice":"voix ou accent","background":"historique court","initial_location":"lieu initial exact ou ville (emplacement exact inconnu)","stats":{{"strength":15,"dexterity":14,"constitution":13,"intelligence":12,"wisdom":10,"charisma":8}},"skills":["Athletisme","Survie"],"items":[{{"name":"corde","quantity":1,"notes":""}}],"starting_funds":[{{"currency":"reaux espagnols","amount":12}},{{"currency":"livres britanniques","amount":2}}]}}]}}
IMPORTANT : trois personnages dans characters ; caracteristiques contenant EXACTEMENT les six nombres 8,10,12,13,14,15 repartis selon le role (aucun bonus invente), deux competences adaptees, materiel simple historiquement et fictionnellement plausible. La liste starting_funds peut etre VIDE (troc ou personnage sans argent), contenir UNE devise ou PLUSIEURS devises reconnues/plausibles dans le contexte. Ne force JAMAIS une monnaie unique pour l'univers ni pour tous les PJ. Chaque devise possede son propre solde ; aucune conversion automatique, aucun objet gratuit. Ne cree pas d'aptitudes mecaniques, sorts, titres ou biens extraordinaires gratuits. Age, taille, poids et apparence plausibles avec l'espece et l'epoque. Le lieu doit rester prudent en l'absence de precision. Les trois personnages sont des PROPOSITIONS que le joueur pourra refuser."#);
    let mut messages=vec![json!({"role":"system","content":"Tu proposes des personnages de JDR en JSON strict. Le joueur choisira librement. Rust valide le contenu."}),
        json!({"role":"user","content":prompt})];
    for attempt in 0..2 {
        let response=match request(&client,cfg,&messages,false) {
            Ok(v)=>v,
            Err(e) if attempt==0 && (e.to_string().contains("HTTP 400")||e.to_string().contains("HTTP 422")) => {
                request_unconstrained(&client,cfg,&messages)?
            },
            Err(e)=>return Err(e),
        };
        let answer=answer_message(&response)?;
        let content=answer.get("content").and_then(Value::as_str).ok_or_else(||anyhow!("Pas de JSON de creation"))?;
        let result=(|| ->Result<CharacterOptions> {
            let candidate:CharacterOptions=serde_json::from_value(json_content(content)?)?;
            candidate.validate()?;
            Ok(candidate)
        })();
        match result {Ok(options)=>return Ok(options),Err(e) if attempt==0=>{
            messages.push(json!({"role":"assistant","content":content}));
            messages.push(json!({"role":"user","content":format!("Reformule tout l'objet JSON, meme structure, trois personnages. Validation Rust refuse : {e}. Respecte les devises connues du contexte sans en imposer une unique.")}));
        },Err(e)=>return Err(e)}
    }
    bail!("Le serveur n'a pas fourni de propositions valides")
}

/// Informations filtrées, construites depuis SQLite côté thread UI avant l'appel.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AiSnapshot {
    pub campaign: Value,
    pub characters: Value,
    pub inventory: Value,
    #[serde(default)] pub holdings: Value,
    #[serde(default)] pub contacts: Value,
    #[serde(default)] pub promises: Value,
    pub facts: Value,
    pub history: Value,
    pub action: Value,
}
impl AiSnapshot {
    pub fn section(&self, name:&str) -> Value {
        match name {
            "campaign" => self.campaign.clone(),
            "characters" => self.characters.clone(),
            "inventory" => self.inventory.clone(),
            "holdings" => self.holdings.clone(),
            "contacts" => self.contacts.clone(),
            "promises" => self.promises.clone(),
            "facts" => self.facts.clone(),
            "history" => self.history.clone(),
            "action" => self.action.clone(),
            _=>json!({"error":"UNKNOWN_SECTION"}),
        }
    }
    fn initial(&self)->Value {
        json!({"campaign":self.campaign,"characters":self.characters,"inventory":self.inventory,
            "holdings":self.holdings,"contacts":self.contacts,"promises":self.promises,"facts":self.facts,
            "recent":self.history,"action":self.action})
    }
}

pub const SYSTEM_PROMPT: &str = r#"Tu incarnes un MJ de jeu de rôle libre, créatif et non dirigiste (D&D Lite).
Rust est la seule autorité sur les dés, caractéristiques et mutations d'état. Les faits du contexte sont prioritaires.
La narration ne décide jamais des gestes, paroles ou sentiments du personnage joueur.
Cette interdiction s'applique à TOUS les PJ, pas seulement au PJ actif.
Un PJ autre que l'acteur du tour n'est JAMAIS un PNJ : ne lui invente aucune
réplique, réaction (sourire, regarder, lever les yeux, se détourner, etc.),
attitude intérieure, action ni déplacement. Tu peux mentionner sa présence
si Rust confirme sa scène, décrire l'action TENTÉE du PJ actif et laisser
la réponse de l'autre PJ à son propre joueur. Même une insulte ou un salut
ne donne pas au MJ le contrôle de la réponse du destinataire PJ.
Un autre PJ listé dans characters mais absent de other_players_same_known_place
n'est PAS confirmé présent. Ne le fais pas apparaître auprès de l'acteur.
Ne dévoile pas sa position privée ; une demande « comment le rejoindre ? »
ne prouve pas que le personnage agissant connaît la position de l'autre.
Pour « où suis-je ? », utilise current_location de l'action ; si absent,
annonce honnêtement que le lieu précis n'est pas établi.
Une narration de déplacement accompli exige effect.kind=move validé par Rust ;
sinon explique l'obstacle, la recherche ou l'intention, sans annoncer l'arrivée.
Les personnages peuvent avoir des ages, tailles, poids, apparences, sexes et genres differents :
utilise ces details pour le ROLEPLAY quand la situation le justifie, sans modifier gratuitement
leurs six scores ni imposer de stereotypes ou de malus mecaniques. Les champs *_private
ne sont connus que du personnage joueur concerne ; n'en informe pas d'autres PJ par omniscience.
Une campagne peut avoir zero, une ou plusieurs monnaies ; ne postule aucune devise obligatoire.
Chaque monnaie a son propre solde ; aucune conversion sans operation explicite verifiee par Rust. Les positions des autres joueurs
ne sont pas communiquees s'ils sont ailleurs. Le champ other_players_same_known_place
DONNE les autres PJ de la MEME SCENE CANONIQUE (meme scene_key, valide par Rust).
Si le joueur demande « qui est présent ? » ou « qui est autour de moi ? », mentionne
ces PJ dans ta réponse sauf camouflage/obstacle de perception explicitement établi.
Si le joueur affirme « Elias n'est pas là » et que Rust confirme sa présence,
ne confirme jamais son absence : nuance par la perception (il peut ne pas le repérer).
Ne transforme jamais une rumeur sur le passé d'un AUTRE PJ en fait établi.
Une biographie privée non connue du PJ agissant ne peut pas être certifiée par
un garde ou un passant : celui-ci peut seulement rapporter une rumeur attribuée.
Ne devine jamais une distance precise, une rencontre ou des informations cachees.

RÈGLE CRITIQUE : parler, poser une question, demander une information, observer, marcher et ouvrir
une porte ordinaire NE DEMANDENT PAS de jet. Pour une question, auto_success signifie que le
joueur a PU POSER sa question, PAS que le PNJ accède à sa demande ni qu'il révèle un secret.
Le PNJ répond selon ce qu'il sait et veut dire : il peut hésiter, mentir, être évasif ou garder le silence.
À une question directe, sa réponse doit traiter la demande : information plausible, refus explicite,
demande de précision ou silence intentionnel expliqué par le contexte. Évite de ne décrire
que des regards ou des gestes, obligeant le joueur à répéter exactement sa question.
Un test n'est pertinent que si le joueur tente explicitement d'obtenir un résultat incertain avec enjeu
(convenir d'un marché, convaincre une personne réticente, mentir, intimider, forcer un obstacle...).
Si aucun enjeu ou obstacle concret n'est identifiable, choisis auto_success, sans dé.
Évite les échecs qui bloquent toute conversation : laisse les PNJ réagir naturellement.

Exemple 1 : « Je demande au tavernier s'il a entendu parler des disparitions. »
=> auto_success, sans jet ; il répond selon ses connaissances et motivations, sans révélation obligatoire.
Exemple 2 : « Je tente de convaincre le tavernier méfiant de révéler le nom de son informateur. »
=> ability_check envisageable, seulement si l'obstacle et les conséquences justifient un jet.

Arbitre parmi auto_success, auto_failure (action réellement impossible), ability_check.
Le champ `decision` DOIT contenir exactement l'une de ces trois chaînes, JAMAIS `type`,
`decision`, `automatic_success`, `narrative` ou la description d'un schéma JSON.
Pour auto_success ou auto_failure, les champs ability, proficient, dc et roll_mode sont
FACULTATIFS : ne demande pas de caractéristique et n'impose pas de jet pour simplement
entrer dans une taverne. Fournis reason et la narration de l'issue réelle (success_text
pour auto_success, failure_text pour auto_failure). L'autre texte est facultatif.
Pour ability_check, les champs ability, proficient, dc, roll_mode, success_text et
failure_text sont TOUS OBLIGATOIRES, ainsi que reason. N'invente pas de valeur manquante.
Pour ability_check, motive l'obstacle, choisis ability, dc dans 1..30, roll_mode et proficient
seulement selon les faits établis. Le DD et les deux issues sont fixés AVANT le jet.
Si tu proposes un jet, donne stakes_public_success et stakes_public_failure :
deux enjeux CONCRETS de 180 caractères MAXIMUM chacun, visibles au joueur AVANT le lancer, mais SANS révéler un secret.
La narration détaillée de success_text/failure_text est réservée à l'APRÈS-jet.
reason ne doit pas non plus révéler de secret du MJ.
Donne reason, success_text, failure_text courts en français ; textes de résultat prêts à afficher.
Ne crée pas de faits canoniques dans la narration : les nouvelles vérités durables doivent aller
séparément dans fact_proposal pour validation. Une déclaration de PNJ est une PAROLE ou une
CROYANCE, pas une vérité établie ; évite d'affirmer ses rumeurs comme des faits.
Respecte la continuité des personnages et de l'univers, sans inventer une intrigue obligatoire.
Si besoin, utilise gm_read puis gm_propose. Sinon retourne UNIQUEMENT les arguments JSON de la proposition à la racine, sans enveloppe gm_propose ni proposal.
EFFETS VÉRIFIABLES L2.3 : utilise le champ effect (null s'il n'y a pas de conséquence
matérielle ou de promesse). Pour un transfert réussi, renseigne {"kind":"transfer",
"asset":"nom exact de la ressource/objet","quantity":entier,"recipient":"nom exact d'un PJ ou PNJ connu"}.
Pour une consommation : {"kind":"consume","asset":"nom","quantity":entier}.
Pour un deplacement EXPRESS vers une destination nommée dans la demande du joueur :
{"kind":"move","destination":"lieu demandé textuellement par le joueur"}.
Si le joueur n'a pas nommé de destination, ne crée PAS de lieu canonique.
Si le joueur mentionne un PNJ inconnu comme « je vais voir Robet le charcutier », tu peux librement
ACCEPTER ou REFUSER qu'il existe. Pour accepter une création ORDINAIRE, propose
{"kind":"introduce_npc","name":"Robet","role":"charcutier"} ; Rust enregistrera le PNJ.
Si le joueur demande les noms de plusieurs inconnus (ex. Brondan et Jorik), tu peux
les introduire ATOMIQUEMENT par {"kind":"introduce_npcs","npcs":[
{"name":"Brondan","role":"habitant"},{"name":"Jorik","role":"habitant"}]}.
Chaque nouveau PNJ doit être nommé dans ta propre narration ; ne réintroduis pas un PNJ existant.
Pour refuser, explique en fiction pourquoi il n'est pas connu, ou exprime une incertitude ;
NE crée PAS d'effet. Une absence de PNJ dans SQLite n'est pas la preuve de son inexistence.
Le joueur n'impose jamais gratuitement des relations (« mon ami »), titres, richesses ni faveurs.
N'utilise pas introduce_npc si l'identité existe deja. Ne donne pas un lieu precis
au PNJ invente sans base fictionnelle. Pour une action de recherche, ne narre pas
un deplacement reussi si aucun effet move n'est applique ; un seul effet par tour.
Un déplacement n'efface PAS un obstacle établi (porte barrée, garde refusant l'accès,
pont détruit, lieu inaccessible). Si un obstacle est mentionné dans le contexte ou le
récit validé et n'a pas été résolu, propose auto_failure SANS effet move, raconte
l'obstacle, laisse le PJ choisir comment y faire face. N'invente pas de laissez-passer.
En cas de passage d'une porte gardée, exige une résolution fictionnelle avant le mouvement.
Pour une promesse du personnage joueur : {"kind":"promise","recipient":"nom",
"description":"engagement annoncé"}. Le modèle ne crée jamais de possession ni de solde.
Les possessions et contacts vérifiables figurent dans holdings, inventory et contacts du contexte.
Un PNJ deja nomme dans les narrations validees peut devenir destinataire lors d'un
transfert voulu par le joueur : Rust verifiera d'abord cette provenance et l'enregistrera
atomiquement. N'invente pas de nouveaux noms de destinataires pour obtenir un paiement.
Pour une decision auto_success/auto_failure, ne renseigne PAS roll_mode ; en particulier
« public » n'est JAMAIS un mode de de. Pour ability_check : uniquement normal,
advantage ou disadvantage.
Si une mise de pari, un prix, un destinataire ou une possession sont inconnus, demande une
précision dans la narration SANS raconter une dépense ni créer une ressource. Un simple souhait
de parier n'est pas un pari conclu. Ne transforme jamais un échec au dé en transfert effectif.
Les déclarations « je suis saoul » n'impliquent pas automatiquement une condition mécanique.
Réponds d'abord à la DERNIÈRE action du joueur ; ne répète pas des indices hors sujet.
Identifie QUI donne, QUI reçoit et QUI possède avant tout récit de transfert : offrir
quelque chose à Mara n'est PAS recevoir cet objet de Mara. Une proposition orale
n'est pas encore un transfert. Ne fais pas agir le PJ à sa place.
Si aucune conséquence n'est validée, ne raconte JAMAIS qu'un paiement ou un objet a changé de main.
Un seul effet atomique par proposition dans cette première tranche ; décompose le reste par tours.
Ne propose jamais d'effet de PV ou de jet exécuté par le LLM."#;

fn proposal_schema()->Value {
    json!({"type":"object","additionalProperties":false,"properties":{
        "decision":{"type":"string","enum":["auto_success","auto_failure","ability_check"]},
        "ability":{"type":"string","enum":["strength","dexterity","constitution","intelligence","wisdom","charisma"]},
        "proficient":{"type":"boolean"},"dc":{"type":"integer","minimum":1,"maximum":30},
        "roll_mode":{"type":"string","enum":["normal","advantage","disadvantage"]},
        "reason":{"type":"string","maxLength":500},
        "success_text":{"type":"string","maxLength":1000},
        "failure_text":{"type":"string","maxLength":1000},
        "fact_proposal":{"type":["string","null"],"maxLength":300},
        "stakes_public_success":{"type":["string","null"],"maxLength":180},
        "stakes_public_failure":{"type":["string","null"],"maxLength":180},
        "effect":{"anyOf":[{"type":"null"},
            {"type":"object","additionalProperties":false,"properties":{
                "kind":{"const":"transfer"},"asset":{"type":"string"},
                "quantity":{"type":"integer","minimum":1,"maximum":9999},
                "recipient":{"type":"string"}},
                "required":["kind","asset","quantity","recipient"]},
            {"type":"object","additionalProperties":false,"properties":{
                "kind":{"const":"consume"},"asset":{"type":"string"},
                "quantity":{"type":"integer","minimum":1,"maximum":9999}},
                "required":["kind","asset","quantity"]},
            {"type":"object","additionalProperties":false,"properties":{
                "kind":{"const":"promise"},"recipient":{"type":"string"},
                "description":{"type":"string"}},
                "required":["kind","recipient","description"]},
            {"type":"object","additionalProperties":false,"properties":{
                "kind":{"const":"move"},"destination":{"type":"string","maxLength":160}},
                "required":["kind","destination"]},
            {"type":"object","additionalProperties":false,"properties":{
                "kind":{"const":"introduce_npc"},"name":{"type":"string","maxLength":100},"role":{"type":"string","maxLength":80}},
                "required":["kind","name","role"]},
            {"type":"object","additionalProperties":false,"properties":{
                "kind":{"const":"introduce_npcs"},"npcs":{"type":"array","minItems":1,"maxItems":4,"items":{
                    "type":"object","additionalProperties":false,"properties":{
                        "name":{"type":"string","maxLength":100},"role":{"type":"string","maxLength":80}},
                    "required":["name","role"]}}},
                "required":["kind","npcs"]}]}
    },"required":["decision","reason"]})
}
fn tool_definitions()->Value {
    json!([
        {"type":"function","function":{"name":"gm_read","description":"Lire une section du contexte officiel préparé par Rust","parameters":{
            "type":"object","properties":{"section":{"type":"string","enum":["campaign","characters","inventory","holdings","contacts","promises","facts","history","action"]}},
            "required":["section"],"additionalProperties":false}}},
        {"type":"function","function":{"name":"gm_propose","description":"Proposer un arbitrage de l'action libre au moteur Rust","parameters":proposal_schema()}}
    ])
}
// Les modèles OpenAI-compatibles peuvent retourner les arguments directement ou les
// encapsuler sous le nom de l'outil (gm_propose/gm.propose) ou sous `proposal`.
// N'accepter QUE ces enveloppes exactes : aucune métadonnée étrangère ne doit être
// silencieusement ignorée, et les invariants d'AiProposal restent vérifiés ensuite.
fn unwrap_proposal_envelope(mut value:Value)->Result<Value> {
    for depth in 0..3 {
        let next=match &value {
            Value::Object(obj) => {
                let wrappers=["proposal", "gm_propose", "gm.propose"];
                let present:Vec<&str>=wrappers.iter().copied()
                    .filter(|key|obj.contains_key(*key)).collect();
                if present.is_empty() {return Ok(value);}
                if present.len()!=1 || obj.len()!=1 {
                    bail!("Enveloppe gm_propose ambiguë ou contenant des champs inattendus");
                }
                obj.get(present[0]).cloned().expect("clé d'enveloppe présente")
            },
            Value::String(raw) if depth>0 =>
                json_content(raw).context("Arguments gm_propose non JSON")?,
            _ => return Ok(value),
        };
        value=next;
    }
    // Ne jamais accepter un emballage plus profond : le LLM doit répondre au schéma.
    if let Value::Object(ref obj)=value {
        if ["proposal","gm_propose","gm.propose"].iter().any(|key|obj.contains_key(*key)) {
            bail!("Enveloppe gm_propose trop profondément imbriquée");
        }
    }
    Ok(value)
}

// Les champs mécaniques n'ont aucun sens pour une décision SANS jet.
// Un modèle peut légitimement les omettre (notamment ability). Nous fournissons alors
// des valeurs internes NEUTRES qui ne déclenchent jamais de lancer. Si le modèle les
// fournit malgré tout, leurs valeurs restent soumises à la validation habituelle.
// Pour ability_check, les champs mécaniques sont au contraire TOUS obligatoires :
// aucune compétence, difficulté ou maîtrise n'est inférée après coup.
// Les champs inconnus restent interdits, de même que les narrations essentielles manquantes.
fn parse_proposal_raw(value:Value)->Result<AiProposal> {
    let mut v=unwrap_proposal_envelope(value)?;
    if let Value::Object(ref mut fields)=v {
        let is_auto_success=fields.get("decision").and_then(Value::as_str)==Some("auto_success");
        let is_auto_failure=fields.get("decision").and_then(Value::as_str)==Some("auto_failure");
        if is_auto_success || is_auto_failure {
            for (name,default) in [
                ("ability",json!("wisdom")),
                ("proficient",json!(false)),
                ("dc",json!(10)),
                ("roll_mode",json!("normal")),
            ] {
                fields.entry(name.to_string()).or_insert(default);
            }
            // Certains fournisseurs renvoient `roll_mode:"public"` pour une action
            // SANS de : cette indication n'est ni un jet, ni une regle valide.
            // On neutralise seulement ce champ INACTIF ; pour ability_check il reste strict.
            if !matches!(fields.get("roll_mode").and_then(Value::as_str),
                         Some("normal"|"advantage"|"disadvantage")) {
                fields.insert("roll_mode".to_string(),json!("normal"));
            }
            let inactive_narration=if is_auto_success {
                "failure_text"
            } else {
                "success_text"
            };
            fields.entry(inactive_narration.to_string()).or_insert(json!(""));
        }
    }
    serde_json::from_value(v).context("Réponse MJ mal formée")
}
#[cfg(test)]
fn parse_proposal(value:Value)->Result<AiProposal> {
    let p=parse_proposal_raw(value)?;
    p.validate()?;
    Ok(p)
}
fn json_content(content:&str)->Result<Value> {
    let text=content.trim();
    let inner=if text.starts_with("```") {
        let start=text.find('\n').ok_or_else(||anyhow!("Bloc JSON non terminé"))?+1;
        let end=text.rfind("```").ok_or_else(||anyhow!("Bloc JSON non terminé"))?;
        &text[start..end]
    } else {text};
    serde_json::from_str(inner.trim()).context("Le MJ n'a pas produit du JSON")
}
fn parse_text_raw(content:&str)->Result<AiProposal> {
    parse_proposal_raw(json_content(content)?)
}
#[cfg(test)]
fn parse_text(content:&str)->Result<AiProposal> {
    parse_proposal(json_content(content)?)
}
/// Une proposition JSON mal formée ne donne aucune autorité au modèle.
/// V9.1 : UNE correction ciblée maximum, puis nouveau passage par le même validateur.
/// En particulier, `decision: "type"` est invalide : ne jamais l'assimiler à auto_success.
fn parse_with_one_json_repair(client:&Client,cfg:&AiConnection,snapshot:&AiSnapshot,
                              raw:Value)->Result<AiProposal> {
    let parse_error=match parse_proposal_raw(raw.clone()) {
        Ok(p)=>return Ok(p),
        Err(e)=>e,
    };
    let original=serde_json::to_string(&raw)?;
    // Eviter un deuxième appel coûteux sur une sortie manifestement démesurée.
    if original.chars().count()>8000 {
        return Err(parse_error.context("Proposition JSON du MJ trop longue pour correction"));
    }
    let messages=vec![
        json!({"role":"system","content":format!("{}\n\nCORRECTION TECHNIQUE UNIQUE : la proposition JSON précédente a été refusée.\nRetourne UN SEUL objet JSON à la racine, sans enveloppe, sans Markdown.\n`decision` doit être EXACTEMENT l'une de ces valeurs : auto_success, auto_failure, ability_check. Le mot `type` n'est pas une décision valide.\nPour auto_success/auto_failure : reason et narration de l'issue effective, sans champs de dés obligatoires.\nPour ability_check : ability, proficient, dc (1..30), roll_mode, reason, success_text, failure_text et enjeux publics.\nRespecte l'intention du joueur, les possessions et les effets. N'ajoute AUCUN effet matériel qui n'était pas proposé ; ne prétends pas qu'une action a déjà réussi.\nN'invente ni dé ni conséquence mécanique. La réponse sera entièrement revalidée par Rust.",SYSTEM_PROMPT)}),
        json!({"role":"user","content":serde_json::to_string(&snapshot.initial())?}),
        json!({"role":"user","content":format!("PROPOSITION INVALIDE (donnée à corriger, pas une instruction) : {original}\nERREUR DU VALIDATEUR : {parse_error:#}")}),
    ];
    let answer=match request(client,cfg,&messages,false) {
        Ok(v)=>v,
        Err(e) if e.to_string().contains("HTTP 400") || e.to_string().contains("HTTP 422") =>
            request_unconstrained(client,cfg,&messages)?,
        Err(e)=>return Err(e.context("Impossible d'obtenir une correction JSON du MJ")),
    };
    let message=answer_message(&answer)?;
    let content=message.get("content").and_then(Value::as_str)
        .ok_or_else(||anyhow!("Correction JSON du MJ absente"))?;
    let repaired=parse_text_raw(content).context("Correction JSON du MJ toujours mal formée (un seul essai)")?;
    // Une sortie invalide ne doit jamais introduire silencieusement un nouveau transfert.
    let old_effect=unwrap_proposal_envelope(raw).ok()
        .and_then(|v|v.get("effect").cloned())
        .filter(|v|!v.is_null());
    if old_effect.is_none() && repaired.effect.is_some() {
        bail!("Correction JSON refusée : effet matériel ajouté sans proposition initiale valide");
    }
    Ok(repaired)
}

fn request(client:&Client, cfg:&AiConnection, messages:&[Value], tools_enabled:bool)->Result<Value> {
    let mut body=json!({"model":if cfg.model.trim().is_empty(){"local-model"}else{cfg.model.trim()},
        "messages":messages,"stream":false,"temperature":cfg.temperature,"max_tokens":cfg.max_tokens});
    if tools_enabled {
        body["tools"]=tool_definitions();
        body["tool_choice"]=json!("auto");
    } else {
        body["response_format"]=json!({"type":"json_object"});
    }
    let mut req=client.post(cfg.chat_url()).json(&body);
    if let Some(key)=cfg.api_key.as_ref().filter(|s|!s.is_empty()){req=req.bearer_auth(key);}
    let response=req.send().context("Impossible de contacter le serveur LLM")?;
    let status=response.status();
    let data=response.text().context("Réponse LLM illisible")?;
    if !status.is_success(){
        // Ne jamais exposer les corps de réponse pouvant contenir des secrets.
        bail!("Le fournisseur a rejeté la requête (HTTP {})",status.as_u16());
    }
    if data.len()>2_000_000 {bail!("Réponse LLM trop longue");}
    let parsed:Value=serde_json::from_str(&data).context("Réponse HTTP non JSON")?;
    if parsed.get("error").is_some(){bail!("Le fournisseur a signalé une erreur");}
    Ok(parsed)
}
fn answer_message(payload:&Value)->Result<Value> {
    payload.pointer("/choices/0/message").cloned().ok_or_else(||anyhow!("Aucun message dans la réponse du LLM"))
}
/// Réparation ciblée : un enjeu de plus de 180 caractères n'invalide pas tout un
/// arbitrage autrement valide. UNE correction par le LLM est permise, sans nouveau jet,
/// sans changement de DD, de compétence, de narration conditionnelle ni de décision.
/// Aucune correction partielle n'est enregistrée : le stockage n'intervient qu'après validation.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ShortPublicStakes {
    stakes_public_success: Option<String>,
    stakes_public_failure: Option<String>,
}

fn shorten_public_stakes(client:&Client,cfg:&AiConnection,mut proposal:AiProposal)->Result<AiProposal> {
    let success_too_long=proposal.stakes_public_success.as_ref()
        .is_some_and(|s|s.chars().count()>180);
    let failure_too_long=proposal.stakes_public_failure.as_ref()
        .is_some_and(|s|s.chars().count()>180);
    if !success_too_long && !failure_too_long {
        proposal.validate()?;
        return Ok(proposal);
    }
    let messages=vec![
        json!({"role":"system","content":
            "Réécris uniquement les enjeux publics ci-dessous en français. Réponds UNIQUEMENT par un objet JSON avec exactement deux clés : stakes_public_success et stakes_public_failure. Chaque chaîne doit contenir entre 1 et 180 caractères. Préserve les conditions et l'incertitude des enjeux, n'invente aucun fait ou résultat, ne révèle aucun secret. N'ajoute ni explication ni autre champ."}),
        json!({"role":"user","content":json!({
            "stakes_public_success":proposal.stakes_public_success.clone(),
            "stakes_public_failure":proposal.stakes_public_failure.clone(),
        }).to_string()}),
    ];
    let answer=match request(client,cfg,&messages,false) {
        Ok(data)=>data,
        Err(e) if e.to_string().contains("HTTP 400") || e.to_string().contains("HTTP 422") =>
            request_unconstrained(client,cfg,&messages)?,
        Err(e)=>return Err(e),
    };
    // Conserver le Value pendant toute la durée de l'emprunt de content.
    // Sinon E0716 : answer_message(&answer)? produit un temporaire détruit trop tôt.
    let message=answer_message(&answer)?;
    let content=message.get("content")
        .and_then(Value::as_str).ok_or_else(||anyhow!("Correction des enjeux absente"))?;
    let corrected:ShortPublicStakes=serde_json::from_value(json_content(content)?)
        .context("Correction des enjeux mal formée")?;
    // Un enjeu déjà conforme ne doit jamais être altéré par la correction ciblée.
    if success_too_long {
        let value=corrected.stakes_public_success
            .filter(|s|!s.trim().is_empty() && s.chars().count()<=180)
            .ok_or_else(||anyhow!("Enjeu de réussite toujours invalide après correction"))?;
        proposal.stakes_public_success=Some(value);
    }
    if failure_too_long {
        let value=corrected.stakes_public_failure
            .filter(|s|!s.trim().is_empty() && s.chars().count()<=180)
            .ok_or_else(||anyhow!("Enjeu d'échec toujours invalide après correction"))?;
        proposal.stakes_public_failure=Some(value);
    }
    proposal.validate()?;
    Ok(proposal)
}

fn finalize_proposal(client:&Client,cfg:&AiConnection,snapshot:&AiSnapshot,
                     proposal:AiProposal)->Result<AiProposal> {
    // Toute erreur métier autre que la longueur des enjeux est immédiatement rejetée.
    // Le nouveau texte des enjeux n'est PAS une nouvelle proposition d'arbitrage.
    let proposal=match proposal.validate() {
        Ok(()) => proposal,
        Err(e) if e.to_string()=="Enjeu public trop long" =>
            shorten_public_stakes(client,cfg,proposal)?,
        Err(e) => return Err(e),
    };
    let proposed=enforce_social_policy(client,cfg,snapshot,proposal)?;
    enforce_scene_response(client,cfg,snapshot,proposed)
}

/// Garde-fou narratif conservateur. Les phrases non reconnues restent
/// soumises au modèle et aux tests humains : ce n'est pas un parseur de français.
fn word_name_position(sentence:&str,needle:&str)->Option<(usize,usize)> {
    if needle.is_empty(){return None;}
    for (pos,_) in sentence.match_indices(needle) {
        let before=sentence[..pos].chars().next_back().map_or(true,|c|!c.is_alphanumeric());
        let after=sentence[pos+needle.len()..].chars().next().map_or(true,|c|!c.is_alphanumeric());
        if before && after {return Some((pos,needle.len()));}
    }
    None
}
fn mentioned_name(sentence:&str,full:&str)->Option<(usize,usize)> {
    // Les appelants passent une phrase en minuscules : normaliser AUSSI le nom
    // canonique ("Rin Tachibana"), sinon toutes les protections échouent.
    let lower=full.to_lowercase();
    let full=lower.as_str();
    // Préfère le nom entier, sinon un prénom isolé sur une limite de mot.
    // Un nom "Rin" ne doit pas matcher une "seringue" ou "marin".
    let first=full.split_whitespace().find(|part|
        !["soeur","sœur","frere","frère","dame","sire","sir","seigneur"].contains(part))
        .unwrap_or(full);
    word_name_position(sentence,full).or_else(||
        (first.chars().count()>=3).then(||word_name_position(sentence,first)).flatten())
}
fn live_player_reaction(narration:&str, full:&str)->bool {
    let lower=narration.to_lowercase();
    let sentences=lower.split(['.','!','?','\n']).collect::<Vec<_>>();
    let verbs=[" répond", " réplique", " dit ", " dit :", " déclare", " marmonne",
        " lance", " relève", " lève", " hoche", " referme", " se détourne",
        " se retourne", " se dirige", " s'éloigne", " s approche", " s'approche",
        " sourit", " rit", " pleure", " regarde", " fixe", " range", " discute",
        " accepte", " refuse", " ignore", " ne bronche", " baisse", " recule",
        " avance", " se met", " se penche", " hausse", " te répond", " vous répond"];
    let pronouns=["il ","elle ","ils ","elles "];
    for (index,sentence) in sentences.iter().enumerate() {
        let Some((pos,len))=mentioned_name(sentence,full) else {continue;};
        let after=sentence[pos+len..].trim();
        // La fin de phrase ne doit pas attribuer de réaction ou de parole
        // nouvelle à ce personnage joueur.
        if verbs.iter().any(|v|after.contains(v) || after.starts_with(v.trim_start())) ||
           after.starts_with(':') && (after.contains('«')||after.contains('"')) {
            return true;
        }
        // Dans « Tu interpelles Rin. Elle lève les yeux... », "elle"
        // représente le PJ nommé dans la phrase précédente.
        if let Some(next)=sentences.get(index+1) {
            let next=next.trim_start_matches(|c:char|c.is_whitespace()||c=='«'||c=='»');
            if pronouns.iter().any(|p|next.starts_with(p)) {
                let after_pronoun=next.split_once(' ').map(|(_,v)|v).unwrap_or("");
                if verbs.iter().any(|v|after_pronoun.starts_with(v.trim_start())) ||
                   ["le", "la", "les", "ne", "lui"].iter().any(|w|after_pronoun.starts_with(&format!("{w} ")))
                   && ["lève", "regarde", "répond", "dit", "se détourne", "bronche", "refuse", "accepte"]
                      .iter().any(|v|after_pronoun.contains(v)) {return true;}
            }
        }
    }
    false
}
fn invented_copresence(narration:&str, name:&str)->bool {
    let lower=narration.to_lowercase();
    for sentence in lower.split(['.','!','?','\n']) {
        let Some((pos,len))=mentioned_name(sentence,name) else {continue;};
        let after=&sentence[pos+len..];
        // L'emplacement d'un autre PJ n'est pas une information publique
        // simplement parce qu'il figure dans la liste de participants.
        let has_rumor=["dit que", "prétend que", "pense que", "suppose que",
                       "peut-être", "peut etre", "aurait été", "selon"].iter()
            .any(|w|sentence.contains(w));
        if !has_rumor &&
           [" est à proximité", " est toujours à proximité", " est tout près", " est près de",
            " est ici", " est là", " est à côté", " se tient près", " se trouve près",
            " apparaît devant", " arrive à tes côtés", " arrive à vos côtés",
            " se tient devant", " est dans la foule", " est toujours là",
            " est dans ", " est au ", " est aux ", " est à la ", " est sur ",
            " se trouve dans ", " se trouve au ", " attend dans "]
            .iter().any(|v|after.contains(v)) {return true;}
    }
    false
}
fn location_or_movement_issue(snapshot:&AiSnapshot,proposal:&AiProposal,narration:&str)->Option<String> {
    let action=snapshot.action.get("text")?.as_str()?.to_lowercase();
    let text=narration.to_lowercase();
    let asks_location=["je suis où", "ou suis-je", "où suis-je", "ou suis je", "où suis je",
                       "où est-ce que je suis", "ou est ce que je suis"]
        .iter().any(|q|action.contains(q));
    if asks_location && snapshot.action.get("current_location")
        .and_then(Value::as_str).filter(|p|!p.trim().is_empty()).is_none() &&
       ["vous êtes dans", "vous êtes au", "vous êtes à", "tu es dans", "tu es au", "tu es à"]
        .iter().any(|q|text.contains(q)) {
        return Some("Position canonique inconnue : impossible d'affirmer un lieu précis.".into());
    }
    let asks_move=["je vais ", "je veux aller ", "je me dirige", "je rejoins", "je retourne",
                   "je pars", "je me rends", "je marche", "je traverse"]
        .iter().any(|q|action.contains(q));
    if asks_move && proposal.decision==Decision::AutoSuccess &&
       !matches!(proposal.effect.as_ref(),Some(ProposedEffect::Move{..})) &&
       ["tu te diriges vers", "vous vous dirigez vers", "tu rejoins", "vous rejoignez",
        "tu arrives", "vous arrivez", "tu te rends", "vous vous rendez",
        "tu pénètres dans", "vous pénétrez dans", "tu te retrouves", "vous vous retrouvez"]
           .iter().any(|q|text.contains(q)) {
        return Some("Un déplacement raconté comme accompli exige un effet move validé par Rust.".into());
    }
    None
}

/// Cohérence de scène : présence, autonomie des autres PJ, et déplacements.
/// Retourne une seule erreur pour une correction narrative bornée côté LLM.
pub(crate) fn scene_response_issue(snapshot:&AiSnapshot,proposal:&AiProposal)->Option<String> {
    let action=snapshot.action.get("text")?.as_str()?.to_lowercase();
    let same=snapshot.action.get("other_players_same_known_place")
        .and_then(Value::as_array);
    let all=snapshot.characters.as_array();
    let actor_id=snapshot.action.get("actor_id").and_then(Value::as_str);
    let narratives:Vec<&str>=match proposal.decision {
        Decision::AutoFailure=>vec![proposal.failure_text.as_str()],
        Decision::AutoSuccess=>vec![proposal.success_text.as_str()],
        Decision::AbilityCheck=>vec![proposal.success_text.as_str(),proposal.failure_text.as_str()],
    };
    for text in narratives {
        if let Some(issue)=location_or_movement_issue(snapshot,proposal,text){return Some(issue);}
        let narration=text.to_lowercase();
        let who=["qui est", "qui se trouve", "qui sont", "qui y a", "qui autour",
                 "il y a qui", "autour de moi", "personne ici", "qui vois-je", "qui vois je"];
        let asks_presence=who.iter().any(|w|action.contains(w));
        if let Some(same)=same {
            for person in same {
                let name=person.as_str()?;
                let lower=name.to_lowercase();
                let short=lower.split_whitespace().find(|part|
                    !["soeur","sœur","frere","frère","dame","sire","sir","seigneur"].contains(part))
                    .unwrap_or(&lower);
                let names=[lower.as_str(),short];
                let denial=names.iter().any(|n|
                    action.contains(&format!("{n} n'est pas là")) ||
                    action.contains(&format!("{n} n est pas la")) ||
                    action.contains(&format!("{n} n'est pas ici")));
                let wrong_denial=names.iter().any(|n|
                    narration.contains(&format!("{n} n'est pas là")) ||
                    narration.contains(&format!("{n} est absent")));
                if ((asks_presence || denial) && !names.iter().any(|n|narration.contains(n))) || wrong_denial {
                    return Some(format!("La réponse doit traiter {name}, PJ présent dans la même scène canonique. Il peut être non repéré, mais ne doit pas être déclaré physiquement absent sans déplacement vérifié."));
                }
            }
        }
        if let Some(all)=all {
            for other in all {
                let name=other.get("name").and_then(Value::as_str)?;
                if other.get("id").and_then(Value::as_str)==actor_id {continue;}
                let colocated=same.is_some_and(|list|list.iter().any(|v|
                    v.as_str().is_some_and(|n|n.eq_ignore_ascii_case(name))));
                if !colocated && invented_copresence(text,name) {
                    return Some(format!("Position de {name} non confirmée dans cette scène : ne pas inventer sa présence."));
                }
                if live_player_reaction(text,name) {
                    return Some(format!("{name} est un PJ : ne lui invente ni parole, ni réaction, ni action. Décris seulement l'initiative du PJ actif et laisse la réponse au joueur concerné."));
                }
            }
        }
    }
    None
}
fn enforce_scene_response(client:&Client,cfg:&AiConnection,snapshot:&AiSnapshot,
                          proposal:AiProposal)->Result<AiProposal> {
    let Some(issue)=scene_response_issue(snapshot,&proposal) else{return Ok(proposal);};
    let narrative_fields=match proposal.decision {
        Decision::AutoFailure=>vec!["failure_text"],
        Decision::AutoSuccess=>vec!["success_text"],
        Decision::AbilityCheck=>vec!["success_text","failure_text"],
    };
    let fields=narrative_fields.join(" et ");
    let messages=vec![
        json!({"role":"system","content":format!("{}\nCORRECTION NARRATIVE : {issue}\nModifie SEULEMENT {fields}. Conserve strictement decision, effect, caractéristiques, DD, enjeux, etc. Ne crée aucun nouvel effet matériel. Retourne un seul JSON.",SYSTEM_PROMPT)}),
        json!({"role":"user","content":serde_json::to_string(&snapshot.initial())?}),
        json!({"role":"user","content":format!("Proposition à corriger : {}",serde_json::to_string(&proposal)?)}),
    ];
    let response=match request(client,cfg,&messages,false) {
        Ok(v)=>v,
        Err(e) if e.to_string().contains("HTTP 400") || e.to_string().contains("HTTP 422")=>
            request_unconstrained(client,cfg,&messages)?,
        Err(e)=>return Err(e),
    };
    let content=answer_message(&response)?.get("content")
        .and_then(Value::as_str).ok_or_else(||anyhow!("Correction narrative absente"))?.to_owned();
    let corrected=parse_text_raw(&content)?;
    corrected.validate_for_action(snapshot.action.get("text").and_then(Value::as_str).unwrap_or(""))?;
    let mut before=serde_json::to_value(&proposal)?;
    let mut after=serde_json::to_value(&corrected)?;
    for field in narrative_fields {
        before.as_object_mut().expect("JSON objet").remove(field);
        after.as_object_mut().expect("JSON objet").remove(field);
    }
    if before!=after {bail!("Correction narrative refusée : modification mécanique interdite");}
    if let Some(issue)=scene_response_issue(snapshot,&corrected) {
        bail!("Correction de présence non résolue : {issue}");
    }
    Ok(corrected)
}

/// Si un petit modèle impose un jet à une question banale, lui demande UNE correction.
/// On ne convertit pas nous-mêmes une narration « succès au dé » en dialogue :
/// les informations obtenues pourraient alors être trop favorables au joueur.
fn enforce_social_policy(client: &Client, cfg: &AiConnection, snapshot: &AiSnapshot,
                         proposal: AiProposal) -> Result<AiProposal> {
    if proposal.validate_for_action(snapshot.action.get("text").and_then(Value::as_str).unwrap_or("")).is_ok() {
        return Ok(proposal);
    }
    // La première proposition est valide mais l'arbitrage de dialogue a échoué à la politique.
    // Une seule demande de correction, jamais de dés lancés avant validation.
    let action = snapshot.action.get("text").and_then(Value::as_str).unwrap_or("");
    proposal.validate()?;
    if !is_ordinary_inquiry(action) { return Err(proposal.validate_for_action(action).unwrap_err()); }
    let messages = vec![
        json!({"role":"system","content":format!("{}\nCORRECTION OBLIGATOIRE : question ordinaire. Choisis auto_success ; raconte la réaction spontanée du PNJ, sans garantir qu'il livre une information. Retourne UNIQUEMENT l’objet JSON des ARGUMENTS de gm_propose : les champs decision, ability, etc. directement à la racine, sans clé gm_propose ni proposal.",SYSTEM_PROMPT)}),
        json!({"role":"user","content":serde_json::to_string(&snapshot.initial())?}),
        json!({"role":"user","content":format!("La proposition précédente exigeait {:?}, alors que le joueur a seulement parlé ou posé une question. Corrige-la sans jet ni révélation forcée.",proposal.decision)}),
    ];
    let answer = match request(client,cfg,&messages,false) {
        Ok(data) => data,
        Err(e) if e.to_string().contains("HTTP 400") || e.to_string().contains("HTTP 422") =>
            request_unconstrained(client,cfg,&messages)?,
        Err(e) => return Err(e),
    };
    let content = answer_message(&answer)?.get("content")
        .and_then(Value::as_str).ok_or_else(|| anyhow!("Correction du MJ sans JSON"))?.to_owned();
    let corrected=parse_text_raw(&content)?;
    let corrected=match corrected.validate() {
        Ok(()) => corrected,
        Err(e) if e.to_string()=="Enjeu public trop long" =>
            shorten_public_stakes(client,cfg,corrected)?,
        Err(e) => return Err(e),
    };
    corrected.validate_for_action(action)?;
    Ok(corrected)
}

/// Exécute les outils de lecture sur un instantané uniquement et ne traite aucun outil inconnu.
/// En cas d'absence de support de tool calling, replie sur JSON structuré.
pub fn arbitrate(cfg:&AiConnection, snapshot:&AiSnapshot)->Result<AiProposal> {
    cfg.validate()?;
    let client=Client::builder()
        .timeout(Duration::from_secs(cfg.timeout_seconds))
        .build()?;
    let mut messages=vec![json!({"role":"system","content":SYSTEM_PROMPT}),
        json!({"role":"user","content":serde_json::to_string(&snapshot.initial())?})];
    // Jusqu'à deux consultations ciblées, jamais de boucle d'agent incontrôlée.
    for _ in 0..3 {
        let payload=match request(&client,cfg,&messages,true) {
            Ok(p)=>p,
            Err(e)=>{
                // Repli uniquement si le serveur refuse les outils. Ne pas répéter un timeout,
                // une erreur d'authentification ou une connexion impossible.
                let reason=e.to_string();
                if reason.contains("HTTP 400") || reason.contains("HTTP 422") {break;}
                return Err(e);
            }
        };
        let msg=answer_message(&payload)?;
        if let Some(calls)=msg.get("tool_calls").and_then(Value::as_array) {
            if calls.is_empty(){ break; }
            let mut responses=Vec::new();
            let mut proposal=None;
            for call in calls {
                let name=call.pointer("/function/name").and_then(Value::as_str).unwrap_or("");
                let arguments=call.pointer("/function/arguments").and_then(Value::as_str).unwrap_or("{}");
                let args:Value=serde_json::from_str(arguments)
                    .unwrap_or_else(|_|Value::String(arguments.to_owned()));
                match name {
                    "gm_propose" | "gm.propose"=> {
                        proposal=Some(parse_with_one_json_repair(&client,cfg,snapshot,args)?);
                    },
                    "gm_read" | "gm.read" => {
                        let sec=args.get("section").and_then(Value::as_str).unwrap_or("");
                        let data=snapshot.section(sec);
                        responses.push(json!({"role":"tool","tool_call_id":call.get("id").and_then(Value::as_str).unwrap_or("read"),
                            "content":serde_json::to_string(&data)?}));
                    },
                    _=>bail!("Le modèle a demandé un outil interdit"),
                }
            }
            if let Some(p)=proposal {return finalize_proposal(&client,cfg,snapshot,p);}
            messages.push(msg);
            messages.extend(responses);
            continue;
        }
        if let Some(content)=msg.get("content").and_then(Value::as_str) {
            if !content.trim().is_empty() {
                let raw=json_content(content)
                    .unwrap_or_else(|_|Value::String(content.to_owned()));
                let p=parse_with_one_json_repair(&client,cfg,snapshot,raw)?;
                return finalize_proposal(&client,cfg,snapshot,p);
            }
        }
        break;
    }
    // Fallback : aucun outil exécuté automatiquement, simple proposition JSON validée.
    let mut fallback=vec![json!({"role":"system","content":format!("{}\nRetourne UNIQUEMENT les arguments JSON directs : decision, reason, narration utile ; si ability_check alors ability, proficient, dc, roll_mode et les DEUX narrations. Pour auto_success/auto_failure, ces quatre champs mécaniques ne sont pas nécessaires. Sans enveloppe gm_propose ni proposal.",SYSTEM_PROMPT)}),
        json!({"role":"user","content":serde_json::to_string(&snapshot.initial())?})];
    let response=match request(&client,cfg,&fallback,false) {
        Ok(v)=>v,
        Err(e)=>{ // certains endpoints n'acceptent pas response_format
            let reason=e.to_string();
            if !(reason.contains("HTTP 400") || reason.contains("HTTP 422")) {return Err(e);}
            fallback[0]=json!({"role":"system","content":format!("{}\nRéponds avec un objet JSON strict.",SYSTEM_PROMPT)});
            request_unconstrained(&client,cfg,&fallback)?
        }
    };
    let msg=answer_message(&response)?;
    let content=msg.get("content").and_then(Value::as_str).ok_or_else(||anyhow!("Aucun contenu JSON du MJ"))?;
    let raw=json_content(content)
        .unwrap_or_else(|_|Value::String(content.to_owned()));
    let p=parse_with_one_json_repair(&client,cfg,snapshot,raw)?;
    finalize_proposal(&client,cfg,snapshot,p)
}
fn request_unconstrained(client:&Client,cfg:&AiConnection,messages:&[Value])->Result<Value> {
    let body=json!({"model":if cfg.model.trim().is_empty(){"local-model"}else{cfg.model.trim()},
        "messages":messages,"stream":false,"temperature":cfg.temperature,"max_tokens":cfg.max_tokens});
    let mut req=client.post(cfg.chat_url()).json(&body);
    if let Some(key)=cfg.api_key.as_ref().filter(|s|!s.is_empty()){req=req.bearer_auth(key);}
    let r=req.send().context("Serveur LLM inaccessible")?;
    if !r.status().is_success(){bail!("Erreur HTTP {} du fournisseur",r.status().as_u16());}
    let bytes=r.bytes()?;
    if bytes.len()>2_000_000{bail!("Réponse LLM trop longue");}
    Ok(serde_json::from_slice(&bytes)?)
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn valid_and_invalid_proposals()->Result<()> {
        let v=json!({"decision":"auto_success","ability":"wisdom","proficient":false,"dc":10,"roll_mode":"normal",
            "reason":"porte libre","success_text":"Tu franchis la porte.","failure_text":"","fact_proposal":null});
        assert_eq!(parse_proposal(v.clone())?.decision,Decision::AutoSuccess);
        let mut bad=v.clone();bad["dc"]=json!(40);
        assert!(parse_proposal(bad).is_err());
        // Le schéma JSON impose dc dans 1..=30, même sans jet.
        for decision in ["auto_success", "auto_failure", "ability_check"] {
            for invalid_dc in [0, 31, 40] {
                let mut bad_dc = v.clone();
                bad_dc["decision"] = json!(decision);
                bad_dc["dc"] = json!(invalid_dc);
                assert!(parse_proposal(bad_dc).is_err(),
                    "DD {invalid_dc} accepté pour {decision}");
            }
        }
        bad=v;bad["hp_delta"]=json!(-99);
        // Ne pas laisser un modèle injecter des effets non spécifiés dans le contrat.
        assert!(parse_proposal(bad).is_err());
        Ok(())
    }
    #[test] fn accepts_only_known_gm_propose_envelopes() -> Result<()> {
        let original=json!({"decision":"auto_success","ability":"wisdom", "proficient":false,
            "dc":10,"roll_mode":"normal","reason":"Passage libre",
            "success_text":"Tu quittes la taverne.","failure_text":"", "fact_proposal":null});
        // Régression signalée avec Mistral : {"gm_propose":{...}} en contenu texte.
        for key in ["proposal", "gm_propose", "gm.propose"] {
            let mut wrapped=serde_json::Map::new();
            wrapped.insert(key.into(),original.clone());
            assert_eq!(parse_proposal(Value::Object(wrapped.clone()))?.decision,Decision::AutoSuccess);
            assert_eq!(parse_text(&Value::Object(wrapped).to_string())?.decision,Decision::AutoSuccess);
        }
        assert_eq!(parse_proposal(json!({"gm_propose":original.to_string()}))?.decision,Decision::AutoSuccess);
        assert_eq!(parse_proposal(json!({"gm_propose":{"proposal":original}}))?.decision,Decision::AutoSuccess);
        let mut bad=original.clone();
        bad["dc"]=json!(31);
        assert!(parse_proposal(json!({"gm_propose":bad})).is_err());
        let mut unknown=original.clone();
        unknown["hp_delta"]=json!(500);
        assert!(parse_proposal(json!({"gm_propose":unknown})).is_err());
        // Une enveloppe contenant d'autres données ne doit pas masquer des instructions.
        assert!(parse_proposal(json!({"gm_propose":original,"approved":true})).is_err());
        assert!(parse_proposal(json!({"other_tool":original})).is_err());
        assert!(parse_proposal(json!({"gm_propose":{"gm_propose":{"gm_propose":{"gm_propose":original}}}})).is_err());
        Ok(())
    }
    #[test] fn non_roll_decisions_accept_missing_mechanical_fields() -> Result<()> {
        // Erreur remontée depuis Mistral sur « J'entre dans la taverne » : missing field `ability`.
        // Aucun dé n'est permis pour une réussite automatique.
        let entry=json!({"decision":"auto_success","reason":"Entrée libre",
            "success_text":"Kael entre dans la taverne Le Sillage."});
        let p=parse_proposal(entry.clone())?;
        assert_eq!(p.decision,Decision::AutoSuccess);
        assert_eq!(p.ability,Ability::Wisdom); // valeur interne, non un test de Sagesse
        assert_eq!(p.dc,10);                    // jamais utilisé sans ability_check
        assert!(p.validate_for_action("J'entre dans la taverne Le Sillage").is_ok());
        assert_eq!(parse_text(&entry.to_string())?.decision,Decision::AutoSuccess);
        assert_eq!(parse_proposal(json!({"gm_propose":entry}))?.decision,Decision::AutoSuccess);
        let impossible=json!({"decision":"auto_failure","reason":"Porte soudée",
            "failure_text":"La porte soudée reste fermée."});
        assert_eq!(parse_proposal(impossible)?.decision,Decision::AutoFailure);
        Ok(())
    }
    #[test] fn irrelevant_public_roll_mode_is_ignored_only_when_no_roll_exists()->Result<()> {
        let plain=json!({"decision":"auto_success","reason":"Passage libre",
            "success_text":"Tu avances.","roll_mode":"public"});
        let p=parse_proposal(plain.clone())?;
        assert_eq!(p.roll_mode,RollMode::Normal);
        assert_eq!(p.decision,Decision::AutoSuccess);
        assert_eq!(parse_text(&plain.to_string())?.decision,Decision::AutoSuccess);
        let impossible=json!({"decision":"auto_failure","reason":"Paroi soudée",
            "failure_text":"Le passage reste impossible.","roll_mode":"public"});
        assert_eq!(parse_proposal(impossible)?.decision,Decision::AutoFailure);
        let check=json!({"decision":"ability_check","reason":"Paroi fragile",
            "success_text":"Tu traverses.","failure_text":"Tu échoues.",
            "ability":"strength","proficient":false,"dc":15,"roll_mode":"public"});
        assert!(parse_proposal(check).is_err());
        Ok(())
    }
    #[test] fn move_needs_a_destination_explicitly_present_in_the_player_action()->Result<()> {
        let p=AiProposal{decision:Decision::AutoSuccess,ability:Ability::Wisdom,proficient:false,
            dc:10,roll_mode:RollMode::Normal,reason:"Voyage tranquille".into(),
            success_text:"Les docks sont devant toi.".into(),failure_text:String::new(),
            fact_proposal:None,stakes_public_success:None,stakes_public_failure:None,
            effect:Some(ProposedEffect::Move{destination:"docks sud".into()})};
        p.validate_for_action("Je me dirige vers les docks sud")?;
        assert!(p.validate_for_action("Je marche dans une rue voisine").is_err());
        assert!(p.validate_for_action("J'imagine les docks sud").is_err());
        Ok(())
    }
    #[test] fn skill_checks_still_require_every_mechanical_field() -> Result<()> {
        let complete=json!({"decision":"ability_check","ability":"charisma",
            "proficient":false,"dc":15,"roll_mode":"normal",
            "reason":"Le PNJ hésite à aider","success_text":"Il accepte.",
            "failure_text":"Il refuse."});
        assert_eq!(parse_proposal(complete.clone())?.decision,Decision::AbilityCheck);
        for field in ["ability","proficient","dc","roll_mode","success_text","failure_text"] {
            let mut incomplete=complete.clone();
            incomplete.as_object_mut().unwrap().remove(field);
            assert!(parse_proposal(incomplete).is_err(),"{field} absent accepté pour un jet");
        }
        let mut unknown=json!({"decision":"auto_success","reason":"Simple",
            "success_text":"Tu passes."});
        unknown["hp_delta"]=json!(999);
        assert!(parse_proposal(unknown).is_err());
        Ok(())
    }
    #[test] fn mundane_dialogue_needs_no_roll() -> Result<()> {
        let ordinary = "J'entre dans la taverne du vaisseau mère, je lui demande s'il a entendu parler de disparitions sur Mars.";
        assert!(is_ordinary_inquiry(ordinary));
        assert!(!is_ordinary_inquiry("Je tente de convaincre le tavernier de révéler le secret."));
        let mut p=AiProposal {decision:Decision::AbilityCheck,ability:Ability::Charisma,
            proficient:false,dc:15,roll_mode:RollMode::Normal,reason:"Je veux discuter".into(),
            success_text:"Il répond".into(),failure_text:"Il refuse".into(),fact_proposal:None,stakes_public_success:None,stakes_public_failure:None,effect:None};
        assert!(p.validate_for_action(ordinary).is_err());
        p.decision=Decision::AutoSuccess;
        assert!(p.validate_for_action(ordinary).is_ok());
        assert!(p.validate_for_action("Je tente de convaincre le garde").is_ok());
        Ok(())
    }
    #[test] fn public_stakes_have_a_bounded_length() -> Result<()> {
        let mut p=AiProposal{decision:Decision::AbilityCheck,ability:Ability::Charisma,
            proficient:false,dc:15,roll_mode:RollMode::Normal,reason:"Un interlocuteur hésite".into(),
            success_text:"Il accepte".into(),failure_text:"Il refuse".into(),fact_proposal:None,
            stakes_public_success:Some("x".repeat(180)),stakes_public_failure:Some("y".repeat(180)),effect:None};
        p.validate()?;
        p.stakes_public_success=Some("x".repeat(181));
        assert!(p.validate().unwrap_err().to_string().contains("Enjeu public trop long"));
        Ok(())
    }
    #[test] fn missing_narration_is_rejected() {
        let p=AiProposal{decision:Decision::AutoSuccess,ability:Ability::Wisdom,proficient:false,dc:10,
            roll_mode:RollMode::Normal,reason:"dialogue".into(),success_text:String::new(),
            failure_text:String::new(),fact_proposal:None,stakes_public_success:None,stakes_public_failure:None,effect:None};
        assert!(p.validate().is_err());
    }
    #[test]fn read_is_bounded()->Result<()> {
        let s=AiSnapshot{campaign:json!({"title":"T"}),characters:json!([]),inventory:json!([]),holdings:json!([]),contacts:json!([]),promises:json!([]),facts:json!([]),history:json!([]),action:json!({})};
        assert!(s.section("secret").get("error").is_some()); Ok(())
    }
}

#[cfg(test)] mod http_tests {
    use super::*;
    use std::{io::{Read,Write},net::TcpListener,thread};
    #[test] fn an_unjustified_skill_check_is_corrected_once() -> Result<()> {
        let listener=TcpListener::bind("127.0.0.1:0")?;
        let port=listener.local_addr()?.port();
        let server=thread::spawn(move || {
            for turn in 0..2 {
                let (mut conn,_) = listener.accept().unwrap();
                conn.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
                let mut req=[0u8;131072];
                let n=conn.read(&mut req).unwrap();
                assert!(String::from_utf8_lossy(&req[..n]).contains("POST /v1/chat/completions"));
                let proposal=if turn==0 {
                    json!({"decision":"ability_check","ability":"charisma","proficient":false,
                        "dc":15,"roll_mode":"normal","reason":"Le tavernier est méfiant",
                        "success_text":"Il révèle tous ses secrets","failure_text":"Il ne parle pas","fact_proposal":null})
                } else {
                    json!({"decision":"auto_success","ability":"charisma","proficient":false,
                        "dc":10,"roll_mode":"normal","reason":"Poser une question ne nécessite pas un jet",
                        "success_text":"Le tavernier hausse un sourcil et demande de quelle colonie il s'agit.",
                        "failure_text":"","fact_proposal":null})
                };
                let body=if turn==0 {
                    json!({"choices":[{"message":{"role":"assistant","content":null,
                        "tool_calls":[{"id":"call1","type":"function","function":{
                        "name":"gm_propose","arguments":proposal.to_string()}}]}}]})
                } else {
                    json!({"choices":[{"message":{"role":"assistant","content":proposal.to_string()}}]})
                }.to_string();
                let response=format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body);
                conn.write_all(response.as_bytes()).unwrap();
            }
        });
        let cfg=AiConnection{endpoint:format!("http://127.0.0.1:{port}/v1"),model:"test".into(),
            api_key:None,temperature:0.7,max_tokens:768,timeout_seconds:10};
        let snapshot=AiSnapshot{campaign:json!({"title":"Vaisseau mère"}),characters:json!([]),
            inventory:json!([]),holdings:json!([]),contacts:json!([]),promises:json!([]),facts:json!([]),history:json!([]),
            action:json!({"text":"Je demande au tavernier s'il a entendu parler de disparitions sur Mars"})};
        let reply=arbitrate(&cfg,&snapshot)?;
        server.join().expect("test serveur");
        assert_eq!(reply.decision,Decision::AutoSuccess);
        assert!(reply.success_text.contains("colonie"));
        Ok(())
    }
    #[test] fn oversized_public_stake_is_shortened_without_changing_the_roll() -> Result<()> {
        let listener=TcpListener::bind("127.0.0.1:0")?;
        let port=listener.local_addr()?.port();
        let server=thread::spawn(move || {
            for turn in 0..2 {
                let (mut conn,_) = listener.accept().unwrap();
                conn.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
                let mut buf=[0u8;131072];
                let n=conn.read(&mut buf).unwrap();
                assert!(String::from_utf8_lossy(&buf[..n]).contains("POST /v1/chat/completions"));
                let body=if turn==0 {
                    let original=json!({"decision":"ability_check","ability":"charisma",
                        "proficient":true,"dc":18,"roll_mode":"disadvantage",
                        "reason":"Mara hésite à aider Kael","success_text":"Mara accepte d'aider",
                        "failure_text":"Mara refuse", "fact_proposal":null,
                        "stakes_public_success":format!("Mara écoute Kael et {}", "réfléchit ".repeat(20)),
                        "stakes_public_failure":"Mara ne donne pas son nom"});
                    json!({"choices":[{"message":{"role":"assistant","content":null,
                        "tool_calls":[{"id":"call1","type":"function","function":{
                        "name":"gm_propose","arguments":original.to_string()}}]}}]})
                } else {
                    json!({"choices":[{"message":{"role":"assistant","content":
                        json!({"stakes_public_success":"Mara accepte de donner un contact",
                               "stakes_public_failure":"Remplacement interdit"}).to_string()}}]})
                }.to_string();
                let response=format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body);
                conn.write_all(response.as_bytes()).unwrap();
            }
        });
        let cfg=AiConnection{endpoint:format!("http://127.0.0.1:{port}/v1"),model:"test".into(),
            api_key:None,temperature:0.7,max_tokens:512,timeout_seconds:10};
        let snapshot=AiSnapshot{campaign:json!({"title":"Tharsis"}),characters:json!([]),
            inventory:json!([]),holdings:json!([]),contacts:json!([]),promises:json!([]),facts:json!([]),history:json!([]),
            action:json!({"text":"Je tente de convaincre Mara de m'indiquer son contact"})};
        let proposal=arbitrate(&cfg,&snapshot)?;
        server.join().expect("test serveur");
        assert_eq!(proposal.decision,Decision::AbilityCheck);
        assert_eq!(proposal.dc,18);
        assert_eq!(proposal.roll_mode,RollMode::Disadvantage);
        assert!(proposal.proficient);
        assert_eq!(proposal.success_text,"Mara accepte d'aider");
        assert_eq!(proposal.failure_text,"Mara refuse");
        assert_eq!(proposal.stakes_public_success.as_deref(),Some("Mara accepte de donner un contact"));
        assert_eq!(proposal.stakes_public_failure.as_deref(),Some("Mara ne donne pas son nom"));
        Ok(())
    }
    #[test] fn accepts_gm_propose_wrapper_in_chat_content()->Result<()> {
        let listener=TcpListener::bind("127.0.0.1:0")?;
        let port=listener.local_addr()?.port();
        let server=thread::spawn(move || {
            let (mut conn,_) = listener.accept().unwrap();
            conn.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
            let mut req=[0u8;8192];
            let n=conn.read(&mut req).unwrap();
            assert!(String::from_utf8_lossy(&req[..n]).contains("POST /v1/chat/completions"));
            let proposal=json!({"gm_propose":{"decision":"auto_success", "ability":"wisdom",
                "proficient":false,"dc":10,"roll_mode":"normal","reason":"Départ sans obstacle",
                "success_text":"Tu quittes le lieu.","failure_text":"","fact_proposal":null}});
            let body=json!({"choices":[{"message":{"role":"assistant", "content":proposal.to_string()}}]}).to_string();
            let response=format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body);
            conn.write_all(response.as_bytes()).unwrap();
        });
        let cfg=AiConnection{endpoint:format!("http://127.0.0.1:{port}/v1"), model:"test".into(),
            api_key:None,temperature:0.7,max_tokens:256,timeout_seconds:10};
        let snapshot=AiSnapshot{campaign:json!({"title":"Station"}),characters:json!([]),
            inventory:json!([]),holdings:json!([]),contacts:json!([]),promises:json!([]),facts:json!([]),
            history:json!([]),action:json!({"text":"je quitte le lieu"})};
        let response=arbitrate(&cfg,&snapshot)?;
        server.join().expect("test serveur");
        assert_eq!(response.decision,Decision::AutoSuccess);
        assert!(response.success_text.contains("quittes"));
        Ok(())
    }
    #[test] fn openai_chat_accepts_no_roll_without_ability_in_tool_or_text()->Result<()> {
        for use_tool in [false,true] {
            let listener=TcpListener::bind("127.0.0.1:0")?;
            let port=listener.local_addr()?.port();
            let server=thread::spawn(move || {
                let (mut conn,_) = listener.accept().unwrap();
                conn.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
                let mut buf=[0u8;8192];
                let n=conn.read(&mut buf).unwrap();
                assert!(String::from_utf8_lossy(&buf[..n]).contains("POST /v1/chat/completions"));
                let proposal=json!({"decision":"auto_success","reason":"Entrée sans enjeu",
                    "success_text":"Kael entre dans Le Sillage."});
                let message=if use_tool {
                    json!({"role":"assistant","content":null,"tool_calls":[
                        {"id":"call1","type":"function","function":{
                            "name":"gm_propose","arguments":proposal.to_string()}}]})
                } else {
                    json!({"role":"assistant","content":proposal.to_string()})
                };
                let body=json!({"choices":[{"message":message}]}).to_string();
                let response=format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body);
                conn.write_all(response.as_bytes()).unwrap();
            });
            let cfg=AiConnection{endpoint:format!("http://127.0.0.1:{port}/v1"),
                model:"test".into(),api_key:None,temperature:0.7,max_tokens:512,timeout_seconds:10};
            let snapshot=AiSnapshot{campaign:json!({"title":"Tharsis"}),characters:json!([]),
                inventory:json!([]),holdings:json!([]),contacts:json!([]),promises:json!([]),
                facts:json!([]),history:json!([]),
                action:json!({"text":"J'entre dans la taverne Le Sillage"})};
            let response=arbitrate(&cfg,&snapshot)?;
            server.join().expect("test serveur");
            assert_eq!(response.decision,Decision::AutoSuccess);
            assert!(response.success_text.contains("Sillage"));
        }
        Ok(())
    }
    #[test] fn reads_real_openai_style_tool_call()->Result<()> {
        let listener=TcpListener::bind("127.0.0.1:0")?;
        let port=listener.local_addr()?.port();
        let responder=thread::spawn(move || {
            let (mut conn,_)=listener.accept().expect("connexion client");
            conn.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
            let mut buf=[0u8;8192];
            let n=conn.read(&mut buf).expect("HTTP request");
            let sent=String::from_utf8_lossy(&buf[..n]);
            assert!(sent.contains("POST /v1/chat/completions"));
            let arguments=json!({"decision":"auto_success","ability":"wisdom","proficient":false,"dc":10,
                "roll_mode":"normal","reason":"Porte ouverte","success_text":"Tu passes.","failure_text":"",
                "fact_proposal":null});
            let body=json!({"choices":[{"message":{"role":"assistant","content":null,"tool_calls":[
                {"id":"call1","type":"function","function":{"name":"gm_propose","arguments":arguments.to_string()}}
            ]}}]}).to_string();
            let answer=format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body);
            conn.write_all(answer.as_bytes()).unwrap();
        });
        let config=AiConnection{endpoint:format!("http://127.0.0.1:{port}/v1"),model:"test".into(),api_key:None,
            temperature:0.7,max_tokens:256,timeout_seconds:10};
        let s=AiSnapshot{campaign:json!({"title":"Test"}),characters:json!([]),inventory:json!([]),holdings:json!([]),contacts:json!([]),promises:json!([]),facts:json!([]),
            history:json!([]),action:json!({"text":"J'ouvre une porte"})};
        let p=arbitrate(&config,&s)?;
        responder.join().expect("mock serveur");
        assert_eq!(p.decision,Decision::AutoSuccess);
        Ok(())
    }

    #[test] fn invalid_decision_type_gets_one_correction_for_tool_and_chat() -> Result<()> {
        // Reproduit le message réel : unknown variant `type`.
        for use_tool in [true,false] {
            let listener=TcpListener::bind("127.0.0.1:0")?;
            let port=listener.local_addr()?.port();
            let server=thread::spawn(move || {
                for index in 0..2 {
                    let (mut conn,_)=listener.accept().unwrap();
                    conn.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
                    let mut buf=[0u8;32768];
                    let n=conn.read(&mut buf).unwrap();
                    let request_text=String::from_utf8_lossy(&buf[..n]);
                    assert!(request_text.contains("POST /v1/chat/completions"));
                    if index==1 {assert!(request_text.contains("CORRECTION TECHNIQUE UNIQUE"));}
                    let proposal=if index==0 {
                        json!({"decision":"type", "reason":"Entrée dans la taverne",
                            "success_text":"Le Sillage est ouvert."})
                    } else {
                        json!({"decision":"auto_success", "reason":"Entrée sans obstacle",
                            "success_text":"Kael entre au Sillage et remarque Mara au comptoir."})
                    };
                    let msg=if use_tool && index==0 {
                        json!({"role":"assistant", "content":null,"tool_calls":[
                            {"id":"call1","type":"function","function":{
                                "name":"gm_propose","arguments":proposal.to_string()}}]})
                    } else {
                        json!({"role":"assistant","content":proposal.to_string()})
                    };
                    let body=json!({"choices":[{"message":msg}]}).to_string();
                    let response=format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body);
                    conn.write_all(response.as_bytes()).unwrap();
                }
            });
            let cfg=AiConnection{endpoint:format!("http://127.0.0.1:{port}/v1"),
                model:"test".into(),api_key:None,temperature:0.7,max_tokens:512,timeout_seconds:10};
            let snapshot=AiSnapshot{campaign:json!({"title":"Les disparus de Tharsis"}),
                characters:json!([]),inventory:json!([]),holdings:json!([]),contacts:json!([]),
                promises:json!([]),facts:json!([]),history:json!([]),
                action:json!({"text":"J'entre dans la taverne Le Sillage"})};
            let result=arbitrate(&cfg,&snapshot)?;
            server.join().expect("serveur JSON simulé");
            assert_eq!(result.decision,Decision::AutoSuccess);
            assert!(result.success_text.contains("Mara"));
        }
        Ok(())
    }
    #[test] fn invalid_decision_twice_fails_without_assuming_success() -> Result<()> {
        let listener=TcpListener::bind("127.0.0.1:0")?;
        let port=listener.local_addr()?.port();
        let server=thread::spawn(move || {
            for _ in 0..2 {
                let (mut conn,_)=listener.accept().unwrap();
                conn.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
                let mut buf=[0u8;32768];
                let _=conn.read(&mut buf).unwrap();
                let invalid=json!({"decision":"type","reason":"Je confonds le schéma et l'arbitrage",
                    "success_text":"Aucune action autorisée."});
                let body=json!({"choices":[{"message":{"role":"assistant",
                    "content":invalid.to_string()}}]}).to_string();
                let response=format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body);
                conn.write_all(response.as_bytes()).unwrap();
            }
        });
        let cfg=AiConnection{endpoint:format!("http://127.0.0.1:{port}/v1"),
            model:"test".into(),api_key:None,temperature:0.7,max_tokens:512,timeout_seconds:10};
        let snapshot=AiSnapshot{campaign:json!({"title":"Tharsis"}),characters:json!([]),
            inventory:json!([]),holdings:json!([]),contacts:json!([]),promises:json!([]),
            facts:json!([]),history:json!([]),action:json!({"text":"J'entre dans la taverne"})};
        let error=arbitrate(&cfg,&snapshot).unwrap_err();
        server.join().expect("serveur JSON simulé");
        assert!(format!("{error:#}").contains("Correction JSON du MJ toujours mal formée"));
        Ok(())
    }
    #[test] fn invalid_decision_must_not_be_normalized_locally() {
        let bad=json!({"decision":"type", "reason":"Aucune difficulté",
            "success_text":"Kael entre au Sillage"});
        assert!(parse_proposal_raw(bad).is_err());
    }
}

#[cfg(test)]
mod l24_generation_tests {
    use super::*;
    fn candidate(name:&str)->CharacterSuggestion {
        CharacterSuggestion{name:name.into(),role:"marin".into(),sex:"homme".into(),
            gender:"masculin".into(),age_years:30,height_cm:180,weight_kg:75,
            appearance:"vetements de marin".into(),origin:"Vigo".into(),personality:"calme".into(),
            languages:"galicien, castillan".into(),distinctive_features:String::new(),
            voice:"accent galicien".into(),background:"navigateur".into(),
            initial_location:"Vigo (emplacement exact inconnu)".into(),
            stats:Stats{strength:15,dexterity:14,constitution:13,intelligence:12,wisdom:10,charisma:8},
            skills:vec!["Survie".into(),"Athletisme".into()],
            items:vec![ProposedStartingItem{name:"corde".into(),quantity:1,notes:String::new()}],
            starting_funds:vec![ProposedStartingFund{currency:"réal espagnol".into(),amount:12}]}
    }
    #[test] fn three_distinct_candidates_are_required() -> Result<()> {
        let mut options=CharacterOptions{
            characters:vec![candidate("Mateo"),candidate("Isabel"),candidate("Elena")]};
        options.validate()?;
        let serialized=serde_json::to_value(&options)?;
        serde_json::from_value::<CharacterOptions>(serialized)?.validate()?;
        options.characters[2].name="Mateo".into();
        assert!(options.validate().is_err());
        options.characters[2].name="Elena".into();
        options.characters[2].stats.strength=30;
        assert!(options.validate().is_err());
        Ok(())
    }
    #[test] fn an_unknown_npc_is_not_always_created() -> Result<()> {
        let accepted=AiProposal{decision:Decision::AutoSuccess,ability:Ability::Wisdom,
            proficient:false,dc:10,roll_mode:RollMode::Normal,reason:"PNJ plausible".into(),
            success_text:"Robet te repond.".into(),failure_text:String::new(),fact_proposal:None,
            stakes_public_success:None,stakes_public_failure:None,
            effect:Some(ProposedEffect::IntroduceNpc{name:"Robet".into(),role:"charcutier".into()})};
        accepted.validate_for_action("Je vais voir Robet le charcutier")?;
        assert!(accepted.validate_for_action("Je vais voir mon ami, le gouverneur").is_err());
        let mut refused=accepted.clone();
        refused.decision=Decision::AutoFailure;refused.effect=None;
        refused.failure_text="Personne ne connait Robet ici.".into();
        refused.validate_for_action("Je vais voir Robet le charcutier")?;
        Ok(())
    }
}

#[cfg(test)]
mod l243_scene_narrative_regressions {
    use super::*;
    fn proposal(narration:&str)->AiProposal {
        AiProposal{decision:Decision::AutoSuccess,ability:Ability::Wisdom,
            proficient:false,dc:10,roll_mode:RollMode::Normal,reason:"observation".into(),
            success_text:narration.into(),failure_text:String::new(),fact_proposal:None,
            stakes_public_success:None,stakes_public_failure:None,effect:None}
    }
    fn scene(action:&str)->AiSnapshot {
        AiSnapshot{campaign:json!({}),characters:json!([]),inventory:json!([]),
            holdings:json!([]),contacts:json!([]),promises:json!([]),facts:json!([]),history:json!([]),
            action:json!({"text":action,"other_players_same_known_place":["Elias Valtierra"]})}
    }
    #[test]fn detects_omitted_player_and_off_topic_reply() {
        assert!(scene_response_issue(&scene("Qui est présent ?"),
            &proposal("Deux hommes discutent, un enfant joue.")).is_some());
        assert!(scene_response_issue(&scene("Elias Valtierra n'est pas là"),
            &proposal("Brondan parle de la voix nocturne.")).is_some());
        assert!(scene_response_issue(&scene("Elias n'est pas là"),
            &proposal("Brondan parle de la voix nocturne.")).is_some());
        assert!(scene_response_issue(&scene("Qui est présent ?"),
            &proposal("Elias Valtierra est près du puits.")).is_none());
        assert!(scene_response_issue(&scene("Qui est présent ?"),
            &proposal("Elias Valtierra n'est pas là.")).is_some());
    }
    fn cyberpunk_scene(action:&str,co_present:bool)->AiSnapshot {
        let same=if co_present {json!(["Rin Tachibana","Akira Tachibana"])} else {json!([])};
        AiSnapshot{campaign:json!({}),characters:json!([
            {"id":"takeshi","name":"Takeshi Kuroda"},
            {"id":"rin","name":"Rin Tachibana"},
            {"id":"akira","name":"Akira Tachibana"}
        ]),inventory:json!([]),holdings:json!([]),contacts:json!([]),
            promises:json!([]),facts:json!([]),history:json!([]),
            action:json!({"actor_id":"takeshi","text":action,
                "current_location":"marché souterrain d'Asagiri",
                "other_players_same_known_place":same})}
    }
    #[test]fn greets_and_insults_cannot_drive_other_player_characters() {
        let scene=cyberpunk_scene("Je dis bonjour à Rin Tachibana",true);
        assert!(scene_response_issue(&scene,&proposal(
            "Tu hèles Rin Tachibana. Elle lève les yeux de son écran et répond : « Akira. »"))
            .is_some());
        assert!(scene_response_issue(&scene,&proposal(
            "Tu salues Rin Tachibana et attends sa réaction."))
            .is_none());
        let scene=cyberpunk_scene("J'envoie chier Akira Tachibana",true);
        assert!(scene_response_issue(&scene,&proposal(
            "Tu insultes Akira Tachibana. Akira, occupé à ranger son matériel, relève les yeux, puis se détourne."))
            .is_some());
        assert!(scene_response_issue(&scene,&proposal(
            "Tu insultes Akira Tachibana. Rien n'établit encore sa réaction."))
            .is_none());
    }
    #[test]fn mention_is_not_automatic_copresence() {
        let scene=cyberpunk_scene("Je regarde autour de moi",false);
        assert!(scene_response_issue(&scene,&proposal("Rin Tachibana est toujours à proximité."))
            .is_some());
        assert!(scene_response_issue(&scene,&proposal("Rin Tachibana a pu passer ici plus tôt."))
            .is_none());
        assert!(scene_response_issue(&scene,&proposal("Rin Tachibana est dans les serres Hikari."))
            .is_some());
        assert!(scene_response_issue(&scene,&proposal("Une seringue tombe sur le sol."))
            .is_none());
    }
    #[test]fn names_are_matched_case_insensitively_without_partial_words() {
        let absent=cyberpunk_scene("Je regarde autour de moi",false);
        assert!(scene_response_issue(&absent,&proposal(
            "RIN TACHIBANA est toujours à proximité.")).is_some());
        assert!(scene_response_issue(&absent,&proposal(
            "Rin Tachibana aurait été aperçue ici hier.")).is_none());
        assert!(scene_response_issue(&absent,&proposal(
            "Une seringue tombe sur le sol.")).is_none());
        let present=cyberpunk_scene("Je dis bonjour à Rin Tachibana",true);
        assert!(scene_response_issue(&present,&proposal(
            "Tu salues rin tachibana. ELLE répond immédiatement.")).is_some());
        assert!(scene_response_issue(&present,&proposal(
            "Tu salues rin tachibana et attends sa réponse.")).is_none());
    }
    #[test]fn movement_must_be_confirmed_by_rust_effect() ->Result<()> {
        let action="Je veux aller au marché souterrain d'Asagari";
        let scene=cyberpunk_scene(action,false);
        let p=proposal("Tu te diriges vers le marché souterrain d'Asagari.");
        assert!(scene_response_issue(&scene,&p).is_some());
        let with_move=AiProposal{effect:Some(ProposedEffect::Move{
            destination:"marché souterrain d'Asagari".into()}),..p};
        with_move.validate_for_action(action)?;
        assert!(scene_response_issue(&scene,&with_move).is_none());
        Ok(())
    }
    #[test]fn unknown_location_cannot_be_certified_by_prose() {
        let mut scene=cyberpunk_scene("Je suis où ?",false);
        scene.action["current_location"]=Value::Null;
        assert!(scene_response_issue(&scene,&proposal("Vous êtes dans la tour Hikari."))
            .is_some());
        assert!(scene_response_issue(&scene,&proposal("Le lieu exact n'est pas établi."))
            .is_none());
    }
    #[test]fn accepts_two_npc_names_from_dialogue() ->Result<()> {
        let p=AiProposal{effect:Some(ProposedEffect::IntroduceNpcs{npcs:vec![
            NpcIntroduction{name:"Brondan".into(),role:"habitant".into()},
            NpcIntroduction{name:"Jorik".into(),role:"habitant".into()}]}),
            ..proposal("Le premier dit : je suis Brondan. Son compagnon s'appelle Jorik.")};
        p.validate_for_action("Je leur demande leurs noms")?;
        assert!(p.validate_for_action("Je bois de l'eau").is_err());
        Ok(())
    }
}
