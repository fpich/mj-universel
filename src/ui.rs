//! UI L2.3 : chat de jeu chronologique, jets autorisés uniquement par UARE.
use crate::storage::{AppSettings, Campaign, CampaignMode, Storage, Character, CharacterProfile};
use crate::uare::{ActionRequest,Decision};
use uuid::Uuid;
use anyhow::Result;
use eframe::egui;
use crate::llm::{self,AiProposal,AiConnection,CharacterOptions};
use crate::scenarios::SCENARIOS;
use std::{sync::mpsc::{self,Receiver,TryRecvError},thread};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Screen { Home, Create, Campaign, Play, Settings }

struct CreationDraft {
    title: String,
    context: String,
    players: u8,
    preset_search: String,
    preset_players: u8, // 0 = afficher les scénarios de toutes les tailles
}
impl Default for CreationDraft {
    fn default() -> Self {
        Self { title:String::new(), context:String::new(), players:1,
            preset_search:String::new(), preset_players:0 }
    }
}

struct EditionDraft {
    id: String,
    title: String,
    mode: CampaignMode,
    context: String,
    participants: Vec<String>,
}

#[derive(Clone,Copy,PartialEq,Eq)]
enum PlayTab { Story, Character, Journal, Memory }
struct PlayDraft {
 tab:PlayTab, selected:Option<String>,character_draft:Option<Character>,action_text:String,
 item_name:String,item_quantity:i32,item_notes:String,
 holding_name:String,holding_quantity:u32,npc_name:String,currency_name:String,profile_draft:Option<CharacterProfile>,
 abandon_confirm:bool, history_limit:usize,
}
impl Default for PlayDraft {
 fn default()->Self {Self{tab:PlayTab::Story,selected:None,character_draft:None,
  action_text:String::new(),item_name:String::new(),item_quantity:1,item_notes:String::new(),
  holding_name:String::new(),holding_quantity:1,npc_name:String::new(),currency_name:String::new(),profile_draft:None,abandon_confirm:false,history_limit:120}}
}

struct SetupPending {
 receiver:Receiver<std::result::Result<CharacterOptions,String>>,
 campaign:String,character:String,state_version:i64,
}
struct AiPending {
 receiver:Receiver<std::result::Result<AiProposal,String>>,
 key:String,campaign:String,actor:String,action:String,state_version:i64,model:String,
}
pub struct GameApp {
    storage: Storage,
    screen: Screen,
    campaigns: Vec<Campaign>,
    selected: Option<String>,
    creation: CreationDraft,
    edition: Option<EditionDraft>,
    settings: AppSettings,
    settings_draft: AppSettings,
    show_archived: bool,
    delete_confirmation: Option<String>,
    start_confirmation: bool,
    toast: Option<(bool,String)>,
    play:PlayDraft,
    ai_pending:Option<AiPending>,
    setup_pending:Option<SetupPending>,
    setup_choices:Option<(String,CharacterOptions)>,
    api_key_session:Option<String>,
    api_key_input:String,
    api_master:String,
}

impl GameApp {
    pub fn new(storage: Storage) -> Result<Self> {
        let campaigns=storage.list_campaigns()?;
        let settings=storage.load_settings()?;
        let selected=settings.last_campaign_id.clone().filter(|id| campaigns.iter().any(|c| &c.id==id));
        let mut app=Self {
            storage,
            screen: if selected.is_some() { Screen::Campaign } else { Screen::Home },
            campaigns, selected,
            creation: CreationDraft::default(), edition: None,
            settings_draft:settings.clone(), settings,
            show_archived:true, delete_confirmation:None, start_confirmation:false, toast:None, play:PlayDraft::default(),
            ai_pending:None,setup_pending:None,setup_choices:None,api_key_session:None,api_key_input:String::new(),api_master:String::new(),
        };
        if let Some(id)=app.selected.clone() {
            app.load_edition(&id)?;
        }
        Ok(app)
    }

    fn message(&mut self, result: Result<()>, success: &str) {
        self.toast=Some(match result {
            Ok(()) => (false,success.to_owned()),
            Err(err) => (true,format!("{err:#}")),
        });
    }

    fn refresh_campaigns(&mut self) -> Result<()> {
        self.campaigns=self.storage.list_campaigns()?;
        Ok(())
    }

    fn load_edition(&mut self,id:&str) -> Result<()> {
        let camp=self.storage.get_campaign(id)?
            .ok_or_else(|| anyhow::anyhow!("Campagne introuvable"))?;
        let participants=self.storage.participants(id)?;
        self.edition=Some(EditionDraft {
            id:id.to_owned(),
            title:camp.title, mode:camp.mode, context:camp.seed_context,
            participants:participants.into_iter().map(|p|p.display_name).collect(),
        });
        Ok(())
    }

    fn open_campaign(&mut self,id:&str) {
        let result=(|| -> Result<()> {
            self.load_edition(id)?;
            self.selected=Some(id.to_owned());
            self.settings.last_campaign_id=Some(id.to_owned());
            self.settings_draft.last_campaign_id=Some(id.to_owned());
            self.storage.save_settings(&self.settings)?;
            self.screen=Screen::Campaign;
            self.play=PlayDraft::default();self.setup_pending=None;self.setup_choices=None;
            self.delete_confirmation=None;
            self.start_confirmation=false;
            Ok(())
        })();
        self.message(result,"Campagne ouverte et sélection mémorisée.");
    }

    fn create_campaign(&mut self) {
        let result=(|| -> Result<String> {
            let id=self.storage.create_campaign(
                &self.creation.title, CampaignMode::Free, &self.creation.context, self.creation.players)?;
            self.refresh_campaigns()?;
            Ok(id)
        })();
        match result {
            Ok(id) => {
                self.creation=CreationDraft::default();
                self.open_campaign(&id);
                self.toast=Some((false,"Campagne enregistrée dans SQLite.".into()));
            }
            Err(err) => self.toast=Some((true,format!("{err:#}"))),
        }
    }

    fn save_edition(&mut self) {
        let result=(|| -> Result<()> {
            let draft=self.edition.as_ref().ok_or_else(||anyhow::anyhow!("Aucune édition en cours"))?;
            // Les edits préparatoires des fiches/biens font aussi évoluer state_version.
            // Le processus local utilise la dernière version pour ce formulaire initial.
            let version=self.storage.get_campaign(&draft.id)?
                .ok_or_else(||anyhow::anyhow!("Campagne introuvable"))?.state_version;
            self.storage.save_setup(&draft.id,version,&draft.title,
                draft.mode,&draft.context,&draft.participants)?;
            let id=draft.id.clone();
            self.refresh_campaigns()?;
            self.load_edition(&id)?;
            Ok(())
        })();
        self.message(result,"Modifications de la campagne enregistrées.");
    }

    fn archive(&mut self,id:&str,archived:bool) {
        let result=(|| -> Result<()> {
            self.storage.set_archived(id,archived)?;
            self.refresh_campaigns()?;
            if self.selected.as_deref()==Some(id) { self.load_edition(id)?; }
            Ok(())
        })();
        self.message(result,if archived {"Campagne archivée."} else {"Campagne réactivée."});
    }

    fn delete_campaign(&mut self,id:&str) {
        let result=(|| -> Result<()> {
            if !self.storage.delete_campaign(id)? { anyhow::bail!("Campagne introuvable"); }
            if self.selected.as_deref()==Some(id) {
                self.selected=None;
                self.edition=None;
                self.settings.last_campaign_id=None;
                self.settings_draft.last_campaign_id=None;
                self.storage.save_settings(&self.settings)?;
            }
            self.refresh_campaigns()?;
            self.screen=Screen::Home;
            self.delete_confirmation=None;
            self.start_confirmation=false;
            Ok(())
        })();
        self.message(result,"Campagne et joueurs supprimés définitivement.");
    }

    fn navigation(&mut self,ctx:&egui::Context) {
        egui::SidePanel::left("navigation").resizable(false).default_width(200.0).show(ctx,|ui| {
            ui.add_space(12.0);
            ui.heading("MJ universel");
            ui.small("L2.4.2 · Économie libre et contextes préchargés");
            ui.separator();
            if ui.selectable_label(self.screen==Screen::Home,"Campagnes").clicked() { self.screen=Screen::Home; }
            if ui.selectable_label(self.screen==Screen::Create,"Nouvelle campagne").clicked() {
                self.screen=Screen::Create; self.delete_confirmation=None;
            }
            if self.selected.is_some() && ui.selectable_label(self.screen==Screen::Campaign,"Campagne sélectionnée").clicked() {
                self.screen=Screen::Campaign;
            }
            if self.selected.is_some() && ui.selectable_label(self.screen==Screen::Play,"Jouer · Chat RP").clicked() {
                self.screen=Screen::Play;
            }
            if ui.selectable_label(self.screen==Screen::Settings,"Paramètres").clicked() {
                self.screen=Screen::Settings; self.settings_draft=self.settings.clone();
            }
            ui.separator();
            ui.small(format!("{} campagne(s)",self.campaigns.len()));
            ui.add_space(10.0);
            ui.label("État du lot");
            ui.small("✓ Navigation native");
            ui.small("✓ SQLite · schéma L2 neuf");
            ui.small("✓ CRUD campagnes");
            ui.small("✓ Paramètres persistants");
            ui.small("✓ Personnages, inventaire");
            ui.small("✓ Dés et actions libres");
            ui.small("✓ Journal atomique SQLite");
            ui.add_space(8.0);
            ui.weak("MJ IA : gm_read / gm_propose");
            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT),|ui| {
                ui.separator();
                ui.small("Aucun fichier de configuration externe");
            });
        });
    }

    fn home(&mut self,ui:&mut egui::Ui) {
        ui.heading("Mes campagnes");
        ui.label("Créer, reprendre, archiver ou supprimer une campagne. Les données restent dans SQLite.");
        ui.horizontal(|ui| {
            if ui.button("+ Nouvelle campagne").clicked() {
                self.screen=Screen::Create;
                self.creation=CreationDraft::default();
            }
            ui.checkbox(&mut self.show_archived,"Afficher les archives");
            if ui.button("Actualiser").clicked() {
                let result=self.refresh_campaigns(); self.message(result,"Liste actualisée.");
            }
        });
        ui.separator();
        let visible:Vec<_>=self.campaigns.iter().filter(|c|self.show_archived||!c.archived).cloned().collect();
        if visible.is_empty() { ui.add_space(20.0); ui.label("Aucune campagne. Créez votre premier univers !"); }
        egui::ScrollArea::vertical().show(ui,|ui| {
            for c in visible {
                egui::Frame::group(ui.style()).show(ui,|ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.strong(&c.title);
                            ui.small(format!("{} joueur(s) · {} · version {}",
                                c.player_count,
                                if c.archived {"Archivée"} else {"Active"},c.state_version));
                            if !c.seed_context.trim().is_empty() {
                                ui.label(egui::RichText::new(c.seed_context.chars().take(140).collect::<String>()).weak().small());
                            }
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui| {
                            if ui.button("Ouvrir").clicked() { self.open_campaign(&c.id); }
                            if ui.button(if c.archived {"Réactiver"} else {"Archiver"}).clicked() {
                                self.archive(&c.id,!c.archived);
                            }
                            if ui.button("Supprimer…").clicked() {
                                self.delete_confirmation=Some(c.id.clone());
                            }
                        });
                    });
                    if self.delete_confirmation.as_deref()==Some(c.id.as_str()) {
                        ui.separator();
                        ui.colored_label(egui::Color32::YELLOW,
                            "Suppression irréversible de cette campagne et de ses participants. Confirmer ?");
                        ui.horizontal(|ui| {
                            if ui.button("Confirmer la suppression").clicked() { self.delete_campaign(&c.id); }
                            if ui.button("Annuler").clicked() { self.delete_confirmation=None; }
                        });
                    }
                });
                ui.add_space(6.0);
            }
        });
    }

    fn create(&mut self,ui:&mut egui::Ui) {
        ui.heading("Créer une campagne");
        ui.label("Choisis un scénario préchargé ou écris directement ton propre contexte. Aucun mode à choisir.");
        ui.separator();

        // Toujours réserver un bas d'écran visible pour les actions, même si le texte est long.
        let scroll_height=(ui.available_height()-66.0).max(100.0);
        egui::ScrollArea::vertical()
            .id_salt("create_campaign_scroll")
            .max_height(scroll_height)
            .auto_shrink([false, false])
            .show(ui,|ui| {
                ui.heading("Scénarios préchargés (20)");
                ui.weak("Le scénario choisi remplit le nom, le nombre de joueurs et le contexte. Tu peux tout modifier avant la création.");
                ui.horizontal(|ui| {
                    ui.label("Chercher :");
                    ui.add(egui::TextEdit::singleline(&mut self.creation.preset_search)
                        .hint_text("Fantastique, enquête, spatial…").desired_width(230.0));
                    egui::ComboBox::from_id_salt("preset_players_filter")
                        .selected_text(if self.creation.preset_players==0 {
                            "Tous les groupes".to_owned()
                        } else {
                            format!("{} joueur(s)",self.creation.preset_players)
                        })
                        .show_ui(ui,|ui| {
                            ui.selectable_value(&mut self.creation.preset_players,0,"Tous les groupes");
                            for n in 1..=6 {
                                ui.selectable_value(&mut self.creation.preset_players,n,format!("{} joueur(s)",n));
                            }
                        });
                });
                let query=self.creation.preset_search.trim().to_lowercase();
                let filter=self.creation.preset_players;
                let mut selected=None;
                egui::ScrollArea::vertical()
                    .id_salt("presets_scroll")
                    .max_height(238.0)
                    .show(ui,|ui| {
                        let mut shown=0;
                        for (index,scenario) in SCENARIOS.iter().enumerate() {
                            if filter!=0 && scenario.recommended_players!=filter { continue; }
                            if !query.is_empty() && !format!("{} {} {}",scenario.title,scenario.genre,scenario.summary)
                                .to_lowercase().contains(&query) { continue; }
                            shown+=1;
                            ui.push_id(index,|ui| {
                                ui.group(|ui| {
                                    ui.horizontal_wrapped(|ui| {
                                        ui.strong(scenario.title);
                                        ui.weak(format!("{} · {} joueur(s)",scenario.genre,scenario.recommended_players));
                                    });
                                    ui.label(scenario.summary);
                                    if ui.small_button("Utiliser ce scénario").clicked() { selected=Some(index); }
                                });
                            });
                        }
                        if shown==0 { ui.weak("Aucun scénario avec ces filtres."); }
                    });
                if let Some(index)=selected {
                    let scenario=&SCENARIOS[index];
                    self.creation.title=scenario.title.to_owned();
                    self.creation.context=scenario.context.to_owned();
                    self.creation.players=scenario.recommended_players;
                }

                ui.separator();
                ui.heading("Ta campagne");
                ui.weak("Un contexte court suffit : le MJ développera les détails manquants. Un contexte long sera respecté comme canon initial.");
                ui.label("Nom de la campagne");
                ui.add_sized([ui.available_width(),24.0],egui::TextEdit::singleline(&mut self.creation.title));
                ui.horizontal(|ui| {
                    ui.label("Nombre de joueurs (1 à 6)");
                    ui.add(egui::DragValue::new(&mut self.creation.players).range(1..=6).speed(1));
                });
                ui.label("Contexte initial (modifiable librement)");
                ui.add_sized([ui.available_width(),228.0],
                    egui::TextEdit::multiline(&mut self.creation.context)
                        .desired_rows(10)
                        .hint_text("Ex. : Vigo en 1800. Deux voyageurs arrivent séparément au port. Le MJ invente le reste…"));
                ui.weak(format!("{} / 40 000 caractères",self.creation.context.chars().count()));
                ui.add_space(6.0);
            });

        // Ces deux boutons sont HORS de la zone déroulante, donc toujours accessibles.
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("Créer et préparer les personnages").clicked() { self.create_campaign(); }
            if ui.button("Annuler").clicked() { self.screen=Screen::Home; }
        });
    }

    fn campaign(&mut self,ui:&mut egui::Ui) {
        let Some(c)=self.selected.as_ref().and_then(|id| self.campaigns.iter().find(|c|&c.id==id)).cloned() else {
            ui.label("Aucune campagne sélectionnée.");
            if ui.button("Revenir aux campagnes").clicked() { self.screen=Screen::Home; }
            return;
        };
        ui.heading(&c.title);
        ui.horizontal(|ui| {
            ui.label(format!("{} joueur(s) · état v{}",c.player_count,c.state_version));
            if c.archived { ui.colored_label(egui::Color32::YELLOW,"Archivée"); }
        });
        ui.weak("Le MJ propose trois personnages ; tu choisis et personnalises avant le verrouillage par Rust.");
        let started=match self.storage.setup_complete(&c.id) {
            Ok(value)=>value,
            Err(e)=>{ui.colored_label(egui::Color32::LIGHT_RED,format!("{e:#}"));return;},
        };
        if started {
            if !c.archived && ui.button("▶ Reprendre la partie (chat RP)").clicked(){self.screen=Screen::Play;}
            ui.label("Campagne commencée : fiches, équipements et contexte initial verrouillés. Les évolutions se font par actions validées.");
        } else {
            ui.strong("Étape obligatoire : préparer les joueurs, puis commencer la partie.");
            ui.weak("Personnalise chaque fiche, les objets, les ressources et les contacts initiaux. La validation verrouille définitivement l'édition manuelle.");
        }
        ui.separator();
        egui::ScrollArea::vertical().show(ui,|ui| {
            if !started {
                if let Some(draft)=self.edition.as_mut() {
                    ui.heading("Informations initiales");
                    ui.label("Nom de la campagne");
                    ui.text_edit_singleline(&mut draft.title);
                    ui.weak("Tu peux modifier le contexte avant de commencer la partie. Le MJ complète ce qui n'est pas défini sans contredire les faits fixés.");
                    ui.label("Canon initial / instructions de création");
                    ui.add_sized([ui.available_width(),160.0],egui::TextEdit::multiline(&mut draft.context).desired_rows(6));
                    ui.heading("Participants");
                    for (index,name) in draft.participants.iter_mut().enumerate() {
                        ui.horizontal(|ui| {ui.label(format!("Joueur {}",index+1));ui.text_edit_singleline(name);});
                    }
                }
                if ui.button("Enregistrer les informations initiales").clicked() {self.save_edition();}
                ui.separator();
                ui.heading("Création des fiches et des objets");
                ui.weak("Tu peux choisir un personnage genere par le MJ. Le choix remplace la dotation initiale du PJ selectionne, pas celle des autres.");
                ui.heading("Devises connues (facultatif)");
                if let Ok(currencies)=self.storage.currency_list(&c.id) {
                    if currencies.is_empty(){ui.weak("Aucune devise imposee : l'univers peut fonctionner par troc ou avec plusieurs monnaies.");}
                    for name in currencies {ui.label(name);}
                }
                ui.horizontal(|ui|{
                    ui.label("Ajouter une devise (facultatif)");ui.text_edit_singleline(&mut self.play.currency_name);
                    if ui.button("Enregistrer pour toute la campagne").clicked(){
                        let result=self.storage.add_currency(&c.id,&self.play.currency_name);
                        if result.is_ok(){self.play.currency_name.clear();}
                        self.message(result,"Devise reconnue dans la campagne.");
                    }
                });
                match self.storage.characters(&c.id) {
                    Ok(chars) if !chars.is_empty()=>{
                        if !chars.iter().any(|ch|Some(ch.id.as_str())==self.play.selected.as_deref()) {
                            self.choose_character(&chars[0]);
                        }
                        let current=self.play.selected.clone().unwrap_or_default();
                        let label=chars.iter().find(|ch|ch.id==current).map(|ch|ch.name.as_str()).unwrap_or("Joueur");
                        egui::ComboBox::from_id_salt("setup_character_picker").selected_text(label).show_ui(ui,|ui|{
                            for ch in &chars {if ui.selectable_label(ch.id==current,&ch.name).clicked(){self.choose_character(ch);}}
                        });
                        if let Some(ch)=chars.iter().find(|ch|Some(ch.id.as_str())==self.play.selected.as_deref()) {
                            if !self.setup_ai_panel(ui,&c.id,ch) {
                                self.play_character(ui,&c.id,ch,true);
                            }
                        }
                    },
                    Ok(_)=>{ui.label("Aucun personnage créé.");},
                    Err(e)=>{ui.colored_label(egui::Color32::LIGHT_RED,format!("{e:#}"));},
                }
                ui.separator();
                ui.heading("Validation de la création");
                if !self.start_confirmation {
                    if ui.button("J'ai terminé les fiches et objets : commencer la partie").clicked(){self.start_confirmation=true;}
                } else {
                    ui.colored_label(egui::Color32::YELLOW,"Confirmation : les fiches, objets, ressources et participants ne seront plus modifiables directement pendant le jeu.");
                    ui.horizontal(|ui| {
                        if ui.button("Confirmer et verrouiller").clicked(){
                            let result=(|| -> Result<()> {
                                // Enregistrer le formulaire initial et la fiche active avant de verrouiller.
                                if let Some(draft)=self.edition.as_ref(){
                                    let version=self.storage.get_campaign(&c.id)?
                                        .ok_or_else(||anyhow::anyhow!("Campagne introuvable"))?.state_version;
                                    self.storage.save_setup(&c.id,version,&draft.title,draft.mode,&draft.context,&draft.participants)?;
                                }
                                if let Some(ch)=self.play.character_draft.as_ref(){
                                    self.storage.save_character(&c.id,ch)?;
                                }
                                self.storage.finalize_setup(&c.id)?;
                                self.refresh_campaigns()?;
                                self.load_edition(&c.id)?;
                                Ok(())
                            })();
                            if result.is_ok(){self.start_confirmation=false;self.screen=Screen::Play;self.play.tab=PlayTab::Story;}
                            self.message(result,"Campagne commencée. Les fiches initiales sont verrouillées.");
                        }
                        if ui.button("Continuer la préparation").clicked(){self.start_confirmation=false;}
                    });
                }
            } else {
                ui.heading("Canon initial (lecture seule)");
                ui.label(&c.seed_context);
                ui.separator();
                ui.weak("Les noms, fiches et biens initiaux sont verrouillés. Pour consulter les personnages : Partie → Fiche et objets.");
            }
        });
    }

    fn start_setup_ai(&mut self,campaign:&str,ch:&Character) {
        if self.setup_pending.is_some(){return;}
        let result=(||->Result<SetupPending>{
            if self.storage.setup_complete(campaign)?{anyhow::bail!("Preparation deja verrouillee");}
            let c=self.storage.get_campaign(campaign)?.ok_or_else(||anyhow::anyhow!("Campagne absente"))?;
            let config=AiConnection{endpoint:self.settings.llm_endpoint.clone(),model:self.settings.llm_model.clone(),
                api_key:self.api_key_session.clone(),temperature:self.settings.llm_temperature,
                max_tokens:self.settings.llm_max_tokens.max(3000),timeout_seconds:self.settings.llm_timeout_seconds};
            config.validate()?;
            let (tx,rx)=mpsc::channel();
                        let context=c.seed_context.clone();let label=ch.name.clone();
            thread::spawn(move||{
                let outcome=llm::suggest_characters(&config,&context,&label).map_err(|e|format!("{e:#}"));
                let _=tx.send(outcome);
            });
            Ok(SetupPending{receiver:rx,campaign:campaign.into(),character:ch.id.clone(),state_version:c.state_version})
        })();
        match result {Ok(job)=>{self.setup_pending=Some(job);self.setup_choices=None;
            self.toast=Some((false,"Le MJ prepare trois personnages possibles.".into()));},
            Err(e)=>self.toast=Some((true,format!("{e:#}")))}
    }
    fn poll_setup_ai(&mut self) {
        let polled=self.setup_pending.as_ref().map(|p|p.receiver.try_recv());
        match polled {
            Some(Ok(result))=>{
                let job=self.setup_pending.take().unwrap();
                match result {
                    Ok(options)=>{
                        let fresh=self.storage.get_campaign(&job.campaign).ok().flatten()
                            .is_some_and(|c|c.state_version==job.state_version)
                            && self.storage.setup_complete(&job.campaign).ok()==Some(false);
                        if fresh {self.setup_choices=Some((job.character,options));
                            self.toast=Some((false,"Trois propositions disponibles ; selectionne celle qui te plait.".into()));}
                        else {self.toast=Some((true,"La preparation a change pendant la generation ; relance les propositions.".into()));}
                    },
                    Err(error)=>self.toast=Some((true,format!("Generation MJ : {error}"))),
                }
            },
            Some(Err(TryRecvError::Disconnected))=>{
                self.setup_pending=None;
                self.toast=Some((true,"Generation de personnage interrompue.".into()));
            },
            _=>{},
        }
    }
    fn setup_ai_panel(&mut self,ui:&mut egui::Ui,campaign:&str,ch:&Character)->bool {
        let mut chosen_now=false;
        ui.heading("Personnages proposes par le MJ IA");
        let waiting=self.setup_pending.is_some();
        if ui.add_enabled(!waiting,egui::Button::new("Proposer 3 personnages pour ce joueur")).clicked(){
            self.start_setup_ai(campaign,ch);
        }
        if waiting {ui.spinner();ui.weak("Generation en cours, sans modification de la partie.");}
        if let Some((character,options))=self.setup_choices.clone() {
            if character==ch.id {
                for (index,chosen) in options.characters.iter().enumerate() {
                    egui::CollapsingHeader::new(format!("{} · {} · {} ans",chosen.name,chosen.role,chosen.age_years))
                        .id_salt(format!("proposal_{index}")).show(ui,|ui|{
                        ui.label(format!("{} / {} · {} cm · {} kg",chosen.sex,chosen.gender,chosen.height_cm,chosen.weight_kg));
                        ui.label(format!("{} · {}",chosen.origin,chosen.appearance));
                        ui.label(format!("Personnalite : {}",chosen.personality));
                        ui.label(format!("Langues : {} · Voix : {}",chosen.languages,chosen.voice));
                        ui.label(format!("Signes distinctifs : {}",chosen.distinctive_features));
                        ui.label(format!("Histoire : {}",chosen.background));
                        ui.label(format!("Debut : {}",chosen.initial_location));
                        ui.label(format!("FOR {} · DEX {} · CON {} · INT {} · SAG {} · CHA {}",
                            chosen.stats.strength,chosen.stats.dexterity,chosen.stats.constitution,
                            chosen.stats.intelligence,chosen.stats.wisdom,chosen.stats.charisma));
                        ui.label(format!("Competences : {}",chosen.skills.join(", ")));
                        for item in &chosen.items {ui.label(format!("{} x {}",item.quantity,item.name));}
                        if chosen.starting_funds.is_empty(){ui.weak("Aucune monnaie de depart (troc ou sans argent).");}
                        else {for fund in &chosen.starting_funds {
                            ui.label(format!("Dotation : {} {}",fund.amount,fund.currency));
                        }}
                        if ui.button(format!("Choisir ce personnage (proposition {})",index+1)).clicked() {
                            let result=self.storage.apply_character_suggestion(campaign,&ch.id,chosen);
                            if result.is_ok(){self.setup_choices=None;self.play.character_draft=None;
                                self.play.profile_draft=None;chosen_now=true;}
                            self.message(result,"Personnage et dotation valides atomiquement par Rust.");
                        }
                    });
                }
            }
        }
        ui.separator();
        chosen_now
    }
    fn settings(&mut self,ui:&mut egui::Ui) {
        ui.heading("Paramètres persistants");
        ui.label("Réglages enregistrés dans SQLite. Aucun .env, TOML ou JSON externe.");
        ui.separator();
        ui.heading("Fournisseur IA OpenAI-compatible");
        ui.label("URL OpenAI-compatible (llama.cpp par défaut)");
        ui.text_edit_singleline(&mut self.settings_draft.llm_endpoint);
        ui.label("Identifiant de modèle (facultatif pour llama.cpp)");
        ui.text_edit_singleline(&mut self.settings_draft.llm_model);
        ui.weak("llama.cpp local : http://127.0.0.1:8080/v1 — le serveur doit déjà fonctionner.");
  ui.horizontal(|ui|{
    ui.label("Température");ui.add(egui::Slider::new(&mut self.settings_draft.llm_temperature,0.0..=2.0));
    ui.label("Max tokens");ui.add(egui::DragValue::new(&mut self.settings_draft.llm_max_tokens).range(128..=8192));
    ui.label("Délai max (s)");ui.add(egui::DragValue::new(&mut self.settings_draft.llm_timeout_seconds).range(10..=300));
  });
  ui.separator();ui.heading("Clé API (facultative pour llama.cpp)");
  ui.label("Clé API à enregistrer (jamais affichée en clair après sauvegarde)");
  ui.add(egui::TextEdit::singleline(&mut self.api_key_input).password(true));
  ui.label("Phrase secrète maître (au moins 12 caractères, non stockée)");
  ui.add(egui::TextEdit::singleline(&mut self.api_master).password(true));
  ui.horizontal(|ui|{
   if ui.button("Enregistrer clé chiffrée").clicked(){
    let result=self.storage.save_api_key(&self.api_key_input,&self.api_master);
    if result.is_ok(){self.api_key_session=Some(self.api_key_input.clone());self.api_key_input.clear();self.api_master.clear();}
    self.message(result,"Clé chiffrée enregistrée dans SQLite.");
   }
   if ui.button("Déverrouiller clé existante").clicked(){
    match self.storage.unlock_api_key(&self.api_master){
     Ok(key)=>{self.api_key_session=key;self.api_master.clear();self.toast=Some((false,"Clé déverrouillée pour cette session.".into()));},
     Err(e)=>self.toast=Some((true,format!("{e:#}"))),
    }
   }
   if ui.button("Verrouiller").clicked(){self.api_key_session=None;}
   if ui.button("Effacer clé").clicked(){
    let result=self.storage.clear_api_key();if result.is_ok(){self.api_key_session=None;}
    self.message(result,"Clé API effacée.");
   }
  });
  ui.weak(if self.api_key_session.is_some(){"Clé active pour cette session"}else{"Aucune clé déverrouillée pour la session"});
        ui.add_space(12.0);
        ui.heading("Budget de contexte IA");
        ui.weak("L2 sélectionne le contexte utile et conserve l’historique dans SQLite. Le compactage sémantique automatique arrivera plus tard.");
        ui.horizontal(|ui| {
            ui.label("Contexte cible (tokens)");
            ui.add(egui::DragValue::new(&mut self.settings_draft.context_target_tokens)
                .range(512..=262_144).speed(128));
        });
        ui.weak("Seuil de compactage réservé à un lot ultérieur ; L2 utilise uniquement un contexte borné et des faits approuvés.");
        ui.horizontal(|ui| {
            ui.label("Seuil de compactage (%)");
            ui.add(egui::Slider::new(&mut self.settings_draft.compaction_threshold_pct,50..=95));
        });
        ui.add_space(10.0);
        if ui.button("Enregistrer les paramètres").clicked() {
            self.settings_draft.last_campaign_id=self.settings.last_campaign_id.clone();
            let result=self.storage.save_settings(&self.settings_draft);
            if result.is_ok() { self.settings=self.settings_draft.clone(); }
            self.message(result,"Paramètres enregistrés.");
        }
        ui.separator();
        ui.strong("Base de données locale");
        ui.monospace(self.storage.path_display());
        ui.weak("Le fichier SQLite est créé automatiquement. Aucune migration de L1 vers L2.");
    }
}


impl GameApp {
 fn start_ai_action(&mut self,campaign:&str,active:&Character) {
  if self.ai_pending.is_some(){return;}
  let result=(|| -> Result<AiPending>{
   if self.storage.pending_ai_roll(campaign)?.is_some() {anyhow::bail!("Résous ou abandonne le jet en attente avant une nouvelle action");}
   let action=self.play.action_text.trim().to_owned();
   let current=self.storage.get_campaign(campaign)?.ok_or_else(||anyhow::anyhow!("Campagne introuvable"))?;
   let snapshot=self.storage.ai_snapshot(campaign,&active.id,&action,self.settings.context_target_tokens)?;
   let config=AiConnection{endpoint:self.settings.llm_endpoint.clone(),model:self.settings.llm_model.clone(),
    api_key:self.api_key_session.clone(),temperature:self.settings.llm_temperature,
    max_tokens:self.settings.llm_max_tokens,timeout_seconds:self.settings.llm_timeout_seconds};
   config.validate()?;
   let (tx,rx)=mpsc::channel();
   let job=AiPending{receiver:rx,key:Uuid::new_v4().to_string(),campaign:campaign.into(),actor:active.id.clone(),
    action,state_version:current.state_version,model:config.model.clone()};
   thread::spawn(move ||{
    let outcome=llm::arbitrate(&config,&snapshot).map_err(|e|format!("{e:#}"));
    let _=tx.send(outcome);
   });
   Ok(job)
  })();
  match result {Ok(job)=>{self.ai_pending=Some(job);self.toast=Some((false,"Demande envoyée au MJ IA.".into()));},
   Err(e)=>self.toast=Some((true,format!("{e:#}")))}
 }
 fn poll_ai(&mut self) {
  let polled=self.ai_pending.as_ref().map(|p|p.receiver.try_recv());
  match polled {
   Some(Ok(result))=>{
    let pending=self.ai_pending.take().unwrap();
    match result {
     Ok(proposal)=>self.apply_ai_action(pending,proposal),
     Err(message)=>self.toast=Some((true,format!("MJ IA : {message}. Aucune action enregistrée."))),
    }
   },
   Some(Err(TryRecvError::Disconnected))=>{
    self.ai_pending=None;
    self.toast=Some((true,"Connexion au MJ interrompue, aucun dé lancé.".into()));
   },
   _=>{},
  }
 }
 fn apply_ai_action(&mut self,job:AiPending,proposal:AiProposal){
  let result=(|| -> Result<bool>{
   let current=self.storage.get_campaign(&job.campaign)?.ok_or_else(||anyhow::anyhow!("Campagne supprimée"))?;
   if current.state_version!=job.state_version || current.archived {
    anyhow::bail!("La campagne a changé pendant la réponse du MJ : l'action doit être renvoyée");
   }
   let request=ActionRequest{key:job.key.clone(),campaign_id:job.campaign.clone(),actor_id:job.actor.clone(),
    text:job.action.clone(),decision:proposal.decision,ability:proposal.ability,proficient:proposal.proficient,
    dc:proposal.dc,roll_mode:proposal.roll_mode,target_id:None,hp_delta:0,
    success_text:proposal.success_text.clone(),failure_text:proposal.failure_text.clone()};
   let roll_required=proposal.decision==Decision::AbilityCheck;
   if roll_required {
    // AUCUN d20 lancé ici. Le jet ne peut être demandé que depuis l'action suspendue.
    self.storage.queue_ai_roll(&request,&proposal,&job.model)?;
   } else {
    self.storage.resolve_ai_action(&request,&proposal,&job.model)?;
   }
   if self.play.action_text.trim()==job.action {self.play.action_text.clear();}
   self.play.character_draft=None;
   Ok(roll_required)
  })();
  match result {
   Ok(true)=>self.toast=Some((false,"Le MJ attend un jet : enjeux figés et sauvegardés. Lance le d20 dans la partie.".into())),
   Ok(false)=>self.toast=Some((false,"Réponse du MJ enregistrée dans le récit de la campagne.".into())),
   Err(e)=>self.toast=Some((true,format!("{e:#}"))),
  }
 }
 fn play_memory(&mut self,ui:&mut egui::Ui,campaign:&str) {
  ui.heading("Faits narratifs proposés par le MJ");
  ui.weak("L2 : seules les propositions factuelles explicites du MJ sont examinées ici. Elles sont facultatives : un dialogue ne crée pas automatiquement un fait canonique. La mémoire avancée des croyances de PNJ appartient à L3.");
  match self.storage.pending_facts(campaign){
   Ok(facts) if facts.is_empty()=>{ui.label("Aucune proposition en attente : le MJ n'a pas proposé de nouveau fait structurel à approuver.");},
   Ok(facts)=>for fact in facts {
    ui.group(|ui|{
     ui.label(&fact.fact);
     ui.horizontal(|ui|{
      if ui.button("Approuver").clicked(){
       let result=self.storage.review_fact(campaign,&fact.id,true);self.message(result,"Fait devenu canonique.");
      }
      if ui.button("Rejeter").clicked(){
       let result=self.storage.review_fact(campaign,&fact.id,false);self.message(result,"Fait rejeté.");
      }
     });
    });
   },
   Err(e)=>{ui.colored_label(egui::Color32::LIGHT_RED,format!("{e:#}"));},
  }
  ui.separator();ui.heading("Faits canoniques approuvés");
  if let Ok(facts)=self.storage.approved_facts(campaign,50){
   if facts.is_empty(){ui.weak("Aucun fait approuvé. Le contexte de création et les événements enregistrés restent disponibles dans SQLite.");}
   for fact in facts {ui.label(format!("• {}",fact.fact));}
  }
 }
}

impl eframe::App for GameApp {
    fn update(&mut self,ctx:&egui::Context,_frame:&mut eframe::Frame) {
        self.poll_setup_ai();
        self.poll_ai();
        if self.ai_pending.is_some()||self.setup_pending.is_some(){ctx.request_repaint_after(std::time::Duration::from_millis(120));}
        self.navigation(ctx);
        egui::CentralPanel::default().show(ctx,|ui| {
            ui.add_space(6.0);
            if let Some((error,message))=self.toast.clone() {
                let color=if error {egui::Color32::LIGHT_RED} else {egui::Color32::LIGHT_GREEN};
                ui.horizontal(|ui| {
                    ui.colored_label(color,&message);
                    if ui.small_button("✕").clicked() { self.toast=None; }
                });
                ui.add_space(6.0);
            }
            match self.screen {
                Screen::Home=>self.home(ui),
                Screen::Create=>self.create(ui),
                Screen::Campaign=>self.campaign(ui),
                Screen::Play=>self.play_screen(ui),
                Screen::Settings=>self.settings(ui),
            }
        });
    }
}

impl GameApp {
 fn choose_character(&mut self,ch:&Character) {
  self.play.selected=Some(ch.id.clone());
  self.play.character_draft=Some(ch.clone());
  self.play.profile_draft=None;
 }
 fn play_screen(&mut self,ui:&mut egui::Ui) {
  let Some(campaign)=self.selected.clone() else {ui.label("Aucune campagne ouverte");return;};
  match self.storage.setup_complete(&campaign) {
   Ok(true)=>{},
   Ok(false)=>{ui.label("Prépare et valide les personnages avant de jouer.");if ui.button("Aller à la préparation").clicked(){self.screen=Screen::Campaign;}return;},
   Err(e)=>{ui.colored_label(egui::Color32::LIGHT_RED,format!("{e:#}"));return;}
  }
  let chars=match self.storage.characters(&campaign){Ok(v)=>v,Err(e)=>{ui.colored_label(egui::Color32::LIGHT_RED,format!("{e:#}"));return;}};
  if chars.is_empty(){ui.label("Aucun personnage");return;}
  if !chars.iter().any(|c|Some(c.id.as_str())==self.play.selected.as_deref()) {self.choose_character(&chars[0]);}
  ui.heading("MJ Universel · Partie en cours");
  ui.horizontal_wrapped(|ui| {
   ui.label("Tu incarnes :");
   let selected=self.play.selected.clone().unwrap_or_default();
   let label=chars.iter().find(|c|c.id==selected).map(|c|c.name.as_str()).unwrap_or("Personnage");
   egui::ComboBox::from_id_salt("active_character").selected_text(label).show_ui(ui,|ui|{
    for ch in &chars {if ui.selectable_label(ch.id==selected,&ch.name).clicked(){self.choose_character(ch);}}
   });
  });
  ui.horizontal_wrapped(|ui|{
   for (tab,title) in [(PlayTab::Story,"Partie · chat RP"),(PlayTab::Character,"Fiche et objets"),
    (PlayTab::Journal,"Journal technique"),(PlayTab::Memory,"Faits proposés")] {
    if ui.selectable_label(self.play.tab==tab,title).clicked(){self.play.tab=tab;}
   }
  });
  ui.separator();
  let active=chars.iter().find(|c|Some(c.id.as_str())==self.play.selected.as_deref()).unwrap();
  if self.play.tab==PlayTab::Story {
   self.play_story(ui,&campaign,&chars,active);
  } else {
   egui::ScrollArea::vertical().show(ui,|ui|{
    match self.play.tab {
     PlayTab::Character=>self.play_character(ui,&campaign,active,false),
     PlayTab::Journal=>self.play_journal(ui,&campaign),
     PlayTab::Memory=>self.play_memory(ui,&campaign),
     PlayTab::Story=>{},
    }
   });
  }
 }
 /// Conversation unique : actions de tous les PJ, narration du MJ et jets, en ordre chronologique.
 fn play_story(&mut self,ui:&mut egui::Ui,campaign:&str,chars:&[Character],active:&Character) {
  let pending=match self.storage.pending_ai_roll(campaign) {
   Ok(p)=>p,Err(e)=>{ui.colored_label(egui::Color32::LIGHT_RED,format!("Erreur de lecture du jet : {e:#}"));return;}
  };
  let history=match self.storage.roleplay_history(campaign,self.play.history_limit+1) {
   Ok(e)=>e,Err(e)=>{ui.colored_label(egui::Color32::LIGHT_RED,format!("Journal indisponible : {e:#}"));return;}
  };
  let more_history=history.len()>self.play.history_limit;
  let height=(ui.available_height()-245.0).max(180.0);
  egui::ScrollArea::vertical().id_salt("rp_story_scroll").max_height(height)
   .stick_to_bottom(true).auto_shrink([false,false]).show(ui,|ui| {
    let mut seen=0;
    if more_history && ui.small_button("Afficher les messages précédents (+120)").clicked() {
     self.play.history_limit=self.play.history_limit.saturating_add(120).min(100_000);
    }
    for e in history.iter().take(self.play.history_limit).rev() {
     if !matches!(e.kind.as_str(),"action_resolved"|"action_pending_roll"|"action_cancelled") {continue;}
     seen+=1;
     let actor_id=e.request.get("actor_id").and_then(|v|v.as_str()).unwrap_or("");
     let player=chars.iter().find(|c|c.id==actor_id).map(|c|c.name.as_str()).unwrap_or("Personnage");
     if e.kind=="action_pending_roll" || (e.kind=="action_resolved" && !history.iter().any(|earlier|earlier.kind=="action_pending_roll" &&
         earlier.request.get("key")==e.request.get("key"))) {
      if let Some(t)=e.request.get("text").and_then(|v|v.as_str()) {
       egui::Frame::group(ui.style()).show(ui,|ui|{
        ui.label(egui::RichText::new(format!("{player} · action")).strong());
        ui.label(t);
       });
      }
     }
     match e.kind.as_str() {
      "action_resolved"=>{
       egui::Frame::group(ui.style()).show(ui,|ui|{
        // Montrer d'abord le d20 : l'ordre visuel doit refléter la causalité du moteur.
        if let Some(r)=e.payload.get("roll").filter(|v|!v.is_null()) {
         let total=r.get("total").and_then(|v|v.as_i64()).unwrap_or(0);
         let dc=e.payload.get("effective_dc").and_then(|v|v.as_u64()).unwrap_or(0);
         let raw=r.get("rolls").and_then(|v|v.as_array()).map(|xs|xs.iter().filter_map(|v|v.as_u64()).map(|n|n.to_string()).collect::<Vec<_>>().join(", ")).unwrap_or_default();
         ui.label(egui::RichText::new(format!("🎲 Résultat du d20 [{}] · total {} · DD {} · {}",raw,total,dc,
          if e.payload.get("success").and_then(|v|v.as_bool()).unwrap_or(false){"réussi"}else{"échoué"})).strong());
        }
        ui.label(egui::RichText::new("MJ · récit").strong());
        if let Some(t)=e.payload.get("narration").and_then(|v|v.as_str()){ui.label(t);}
        if let Some(t)=e.payload.get("applied_effect").and_then(|v|v.as_str()) {
         ui.label(egui::RichText::new(format!("✓ Rust · {t}")).italics());
        }
       });
      },
      "action_pending_roll"=>{
       // Un ancien jet résolu ne doit plus apparaître « en attente » dans le récit.
       let key=e.request.get("key").and_then(|v|v.as_str()).unwrap_or("");
       let status=if pending.as_ref().is_some_and(|p|p.request.key==key) {
        "en attente du joueur"
       } else if history.iter().any(|other|other.kind=="action_cancelled" &&
           other.request.get("key").and_then(|v|v.as_str())==Some(key)) {
        "abandonné avant le jet"
       } else {"résolu"};
       ui.weak(format!("🎲 MJ : test de {} (DD {}) proposé · {}",
        e.request.get("ability").and_then(|v|v.as_str()).unwrap_or("caractéristique"),
        e.payload.get("dc").and_then(|v|v.as_u64()).unwrap_or(0),status));
       // La proposition publique des enjeux ne révèle jamais le texte des issues cachées.
       for (field,title) in [("public_success","Réussite"),("public_failure","Échec")] {
        if let Some(stake)=e.payload.get(field).and_then(|v|v.as_str()).filter(|s|!s.is_empty()) {
         ui.small(format!("{title} : {stake}"));
        }
       }
      },
      "action_cancelled"=>{ui.weak("Action abandonnée par le joueur avant le jet.");},
      _=>{},
     }
     ui.add_space(5.0);
    }
    if seen==0 {ui.label("L'histoire commence ici. Décris librement ce que fait ton personnage.");}
   });
  ui.separator();
  if let Some(pending)=pending {
   ui.group(|ui|{
    ui.strong(format!("🎲 Jet autorisé par le MJ — {} · DD {}",pending.request.ability.label(),pending.request.dc));
    ui.label(&pending.proposal.reason);
    ui.small(format!("Enjeu en cas de réussite : {}",pending.proposal.stakes_public_success.as_deref().filter(|s|!s.is_empty()).unwrap_or("la tentative atteint son objectif, selon les limites du monde")));
    ui.small(format!("Enjeu en cas d'échec : {}",pending.proposal.stakes_public_failure.as_deref().filter(|s|!s.is_empty()).unwrap_or("l'obstacle demeure ou une complication causale survient")));
    ui.label("Le résultat de ce jet résoudra cette action une seule fois.");
    ui.horizontal(|ui|{
     if ui.button("🎲 Lancer le d20 et poursuivre").clicked() {
      match self.storage.finish_pending_roll(campaign,&pending.request.key) {
       Ok(out)=>{self.play.abandon_confirm=false;
        self.toast=Some((false,format!("Jet enregistré : {} · le MJ a repris son récit.",
          out.roll.as_ref().map(|r|r.total.to_string()).unwrap_or_default())));},
       Err(e)=>self.toast=Some((true,format!("{e:#}"))),
      }
     }
     if ui.button("Renoncer à cette tentative").clicked(){self.play.abandon_confirm=true;}
    });
    if self.play.abandon_confirm {
     ui.horizontal(|ui|{
      ui.weak("Abandonner sans lancer ? Cette décision sera conservée dans le journal.");
      if ui.button("Confirmer l'abandon").clicked(){
       let result=self.storage.cancel_pending_roll(campaign,&pending.request.key);
       if result.is_ok(){self.play.abandon_confirm=false;}
       self.message(result,"Action abandonnée avant le lancer.");
      }
      if ui.button("Retour").clicked(){self.play.abandon_confirm=false;}
     });
    }
   });
   ui.weak("Pendant un jet en attente : consultation des fiches et de l'histoire possible, aucune nouvelle action engagée.");
  } else {
   let busy=self.ai_pending.is_some();
   ui.label(egui::RichText::new(format!("{} · Que fais-tu ?",active.name)).strong());
   ui.add_enabled(!busy,egui::TextEdit::multiline(&mut self.play.action_text)
    .desired_rows(3).hint_text("Exprime librement paroles, gestes, tentative, question..."));
   ui.horizontal(|ui|{
    if ui.add_enabled(!busy && !self.play.action_text.trim().is_empty(),egui::Button::new("Envoyer au MJ")).clicked(){
     self.start_ai_action(campaign,active);
    }
    if busy {ui.spinner();ui.weak("Le MJ prépare sa réponse…");}
   });
   ui.weak("Les jets ne sont disponibles que lorsqu'une action est explicitement en attente de résolution.");
  }
 }
 fn play_character(&mut self,ui:&mut egui::Ui,campaign:&str,active:&Character,editable:bool) {
  if !editable {
   ui.heading(format!("Fiche · {}",active.name));
   ui.weak("Lecture seule. Les modifications proviennent des actions, jets et effets vérifiés par Rust.");
   ui.label(format!("{} · Niveau {} · PV {}/{} · CA {}",active.class_name,active.level,active.hp,active.max_hp,active.ac));
   match self.storage.actor_location(campaign,&active.id) {
    Ok(Some(place))=>{ui.label(format!("Lieu enregistré : {place}"));},
    Ok(None)=>{ui.weak("Lieu non encore enregistré comme effet canonique.");},
    Err(e)=>{ui.weak(format!("Lieu indisponible : {e:#}"));},
   }
   ui.label(format!("FOR {} · DEX {} · CON {} · INT {} · SAG {} · CHA {}",
    active.stats.strength,active.stats.dexterity,active.stats.constitution,
    active.stats.intelligence,active.stats.wisdom,active.stats.charisma));
   if let Ok(Some(p))=self.storage.character_profile(&active.id){
    ui.label(format!("{} · {} · {} ans · {} cm · {} kg",p.sex,p.gender,p.age_years,p.height_cm,p.weight_kg));
    ui.label(format!("Apparence : {} · Origine : {}",p.appearance,p.origin));
    ui.label(format!("Personnalite : {}",p.personality));
    ui.label(format!("Langues : {} · Voix : {} · Particularites : {}",p.languages,p.voice,p.distinctive_features));
   }
   if let Ok(currencies)=self.storage.currency_list(campaign){
    if !currencies.is_empty(){ui.weak(format!("Devises reconnues : {}",currencies.join(", ")));}
   }
   ui.label(format!("Compétences : {}",if active.skills.is_empty(){"aucune".to_owned()}else{active.skills.join(", ")}));
   if !active.notes.is_empty(){ui.label(format!("Historique : {}",active.notes));}
   ui.separator();ui.heading("Inventaire");
   match self.storage.inventory(&active.id){
    Ok(items) if items.is_empty()=>{ui.weak("Aucun objet.");},
    Ok(items)=>for item in items {ui.label(format!("{} × {} {}",item.quantity,item.name,item.notes));},
    Err(e)=>{ui.colored_label(egui::Color32::LIGHT_RED,format!("{e:#}"));},
   }
   ui.separator();ui.heading("Ressources et biens");
   if let Ok(items)=self.storage.holdings(campaign,&active.id){
    if items.is_empty(){ui.weak("Aucun bien générique.");}
    for (name,qty) in items {ui.label(format!("{} × {}",qty,name));}
   }
   ui.separator();ui.heading("Contacts connus");
   if let Ok(npcs)=self.storage.known_npcs(campaign){
    for (id,name) in npcs {ui.collapsing(name,|ui|{
      if let Ok(items)=self.storage.npc_holdings(campaign,&id){
       for (asset,qty) in items {ui.label(format!("{} × {}",qty,asset));}
      }
    });}
   }
   ui.separator();ui.heading("Promesses");
   if let Ok(promises)=self.storage.promises(campaign,&active.id){
    for (recipient,description) in promises {ui.label(format!("À {} : {}",recipient,description));}
   }
   return;
  }

  if self.play.character_draft.as_ref().map(|c|c.id.as_str())!=Some(active.id.as_str()) {
   self.play.character_draft=Some(active.clone());
  }
  ui.heading("Fiche du personnage");
  if let Some(ch)=self.play.character_draft.as_mut() {
   ui.horizontal(|ui|{ui.label("Nom");ui.text_edit_singleline(&mut ch.name);
    ui.label("Classe");ui.text_edit_singleline(&mut ch.class_name);});
   ui.horizontal(|ui|{
    ui.label("Niveau");ui.add(egui::DragValue::new(&mut ch.level).range(1..=20));
    ui.label("PV");ui.add(egui::DragValue::new(&mut ch.hp).range(0..=9999));
    ui.label("PV max");ui.add(egui::DragValue::new(&mut ch.max_hp).range(1..=9999));
    ui.label("CA");ui.add(egui::DragValue::new(&mut ch.ac).range(1..=50));
   });
   if ch.hp>ch.max_hp {
    ui.colored_label(egui::Color32::LIGHT_RED,
      format!("PV actuels ({}) supérieurs aux PV maximum ({}). Modifie d'abord les PV maximum.",ch.hp,ch.max_hp));
    if ui.small_button(format!("Aligner les PV max sur {}",ch.hp)).clicked() {
      ch.max_hp=ch.hp;
    }
   }
   ui.separator();
   ui.strong("Caractéristiques et modificateurs");
   egui::Grid::new("stats_grid").num_columns(3).show(ui,|ui| {
    let score=|ui:&mut egui::Ui,label:&str,value:&mut i32| {
     ui.label(label);ui.add(egui::DragValue::new(value).range(1..=30));
     let m=(*value-10).div_euclid(2);
     ui.monospace(format!("{m:+}"));ui.end_row();
    };
    score(ui,"FOR",&mut ch.stats.strength);score(ui,"DEX",&mut ch.stats.dexterity);
    score(ui,"CON",&mut ch.stats.constitution);score(ui,"INT",&mut ch.stats.intelligence);
    score(ui,"SAG",&mut ch.stats.wisdom);score(ui,"CHA",&mut ch.stats.charisma);
   });
   ui.label(format!("Bonus de maîtrise au niveau {} : +{}",ch.level,crate::uare::proficiency_bonus(ch.level)));
   ui.collapsing("Compétences maîtrisées",|ui|{
    const SKILLS:[&str;18]=["Acrobaties","Arcanes","Athlétisme","Discrétion","Dressage","Escamotage","Histoire","Intimidation","Investigation","Médecine","Nature","Perception","Perspicacité","Persuasion","Religion","Représentation","Survie","Tromperie"];
    ui.horizontal_wrapped(|ui|{
     for skill in SKILLS {
      let mut active=ch.skills.iter().any(|v|v.as_str()==skill);
      if ui.checkbox(&mut active,skill).changed(){
       if active {ch.skills.push(skill.to_owned());}else{ch.skills.retain(|v|v!=skill);}
      }
     }
    });
   });
   ui.label("Notes");ui.add(egui::TextEdit::multiline(&mut ch.notes).desired_rows(3));
   if ui.button("Enregistrer la fiche dans SQLite").clicked() {
    let result=self.storage.save_character(campaign,ch);
    self.message(result,"Fiche du personnage enregistrée.");
   }
  }
  ui.separator();ui.heading("Identite, physique et position de depart");
  if self.play.profile_draft.is_none() {
    self.play.profile_draft=Some(self.storage.character_profile(&active.id).ok().flatten().unwrap_or(CharacterProfile{
        sex:"non precise".into(),gender:"non precise".into(),age_years:25,height_cm:170,weight_kg:70,
        appearance:"a preciser".into(),origin:"a preciser".into(),personality:"a preciser".into(),
        initial_location:"emplacement exact inconnu".into(),languages:String::new(),
        distinctive_features:String::new(),voice:String::new(),
    }));
  }
  if let Some(profile)=self.play.profile_draft.as_mut(){
    ui.horizontal(|ui|{ui.label("Sexe");ui.text_edit_singleline(&mut profile.sex);
        ui.label("Genre");ui.text_edit_singleline(&mut profile.gender);});
    ui.horizontal(|ui|{ui.label("Age");ui.add(egui::DragValue::new(&mut profile.age_years).range(1..=5000));
        ui.label("Taille cm");ui.add(egui::DragValue::new(&mut profile.height_cm).range(25..=400));
        ui.label("Poids kg");ui.add(egui::DragValue::new(&mut profile.weight_kg).range(2..=500));});
    ui.label("Apparence");ui.text_edit_singleline(&mut profile.appearance);
    ui.label("Origine");ui.text_edit_singleline(&mut profile.origin);
    ui.label("Personnalite");ui.text_edit_singleline(&mut profile.personality);
    ui.label("Langues parlees");ui.text_edit_singleline(&mut profile.languages);
    ui.label("Signes distinctifs");ui.text_edit_singleline(&mut profile.distinctive_features);
    ui.label("Voix et accent");ui.text_edit_singleline(&mut profile.voice);
    ui.label("Position initiale (propre a ce joueur)");ui.text_edit_singleline(&mut profile.initial_location);
    if ui.button("Enregistrer le profil physique et le lieu").clicked(){
        let result=self.storage.save_character_profile(campaign,&active.id,profile);
        self.message(result,"Profil personnel enregistre dans SQLite.");
    }
  }
  ui.separator();ui.heading("Inventaire");
  match self.storage.inventory(&active.id) {
   Ok(items)=>{for item in items {
    ui.horizontal(|ui|{
     ui.label(format!("{} × {}",item.quantity,item.name));
     if !item.notes.is_empty(){ui.weak(&item.notes);}
     if ui.small_button("Retirer").clicked(){let result=self.storage.delete_item(campaign,&item.id);self.message(result,"Objet supprimé.");}
    });
   }},Err(err)=>{ui.colored_label(egui::Color32::LIGHT_RED,format!("{err:#}"));}
  }
  ui.separator();
  ui.horizontal(|ui|{
   ui.label("Objet");ui.text_edit_singleline(&mut self.play.item_name);
   ui.label("Quantité");ui.add(egui::DragValue::new(&mut self.play.item_quantity).range(1..=9999));
  });
  ui.label("Notes de l'objet");ui.text_edit_singleline(&mut self.play.item_notes);
  if ui.button("Ajouter à l'inventaire").clicked(){
   let result=self.storage.add_item(campaign,&active.id,&self.play.item_name,self.play.item_quantity,&self.play.item_notes);
   if result.is_ok(){self.play.item_name.clear();self.play.item_quantity=1;self.play.item_notes.clear();}
   self.message(result,"Objet ajouté et journalisé.");
  }
  ui.separator();
  ui.heading("Ressources et biens génériques (L2.3)");
  ui.weak("Nom libre : crédits, eau, bière, cristaux, carburant… Édition initiale contrôlée par l'utilisateur ; les effets du MJ ne peuvent pas créer un solde.");
  if let Ok(items)=self.storage.holdings(campaign,&active.id) {
   if items.is_empty() {ui.weak("Aucune ressource générique initialisée.");}
   for (name,quantity) in items {ui.label(format!("• {} × {}",name,quantity));}
  }
  ui.horizontal_wrapped(|ui| {
   ui.label("Nom");ui.text_edit_singleline(&mut self.play.holding_name);
   ui.label("Qté");ui.add(egui::DragValue::new(&mut self.play.holding_quantity).range(1..=9999));
   if ui.button("Doter le personnage (édition explicite)").clicked() {
    let result=self.storage.grant_holding_by_user(campaign,&active.id,&self.play.holding_name,self.play.holding_quantity);
    if result.is_ok(){self.play.holding_name.clear();self.play.holding_quantity=1;}
    self.message(result,"Possession initiale ajoutée et journalisée.");
   }
  });
  ui.separator();
  ui.heading("Contacts connus de la campagne");
  ui.weak("Tu peux enregistrer des contacts initiaux. En jeu, Rust peut aussi reconnaître un PNJ déjà mentionné dans un récit validé au moment d'un transfert.");
  if let Ok(npcs)=self.storage.known_npcs(campaign) {
   for (id,name) in npcs {
    ui.collapsing(name.as_str(),|ui| {
     match self.storage.npc_holdings(campaign,&id) {
      Ok(items) if items.is_empty()=>{ui.weak("Aucune possession reçue.");},
      Ok(items)=>for (asset,quantity) in items {ui.label(format!("{} × {}",asset,quantity));},
      Err(e)=>{ui.weak(format!("{e:#}"));},
     }
    });
   }
  }
  ui.horizontal(|ui| {
   ui.label("PNJ");ui.text_edit_singleline(&mut self.play.npc_name);
   if ui.button("Enregistrer le contact").clicked() {
    let result=self.storage.register_npc(campaign,&self.play.npc_name);
    if result.is_ok(){self.play.npc_name.clear();}
    self.message(result,"Contact enregistré et journalisé.");
   }
  });
  ui.separator();ui.heading("Promesses exprimées par le personnage");
  if let Ok(promises)=self.storage.promises(campaign,&active.id) {
   for (recipient,description) in promises {ui.label(format!("• À {} : {}",recipient,description));}
  }
 }
 fn play_journal(&mut self,ui:&mut egui::Ui,campaign:&str){
  ui.heading("Journal canonique (ordre inverse)");
  match self.storage.events(campaign,200){
   Ok(events)=>{if events.is_empty(){ui.label("Aucun événement enregistré.");}
    for ev in events {egui::CollapsingHeader::new(format!("#{} · {}",ev.seq,ev.kind)).show(ui,|ui|{
     let mut req=ev.request.clone();
     // Les textes conditionnels non retenus peuvent contenir des secrets : jamais affichés au joueur.
     if let Some(fields)=req.as_object_mut(){fields.remove("success_text");fields.remove("failure_text");}
     let mut payload=ev.payload.clone();
     if let Some(fields)=payload.as_object_mut(){fields.remove("success_stake");fields.remove("failure_stake");}
     ui.label("Demande (sans issues secrètes)");ui.monospace(serde_json::to_string_pretty(&req).unwrap_or_default());
     ui.label("Résultat");ui.monospace(serde_json::to_string_pretty(&payload).unwrap_or_default());
    });}
   },Err(err)=>{ui.colored_label(egui::Color32::LIGHT_RED,format!("{err:#}"));}
  }
 }
}
