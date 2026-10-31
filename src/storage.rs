//! Source de vérité L2. Aucune configuration utilisateur hors SQLite.
use anyhow::{anyhow, bail, Context, Result};
use directories::ProjectDirs;
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use std::{collections::{HashMap,HashSet}, fs, path::{Path, PathBuf}, time::{Duration, SystemTime, UNIX_EPOCH}};
use uuid::Uuid;

const SCHEMA_L2: &str = include_str!("../schema/l2.sql");

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CampaignMode { Free, Partial, Complete }

impl CampaignMode {
    pub fn key(self) -> &'static str {
        match self { Self::Free => "free", Self::Partial => "partial", Self::Complete => "complete" }
    }
    pub fn label(self) -> &'static str {
        match self { Self::Free => "Libre", Self::Partial => "Partiel", Self::Complete => "Complet" }
    }
    fn parse(s: &str) -> Result<Self> {
        match s { "free" => Ok(Self::Free), "partial" => Ok(Self::Partial),
            "complete" => Ok(Self::Complete), _ => bail!("Mode de campagne inconnu: {s}") }
    }
}

#[derive(Clone, Debug)]
pub struct Campaign {
    pub id: String,
    pub title: String,
    pub mode: CampaignMode,
    pub seed_context: String,
    pub player_count: u8,
    pub archived: bool,
    pub state_version: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Clone, Debug)]
pub struct Participant {
    pub id: String,
    pub slot: u8,
    pub display_name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    pub llm_endpoint: String,
    pub llm_model: String,
    pub context_target_tokens: u32,
    pub compaction_threshold_pct: u8,
    pub llm_temperature: f32,
    pub llm_max_tokens: u32,
    pub llm_timeout_seconds: u64,
    pub last_campaign_id: Option<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            llm_endpoint: "http://127.0.0.1:8080/v1".into(),
            llm_model: String::new(),
            context_target_tokens: 4096,
            compaction_threshold_pct: 75,
            llm_temperature: 0.7,
            llm_max_tokens: 768,
            llm_timeout_seconds: 90,
            last_campaign_id: None,
        }
    }
}


/// Identifiant stable de scène dérivé d'un lieu explicitement validé par Rust.
/// Ne fait PAS de rapprochement géographique approximatif. Les qualificatifs
/// «devant», «derrière» et «intérieur» sont conservés pour ne pas confondre
/// l'extérieur d'un bâtiment avec l'intérieur. Les parenthèses sont descriptives.
fn scene_key(label:&str)->Option<String> {
    let primary=label.split([',','(']).next()?.trim();
    let normalized:String=primary.to_lowercase().chars().map(|ch|match ch {
        'à'|'â'|'ä'=>'a','ç'=>'c','é'|'è'|'ê'|'ë'=>'e',
        'î'|'ï'=>'i','ô'|'ö'=>'o','ù'|'û'|'ü'=>'u','ÿ'=>'y',
        'œ'=>'o','æ'=>'a', c if c.is_alphanumeric() => c, _=>' '
    }).collect();
    let clean=normalized.split_whitespace().collect::<Vec<_>>().join(" ");
    if clean.is_empty() || ["inconnu","indetermine","non precise","emplacement exact inconnu"]
        .iter().any(|word|clean.contains(word)) {return None;}
    let mut rest=clean.as_str();
    // «sur la place du vieux puits» et «place du vieux puits» identiques.
    // Attention : «devant la porte» reste une scène distincte de «porte».
    for prefix in ["sur la ","sur le ","sur les ","dans la ","dans le ","dans les ",
                   "a la ","au ","aux ","a l ","la ","le ","les ","l "] {
        if let Some(stripped)=rest.strip_prefix(prefix) {rest=stripped;break;}
    }
    (rest.chars().count()>=5).then(||rest.to_string())
}

fn store_actor_scene(tx:&rusqlite::Transaction<'_>,campaign:&str,actor:&str,
                     label:&str,source:&str)->Result<()> {
    let key=scene_key(label).unwrap_or_default();
    tx.execute("INSERT INTO actor_scenes(campaign_id,actor_id,place_key,label,source_action_key)         VALUES(?1,?2,?3,?4,?5) ON CONFLICT(actor_id) DO UPDATE SET         place_key=excluded.place_key,label=excluded.label,source_action_key=excluded.source_action_key",
        params![campaign,actor,key,label.trim(),source])?;
    Ok(())
}

pub struct Storage { conn: Connection, path: Option<PathBuf> }

impl Storage {
    pub fn default_path() -> Result<PathBuf> {
        let dirs = ProjectDirs::from("org", "MJUniversel", "MJUniversel")
            .ok_or_else(|| anyhow!("Répertoire de données du système indisponible"))?;
        Ok(dirs.data_local_dir().join("mj-universel-l2-4-3.sqlite3"))
    }

    pub fn open_default() -> Result<Self> { Self::open(Self::default_path()?) }

    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let parent = path.parent().ok_or_else(|| anyhow!("Chemin SQLite invalide"))?;
        fs::create_dir_all(parent).context("Création du répertoire de données")?;
        let conn = Connection::open(path).context("Ouverture SQLite")?;
        let mut store = Self { conn, path: Some(path.to_owned()) };
        store.initialize()?;
        Ok(store)
    }

    #[cfg(test)]
    pub fn in_memory() -> Result<Self> {
        let mut store = Self { conn: Connection::open_in_memory()?, path: None };
        store.initialize()?;
        Ok(store)
    }

    pub fn path_display(&self) -> String {
        self.path.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| ":memory:".into())
    }

    fn initialize(&mut self) -> Result<()> {
        self.conn.busy_timeout(Duration::from_secs(5))?;
        // IMPORTANT : vérifier la compatibilité AVANT tout PRAGMA susceptible
        // d'écrire dans le fichier (notamment journal_mode=WAL).
        // Une ancienne base doit rester strictement inchangée à l'ouverture.
        let version: i64 = self.conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        match version {
            0 => {
                // Une base non versionnée mais déjà peuplée n'est pas une base neuve.
                // Ne pas se limiter à la présence de la table « campaigns ».
                let existing: i64 = self.conn.query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE name NOT LIKE 'sqlite_%'",
                    [], |r| r.get(0),
                )?;
                if existing != 0 {
                    bail!("Base SQLite non versionnée et non vide : ouverture refusée sans migration");
                }
            }
            27 => {} // Schéma de cette livraison, pas de migration.
            _ => bail!("Base SQLite de version {version} incompatible avec le schéma 27 ; ouverture refusée sans migration."),
        }
        self.conn.execute_batch("PRAGMA foreign_keys=ON; PRAGMA synchronous=FULL;")?;
        if self.path.is_some() {
            self.conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        }
        if version == 0 {
            self.conn.execute_batch(SCHEMA_L2)?;
        }
        let result: String = self.conn.query_row("PRAGMA quick_check", [], |r| r.get(0))?;
        if result != "ok" { bail!("Intégrité SQLite suspecte : {result}"); }
        Ok(())
    }

    pub fn schema_version(&self) -> Result<i64> {
        Ok(self.conn.query_row("PRAGMA user_version", [], |r| r.get(0))?)
    }
    pub fn foreign_keys_enabled(&self) -> Result<bool> {
        let value: i64 = self.conn.query_row("PRAGMA foreign_keys", [], |r| r.get(0))?;
        Ok(value == 1)
    }

    /// Aucune édition directe du canon initial après ce point. Les opérations UARE restent autorisées.
    pub fn setup_complete(&self,campaign:&str)->Result<bool> {
        let row:Option<Option<i64>>=self.conn.query_row(
            "SELECT started_at FROM campaign_setup WHERE campaign_id=?1",
            params![campaign],|r|r.get(0)).optional()?;
        row.map(|value| value.is_some()).ok_or_else(||anyhow!("Campagne inconnue"))
    }

    pub fn finalize_setup(&mut self,campaign:&str)->Result<()> {
        let tx=self.conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        ensure_active(&tx,campaign)?;
        ensure_no_pending(&tx,campaign)?;
        ensure_setup_open(&tx,campaign)?;
        let count:i64=tx.query_row("SELECT COUNT(*) FROM characters c JOIN participants p ON c.participant_id=p.id WHERE p.campaign_id=?1 AND length(trim(c.name))>0",params![campaign],|r|r.get(0))?;
        let expected:i64=tx.query_row("SELECT player_count FROM campaigns WHERE id=?1",params![campaign],|r|r.get(0))?;
        if count!=expected {bail!("Tous les joueurs doivent avoir une fiche avant le début de la partie");}
        tx.execute("UPDATE campaign_setup SET started_at=?1 WHERE campaign_id=?2 AND started_at IS NULL",params![now(),campaign])?;
        event(&tx,campaign,&Uuid::new_v4().to_string(),"campaign_started",&json!({"campaign_id":campaign}),&json!({"characters":count}))?;
        tx.commit()?;
        Ok(())
    }

    pub fn list_campaigns(&self) -> Result<Vec<Campaign>> {
        let mut statement = self.conn.prepare(
            "SELECT id,title,mode,seed_context,player_count,status,state_version,created_at,updated_at \
             FROM campaigns ORDER BY updated_at DESC,id DESC")?;
        let rows = statement.query_map([], |r| {
            Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,
                r.get::<_,String>(3)?,r.get::<_,i64>(4)?,r.get::<_,String>(5)?,
                r.get::<_,i64>(6)?,r.get::<_,i64>(7)?,r.get::<_,i64>(8)?))
        })?;
        let mut result = Vec::new();
        for row in rows {
            let (id,title,mode,seed_context,count,status,state_version,created_at,updated_at)=row?;
            result.push(Campaign { id, title, mode:CampaignMode::parse(&mode)?, seed_context,
                player_count: u8::try_from(count)?, archived:status=="archived",
                state_version, created_at, updated_at });
        }
        Ok(result)
    }

    pub fn get_campaign(&self, id: &str) -> Result<Option<Campaign>> {
        Ok(self.list_campaigns()?.into_iter().find(|campaign| campaign.id == id))
    }

    pub fn participants(&self, campaign_id: &str) -> Result<Vec<Participant>> {
        let mut statement = self.conn.prepare(
            "SELECT id,slot,display_name FROM participants WHERE campaign_id=?1 ORDER BY slot")?;
        let rows = statement.query_map(params![campaign_id], |r| {
            Ok(Participant { id:r.get(0)?, slot:r.get::<_,u8>(1)?, display_name:r.get(2)? })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn create_campaign(&mut self, title: &str, mode: CampaignMode,
                           context: &str, count: u8) -> Result<String> {
        validate_campaign(title,mode,context,count)?;
        let id = Uuid::new_v4().to_string();
        let stamp=now();
        let tx=self.conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute("INSERT INTO campaigns\
            (id,title,mode,seed_context,player_count,status,state_version,created_at,updated_at)\
            VALUES(?1,?2,?3,?4,?5,'active',0,?6,?6)",
            params![id,title.trim(),mode.key(),context,count,stamp])?;
        for slot in 1..=count {
            let pid=Uuid::new_v4().to_string();
            let cid=Uuid::new_v4().to_string();
            let name=format!("Joueur {slot}");
            tx.execute("INSERT INTO participants(id,campaign_id,slot,display_name) VALUES (?1,?2,?3,?4)",
                params![pid, id, slot, name])?;
            tx.execute("INSERT INTO characters(id,participant_id,name,updated_at) VALUES (?1,?2,?3,?4)",
                params![cid,pid,name,stamp])?;
        }
        tx.execute("INSERT INTO campaign_setup(campaign_id,started_at) VALUES(?1,NULL)",params![id])?;
        tx.commit()?;
        Ok(id)
    }

    /// Enregistre l'édition atomiquement, avec protection contre les versions obsolètes.
    pub fn save_setup(&mut self, id: &str, expected_version: i64, title: &str,
                      mode: CampaignMode, context: &str, names: &[String]) -> Result<()> {
        let count=names.len();
        validate_campaign(title, mode, context, u8::try_from(count)?)?;
        for name in names {
            if name.trim().is_empty() || name.chars().count() > 80 { bail!("Nom de joueur invalide"); }
        }
        let tx=self.conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        ensure_no_pending(&tx,id)?;ensure_setup_open(&tx,id)?;
        let current_count: Option<i64>=tx.query_row(
            "SELECT player_count FROM campaigns WHERE id=?1",params![id],|r|r.get(0)).optional()?;
        if current_count != Some(count as i64) { bail!("Le nombre de joueurs ne peut pas changer dans L2"); }
        let n=tx.execute("UPDATE campaigns SET title=?1,mode=?2,seed_context=?3,\
            state_version=state_version+1,updated_at=?4 WHERE id=?5 AND state_version=?6",
            params![title.trim(),mode.key(),context,now(),id,expected_version])?;
        if n!=1 { bail!("Modification refusée : la campagne a changé. Actualisez."); }
        for (i,name) in names.iter().enumerate() {
            let modified=tx.execute(
                "UPDATE participants SET display_name=?1 WHERE campaign_id=?2 AND slot=?3",
                params![name.trim(),id,(i+1) as i64])?;
            if modified!=1 { bail!("Participant manquant, modification annulée"); }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn set_archived(&mut self,id:&str,archived:bool) -> Result<()> {
        let tx=self.conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        ensure_no_pending(&tx,id)?;
        let n=tx.execute("UPDATE campaigns SET status=?1, state_version=state_version+1,\
            updated_at=?2 WHERE id=?3",params![if archived {"archived"} else {"active"},now(),id])?;
        if n!=1 { bail!("Campagne introuvable"); }
        tx.commit()?;
        Ok(())
    }

    pub fn delete_campaign(&mut self,id:&str) -> Result<bool> {
        let tx=self.conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let changed=tx.execute("DELETE FROM campaigns WHERE id=?1",params![id])?;
        tx.commit()?;
        Ok(changed==1)
    }

    pub fn load_settings(&self) -> Result<AppSettings> {
        let data:String=self.conn.query_row(
            "SELECT value_json FROM app_settings WHERE key='general'",[],|r|r.get(0))?;
        Ok(serde_json::from_str(&data).context("Paramètres SQLite invalides")?)
    }

    pub fn save_settings(&mut self, settings: &AppSettings) -> Result<()> {
        if !(settings.llm_endpoint.starts_with("http://") || settings.llm_endpoint.starts_with("https://"))
            || settings.llm_endpoint.len()>2048 {
            bail!("URL du fournisseur invalide");
        }
        if !(512..=262_144).contains(&settings.context_target_tokens) {
            bail!("Budget de contexte hors limites (512–262144)");
        }
        if !(50..=95).contains(&settings.compaction_threshold_pct) {
            bail!("Seuil de compactage hors limites (50–95 %)");
        }
        if settings.llm_model.len()>200 { bail!("Nom de modèle trop long"); }
        if !settings.llm_temperature.is_finite() || !(0.0..=2.0).contains(&settings.llm_temperature) ||
            !(128..=8192).contains(&settings.llm_max_tokens) || !(10..=300).contains(&settings.llm_timeout_seconds) {
            bail!("Réglages IA hors limites");
        }
        crate::llm::AiConnection{endpoint:settings.llm_endpoint.clone(),model:settings.llm_model.clone(),
            api_key:None,temperature:settings.llm_temperature,max_tokens:settings.llm_max_tokens,
            timeout_seconds:settings.llm_timeout_seconds}.validate()?;
        let json=serde_json::to_string(settings)?;
        self.conn.execute("UPDATE app_settings SET value_json=?1,updated_at=?2 WHERE key='general'",
            params![json,now()])?;
        Ok(())
    }
}

fn validate_campaign(title: &str, mode: CampaignMode, context: &str, players: u8) -> Result<()> {
    if title.trim().is_empty() || title.trim().chars().count() > 120 {
        bail!("Le titre doit contenir entre 1 et 120 caractères");
    }
    if !(1..=6).contains(&players) { bail!("Choisissez entre 1 et 6 joueurs"); }
    if context.chars().count() > 40000 { bail!("Contexte initial limité à 40 000 caractères"); }
    if mode!=CampaignMode::Free && context.trim().is_empty() {
        bail!("Fournissez un contexte initial pour le mode partiel ou complet");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_initialization_is_idempotent_and_fk_active() -> Result<()> {
        let mut db=Storage::in_memory()?;
        assert_eq!(db.schema_version()?,27);
        assert!(db.foreign_keys_enabled()?);
        db.initialize()?;
        assert_eq!(db.schema_version()?,27);
        Ok(())
    }

    #[test]
    fn create_edit_archive_delete_and_cascade() -> Result<()> {
        let mut db=Storage::in_memory()?;
        let id=db.create_campaign("Valombre",CampaignMode::Partial,"Village mystérieux",3)?;
        assert_eq!(db.participants(&id)?.len(),3);
        let start=db.get_campaign(&id)?.unwrap();
        assert_eq!(start.state_version,0);
        let names=vec!["Aline".to_string(),"Bran".to_string(),"Cora".to_string()];
        db.save_setup(&id,0,"Valombre II",CampaignMode::Complete,"Une cité assiégée",&names)?;
        assert!(db.save_setup(&id,0,"Ancien",CampaignMode::Free,"",&names).is_err());
        assert_eq!(db.participants(&id)?[0].display_name,"Aline");
        db.set_archived(&id,true)?;
        assert!(db.get_campaign(&id)?.unwrap().archived);
        assert!(db.delete_campaign(&id)?);
        assert!(db.participants(&id)?.is_empty());
        assert!(db.get_campaign(&id)?.is_none());
        Ok(())
    }

    #[test]
    fn rejects_invalid_campaigns_and_settings() -> Result<()> {
        let mut db=Storage::in_memory()?;
        assert!(db.create_campaign(" ",CampaignMode::Free,"",1).is_err());
        assert!(db.create_campaign("X",CampaignMode::Complete,"",1).is_err());
        assert!(db.create_campaign("X",CampaignMode::Free,"",7).is_err());
        let mut s=db.load_settings()?;
        s.context_target_tokens=262145;
        assert!(db.save_settings(&s).is_err());
        s.context_target_tokens=8192;
        s.compaction_threshold_pct=80;
        db.save_settings(&s)?;
        assert_eq!(db.load_settings()?.compaction_threshold_pct,80);
        Ok(())
    }

    #[test]
    fn state_survives_close_and_reopen() -> Result<()> {
        let dir=std::env::temp_dir().join(format!("mj-l2-test-{}",Uuid::new_v4()));
        let path=dir.join("test.sqlite3");
        let id;
        {
            let mut db=Storage::open(&path)?;
            id=db.create_campaign("Persistante",CampaignMode::Free,"Mon monde",2)?;
            let mut s=db.load_settings()?;
            s.last_campaign_id=Some(id.clone());
            db.save_settings(&s)?;
        }
        {
            let db=Storage::open(&path)?;
            assert_eq!(db.get_campaign(&id)?.unwrap().seed_context,"Mon monde");
            assert_eq!(db.participants(&id)?.len(),2);
            assert_eq!(db.load_settings()?.last_campaign_id,Some(id));
            assert!(db.foreign_keys_enabled()?);
        }
        fs::remove_dir_all(&dir)?;
        Ok(())
    }
}

// L2 : état canonique, dés, journal et actions atomiques.
use crate::{dice, uare::{self,ActionRequest,ActionOutcome,Stats}};
// RollMode sert aux tests de stockage, pas au code de production.
#[cfg(test)]
use crate::dice::RollMode;
use serde_json::{json,Value};
use rusqlite::Transaction;

#[derive(Clone,Debug)]
pub struct Character {
 pub id:String,pub participant_id:String,pub slot:u8,pub name:String,pub class_name:String,
 pub level:u8,pub stats:Stats,pub hp:i32,pub max_hp:i32,pub ac:i32,
 pub skills:Vec<String>,pub notes:String,
}

/// Donnees physiques et sociales stabilisees, independantes des scores de des.
#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct CharacterProfile {
    pub sex:String, pub gender:String, pub age_years:u32,
    pub height_cm:u32, pub weight_kg:u32,
    pub appearance:String, pub origin:String, pub personality:String,
    pub initial_location:String, pub languages:String, pub distinctive_features:String, pub voice:String,
}
impl CharacterProfile {
    pub fn validate(&self)->Result<()> {
        let check=|text:&str,max:usize| !text.trim().is_empty() && text.chars().count()<=max;
        if !check(&self.sex,40)||!check(&self.gender,40)||!check(&self.appearance,500)
            ||!check(&self.origin,200)||!check(&self.personality,500)
            ||!check(&self.initial_location,160)||self.languages.chars().count()>200
            ||self.distinctive_features.chars().count()>300||self.voice.chars().count()>200
            ||!(1..=5000).contains(&self.age_years)
            ||!(25..=400).contains(&self.height_cm)||!(2..=500).contains(&self.weight_kg) {
            bail!("Profil physique ou biographique invalide");
        }
        Ok(())
    }
}

impl Storage {
    /// Registre facultatif des devises rencontrees ; aucune devise n'est principale.
    pub fn currency_list(&self,campaign:&str)->Result<Vec<String>> {
        let mut q=self.conn.prepare("SELECT name FROM campaign_currencies WHERE campaign_id=?1 ORDER BY name")?;
        let rows=q.query_map(params![campaign],|r|r.get(0))?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }
    /// Ajout manuel en preparation ; aucune devise n'est imposee a un autre personnage.
    pub fn add_currency(&mut self,campaign:&str,name:&str)->Result<()> {
        if name.trim().is_empty()||name.chars().count()>80 {bail!("Nom de devise invalide");}
        let tx=self.conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        ensure_active(&tx,campaign)?;ensure_setup_open(&tx,campaign)?;ensure_no_pending(&tx,campaign)?;
        tx.execute("INSERT INTO campaign_currencies(id,campaign_id,name) VALUES(?1,?2,?3)",
            params![Uuid::new_v4().to_string(),campaign,name.trim()])?;
        event(&tx,campaign,&Uuid::new_v4().to_string(),"currency_registered",
            &json!({"name":name}),&json!({"registered":true}))?;
        tx.commit()?;Ok(())
    }
    pub fn character_profile(&self,character:&str)->Result<Option<CharacterProfile>> {
        self.conn.query_row("SELECT sex,gender,age_years,height_cm,weight_kg,appearance,origin,personality,initial_location,languages,distinctive_features,voice FROM character_profiles WHERE character_id=?1",
            params![character],|r|Ok(CharacterProfile{sex:r.get(0)?,gender:r.get(1)?,age_years:r.get(2)?,
                height_cm:r.get(3)?,weight_kg:r.get(4)?,appearance:r.get(5)?,origin:r.get(6)?,
                personality:r.get(7)?,initial_location:r.get(8)?,languages:r.get(9)?,
                distinctive_features:r.get(10)?,voice:r.get(11)?})).optional().map_err(Into::into)
    }
    pub fn save_character_profile(&mut self,campaign:&str,character:&str,profile:&CharacterProfile)->Result<()> {
        profile.validate()?;
        let tx=self.conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        ensure_active(&tx,campaign)?;ensure_no_pending(&tx,campaign)?;ensure_setup_open(&tx,campaign)?;
        let known:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM characters c JOIN participants p ON p.id=c.participant_id WHERE c.id=?1 AND p.campaign_id=?2)",params![character,campaign],|r|r.get(0))?;
        if !known {bail!("Personnage hors campagne");}
        tx.execute("INSERT INTO character_profiles(character_id,sex,gender,age_years,height_cm,weight_kg,appearance,origin,personality,initial_location,languages,distinctive_features,voice,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14) ON CONFLICT(character_id) DO UPDATE SET sex=excluded.sex,gender=excluded.gender,age_years=excluded.age_years,height_cm=excluded.height_cm,weight_kg=excluded.weight_kg,appearance=excluded.appearance,origin=excluded.origin,personality=excluded.personality,initial_location=excluded.initial_location,languages=excluded.languages,distinctive_features=excluded.distinctive_features,voice=excluded.voice,updated_at=excluded.updated_at",
            params![character,profile.sex,profile.gender,profile.age_years,profile.height_cm,profile.weight_kg,profile.appearance,profile.origin,profile.personality,profile.initial_location,profile.languages,profile.distinctive_features,profile.voice,now()])?;
        store_actor_scene(&tx,campaign,character,&profile.initial_location,"initial_profile")?;
        event(&tx,campaign,&Uuid::new_v4().to_string(),"character_profile_updated",
            &json!({"character":character}),&json!({"updated":true}))?;
        tx.commit()?;Ok(())
    }
    /// Application de la proposition selectionnee, en une seule transaction atomique.
    /// Le PJ et ses dotations en plusieurs devises deviennent canoniques atomiquement.
    pub fn apply_character_suggestion(&mut self,campaign:&str,character:&str,
        chosen:&crate::llm::CharacterSuggestion)->Result<()> {
        chosen.validate()?;
        let profile=CharacterProfile{sex:chosen.sex.clone(),gender:chosen.gender.clone(),
            age_years:chosen.age_years,height_cm:chosen.height_cm,weight_kg:chosen.weight_kg,
            appearance:chosen.appearance.clone(),origin:chosen.origin.clone(),personality:chosen.personality.clone(),
            initial_location:chosen.initial_location.clone(),languages:chosen.languages.clone(),
            distinctive_features:chosen.distinctive_features.clone(),voice:chosen.voice.clone()};
        profile.validate()?;
        let hp=(8+(chosen.stats.constitution-10).div_euclid(2)).max(1);
        let ac=(10+(chosen.stats.dexterity-10).div_euclid(2)).max(1);
        let tx=self.conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        ensure_active(&tx,campaign)?;ensure_no_pending(&tx,campaign)?;ensure_setup_open(&tx,campaign)?;
        let name_taken:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM characters ch JOIN participants p ON p.id=ch.participant_id WHERE p.campaign_id=?1 AND ch.id<>?2 AND lower(ch.name)=lower(?3))",params![campaign,character,chosen.name.trim()],|r|r.get(0))?;
        if name_taken {bail!("Un autre personnage porte deja ce nom dans cette campagne");}
        let npc_taken:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM known_npcs WHERE campaign_id=?1 AND lower(name)=lower(?2))",params![campaign,chosen.name.trim()],|r|r.get(0))?;
        if npc_taken {bail!("Ce nom est deja celui d'un PNJ de la campagne");}
        let n=tx.execute("UPDATE characters SET name=?1,class_name=?2,level=1,strength=?3,dexterity=?4,constitution=?5,intelligence=?6,wisdom=?7,charisma=?8,hp=?9,max_hp=?9,ac=?10,skills_json=?11,notes=?12,updated_at=?13 WHERE id=?14 AND participant_id IN (SELECT id FROM participants WHERE campaign_id=?15)",
            params![chosen.name.trim(),chosen.role.trim(),chosen.stats.strength,chosen.stats.dexterity,
                chosen.stats.constitution,chosen.stats.intelligence,chosen.stats.wisdom,chosen.stats.charisma,
                hp,ac,serde_json::to_string(&chosen.skills)?,chosen.background,now(),character,campaign])?;
        if n!=1 {bail!("Personnage introuvable dans la campagne");}
        tx.execute("INSERT INTO character_profiles(character_id,sex,gender,age_years,height_cm,weight_kg,appearance,origin,personality,initial_location,languages,distinctive_features,voice,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14) ON CONFLICT(character_id) DO UPDATE SET sex=excluded.sex,gender=excluded.gender,age_years=excluded.age_years,height_cm=excluded.height_cm,weight_kg=excluded.weight_kg,appearance=excluded.appearance,origin=excluded.origin,personality=excluded.personality,initial_location=excluded.initial_location,languages=excluded.languages,distinctive_features=excluded.distinctive_features,voice=excluded.voice,updated_at=excluded.updated_at",
            params![character,profile.sex,profile.gender,profile.age_years,profile.height_cm,profile.weight_kg,profile.appearance,profile.origin,profile.personality,profile.initial_location,profile.languages,profile.distinctive_features,profile.voice,now()])?;
        store_actor_scene(&tx,campaign,character,&profile.initial_location,"initial_profile")?;
        // Le choix remplace la dotation precedente du seul PJ (jamais celle des autres).
        tx.execute("DELETE FROM inventory WHERE character_id=?1",params![character])?;
        tx.execute("DELETE FROM holdings WHERE campaign_id=?1 AND owner_key=?2",params![campaign,format!("character:{character}")])?;
        for item in &chosen.items {
            tx.execute("INSERT INTO inventory(id,character_id,name,quantity,notes) VALUES(?1,?2,?3,?4,?5)",
                params![Uuid::new_v4().to_string(),character,item.name.trim(),item.quantity,item.notes])?;
        }
        for fund in &chosen.starting_funds {
            let currency=fund.currency.trim();
            tx.execute("INSERT OR IGNORE INTO campaign_currencies(id,campaign_id,name) VALUES(?1,?2,?3)",
                params![Uuid::new_v4().to_string(),campaign,currency])?;
            tx.execute("INSERT INTO holdings(campaign_id,owner_key,asset,quantity) VALUES(?1,?2,?3,?4)",
                params![campaign,format!("character:{character}"),currency.to_lowercase(),fund.amount])?;
        }
        event(&tx,campaign,&Uuid::new_v4().to_string(),"character_suggestion_selected",
            &json!({"character_id":character}),
            &json!({"name":chosen.name,"role":chosen.role,"level":1,"stats":chosen.stats,
                "starting_items":chosen.items,"starting_funds":chosen.starting_funds,
                "profile_saved":true}))?;
        tx.commit()?;Ok(())
    }
}

#[derive(Clone,Debug)]
pub struct InventoryItem {pub id:String,pub character_id:String,pub name:String,pub quantity:i32,pub notes:String}
#[derive(Clone,Debug)]
pub struct GameEvent {pub seq:i64,pub kind:String,pub request:Value,pub payload:Value,pub created_at:i64}

/// Arbitrage pré-validé : les deux issues et le DD sont figés AVANT le lancer.
#[derive(Clone,Debug)]
pub struct PendingAiRoll {
 pub request:ActionRequest,
 pub proposal:crate::llm::AiProposal,
 pub model:String,
}
fn ensure_no_pending(tx:&Transaction<'_>,campaign:&str)->Result<()> {
 let active:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM pending_rolls WHERE campaign_id=?1)",params![campaign],|r|r.get(0))?;
 if active { bail!("Un jet autorisé attend la décision du joueur : termine ou abandonne cette action avant toute modification"); }
 Ok(())
}

fn ensure_setup_open(tx:&Transaction<'_>, campaign:&str)->Result<()> {
    let started:Option<Option<i64>>=tx.query_row(
        "SELECT started_at FROM campaign_setup WHERE campaign_id=?1",
        params![campaign], |r|r.get(0)).optional()?;
    match started {
        Some(None) => Ok(()),
        Some(Some(_)) => bail!("Campagne démarrée : fiche, inventaire et dotations verrouillés. Utilisez les actions validées par Rust."),
        None => bail!("Campagne sans état de préparation"),
    }
}
fn ensure_setup_done(tx:&Transaction<'_>, campaign:&str)->Result<()> {
    let started:Option<Option<i64>>=tx.query_row(
        "SELECT started_at FROM campaign_setup WHERE campaign_id=?1",
        params![campaign], |r|r.get(0)).optional()?;
    match started {
        Some(Some(_)) => Ok(()),
        Some(None) => bail!("Préparez les fiches et les possessions, puis confirmez le début de la campagne."),
        None => bail!("Campagne sans état de préparation"),
    }
}
fn ensure_active(tx:&Transaction<'_>,campaign:&str)->Result<()> {
 let status:Option<String>=tx.query_row("SELECT status FROM campaigns WHERE id=?1",params![campaign],|r|r.get(0)).optional()?;
 if status.as_deref()!=Some("active") {bail!("Campagne absente ou archivée");} Ok(())
}
fn event(tx:&Transaction<'_>,campaign:&str,key:&str,kind:&str,request:&Value,payload:&Value)->Result<()> {
 tx.execute("INSERT INTO game_events(id,campaign_id,request_key,kind,request_json,payload_json,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7)",
  params![Uuid::new_v4().to_string(),campaign,key,kind,request.to_string(),payload.to_string(),now()])?;
 tx.execute("UPDATE campaigns SET state_version=state_version+1,updated_at=?1 WHERE id=?2",params![now(),campaign])?;
 Ok(())
}
impl Storage {
 pub fn characters(&self,campaign:&str)->Result<Vec<Character>> {
  let mut stmt=self.conn.prepare("SELECT ch.id,ch.participant_id,p.slot,ch.name,ch.class_name,ch.level,ch.strength,ch.dexterity,ch.constitution,ch.intelligence,ch.wisdom,ch.charisma,ch.hp,ch.max_hp,ch.ac,ch.skills_json,ch.notes FROM characters ch JOIN participants p ON ch.participant_id=p.id WHERE p.campaign_id=?1 ORDER BY p.slot")?;
  let rows=stmt.query_map(params![campaign],|r| {
   Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,u8>(2)?,r.get::<_,String>(3)?,r.get::<_,String>(4)?,r.get::<_,u8>(5)?,
    Stats{strength:r.get(6)?,dexterity:r.get(7)?,constitution:r.get(8)?,intelligence:r.get(9)?,wisdom:r.get(10)?,charisma:r.get(11)?},
    r.get::<_,i32>(12)?,r.get::<_,i32>(13)?,r.get::<_,i32>(14)?,r.get::<_,String>(15)?,r.get::<_,String>(16)?))
  })?;
  let mut result=Vec::new();
  for row in rows {let (id,participant_id,slot,name,class_name,level,stats,hp,max_hp,ac,skills,notes)=row?;
   result.push(Character{id,participant_id,slot,name,class_name,level,stats,hp,max_hp,ac,skills:serde_json::from_str(&skills)?,notes});}
  Ok(result)
 }
 pub fn save_character(&mut self,campaign:&str,ch:&Character)->Result<()> {
  ch.stats.validate()?;
  if ch.name.trim().is_empty()||ch.name.chars().count()>80||ch.class_name.chars().count()>80||ch.notes.chars().count()>10000{bail!("Fiche : texte invalide");}
  if !(1..=20).contains(&ch.level){bail!("Fiche : niveau {} hors limites (1–20)",ch.level);}
  if !(1..=9999).contains(&ch.max_hp){bail!("Fiche : PV maximum {} hors limites (1–9999)",ch.max_hp);}
  if ch.hp<0{bail!("Fiche : PV actuels {} négatifs (minimum : 0)",ch.hp);}
  if ch.hp>ch.max_hp{bail!("Fiche : PV actuels {} supérieurs aux PV maximum {}. Ajuster les PV maximum ou les PV actuels.",ch.hp,ch.max_hp);}
  if !(1..=50).contains(&ch.ac){bail!("Fiche : classe d'armure {} hors limites (1–50)",ch.ac);}
  if ch.skills.len()>18||ch.skills.iter().any(|s|s.chars().count()>40){bail!("Liste de compétences invalide");}
  if ch.level==1 && self.character_profile(&ch.id)?.is_some() {
   let mut scores=[ch.stats.strength,ch.stats.dexterity,ch.stats.constitution,
       ch.stats.intelligence,ch.stats.wisdom,ch.stats.charisma];
   scores.sort_unstable();
   if scores!=[8,10,12,13,14,15] {bail!("Personnage de niveau 1 prepare : les six caracteristiques doivent rester une permutation de 8,10,12,13,14,15");}
  }
  let tx=self.conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
  ensure_active(&tx,campaign)?;ensure_no_pending(&tx,campaign)?;ensure_setup_open(&tx,campaign)?;
  let n=tx.execute("UPDATE characters SET name=?1,class_name=?2,level=?3,strength=?4,dexterity=?5,constitution=?6,intelligence=?7,wisdom=?8,charisma=?9,hp=?10,max_hp=?11,ac=?12,skills_json=?13,notes=?14,updated_at=?15 WHERE id=?16 AND participant_id IN (SELECT id FROM participants WHERE campaign_id=?17)",params![ch.name.trim(),ch.class_name.trim(),ch.level,ch.stats.strength,ch.stats.dexterity,ch.stats.constitution,ch.stats.intelligence,ch.stats.wisdom,ch.stats.charisma,ch.hp,ch.max_hp,ch.ac,serde_json::to_string(&ch.skills)?,ch.notes,now(),ch.id,campaign])?;
  if n!=1{bail!("Personnage introuvable dans la campagne");}
  event(&tx,campaign,&Uuid::new_v4().to_string(),"character_updated",&json!({"character":ch.id}),&json!({"name":ch.name,"level":ch.level,"hp":ch.hp,"max_hp":ch.max_hp,"stats":ch.stats,"ac":ch.ac,"class_name":ch.class_name,"skills":ch.skills,"notes_changed":true}))?;
  tx.commit()?;Ok(())
 }
 pub fn inventory(&self,character:&str)->Result<Vec<InventoryItem>> {
  let mut stmt=self.conn.prepare("SELECT id,character_id,name,quantity,notes FROM inventory WHERE character_id=?1 ORDER BY name,id")?;
  let rows = stmt.query_map(params![character], |r| {
   Ok(InventoryItem {
    id: r.get(0)?,
    character_id: r.get(1)?,
    name: r.get(2)?,
    quantity: r.get(3)?,
    notes: r.get(4)?,
   })
  })?;
  let items = rows.collect::<rusqlite::Result<Vec<_>>>()?;
  Ok(items)
 }
 pub fn add_item(&mut self,campaign:&str,character:&str,name:&str,quantity:i32,notes:&str)->Result<()> {
  if name.trim().is_empty()||name.chars().count()>120||notes.chars().count()>1000||!(1..=9999).contains(&quantity){bail!("Objet invalide");}
  let tx=self.conn.transaction_with_behavior(TransactionBehavior::Immediate)?;ensure_active(&tx,campaign)?;ensure_no_pending(&tx,campaign)?;ensure_setup_open(&tx,campaign)?;
  let known:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM characters ch JOIN participants p ON ch.participant_id=p.id WHERE ch.id=?1 AND p.campaign_id=?2)",params![character,campaign],|r|r.get(0))?;
  if !known{bail!("Personnage étranger à cette campagne");}
  let id=Uuid::new_v4().to_string();
  tx.execute("INSERT INTO inventory(id,character_id,name,quantity,notes) VALUES(?1,?2,?3,?4,?5)",params![id,character,name.trim(),quantity,notes])?;
  event(&tx,campaign,&Uuid::new_v4().to_string(),"item_added",&json!({"character":character}),&json!({"item_id":id,"name":name,"quantity":quantity,"notes":notes}))?;
  tx.commit()?;Ok(())
 }
 pub fn delete_item(&mut self,campaign:&str,item_id:&str)->Result<()> {
  let tx=self.conn.transaction_with_behavior(TransactionBehavior::Immediate)?;ensure_active(&tx,campaign)?;ensure_no_pending(&tx,campaign)?;ensure_setup_open(&tx,campaign)?;
  let old:Option<(String,String,i32)>=tx.query_row("SELECT i.character_id,i.name,i.quantity FROM inventory i JOIN characters c ON i.character_id=c.id JOIN participants p ON c.participant_id=p.id WHERE i.id=?1 AND p.campaign_id=?2",params![item_id,campaign],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional()?;
  let (character,name,quantity)=old.ok_or_else(||anyhow!("Objet introuvable"))?;
  tx.execute("DELETE FROM inventory WHERE id=?1",params![item_id])?;
  event(&tx,campaign,&Uuid::new_v4().to_string(),"item_removed",&json!({"item_id":item_id}),&json!({"character":character,"name":name,"quantity":quantity}))?;
  tx.commit()?;Ok(())
 }
 /// Dialogue de rôle paginé par quantité de messages, indépendant du journal technique.
 pub fn roleplay_history(&self,campaign:&str,limit:usize)->Result<Vec<GameEvent>> {
  let mut stmt=self.conn.prepare("SELECT seq,kind,request_json,payload_json,created_at FROM game_events
    WHERE campaign_id=?1 AND kind IN ('action_resolved','action_pending_roll','action_cancelled')
    ORDER BY seq DESC LIMIT ?2")?;
  let rows=stmt.query_map(params![campaign,limit.min(100_000) as i64],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,i64>(4)?)))?;
  let mut result=Vec::new();
  for row in rows {let (seq,kind,request,payload,created_at)=row?;
   result.push(GameEvent{seq,kind,request:serde_json::from_str(&request)?,payload:serde_json::from_str(&payload)?,created_at});}
  Ok(result)
 }
 pub fn events(&self,campaign:&str,limit:usize)->Result<Vec<GameEvent>> {
  let mut stmt=self.conn.prepare("SELECT seq,kind,request_json,payload_json,created_at FROM game_events WHERE campaign_id=?1 ORDER BY seq DESC LIMIT ?2")?;
  let rows=stmt.query_map(params![campaign,limit.min(500) as i64],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,i64>(4)?)))?;
  let mut out=Vec::new();for row in rows{let (seq,kind,request,payload,created_at)=row?;out.push(GameEvent{seq,kind,request:serde_json::from_str(&request)?,payload:serde_json::from_str(&payload)?,created_at});}Ok(out)
 }
 /// Lecture sûre de la résolution pendante, même après redémarrage.
 pub fn pending_ai_roll(&self,campaign:&str)->Result<Option<PendingAiRoll>> {
  let data:Option<(String,String,String)>=self.conn.query_row(
   "SELECT request_json,proposal_json,provider_model FROM pending_rolls WHERE campaign_id=?1",
   params![campaign],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional()?;
  data.map(|(r,p,m)| Ok(PendingAiRoll{request:serde_json::from_str(&r)?,proposal:serde_json::from_str(&p)?,model:m})).transpose()
 }
 /// Rust ne lance PAS de dés pendant cette étape ; attente enregistrée en transaction.
 pub fn queue_ai_roll(&mut self,req:&ActionRequest,proposal:&crate::llm::AiProposal,model:&str)->Result<()> {
  proposal.validate_for_action(&req.text)?;
  self.validate_scene_narration(req,proposal)?;
  self.validate_effect_preflight(req,proposal)?;
  if req.decision!=uare::Decision::AbilityCheck || req.ability!=proposal.ability ||
     req.proficient!=proposal.proficient || req.dc!=proposal.dc || req.roll_mode!=proposal.roll_mode ||
     req.success_text!=proposal.success_text || req.failure_text!=proposal.failure_text ||
     req.hp_delta!=0 || req.target_id.is_some() || req.decision!=proposal.decision {
      bail!("Arbitrage de jet incorrect ou effet non autorisé");
  }
  if req.key.trim().is_empty() || req.key.len()>128 || req.text.trim().is_empty() || req.text.chars().count()>4000 {
   bail!("Action invalide");
  }
  let tx=self.conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
  ensure_active(&tx,&req.campaign_id)?;ensure_setup_done(&tx,&req.campaign_id)?;ensure_no_pending(&tx,&req.campaign_id)?;
  if let Some(effect)=proposal.effect.as_ref() {
    preflight_effect(&tx,&req.campaign_id,&req.actor_id,effect)?;
  }
  let actor_exists:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM characters c JOIN participants p ON c.participant_id=p.id WHERE c.id=?1 AND p.campaign_id=?2)",
    params![req.actor_id,req.campaign_id],|r|r.get(0))?;
  if !actor_exists {bail!("Acteur inconnu");}
  tx.execute("INSERT INTO pending_rolls(campaign_id,request_key,actor_id,request_json,proposal_json,provider_model,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7)",
   params![req.campaign_id,req.key,req.actor_id,serde_json::to_string(req)?,serde_json::to_string(proposal)?,model,now()])?;
  event(&tx,&req.campaign_id,&format!("{}:pending",req.key),"action_pending_roll",
    &serde_json::to_value(req)?,&json!({"reason":proposal.reason,"dc":proposal.dc,
       "public_success":proposal.stakes_public_success,
       "public_failure":proposal.stakes_public_failure}))?;
  tx.commit()?;Ok(())
 }
 /// Seul cet appel, pour l'action attendue, est autorisé à déclencher un d20 IA.
 pub fn finish_pending_roll(&mut self,campaign:&str,request_key:&str)->Result<ActionOutcome> {
  let pending=self.pending_ai_roll(campaign)?.ok_or_else(||anyhow!("Aucun jet en attente : lancer interdit"))?;
  if pending.request.key!=request_key || pending.request.campaign_id!=campaign {
   bail!("Jet non autorisé pour cette action");
  }
  pending.proposal.validate_for_action(&pending.request.text)?;
  self.validate_scene_narration(&pending.request,&pending.proposal)?;
  self.validate_effect_preflight(&pending.request,&pending.proposal)?;
  self.resolve_action_inner(&pending.request,Some((&pending.proposal,&pending.model)),true)
 }
 pub fn cancel_pending_roll(&mut self,campaign:&str,request_key:&str)->Result<()> {
  let tx=self.conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
  ensure_active(&tx,campaign)?;
  let req:Option<String>=tx.query_row("SELECT request_json FROM pending_rolls WHERE campaign_id=?1 AND request_key=?2",params![campaign,request_key],|r|r.get(0)).optional()?;
  let req=req.ok_or_else(||anyhow!("Aucune action en attente à abandonner"))?;
  tx.execute("DELETE FROM pending_rolls WHERE campaign_id=?1 AND request_key=?2",params![campaign,request_key])?;
  event(&tx,campaign,&format!("{}:cancel",request_key),"action_cancelled",&serde_json::from_str::<Value>(&req)?,&json!({"message":"Le joueur a renoncé avant le lancer."}))?;
  tx.commit()?;Ok(())
 }
 pub fn resolve_action(&mut self,req:&ActionRequest)->Result<ActionOutcome>{
  if req.key.trim().is_empty()||req.key.len()>128{bail!("Identifiant d'action invalide");}
  self.resolve_action_inner(req,None,false)
 }
 pub fn resolve_ai_action(&mut self,req:&ActionRequest,proposal:&crate::llm::AiProposal,model:&str)->Result<ActionOutcome>{
  // Validation indépendante du transport HTTP : impossible de contourner la politique
  // de dialogue en appelant directement la couche de persistance.
  proposal.validate_for_action(&req.text)?;
  self.validate_scene_narration(req,proposal)?;
  if req.decision!=proposal.decision || req.ability!=proposal.ability || req.proficient!=proposal.proficient ||
     req.dc!=proposal.dc || req.roll_mode!=proposal.roll_mode || req.success_text!=proposal.success_text ||
     req.failure_text!=proposal.failure_text || req.hp_delta!=0 || req.target_id.is_some() {
     bail!("La résolution demandée ne correspond pas à l'arbitrage validé du MJ");
  }
  if req.decision==uare::Decision::AbilityCheck {
    bail!("Le test doit être proposé et enregistré avant tout lancer du joueur");
  }
  self.resolve_action_inner(req,Some((proposal,model)),false)
 }
 fn resolve_action_inner(&mut self,req:&ActionRequest, ai:Option<(&crate::llm::AiProposal,&str)>,authorized_pending:bool)->Result<ActionOutcome>{
  if req.key.trim().is_empty()||req.key.len()>128{bail!("Identifiant d'action invalide");}
  let tx=self.conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
  // Retour de la résolution déjà enregistrée sans refaire les dés ni les dégâts.
  let prior:Option<(String,String)>=tx.query_row("SELECT request_json,payload_json FROM game_events WHERE campaign_id=?1 AND request_key=?2 AND kind='action_resolved'",params![req.campaign_id,req.key],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
  let canonical=serde_json::to_value(req)?;
  if let Some((old,done))=prior {
   if serde_json::from_str::<Value>(&old)?!=canonical{bail!("Clé idempotente réutilisée avec d'autres paramètres");}
   if let Some((proposal,_))=ai {
    let previous:Option<String>=tx.query_row(
     "SELECT proposal_json FROM ai_decisions WHERE campaign_id=?1 AND action_key=?2",
     params![req.campaign_id,req.key],|r|r.get(0)).optional()?;
    let previous=previous.ok_or_else(||anyhow!("Proposition IA originale absente pour cette clé"))?;
    if serde_json::from_str::<Value>(&previous)?!=serde_json::to_value(proposal)? {
     bail!("Clé idempotente réutilisée avec une proposition IA différente");
    }
   }
   return Ok(serde_json::from_str(&done)?);
  }
  ensure_active(&tx,&req.campaign_id)?;ensure_setup_done(&tx,&req.campaign_id)?;
  if authorized_pending {
   let stored:Option<String>=tx.query_row("SELECT request_json FROM pending_rolls WHERE campaign_id=?1 AND request_key=?2",params![req.campaign_id,req.key],|r|r.get(0)).optional()?;
   let stored=stored.ok_or_else(||anyhow!("Action en attente introuvable"))?;
   if serde_json::from_str::<Value>(&stored)?!=canonical {bail!("Action attendue modifiée après arbitrage");}
  } else {ensure_no_pending(&tx,&req.campaign_id)?;}
  let actor:Option<(Stats,u8)>=tx.query_row("SELECT c.strength,c.dexterity,c.constitution,c.intelligence,c.wisdom,c.charisma,c.level FROM characters c JOIN participants p ON c.participant_id=p.id WHERE c.id=?1 AND p.campaign_id=?2",params![req.actor_id,req.campaign_id],|r|Ok((Stats{strength:r.get(0)?,dexterity:r.get(1)?,constitution:r.get(2)?,intelligence:r.get(3)?,wisdom:r.get(4)?,charisma:r.get(5)?},r.get(6)?))).optional()?;
  let (stats,level)=actor.ok_or_else(||anyhow!("Acteur inconnu dans la campagne"))?;
  // Contrôler les possessions AVANT de tirer un dé ; aucun échec matériel ne doit
  // provoquer un deuxième lancer ni laisser un premier lancer sans trace.
  if let Some((proposal,_))=ai {
    if let Some(effect)=proposal.effect.as_ref() {
      preflight_effect(&tx,&req.campaign_id,&req.actor_id,effect)?;
    }
  }
  let mut outcome=uare::resolve(req,&stats,level,&mut dice::SystemRng)?;
  if outcome.success&&req.hp_delta!=0 {
   let target=req.target_id.as_ref().ok_or_else(||anyhow!("Cible manquante"))?;
   let before:Option<(i32,i32)>=tx.query_row("SELECT c.hp,c.max_hp FROM characters c JOIN participants p ON c.participant_id=p.id WHERE c.id=?1 AND p.campaign_id=?2",params![target,req.campaign_id],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
   let (hp,max_hp)=before.ok_or_else(||anyhow!("Cible hors de la campagne"))?;
   let next=(hp as i64 + req.hp_delta as i64).clamp(0,max_hp as i64) as i32;
   tx.execute("UPDATE characters SET hp=?1,updated_at=?2 WHERE id=?3",params![next,now(),target])?;
   outcome.applied_hp_delta=next-hp;outcome.target_hp=Some(next);
  }
  if outcome.success {
    if let Some((proposal,_))=ai {
      if let Some(effect)=proposal.effect.as_ref() {
        outcome.applied_effect=Some(apply_effect(&tx,&req.campaign_id,&req.actor_id,&req.key,effect)?);
      }
    }
  }
  if authorized_pending {
    tx.execute("DELETE FROM pending_rolls WHERE campaign_id=?1 AND request_key=?2",params![req.campaign_id,req.key])?;
  }
  event(&tx,&req.campaign_id,&req.key,"action_resolved",&canonical,&serde_json::to_value(&outcome)?)?;
  if let Some((proposal,model))=ai {
   let proposal_json=serde_json::to_string(proposal)?;
   tx.execute("INSERT INTO ai_decisions(campaign_id,action_key,actor_id,provider_model,proposal_json,created_at) VALUES(?1,?2,?3,?4,?5,?6)",
    params![req.campaign_id,req.key,req.actor_id,model,proposal_json,now()])?;
   if let Some(fact)=proposal.fact_proposal.as_ref(){
    tx.execute("INSERT INTO ai_fact_proposals(id,campaign_id,source_action_key,fact_text,status,created_at) VALUES(?1,?2,?3,?4,'pending',?5)",
     params![Uuid::new_v4().to_string(),req.campaign_id,req.key,fact.trim(),now()])?;
   }
  }
  tx.commit()?;Ok(outcome)
 }
}

#[cfg(test)] mod l1_tests {
 use super::*;
 use crate::uare::{Ability,Decision};
 #[test]fn character_inventory_and_events()->Result<()> {
  let mut db=Storage::in_memory()?;
  let camp=db.create_campaign("Valombre",CampaignMode::Free,"",2)?;
  let mut cs=db.characters(&camp)?;assert_eq!(cs.len(),2);
  cs[0].stats.strength=18;cs[0].name="Ada".into();db.save_character(&camp,&cs[0])?;
  assert_eq!(db.characters(&camp)?[0].stats.strength,18);
  db.add_item(&camp,&cs[0].id,"Corde",2,"Chanvre")?;
  let items=db.inventory(&cs[0].id)?;assert_eq!(items.len(),1);
  db.delete_item(&camp,&items[0].id)?;assert!(db.inventory(&cs[0].id)?.is_empty());
  assert_eq!(db.events(&camp,50)?.len(),3);Ok(())
 }
 #[test]fn atomic_action_and_idempotency()->Result<()> {
  let mut db=Storage::in_memory()?;let camp=db.create_campaign("Test",CampaignMode::Free,"",1)?;
  let id=db.characters(&camp)?[0].id.clone();
  db.finalize_setup(&camp)?;
  let req=ActionRequest{key:"unique-1".into(),campaign_id:camp.clone(),actor_id:id.clone(),text:"Je me soigne".into(),decision:Decision::AutoSuccess,ability:Ability::Wisdom,proficient:false,dc:10,roll_mode:RollMode::Normal,target_id:Some(id.clone()),hp_delta:-4,success_text:"Aie".into(),failure_text:String::new()};
  let first=db.resolve_action(&req)?;assert_eq!(first.target_hp,Some(6));
  let second=db.resolve_action(&req)?;assert_eq!(first.target_hp,second.target_hp);
  assert_eq!(db.characters(&camp)?[0].hp,6);
  assert_eq!(db.events(&camp,20)?.len(),2);
  let mut changed=req;changed.hp_delta=-5;assert!(db.resolve_action(&changed).is_err());Ok(())
 }
 #[test]fn invalid_target_rolls_back()->Result<()> {
  let mut db=Storage::in_memory()?;
  let a=db.create_campaign("A",CampaignMode::Free,"",1)?;
  let b=db.create_campaign("B",CampaignMode::Free,"",1)?;
  let actor=db.characters(&a)?[0].id.clone();let foreign=db.characters(&b)?[0].id.clone();
  db.finalize_setup(&a)?;
  let req=ActionRequest{key:"invalid".into(),campaign_id:a.clone(),actor_id:actor,text:"Soigner".into(),decision:Decision::AutoSuccess,ability:Ability::Wisdom,proficient:false,dc:10,roll_mode:RollMode::Normal,target_id:Some(foreign),hp_delta:3,success_text:String::new(),failure_text:String::new()};
  assert!(db.resolve_action(&req).is_err());assert!(db.events(&a,20)?.iter().all(|e|e.kind!="action_resolved"));Ok(())
 }
 #[test]fn persistent_new_database()->Result<()> {
  let dir=std::env::temp_dir().join(format!("mj-l1-state-{}",Uuid::new_v4()));let path=dir.join("l2.sqlite3");let id;
  {let mut db=Storage::open(&path)?;id=db.create_campaign("Rémanence",CampaignMode::Free,"",1)?;let c=db.characters(&id)?[0].id.clone();db.add_item(&id,&c,"Livre",1,"")?;}
  {let db=Storage::open(&path)?;let c=db.characters(&id)?;assert_eq!(c.len(),1);assert_eq!(db.inventory(&c[0].id)?.len(),1);}
  fs::remove_dir_all(dir)?;Ok(())
 }
}

// L2 : contexte minimal, propositions canoniques validées et clé API protégée.
use crate::llm::{AiSnapshot,ProposedEffect};
fn clip_text(text:&str,max:usize)->String {text.chars().take(max).collect()}

/// Recherche locale très simple : les six derniers événements préservent le dialogue en cours ;
/// six événements antérieurs partageant des mots significatifs aident à retrouver un souvenir.
/// Jamais de génération ni d'effacement : la base SQLite contient toujours le journal complet.
fn search_terms(text: &str) -> HashSet<String> {
    const STOP: &[&str] = &["dans","avec","pour","plus","sans","vers","cette","cela","sont",
        "nous","vous","leur","leurs","elle","elles","sais","veux","fait","faire","entre",
        "demande","demander","bonjour","alors","comme","mais","tous","toutes","parle","parler",
        "quel","quelle","quelles","comment","avait","être","donc","suis","depuis","quand"];
    text.to_lowercase().split(|c:char| !c.is_alphanumeric())
        .filter(|w|w.chars().count()>=4 && !STOP.contains(w))
        .map(ToOwned::to_owned).collect()
}
fn select_relevant_events(events:Vec<GameEvent>,action:&str)->Vec<GameEvent>{
    let mut actions:Vec<GameEvent>=events.into_iter().filter(|ev|ev.kind=="action_resolved").collect();
    actions.sort_by(|a,b|b.seq.cmp(&a.seq));
    let terms=search_terms(action);
    let mut kept:Vec<GameEvent>=actions.iter().take(6).cloned().collect();
    let mut older:Vec<(usize,i64,GameEvent)>=actions.into_iter().skip(6).filter_map(|event|{
        let previous_action=event.request.get("text").and_then(Value::as_str).unwrap_or("");
        let narration=event.payload.get("narration").and_then(Value::as_str).unwrap_or("");
        let overlap=search_terms(&format!("{previous_action} {narration}"))
            .intersection(&terms).count();
        (overlap>0).then_some((overlap,event.seq,event))
    }).collect();
    older.sort_by(|a,b|b.0.cmp(&a.0).then(b.1.cmp(&a.1)));
    kept.extend(older.into_iter().take(6).map(|(_,_,e)|e));
    kept.sort_by(|a,b|b.seq.cmp(&a.seq));
    kept
}
#[derive(Clone, Debug)]
pub struct FactProposal {pub id:String,pub fact:String,pub status:String,pub source_action_key:String}
impl Storage {
    pub fn ai_snapshot(&self,campaign:&str,actor:&str,action:&str,target_tokens:u32)->Result<AiSnapshot>{
        let c=self.get_campaign(campaign)?.ok_or_else(||anyhow!("Campagne inconnue"))?;
        if c.archived {bail!("Impossible de jouer dans une campagne archivée");}
        if !self.setup_complete(campaign)? {bail!("La campagne doit être préparée avant le premier tour");}
        if action.trim().is_empty() || action.chars().count()>4000 {bail!("Action vide ou trop longue");}
        let chars=self.characters(campaign)?;
        if !chars.iter().any(|ch|ch.id==actor){bail!("Acteur inconnu");}
        let character_json:Vec<Value>=chars.iter().map(|ch|{
            let profile=self.character_profile(&ch.id).ok().flatten();
            json!({"id":ch.id,"name":ch.name,"class":ch.class_name,
            "level":ch.level,"hp":ch.hp,"max_hp":ch.max_hp,"ac":ch.ac,"stats":ch.stats,"skills":ch.skills,
            "appearance":profile.as_ref().map(|p|&p.appearance),
            "age_years":profile.as_ref().map(|p|p.age_years),"height_cm":profile.as_ref().map(|p|p.height_cm),
            "weight_kg":profile.as_ref().map(|p|p.weight_kg),
            "gender":profile.as_ref().map(|p|&p.gender),
            "sex":profile.as_ref().filter(|_|ch.id==actor).map(|p|&p.sex),
            "origin_private":profile.as_ref().filter(|_|ch.id==actor).map(|p|&p.origin),
            "personality_private":profile.as_ref().filter(|_|ch.id==actor).map(|p|&p.personality),
            "background_private":if ch.id==actor {Some(&ch.notes)}else{None},
            "languages_private":profile.as_ref().filter(|_|ch.id==actor).map(|p|&p.languages),
            "distinctive_features":profile.as_ref().map(|p|&p.distinctive_features),
            "voice":profile.as_ref().map(|p|&p.voice)})
        }).collect();
        let actor_place=self.actor_location(campaign,actor)?;
        let scene_id=actor_place.as_deref().and_then(scene_key);
        let colocated:Vec<String>=if let Some(ref key)=scene_id {
            chars.iter().filter(|ch|ch.id!=actor)
                .filter(|ch|self.actor_location(campaign,&ch.id).ok().flatten()
                    .as_deref().and_then(scene_key).as_ref()==Some(key))
                .map(|ch|ch.name.clone()).collect()
        } else {Vec::new()};
        let inv=self.inventory(actor)?.into_iter().take(40).map(|item|json!({"name":item.name,"quantity":item.quantity})).collect::<Vec<_>>();
        let holdings=self.holdings(campaign,actor)?.into_iter().take(40)
            .map(|(asset,quantity)|json!({"asset":asset,"quantity":quantity})).collect::<Vec<_>>();
        let contacts:Vec<Value>={
            let mut query=self.conn.prepare("SELECT id,name,role FROM known_npcs WHERE campaign_id=?1 ORDER BY name LIMIT 40")?;
            let rows=query.query_map(params![campaign],|r|Ok(json!({"id":r.get::<_,String>(0)?,
                "name":r.get::<_,String>(1)?,"role":r.get::<_,String>(2)?})))?;
            rows.collect::<rusqlite::Result<Vec<_>>>()?
        };
        let promises=self.promises(campaign,actor)?.into_iter().take(20)
            .map(|(recipient,description)|json!({"recipient":recipient,"description":description})).collect::<Vec<_>>();
        let facts:Vec<Value>=self.approved_facts(campaign,30)?.into_iter().map(|f|json!({"fact":f.fact})).collect();
        // La sélection contient toujours les dernières interactions et peut réintroduire
        // des scènes plus anciennes liées à l'action actuelle (dans la limite des 500 dernières).
        let all_events=self.events(campaign,500)?;
        // La dernière action du PJ actif reste accessible même après 6 tours
        // effectués par d'autres participants.
        let active_dialogue=all_events.iter().find(|e|e.kind=="action_resolved" &&
            e.request.get("actor_id").and_then(Value::as_str)==Some(actor))
            .map(|e|json!({"action":e.request.get("text"),
                            "narration":e.payload.get("narration"),"seq":e.seq}));
        let selected=select_relevant_events(all_events,action);
        let mut statuses=HashMap::<String,String>::new();
        {
            let mut stmt=self.conn.prepare("SELECT source_action_key,status FROM ai_fact_proposals WHERE campaign_id=?1")?;
            let rows=stmt.query_map(params![campaign],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?)))?;
            for row in rows { let (key,status)=row?;statuses.insert(key,status); }
        }
        let recent:Vec<Value>=selected.into_iter().map(|e|{
            let key=e.request.get("key").and_then(Value::as_str).unwrap_or("");
            json!({"seq":e.seq,"who":e.request.get("actor_id"),
                "action":e.request.get("text").and_then(Value::as_str).map(|s|clip_text(s,350)),
                "success":e.payload.get("success"),
                "narration_precedente":e.payload.get("narration").and_then(Value::as_str).map(|s|clip_text(s,450)),
                "fait_propose_statut":statuses.get(key)})
        }).collect();
        let max_chars=(target_tokens as usize).saturating_mul(3).clamp(1200,50000);
        // Campagne + personnages + faits sont prioritaires sur le récit ancien.
        let base=AiSnapshot{
            campaign:json!({"id":c.id,"title":c.title,"mode":c.mode.key(),"initial":clip_text(&c.seed_context,max_chars/3),
                "currencies":self.currency_list(campaign)?}),
            characters:json!(character_json),inventory:json!(inv),holdings:json!(holdings),contacts:json!(contacts),promises:json!(promises),facts:json!(facts),history:json!(recent),
            action:json!({"actor_id":actor,"text":action,
                "current_location":actor_place,"scene_key":scene_id,"previous_turn_by_this_player":active_dialogue,
                "other_players_same_known_place":colocated,
                "scene_rule":"Ces PJ sont dans la même scène canonique. Ne les ignore pas si le joueur demande qui est présent. Même lieu ne prouve pas visibilité si dissimulation explicite.",
                "location_of_other_players":"non communiquee si ailleurs; aucune distance precise inferee"}),
        };
        let mut result=base;
        // Budget approximatif en caractères : la vraie consommation varie selon tokenizer.
        while serde_json::to_string(&result)?.chars().count()>max_chars {
            let len=result.history.as_array().map_or(0,Vec::len);
            if len>0 {result.history.as_array_mut().unwrap().pop();continue;}
            let len=result.facts.as_array().map_or(0,Vec::len);
            if len>0 {result.facts.as_array_mut().unwrap().pop();continue;}
            let len=result.inventory.as_array().map_or(0,Vec::len);
            if len>0 {result.inventory.as_array_mut().unwrap().pop();continue;}
            // Les possessions sont des contraintes de validité, ne pas les supprimer
            // du contexte pour prétendre les avoir vérifiées.

            if let Some(initial)=result.campaign.get_mut("initial"){
                let text=initial.as_str().unwrap_or("");
                if text.chars().count()>200 { *initial=json!(clip_text(text,text.chars().count()/2));continue;}
            }
            // Fiches et action courante sont essentielles ; ne jamais les tronquer silencieusement.
            bail!("Budget de contexte trop faible pour l'état minimal. Augmentez-le dans Paramètres.");
        }
        Ok(result)
    }
    pub fn approved_facts(&self,campaign:&str,limit:usize)->Result<Vec<FactProposal>> {
        self.facts_by_status(campaign,"approved",limit)
    }
    pub fn pending_facts(&self,campaign:&str)->Result<Vec<FactProposal>> {
        self.facts_by_status(campaign,"pending",100)
    }
    fn facts_by_status(&self,campaign:&str,status:&str,limit:usize)->Result<Vec<FactProposal>> {
        let mut stmt=self.conn.prepare("SELECT id,fact_text,status,source_action_key FROM ai_fact_proposals WHERE campaign_id=?1 AND status=?2 ORDER BY created_at DESC,id DESC LIMIT ?3")?;
        let rows=stmt.query_map(params![campaign,status,limit.min(100) as i64],|r|Ok(FactProposal{id:r.get(0)?,fact:r.get(1)?,status:r.get(2)?,source_action_key:r.get(3)?}))?;
        let result=rows.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(result)
    }
    pub fn review_fact(&mut self,campaign:&str,id:&str,approve:bool)->Result<()> {
        let tx=self.conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        ensure_active(&tx,campaign)?;ensure_no_pending(&tx,campaign)?;
        let status=if approve{"approved"}else{"rejected"};
        let affected=tx.execute("UPDATE ai_fact_proposals SET status=?1, reviewed_at=?2 WHERE id=?3 AND campaign_id=?4 AND status='pending'",params![status,now(),id,campaign])?;
        if affected!=1{bail!("Proposition absente ou déjà examinée");}
        event(&tx,campaign,&Uuid::new_v4().to_string(),"fact_reviewed",&json!({"fact_id":id}),&json!({"status":status}))?;
        tx.commit()?;Ok(())
    }
    pub fn save_api_key(&mut self,secret:&str,passphrase:&str)->Result<()> {
        let (salt,nonce,ciphertext)=crate::secure::seal(secret,passphrase)?;
        self.conn.execute("INSERT INTO api_credentials(id,salt,nonce,ciphertext,created_at) VALUES(1,?1,?2,?3,?4) ON CONFLICT(id) DO UPDATE SET salt=excluded.salt,nonce=excluded.nonce,ciphertext=excluded.ciphertext,created_at=excluded.created_at",
            params![salt,nonce,ciphertext,now()])?; Ok(())
    }
    pub fn api_key_exists(&self)->Result<bool>{
        let exists:bool=self.conn.query_row("SELECT EXISTS(SELECT 1 FROM api_credentials WHERE id=1)",[],|r|r.get(0))?;
        Ok(exists)
    }
    pub fn unlock_api_key(&self,passphrase:&str)->Result<Option<String>> {
        let row:Option<(Vec<u8>,Vec<u8>,Vec<u8>)>=self.conn.query_row("SELECT salt,nonce,ciphertext FROM api_credentials WHERE id=1",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional()?;
        row.map(|(s,n,c)|crate::secure::unseal(&s,&n,&c,passphrase)).transpose()
    }
    pub fn clear_api_key(&mut self)->Result<()> {self.conn.execute("DELETE FROM api_credentials WHERE id=1",[])?;Ok(())}
}

#[cfg(test)] mod l2_tests {
    use super::*;
    use crate::{llm::AiProposal,uare::{Ability,Decision}};
    #[test]fn ai_atomic_and_facts()->Result<()> {
        let mut db=Storage::in_memory()?;
        let c=db.create_campaign("Ville",CampaignMode::Partial,"La porte est ouverte",1)?;
        let ch=db.characters(&c)?[0].clone();
        db.finalize_setup(&c)?;
        let snapshot=db.ai_snapshot(&c,&ch.id,"J'ouvre la porte",4096)?;
        assert!(snapshot.campaign.to_string().contains("porte"));
        let p=AiProposal{decision:Decision::AutoSuccess,ability:Ability::Wisdom,dc:10,proficient:false,
          roll_mode:RollMode::Normal,reason:"Libre".into(),success_text:"La porte s'ouvre".into(),
          failure_text:String::new(),fact_proposal:Some("La porte donne sur une cour".into()),stakes_public_success:None,stakes_public_failure:None,effect:None};
        let req=ActionRequest{key:"ai-unique".into(),campaign_id:c.clone(),actor_id:ch.id.clone(),text:"J'ouvre la porte".into(),
          decision:p.decision,ability:p.ability,proficient:p.proficient,dc:p.dc,roll_mode:p.roll_mode,
          target_id:None,hp_delta:0,success_text:p.success_text.clone(),failure_text:p.failure_text.clone()};
        assert!(db.resolve_ai_action(&req,&p,"test-model")?.success);
        assert!(db.resolve_ai_action(&req,&p,"test-model")?.success);
        assert_eq!(db.pending_facts(&c)?.len(),1);
        let pending_context=db.ai_snapshot(&c,&ch.id,"Que sait-on de cette porte ?",4096)?;
        assert_eq!(pending_context.history.as_array().unwrap()[0]["fait_propose_statut"],"pending");
        let id=db.pending_facts(&c)?[0].id.clone();
        db.review_fact(&c,&id,true)?;
        assert_eq!(db.approved_facts(&c,10)?.len(),1);
        assert_eq!(db.events(&c,10)?.iter().filter(|e|e.kind=="action_resolved").count(),1);
        Ok(())
    }
    #[test]fn storage_rejects_mundane_skill_check_without_any_roll()->Result<()> {
        let mut db=Storage::in_memory()?;
        let campaign=db.create_campaign("Vaisseau",CampaignMode::Free,"",1)?;
        let actor=db.characters(&campaign)?[0].id.clone();
        db.finalize_setup(&campaign)?;
        let action="Je demande au tavernier s'il a entendu parler de disparitions sur Mars";
        let p=AiProposal{decision:Decision::AbilityCheck,ability:Ability::Charisma,
            proficient:false,dc:15,roll_mode:RollMode::Normal,reason:"Renseignements".into(),
            success_text:"Le tavernier répond".into(),failure_text:"Le tavernier refuse".into(),
            fact_proposal:None,stakes_public_success:None,stakes_public_failure:None,effect:None};
        let req=ActionRequest{key:"inquiry-no-roll".into(),campaign_id:campaign.clone(),actor_id:actor,
            text:action.into(),decision:p.decision,ability:p.ability,proficient:p.proficient,
            dc:p.dc,roll_mode:p.roll_mode,target_id:None,hp_delta:0,
            success_text:p.success_text.clone(),failure_text:p.failure_text.clone()};
        assert!(db.resolve_ai_action(&req,&p,"mock").is_err());
        assert!(db.events(&campaign,10)?.iter().all(|e|e.kind!="action_resolved"));
        Ok(())
    }
    #[test]fn recurring_topic_recovers_old_dialogue()->Result<()> {
        let mut db=Storage::in_memory()?;
        let c=db.create_campaign("Vaisseau Mère",CampaignMode::Partial,"Une station spatiale et Mars",1)?;
        let actor=db.characters(&c)?[0].id.clone();
        db.finalize_setup(&c)?;
        for idx in 0..25 {
            let action=if idx==0{"Je parle au tavernier des disparitions sur Mars".to_string()}
                else {format!("Je prends le temps de regarder le couloir numéro {idx}")};
            let narration=if idx==0{"Le tavernier dit avoir entendu une rumeur à propos des docks martiens"}
                else {"Le couloir est calme"};
            let request=ActionRequest{key:format!("history-{idx}"),campaign_id:c.clone(),actor_id:actor.clone(),
                text:action,decision:Decision::AutoSuccess,ability:Ability::Wisdom,proficient:false,
                dc:10,roll_mode:RollMode::Normal,target_id:None,hp_delta:0,
                success_text:narration.into(),failure_text:String::new()};
            db.resolve_action(&request)?;
        }
        let context=db.ai_snapshot(&c,&actor,"Je retourne voir le tavernier pour parler de Mars et des disparitions",4096)?;
        let hist=context.history.as_array().unwrap();
        assert!(hist.iter().any(|x|x.get("action").and_then(Value::as_str).unwrap_or("").contains("disparitions sur Mars")));
        assert!(hist.len()<=12);
        Ok(())
    }
    #[test]fn encrypted_settings_roundtrip()->Result<()> {
        let mut db=Storage::in_memory()?;
        db.save_api_key("test_key","phrase master tres longue")?;
        assert!(db.api_key_exists()?);
        assert_eq!(db.unlock_api_key("phrase master tres longue")?,Some("test_key".into()));
        assert!(db.unlock_api_key("mauvaise phrase").is_err());
        db.clear_api_key()?;
        assert!(!db.api_key_exists()?);
        Ok(())
    }

    fn sample_pending(campaign:&str,actor:&str,key:&str)->(ActionRequest,AiProposal){
        let proposal=AiProposal{decision:Decision::AbilityCheck,ability:Ability::Charisma,
            proficient:false,dc:15,roll_mode:RollMode::Normal,
            reason:"Le témoin hésite entre aider et se protéger".into(),
            success_text:"Il révèle l'identité du mécanicien".into(),
            failure_text:"Il garde le nom secret, mais indique les docks sud".into(),
            fact_proposal:None,stakes_public_success:None,stakes_public_failure:None,effect:None};
        let request=ActionRequest{key:key.into(),campaign_id:campaign.into(),actor_id:actor.into(),
            text:"Je tente de persuader le tavernier de me donner le nom".into(),
            decision:proposal.decision,ability:proposal.ability,proficient:proposal.proficient,
            dc:proposal.dc,roll_mode:proposal.roll_mode,target_id:None,hp_delta:0,
            success_text:proposal.success_text.clone(),failure_text:proposal.failure_text.clone()};
        (request,proposal)
    }
    #[test] fn only_authorized_pending_roll_can_resolve()->Result<()> {
        let mut db=Storage::in_memory()?;
        let campaign=db.create_campaign("Docks",CampaignMode::Free,"",1)?;
        let ch=db.characters(&campaign)?[0].clone();
        db.finalize_setup(&campaign)?;
        let (request,proposal)=sample_pending(&campaign,&ch.id,"pending-one");
        assert!(db.finish_pending_roll(&campaign,&request.key).is_err());
        assert!(db.resolve_ai_action(&request,&proposal,"mock").is_err());
        db.queue_ai_roll(&request,&proposal,"mock")?;
        assert_eq!(db.events(&campaign,50)?.len(),2);
        assert_eq!(db.events(&campaign,50)?[0].kind,"action_pending_roll");
        assert!(db.queue_ai_roll(&request,&proposal,"mock").is_err());
        assert!(db.save_character(&campaign,&ch).is_err());
        let wrong=ActionRequest{key:"wrong-action".into(),..request.clone()};
        assert!(db.resolve_action(&wrong).is_err());
        assert!(db.finish_pending_roll(&campaign,"wrong-action").is_err());
        let out=db.finish_pending_roll(&campaign,&request.key)?;
        assert!(out.roll.is_some());
        assert_eq!(out.effective_dc,Some(15));
        assert_eq!(db.events(&campaign,10)?.len(),3);
        assert!(db.pending_ai_roll(&campaign)?.is_none());
        assert!(db.finish_pending_roll(&campaign,&request.key).is_err());
        assert_eq!(db.events(&campaign,10)?.len(),3);
        Ok(())
    }
    #[test] fn pending_event_contains_only_public_stakes()->Result<()> {
        let mut db=Storage::in_memory()?;
        let campaign=db.create_campaign("Enjeux publics",CampaignMode::Free,"",1)?;
        let actor=db.characters(&campaign)?[0].id.clone();
        db.finalize_setup(&campaign)?;
        let (req,mut proposal)=sample_pending(&campaign,&actor,"public-stakes");
        proposal.stakes_public_success=Some("Obtenir le nom du mécanicien".into());
        proposal.stakes_public_failure=Some("Le nom reste secret".into());
        db.queue_ai_roll(&req,&proposal,"mock")?;
        let e=db.events(&campaign,10)?.remove(0);
        assert_eq!(e.kind,"action_pending_roll");
        assert_eq!(e.payload["public_success"],"Obtenir le nom du mécanicien");
        assert_eq!(e.payload["public_failure"],"Le nom reste secret");
        assert!(e.payload.get("success_stake").is_none());
        assert!(e.payload.get("failure_stake").is_none());
        Ok(())
    }
    #[test] fn pending_persists_across_restart_and_cancellation()->Result<()> {
        let dir=std::env::temp_dir().join(format!("mj-l22-pending-{}",Uuid::new_v4()));
        let path=dir.join("game.sqlite3");
        let campaign;let key;
        {
            let mut db=Storage::open(&path)?;
            campaign=db.create_campaign("Mémoire",CampaignMode::Free,"",1)?;
            let actor=db.characters(&campaign)?[0].id.clone();
            db.finalize_setup(&campaign)?;
            let (req,proposal)=sample_pending(&campaign,&actor,"restart-pending");
            key=req.key.clone();
            db.queue_ai_roll(&req,&proposal,"mock")?;
        }
        {
            let mut db=Storage::open(&path)?;
            let persisted=db.pending_ai_roll(&campaign)?.expect("Le jet doit survivre au redémarrage");
            assert_eq!(persisted.request.key,key);
            db.cancel_pending_roll(&campaign,&key)?;
            assert!(db.pending_ai_roll(&campaign)?.is_none());
            assert!(db.finish_pending_roll(&campaign,&key).is_err());
            assert_eq!(db.events(&campaign,10)?.len(),3);
        }
        fs::remove_dir_all(dir)?;
        Ok(())
    }
    #[test] fn incompatible_schema_is_rejected_without_writing_database()->Result<()> {
        let dir=std::env::temp_dir().join(format!("mj-l2-final-old-{}",Uuid::new_v4()));
        fs::create_dir_all(&dir)?;
        let path=dir.join("legacy.sqlite3");
        {
            let conn=Connection::open(&path)?;
            conn.execute_batch("CREATE TABLE legacy_note(text TEXT NOT NULL); INSERT INTO legacy_note VALUES('à garder'); PRAGMA user_version=21;")?;
        }
        let bytes_before=fs::read(&path)?;
        assert!(Storage::open(&path).is_err());
        assert_eq!(fs::read(&path)?,bytes_before,"L'ouverture ne doit pas altérer la base incompatible");
        assert!(!dir.join("legacy.sqlite3-wal").exists());
        assert!(!dir.join("legacy.sqlite3-shm").exists());
        {
            let conn=Connection::open(&path)?;
            let content:String=conn.query_row("SELECT text FROM legacy_note",[],|r|r.get(0))?;
            assert_eq!(content,"à garder");
        }
        fs::remove_dir_all(dir)?;
        Ok(())
    }
    #[test] fn unversioned_nonempty_database_is_rejected_without_writing()->Result<()> {
        let dir=std::env::temp_dir().join(format!("mj-l2-final-unknown-{}",Uuid::new_v4()));
        fs::create_dir_all(&dir)?;
        let path=dir.join("unknown.sqlite3");
        {
            let conn=Connection::open(&path)?;
            conn.execute_batch("CREATE TABLE unrelated(id INTEGER PRIMARY KEY, name TEXT); INSERT INTO unrelated(name) VALUES('origine');")?;
        }
        let bytes_before=fs::read(&path)?;
        assert!(Storage::open(&path).is_err());
        assert_eq!(fs::read(&path)?,bytes_before);
        assert!(!dir.join("unknown.sqlite3-wal").exists());
        assert!(!dir.join("unknown.sqlite3-shm").exists());
        fs::remove_dir_all(dir)?;
        Ok(())
    }
    #[test] fn old_schema_is_rejected_without_migration()->Result<()> {
        let dir=std::env::temp_dir().join(format!("mj-l232-reject-old-{}",Uuid::new_v4()));
        let path=dir.join("old.sqlite3");
        let campaign;
        { let mut db=Storage::open(&path)?;
          campaign=db.create_campaign("Ancienne campagne",CampaignMode::Free,"",1)?; }
        { let conn=Connection::open(&path)?;
          conn.execute_batch("PRAGMA user_version=21;")?; }
        assert!(Storage::open(&path).is_err());
        { let conn=Connection::open(&path)?;
          assert_eq!(conn.query_row("PRAGMA user_version",[],|r|r.get::<_,i64>(0))?,21);
          let title:String=conn.query_row("SELECT title FROM campaigns WHERE id=?1",params![campaign],|r|r.get(0))?;
          assert_eq!(title,"Ancienne campagne"); }
        fs::remove_dir_all(dir)?;
        Ok(())
    }

}

// --- L2.3 : effets matériels génériques, sous autorité exclusive de Rust ---
// Le modèle n'a ni SQL, ni outil de création de richesse. Les seules sources initiales
// de possessions sont les éditions explicites de l'utilisateur dans Fiche et objets.

#[derive(Debug)]
enum AssetSource { Holding, LegacyInventory(String) }

/// Retourne uniquement un destinataire connu, en rejetant toute ambiguïté.
/// Une mention en narration ne suffit PAS à inventer un propriétaire ou des ressources.
fn known_recipient_owner(conn:&Connection,campaign:&str,recipient:&str)->Result<Option<String>> {
    let mut owners=Vec::new();
    let mut stmt=conn.prepare("SELECT ch.id FROM characters ch JOIN participants p ON p.id=ch.participant_id WHERE p.campaign_id=?1 AND lower(ch.name)=lower(?2)")?;
    for id in stmt.query_map(params![campaign,recipient.trim()],|row|row.get::<_,String>(0))? {
        owners.push(format!("character:{}",id?));
    }
    let mut stmt=conn.prepare("SELECT id FROM known_npcs WHERE campaign_id=?1 AND lower(name)=lower(?2)")?;
    for id in stmt.query_map(params![campaign,recipient.trim()],|row|row.get::<_,String>(0))? {
        owners.push(format!("npc:{}",id?));
    }
    match owners.len() {
        0=>Ok(None),
        1=>Ok(Some(owners.remove(0))),
        _=>bail!("Destinataire ambigu : plusieurs personnages portent le nom « {} »",recipient),
    }
}

/// Reconnaissance d'un PNJ déjà mentionné PAR LE MJ dans une narration durable.
/// La provenance est une observation de scène, pas une preuve de sa biographie.
/// Les noms sont compares comme mots complets pour eviter Jax/Jack/Jaxson.
fn name_occurs_as_words(text:&str,name:&str)->bool {
    let words=|value:&str| {
        let lower=value.to_lowercase();
        lower.split(|c:char|!c.is_alphanumeric())
            .filter(|word|!word.is_empty()).collect::<Vec<_>>().join(" ")
    };
    let needle=words(name);
    if needle.chars().count()<3 {return false;}
    format!(" {} ",words(text)).contains(&format!(" {needle} "))
}
fn npc_narrative_source(conn:&Connection,campaign:&str,recipient:&str)->Result<Option<i64>> {
    let recipient=recipient.trim();
    if recipient.chars().count()>100 || recipient.chars().count()<3 {return Ok(None);}
    let mut stmt=conn.prepare("SELECT seq,payload_json FROM game_events WHERE campaign_id=?1 AND kind='action_resolved' ORDER BY seq DESC LIMIT 500")?;
    let events=stmt.query_map(params![campaign],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?)))?;
    for record in events {
        let (seq,payload)=record?;
        let data:Value=serde_json::from_str(&payload)?;
        if data.get("narration").and_then(Value::as_str)
            .is_some_and(|narrative|name_occurs_as_words(narrative,recipient)) {
            return Ok(Some(seq));
        }
    }
    Ok(None)
}
fn verified_recipient_owner(conn:&Connection,campaign:&str,recipient:&str)->Result<Option<String>> {
    if let Some(owner)=known_recipient_owner(conn,campaign,recipient)? {return Ok(Some(owner));}
    if npc_narrative_source(conn,campaign,recipient)?.is_some(){return Ok(None);}
    bail!("Destinataire inconnu : « {} ». Aucun PNJ de ce nom dans les narrations enregistrées.",recipient)
}

fn find_asset_source(conn:&Connection,campaign:&str,actor:&str,asset:&str,quantity:u32)->Result<AssetSource> {
    let owner=format!("character:{actor}");
    let ledger:Option<i64>=conn.query_row(
        "SELECT quantity FROM holdings WHERE campaign_id=?1 AND owner_key=?2 AND asset=?3 COLLATE NOCASE",
        params![campaign,owner,asset.trim()],|r|r.get(0)).optional()?;
    let mut stmt=conn.prepare("SELECT inv.id,inv.quantity FROM inventory inv JOIN characters ch ON inv.character_id=ch.id JOIN participants p ON p.id=ch.participant_id WHERE p.campaign_id=?1 AND ch.id=?2 AND inv.name=?3 COLLATE NOCASE")?;
    let rows=stmt.query_map(params![campaign,actor,asset.trim()],|r|Ok((r.get::<_,String>(0)?,r.get::<_,i64>(1)?)))?;
    let existing=rows.collect::<rusqlite::Result<Vec<_>>>()?;
    if ledger.unwrap_or(0)>0 && !existing.is_empty() {
        bail!("Possession « {} » en double (ressources et inventaire). Regroupe-la avant de jouer.",asset);
    }
    if existing.len()>1 {bail!("Objet « {} » en plusieurs piles : regroupe-les avant le transfert",asset);}
    let amount=i64::from(quantity);
    if ledger.unwrap_or(0)>=amount {return Ok(AssetSource::Holding);}
    if let Some((id,stock))=existing.first() {
        if *stock>=amount{return Ok(AssetSource::LegacyInventory(id.clone()));}
    }
    let available=ledger.unwrap_or(0).max(existing.first().map_or(0,|(_,stock)|*stock));
    bail!("Possession insuffisante : « {} » × {} demandé, {} disponible. Aucun transfert réalisé.",asset,quantity,available)
}

fn validate_new_npc_names(conn:&Connection,campaign:&str,names:&[&str])->Result<()> {
    let mut uniques=HashSet::new();
    for name in names {
        let lower=name.trim().to_lowercase();
        if !uniques.insert(lower){bail!("PNJ proposés en double");}
        let exists:bool=conn.query_row("SELECT EXISTS(SELECT 1 FROM known_npcs WHERE campaign_id=?1 AND name=?2 COLLATE NOCASE)",params![campaign,name],|r|r.get(0))?;
        if exists{bail!("Le PNJ {name} existe déjà");}
        let player:bool=conn.query_row("SELECT EXISTS(SELECT 1 FROM characters c JOIN participants p ON c.participant_id=p.id WHERE p.campaign_id=?1 AND c.name=?2 COLLATE NOCASE)",params![campaign,name],|r|r.get(0))?;
        if player{bail!("Un PNJ ne peut pas réutiliser le nom d'un PJ : {name}");}
    }
    Ok(())
}

fn preflight_effect(conn:&Connection,campaign:&str,actor:&str,effect:&ProposedEffect)->Result<()> {
    effect.validate()?;
    match effect {
        ProposedEffect::Transfer{asset,quantity,recipient}=>{
            let destination=verified_recipient_owner(conn,campaign,recipient)?;
            if destination.is_some_and(|owner|owner==format!("character:{actor}")) {
                bail!("Un transfert vers soi-même n'a pas de sens");
            }
            find_asset_source(conn,campaign,actor,asset,*quantity)?;
        },
        ProposedEffect::Consume{asset,quantity}=>{
            find_asset_source(conn,campaign,actor,asset,*quantity)?;
        },
        ProposedEffect::Promise{..}=>{}, // parole enregistrée, PAS retrait d'argent
        ProposedEffect::Move{..}=>{}, // permission d'intention validée dans llm::validate_for_action
        ProposedEffect::IntroduceNpc{name,..}=>validate_new_npc_names(conn,campaign,&[name.as_str()])?,
        ProposedEffect::IntroduceNpcs{npcs}=>{
            let names=npcs.iter().map(|npc|npc.name.as_str()).collect::<Vec<_>>();
            validate_new_npc_names(conn,campaign,&names)?;
        },
    }
    Ok(())
}

fn deduct_asset(tx:&rusqlite::Transaction<'_>,campaign:&str,actor:&str,asset:&str,quantity:u32)->Result<()> {
    match find_asset_source(tx,campaign,actor,asset,quantity)? {
        AssetSource::Holding=>{
            let n=tx.execute("UPDATE holdings SET quantity=quantity-?1 WHERE campaign_id=?2 AND owner_key=?3 AND asset=?4 COLLATE NOCASE AND quantity>=?1",
                params![quantity,campaign,format!("character:{actor}"),asset.trim()])?;
            if n!=1{bail!("Le stock a changé : débit annulé");}
        },
        AssetSource::LegacyInventory(item_id)=>{
            // CHECK(quantity BETWEEN 1 AND 9999) interdit tout UPDATE intermediaire a zero.
            // Suppression directe si la pile est entierement consommee, sinon debit borne.
            let stock:i64=tx.query_row("SELECT quantity FROM inventory WHERE id=?1",
                params![item_id],|r|r.get(0))?;
            let debit=i64::from(quantity);
            if stock==debit {
                let n=tx.execute("DELETE FROM inventory WHERE id=?1 AND quantity=?2",
                    params![item_id,stock])?;
                if n!=1 {bail!("L'objet a changé : suppression annulée");}
            } else if stock>debit {
                let n=tx.execute("UPDATE inventory SET quantity=quantity-?1 WHERE id=?2 AND quantity>?1",
                    params![quantity,item_id])?;
                if n!=1 {bail!("L'objet a changé : débit annulé");}
            } else {bail!("Possession insuffisante : débit annulé");}
        }
    }
    Ok(())
}

fn apply_effect(tx:&rusqlite::Transaction<'_>,campaign:&str,actor:&str,action_key:&str,effect:&ProposedEffect)->Result<String> {
    preflight_effect(tx,campaign,actor,effect)?;
    match effect {
        ProposedEffect::Transfer{asset,quantity,recipient}=>{
            let destination=match verified_recipient_owner(tx,campaign,recipient)? {
                Some(owner)=>owner,
                None=>{
                    // Uniquement si ce PNJ a deja ete cite dans un recit commite.
                    // Meme transaction que le transfert : echec => pas de PNJ cree.
                    let source_seq=npc_narrative_source(tx,campaign,recipient)?
                        .ok_or_else(||anyhow!("Source narrative du destinataire absente"))?;
                    let npc_id=Uuid::new_v4().to_string();
                    tx.execute("INSERT INTO known_npcs(id,campaign_id,name) VALUES(?1,?2,?3)",
                        params![npc_id,campaign,recipient.trim()])?;
                    event(tx,campaign,&Uuid::new_v4().to_string(),"npc_registered",
                        &json!({"name":recipient.trim(),"source_seq":source_seq,"action_key":action_key}),
                        &json!({"source":"validated_prior_narration","npc_id":npc_id}))?;
                    format!("npc:{npc_id}")
                }
            };
            deduct_asset(tx,campaign,actor,asset,*quantity)?;
            let name=asset.trim().to_lowercase();
            tx.execute("INSERT INTO holdings(campaign_id,owner_key,asset,quantity) VALUES(?1,?2,?3,?4) ON CONFLICT(campaign_id,owner_key,asset) DO UPDATE SET quantity=quantity+excluded.quantity",
                params![campaign,destination,name,quantity])?;
            Ok(format!("Transfert validé : {} × {} vers {}",quantity,asset.trim(),recipient.trim()))
        },
        ProposedEffect::Consume{asset,quantity}=>{
            deduct_asset(tx,campaign,actor,asset,*quantity)?;
            Ok(format!("Consommation validée : {} × {}",quantity,asset.trim()))
        },
        ProposedEffect::Promise{recipient,description}=>{
            tx.execute("INSERT INTO promises(id,campaign_id,actor_id,recipient,description,source_action_key,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7)",
                params![Uuid::new_v4().to_string(),campaign,actor,recipient.trim(),description.trim(),action_key,now()])?;
            Ok(format!("Promesse déclarée à {} : {} (pas de dépense)",recipient.trim(),description.trim()))
        },
        ProposedEffect::IntroduceNpc{name,role}=>{
            let id=Uuid::new_v4().to_string();
            tx.execute("INSERT INTO known_npcs(id,campaign_id,name,role) VALUES(?1,?2,?3,?4)",params![id,campaign,name.trim(),role.trim()])?;
            event(tx,campaign,&Uuid::new_v4().to_string(),"npc_introduced",
                &json!({"actor_id":actor,"action_key":action_key}),
                &json!({"npc_id":id,"name":name.trim(),"role":role.trim(),"source":"proposal_validated_by_rust"}))?;
            Ok(format!("PNJ introduit : {} ({})",name.trim(),role.trim()))
        },
        ProposedEffect::IntroduceNpcs{npcs}=>{
            for npc in npcs {
                let id=Uuid::new_v4().to_string();
                tx.execute("INSERT INTO known_npcs(id,campaign_id,name,role) VALUES(?1,?2,?3,?4)",
                    params![id,campaign,npc.name.trim(),npc.role.trim()])?;
                event(tx,campaign,&Uuid::new_v4().to_string(),"npc_introduced",
                    &json!({"actor_id":actor,"action_key":action_key}),
                    &json!({"npc_id":id,"name":npc.name.trim(),"role":npc.role.trim(),"source":"proposal_validated_by_rust"}))?;
            }
            Ok(format!("{} PNJ introduits et enregistrés",npcs.len()))
        },
        ProposedEffect::Move{destination}=>{
            store_actor_scene(tx,campaign,actor,destination,action_key)?;
            event(tx,campaign,&Uuid::new_v4().to_string(),"actor_moved",
                &json!({"actor_id":actor,"action_key":action_key}),
                &json!({"destination":destination.trim(),"place_key":scene_key(destination),"source_action_key":action_key}))?;
            Ok(format!("Déplacement validé : {}",destination.trim()))
        }
    }
}

impl Storage {
    /// Deuxième validation indépendante du fournisseur : le récit affiché ne
    /// peut pas faire agir un autre PJ, même si la proposition est injectée sans LLM.
    fn validate_scene_narration(&self,request:&ActionRequest,
                                proposal:&crate::llm::AiProposal)->Result<()> {
        // Une action déjà validée doit rester idempotente même si les autres PJ
        // ont déménagé depuis ; resolve_action_inner vérifiera toujours que
        // la même clé désigne exactement la même requête et la même proposition.
        let committed:bool=self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM game_events WHERE campaign_id=?1 AND request_key=?2 AND kind='action_resolved')",
            params![request.campaign_id,request.key],|r|r.get(0))?;
        if committed {return Ok(());}
        // Ne relit pas 500 événements pour cette barrière de sécurité :
        // les seules données utiles sont la position officielle et les noms des PJ.
        let chars=self.characters(&request.campaign_id)?;
        let place=self.actor_location(&request.campaign_id,&request.actor_id)?;
        let key=place.as_deref().and_then(scene_key);
        let colocated=if let Some(ref key)=key {
            chars.iter().filter(|ch|ch.id!=request.actor_id)
                .filter(|ch|self.actor_location(&request.campaign_id,&ch.id).ok().flatten()
                    .as_deref().and_then(scene_key).as_ref()==Some(key))
                .map(|ch|ch.name.clone()).collect::<Vec<_>>()
        }else{Vec::new()};
        let identities=chars.iter().map(|ch|json!({"id":ch.id,"name":ch.name}))
            .collect::<Vec<_>>();
        let snapshot=crate::llm::AiSnapshot{
            campaign:json!({}),characters:json!(identities),inventory:json!([]),
            holdings:json!([]),contacts:json!([]),promises:json!([]),facts:json!([]),
            history:json!([]),action:json!({"actor_id":request.actor_id,
                "text":request.text,"current_location":place,
                "other_players_same_known_place":colocated})};
        if let Some(issue)=crate::llm::scene_response_issue(&snapshot,proposal){bail!("Narration multijoueur refusée : {issue}");}
        Ok(())
    }
    fn validate_effect_preflight(&self,request:&ActionRequest,proposal:&crate::llm::AiProposal)->Result<()> {
        if let Some(effect)=proposal.effect.as_ref() {
            preflight_effect(&self.conn,&request.campaign_id,&request.actor_id,effect)?;
        }
        Ok(())
    }
    /// Enregistrement explicite d'un PNJ connu, par l'utilisateur, non par le LLM.
    pub fn register_npc(&mut self,campaign:&str,name:&str)->Result<()> {
        let name=name.trim();
        if name.is_empty()||name.chars().count()>100 {bail!("Nom de contact invalide");}
        let tx=self.conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        ensure_active(&tx,campaign)?;ensure_no_pending(&tx,campaign)?;ensure_setup_open(&tx,campaign)?;
        let used:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM characters c JOIN participants p ON p.id=c.participant_id WHERE p.campaign_id=?1 AND lower(c.name)=lower(?2))",params![campaign,name],|r|r.get(0))?;
        if used {bail!("Un personnage joueur possède déjà ce nom");}
        let existing:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM known_npcs WHERE campaign_id=?1 AND lower(name)=lower(?2))",params![campaign,name],|r|r.get(0))?;
        if existing {bail!("Ce PNJ est déjà enregistré");}
        tx.execute("INSERT INTO known_npcs(id,campaign_id,name) VALUES(?1,?2,?3)",params![Uuid::new_v4().to_string(),campaign,name])?;
        event(&tx,campaign,&Uuid::new_v4().to_string(),"npc_registered",&json!({"name":name}),&json!({"source":"user"}))?;
        tx.commit()?;Ok(())
    }
    pub fn known_npcs(&self,campaign:&str)->Result<Vec<(String,String)>> {
        let mut stmt=self.conn.prepare("SELECT id,name FROM known_npcs WHERE campaign_id=?1 ORDER BY name")?;
        let rows=stmt.query_map(params![campaign],|r|Ok((r.get(0)?,r.get(1)?)))?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }
    /// Dotation explicite de préparation du personnage. Aucun gain n'est inféré par le LLM.
    pub fn grant_holding_by_user(&mut self,campaign:&str,actor:&str,name:&str,quantity:u32)->Result<()> {
        let name=name.trim().to_lowercase();
        if name.is_empty()||name.chars().count()>120||!(1..=9999).contains(&quantity){bail!("Ressource invalide");}
        let tx=self.conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        ensure_active(&tx,campaign)?;ensure_no_pending(&tx,campaign)?;ensure_setup_open(&tx,campaign)?;
        let known:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM characters c JOIN participants p ON p.id=c.participant_id WHERE p.campaign_id=?1 AND c.id=?2)",params![campaign,actor],|r|r.get(0))?;
        if !known{bail!("Personnage inconnu dans cette campagne");}
        tx.execute("INSERT INTO holdings(campaign_id,owner_key,asset,quantity) VALUES(?1,?2,?3,?4) ON CONFLICT(campaign_id,owner_key,asset) DO UPDATE SET quantity=quantity+excluded.quantity",params![campaign,format!("character:{actor}"),name,quantity])?;
        event(&tx,campaign,&Uuid::new_v4().to_string(),"holding_user_granted",&json!({"actor":actor,"asset":name,"quantity":quantity}),&json!({"source":"user"}))?;
        tx.commit()?;Ok(())
    }
    pub fn holdings(&self,campaign:&str,actor:&str)->Result<Vec<(String,i64)>> {
        let owner=format!("character:{actor}");
        let mut stmt=self.conn.prepare("SELECT asset,quantity FROM holdings WHERE campaign_id=?1 AND owner_key=?2 AND quantity>0 ORDER BY asset")?;
        let rows=stmt.query_map(params![campaign,owner],|r|Ok((r.get(0)?,r.get(1)?)))?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }
    pub fn npc_holdings(&self,campaign:&str,npc_id:&str)->Result<Vec<(String,i64)>> {
        let owner=format!("npc:{npc_id}");
        let mut stmt=self.conn.prepare("SELECT asset,quantity FROM holdings WHERE campaign_id=?1 AND owner_key=?2 AND quantity>0 ORDER BY asset")?;
        let rows=stmt.query_map(params![campaign,owner],|r|Ok((r.get(0)?,r.get(1)?)))?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }
    /// Lieu canonique le plus recent issu d'un mouvement effectivement valide.
    /// Les anciennes narrations sans `actor_moved` ne sont pas retroactivement des lieux.
    pub fn actor_location(&self,campaign:&str,actor:&str)->Result<Option<String>> {
        let saved:Option<String>=self.conn.query_row(
            "SELECT label FROM actor_scenes WHERE campaign_id=?1 AND actor_id=?2",
            params![campaign,actor],|r|r.get(0)).optional()?;
        if saved.is_some(){return Ok(saved);}
        let location=self.conn.query_row(
            "SELECT json_extract(payload_json,'$.destination') FROM game_events \
             WHERE campaign_id=?1 AND kind='actor_moved' \
             AND json_extract(request_json,'$.actor_id')=?2 ORDER BY seq DESC LIMIT 1",
            params![campaign,actor],|r|r.get(0)).optional()?;
        if location.is_some() {return Ok(location);}
        Ok(self.character_profile(actor)?.map(|p|p.initial_location))
    }
    pub fn promises(&self,campaign:&str,actor:&str)->Result<Vec<(String,String)>> {
        let mut stmt=self.conn.prepare("SELECT recipient,description FROM promises WHERE campaign_id=?1 AND actor_id=?2 ORDER BY created_at DESC,id DESC LIMIT 40")?;
        let rows=stmt.query_map(params![campaign,actor],|r|Ok((r.get(0)?,r.get(1)?)))?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }
}

#[cfg(test)]
mod l23_tests {
    use super::*;
    use crate::llm::{AiProposal,ProposedEffect};
    use crate::uare::{Ability,Decision};
    use crate::dice::RollMode;

    pub(super) fn auto(campaign:&str,actor:&str,key:&str,text:&str,effect:Option<ProposedEffect>)->(ActionRequest,AiProposal) {
        let p=AiProposal{decision:Decision::AutoSuccess,ability:Ability::Charisma,
            proficient:false,dc:10,roll_mode:RollMode::Normal,reason:"Action vérifiée".into(),
            success_text:"Action accomplie après validation Rust.".into(),
            failure_text:"Action impossible".into(),fact_proposal:None,
            stakes_public_success:None,stakes_public_failure:None,effect};
        let r=ActionRequest{key:key.into(),campaign_id:campaign.into(),actor_id:actor.into(),text:text.into(),
            decision:p.decision,ability:p.ability,proficient:p.proficient,dc:p.dc,
            roll_mode:p.roll_mode,target_id:None,hp_delta:0,
            success_text:p.success_text.clone(),failure_text:p.failure_text.clone()};
        (r,p)
    }
    #[test] fn invalid_transfers_do_not_create_money_or_events()->Result<()> {
        let mut db=Storage::in_memory()?;
        let c=db.create_campaign("Tharsis",CampaignMode::Partial,"Station Atlas",1)?;
        let actor=db.characters(&c)?[0].id.clone();
        db.register_npc(&c,"Mara")?;
        db.grant_holding_by_user(&c,&actor,"crédits",20)?;
        db.finalize_setup(&c)?;
        let previous=db.events(&c,100)?.len();
        let (r,p)=auto(&c,&actor,"give-too-much","Je donne 500 crédits à Mara",
            Some(ProposedEffect::Transfer{asset:"crédits".into(),quantity:500,recipient:"Mara".into()}));
        assert!(db.resolve_ai_action(&r,&p,"mock").is_err());
        assert_eq!(db.holdings(&c,&actor)?[0].1,20);
        assert_eq!(db.events(&c,100)?.len(),previous);
        assert!(db.npc_holdings(&c,&db.known_npcs(&c)?[0].0)?.is_empty());
        Ok(())
    }
    #[test] fn valid_transfer_is_atomic_and_idempotent()->Result<()> {
        let mut db=Storage::in_memory()?;
        let c=db.create_campaign("Campagne",CampaignMode::Free,"",1)?;
        let actor=db.characters(&c)?[0].id.clone();
        db.register_npc(&c,"Mara")?;
        db.grant_holding_by_user(&c,&actor,"crédits",20)?;
        db.finalize_setup(&c)?;
        let (r,p)=auto(&c,&actor,"pay-five","Je donne 5 crédits à Mara",
            Some(ProposedEffect::Transfer{asset:"crédits".into(),quantity:5,recipient:"Mara".into()}));
        let first=db.resolve_ai_action(&r,&p,"mock")?;
        assert!(first.applied_effect.as_deref().unwrap_or("").contains("Transfert"));
        let second=db.resolve_ai_action(&r,&p,"mock")?;
        assert_eq!(first.applied_effect,second.applied_effect);
        assert_eq!(db.holdings(&c,&actor)?[0].1,15);
        let mara=db.known_npcs(&c)?[0].0.clone();
        assert_eq!(db.npc_holdings(&c,&mara)?[0].1,5);
        assert_eq!(db.events(&c,100)?.iter().filter(|ev|ev.kind=="action_resolved").count(),1);
        Ok(())
    }
    #[test] fn repeated_full_balance_spend_is_idempotent()->Result<()> {
        let mut db=Storage::in_memory()?;
        let c=db.create_campaign("Wallet",CampaignMode::Free,"",1)?;
        let actor=db.characters(&c)?[0].id.clone();
        db.register_npc(&c,"Mara")?;
        db.grant_holding_by_user(&c,&actor,"crédits",7)?;
        db.finalize_setup(&c)?;
        let (r,p)=auto(&c,&actor,"everything","Je paie 7 crédits à Mara",
            Some(ProposedEffect::Transfer{asset:"crédits".into(),quantity:7,recipient:"Mara".into()}));
        db.resolve_ai_action(&r,&p,"mock")?;
        assert!(db.holdings(&c,&actor)?.is_empty());
        db.resolve_ai_action(&r,&p,"mock")?;
        let mara=db.known_npcs(&c)?[0].0.clone();
        assert_eq!(db.npc_holdings(&c,&mara)?[0].1,7);
        let mut altered=p.clone();
        altered.effect=Some(ProposedEffect::Transfer{asset:"crédits".into(),quantity:6,recipient:"Mara".into()});
        assert!(db.resolve_ai_action(&r,&altered,"mock").is_err());
        Ok(())
    }
    #[test] fn missing_contact_or_structured_effect_is_rejected()->Result<()> {
        let mut db=Storage::in_memory()?;
        let c=db.create_campaign("Test",CampaignMode::Free,"",1)?;
        let actor=db.characters(&c)?[0].id.clone();
        db.grant_holding_by_user(&c,&actor,"eau",2)?;
        db.finalize_setup(&c)?;
        let (r,p)=auto(&c,&actor,"missing-npc","Je donne 1 eau à Mara",
            Some(ProposedEffect::Transfer{asset:"eau".into(),quantity:1,recipient:"Mara".into()}));
        assert!(db.resolve_ai_action(&r,&p,"mock").is_err());
        let (r,p)=auto(&c,&actor,"missing-effect","Je donne 1 eau à Mara",None);
        assert!(db.resolve_ai_action(&r,&p,"mock").is_err());
        assert_eq!(db.holdings(&c,&actor)?[0].1,2);
        Ok(())
    }
    #[test] fn promise_is_distinct_from_payment_and_persists()->Result<()> {
        let dir=std::env::temp_dir().join(format!("mj-l23-promises-{}",Uuid::new_v4()));
        let path=dir.join("db.sqlite3");
        let c;let actor;
        {
            let mut db=Storage::open(&path)?;
            c=db.create_campaign("Promesses",CampaignMode::Free,"",1)?;
            actor=db.characters(&c)?[0].id.clone();
            db.grant_holding_by_user(&c,&actor,"crédits",10)?;
            db.finalize_setup(&c)?;
            let (r,p)=auto(&c,&actor,"promise-one","Je promets à Mara 500 crédits demain",
                Some(ProposedEffect::Promise{recipient:"Mara".into(),description:"payer 500 crédits demain".into()}));
            db.resolve_ai_action(&r,&p,"mock")?;
        }
        {
            let db=Storage::open(&path)?;
            assert_eq!(db.schema_version()?,27);
            assert_eq!(db.holdings(&c,&actor)?[0].1,10);
            assert_eq!(db.promises(&c,&actor)?.len(),1);
            let snap=db.ai_snapshot(&c,&actor,"Que dois-je à Mara ?",4096)?;
            assert!(snap.promises.to_string().contains("500 crédits"));
        }
        fs::remove_dir_all(dir)?;
        Ok(())
    }
    #[test] fn legacy_inventory_can_be_consumed_without_a_free_gain()->Result<()> {
        let mut db=Storage::in_memory()?;
        let c=db.create_campaign("Objets",CampaignMode::Free,"",1)?;
        let actor=db.characters(&c)?[0].id.clone();
        db.add_item(&c,&actor,"Bière",2,"")?;
        db.finalize_setup(&c)?;
        let (r,p)=auto(&c,&actor,"drink","Je bois une bière que je possède",
            Some(ProposedEffect::Consume{asset:"bière".into(),quantity:1}));
        db.resolve_ai_action(&r,&p,"mock")?;
        assert_eq!(db.inventory(&actor)?[0].quantity,1);
        Ok(())
    }

    #[test] fn final_inventory_unit_can_be_consumed_then_second_attempt_fails()->Result<()> {
        let mut db=Storage::in_memory()?;
        let c=db.create_campaign("Dernière eau",CampaignMode::Free,"",1)?;
        let actor=db.characters(&c)?[0].id.clone();
        db.add_item(&c,&actor,"eau",1,"")?;
        db.finalize_setup(&c)?;
        let (r,p)=auto(&c,&actor,"last-water","Je bois une eau que je possède",
            Some(ProposedEffect::Consume{asset:"eau".into(),quantity:1}));
        let first=db.resolve_ai_action(&r,&p,"mock")?;
        assert!(first.applied_effect.is_some());
        assert!(db.inventory(&actor)?.is_empty());
        assert_eq!(first.applied_effect,db.resolve_ai_action(&r,&p,"mock")?.applied_effect);
        let count=db.events(&c,500)?.iter().filter(|e|e.kind=="action_resolved").count();
        let (other_r,other_p)=auto(&c,&actor,"second-water","Je bois une eau que je possède",
            Some(ProposedEffect::Consume{asset:"eau".into(),quantity:1}));
        assert!(db.resolve_ai_action(&other_r,&other_p,"mock").is_err());
        assert_eq!(db.events(&c,500)?.iter().filter(|e|e.kind=="action_resolved").count(),count);
        Ok(())
    }
    #[test] fn entire_inventory_stack_can_be_transferred_transactionally()->Result<()> {
        let mut db=Storage::in_memory()?;
        let c=db.create_campaign("Rations",CampaignMode::Free,"",1)?;
        let actor=db.characters(&c)?[0].id.clone();
        db.add_item(&c,&actor,"ration",1,"")?;
        db.register_npc(&c,"Jax")?;
        db.finalize_setup(&c)?;
        let (r,p)=auto(&c,&actor,"ration-jax","Je donne une ration à Jax",
            Some(ProposedEffect::Transfer{asset:"ration".into(),quantity:1,recipient:"Jax".into()}));
        db.resolve_ai_action(&r,&p,"mock")?;
        assert!(db.inventory(&actor)?.is_empty());
        let npcs=db.known_npcs(&c)?;
        let id=&npcs[0].0;
        assert_eq!(db.npc_holdings(&c,id)?[0].1,1);
        db.resolve_ai_action(&r,&p,"mock")?;
        assert_eq!(db.npc_holdings(&c,id)?[0].1,1);
        Ok(())
    }
    #[test] fn named_npc_from_prior_narration_can_receive_gift_without_manual_edit()->Result<()> {
        let mut db=Storage::in_memory()?;
        let c=db.create_campaign("Contacts vivants",CampaignMode::Free,"",1)?;
        let actor=db.characters(&c)?[0].id.clone();
        db.add_item(&c,&actor,"ration",2,"")?;
        db.finalize_setup(&c)?;
        let (dialogue,p)=auto(&c,&actor,"npc-intro","Je parcours les docks",None);
        let mut intro=p.clone();
        intro.success_text="Un mécanicien nommé Jax te fait signe près du hangar.".into();
        let intro_req=ActionRequest{success_text:intro.success_text.clone(),..dialogue};
        db.resolve_ai_action(&intro_req,&intro,"mock")?;
        assert!(db.known_npcs(&c)?.is_empty());
        let (r,p)=auto(&c,&actor,"gift-jax","Je donne une ration à Jax",
            Some(ProposedEffect::Transfer{asset:"ration".into(),quantity:1,recipient:"Jax".into()}));
        db.resolve_ai_action(&r,&p,"mock")?;
        assert_eq!(db.inventory(&actor)?[0].quantity,1);
        let npcs=db.known_npcs(&c)?;
        assert_eq!(npcs.len(),1);
        assert_eq!(npcs[0].1,"Jax");
        assert_eq!(db.npc_holdings(&c,&npcs[0].0)?[0].1,1);
        assert_eq!(db.events(&c,100)?.iter().filter(|e|e.kind=="npc_registered").count(),1);
        db.resolve_ai_action(&r,&p,"mock")?;
        assert_eq!(db.npc_holdings(&c,&npcs[0].0)?[0].1,1);
        assert_eq!(db.events(&c,100)?.iter().filter(|e|e.kind=="npc_registered").count(),1);
        assert!(db.register_npc(&c,"Autre PNJ").is_err());
        Ok(())
    }
    #[test] fn npc_mention_must_be_in_prior_committed_narration()->Result<()> {
        let mut db=Storage::in_memory()?;
        let c=db.create_campaign("Sans contact",CampaignMode::Free,"",1)?;
        let actor=db.characters(&c)?[0].id.clone();
        db.add_item(&c,&actor,"ration",1,"")?;
        db.finalize_setup(&c)?;
        let (r,p)=auto(&c,&actor,"fictional-jax","Je donne une ration à Jax",
            Some(ProposedEffect::Transfer{asset:"ration".into(),quantity:1,recipient:"Jax".into()}));
        assert!(db.resolve_ai_action(&r,&p,"mock").is_err());
        assert!(db.known_npcs(&c)?.is_empty());
        assert_eq!(db.inventory(&actor)?[0].quantity,1);
        Ok(())
    }
    #[test] fn explicit_movement_is_canonical_after_commit_and_survives_restart()->Result<()> {
        let dir=std::env::temp_dir().join(format!("mj-l233-location-{}",Uuid::new_v4()));
        let path=dir.join("game.sqlite3");
        let c;let actor;
        {
            let mut db=Storage::open(&path)?;
            c=db.create_campaign("Explorer",CampaignMode::Free,"",1)?;
            actor=db.characters(&c)?[0].id.clone();
            db.finalize_setup(&c)?;
            assert!(db.actor_location(&c,&actor)?.is_none());
            let (r,p)=auto(&c,&actor,"move-to-docks","Je quitte la taverne et je me dirige vers les docks sud",
                Some(ProposedEffect::Move{destination:"docks sud".into()}));
            db.resolve_ai_action(&r,&p,"mock")?;
            assert_eq!(db.actor_location(&c,&actor)?.as_deref(),Some("docks sud"));
            db.resolve_ai_action(&r,&p,"mock")?;
            assert_eq!(db.events(&c,100)?.iter().filter(|e|e.kind=="actor_moved").count(),1);
            let (r,p)=auto(&c,&actor,"bad-move","Je me dirige vers les docks sud",
                Some(ProposedEffect::Move{destination:"salle secrète".into()}));
            assert!(db.resolve_ai_action(&r,&p,"mock").is_err());
            assert_eq!(db.actor_location(&c,&actor)?.as_deref(),Some("docks sud"));
        }
        {
            let db=Storage::open(&path)?;
            assert_eq!(db.actor_location(&c,&actor)?.as_deref(),Some("docks sud"));
            assert_eq!(db.ai_snapshot(&c,&actor,"Où suis-je ?",4096)?.action["current_location"],"docks sud");
        }
        fs::remove_dir_all(dir)?;
        Ok(())
    }
    #[test] fn pending_effect_is_checked_before_roll_and_failure_keeps_stock()->Result<()> {
        let mut db=Storage::in_memory()?;
        let c=db.create_campaign("Jets",CampaignMode::Free,"",1)?;
        let actor=db.characters(&c)?[0].id.clone();
        db.register_npc(&c,"Mara")?;
        db.grant_holding_by_user(&c,&actor,"crédits",5)?;
        db.finalize_setup(&c)?;
        let mut p=auto(&c,&actor,"roll","Je tente de convaincre Mara de prendre les 5 crédits que je lui donne",
            Some(ProposedEffect::Transfer{asset:"crédits".into(),quantity:6,recipient:"Mara".into()})).1;
        p.decision=Decision::AbilityCheck;
        p.dc=30; // même avec +0, le jet ne peut réussir
        let r=ActionRequest{key:"roll".into(),campaign_id:c.clone(),actor_id:actor.clone(),
            text:"Je tente de convaincre Mara de prendre les 5 crédits que je lui donne".into(),
            decision:p.decision,ability:p.ability,proficient:p.proficient,dc:p.dc,
            roll_mode:p.roll_mode,target_id:None,hp_delta:0,
            success_text:p.success_text.clone(),failure_text:p.failure_text.clone()};
        assert!(db.queue_ai_roll(&r,&p,"mock").is_err());
        assert!(db.pending_ai_roll(&c)?.is_none());
        p.effect=Some(ProposedEffect::Transfer{asset:"crédits".into(),quantity:5,recipient:"Mara".into()});
        db.queue_ai_roll(&r,&p,"mock")?;
        let out=db.finish_pending_roll(&c,&r.key)?;
        assert!(!out.success);
        assert!(out.applied_effect.is_none());
        assert_eq!(db.holdings(&c,&actor)?[0].1,5);
        Ok(())
    }
    #[test] fn fresh_schema_reopens_with_pending_roll()->Result<()> {
        let dir=std::env::temp_dir().join(format!("mj-l232-fresh-pending-{}",Uuid::new_v4()));
        let path=dir.join("game.sqlite3");
        let campaign;
        { let mut db=Storage::open(&path)?;
          campaign=db.create_campaign("Sauvegarde",CampaignMode::Free,"",1)?;
          let actor=db.characters(&campaign)?[0].id.clone();
          db.finalize_setup(&campaign)?;
          let (r,p)=auto(&campaign,&actor,"pending","Je tente de convaincre Mara",None);
          let mut p=p;p.decision=Decision::AbilityCheck;
          let r=ActionRequest{decision:Decision::AbilityCheck,..r};
          db.queue_ai_roll(&r,&p,"mock")?;
        }
        { let mut db=Storage::open(&path)?;
          assert_eq!(db.schema_version()?,27);
          assert!(db.setup_complete(&campaign)?);
          assert!(db.pending_ai_roll(&campaign)?.is_some());
          db.cancel_pending_roll(&campaign,"pending")?;
          assert!(db.pending_ai_roll(&campaign)?.is_none());
        }
        fs::remove_dir_all(dir)?;
        Ok(())
    }

}

#[cfg(test)]
mod setup_l232_tests {
    use super::*;
    use crate::{llm::{AiProposal,ProposedEffect},dice::RollMode,uare::{Ability,Decision}};

    #[test]
    fn setup_lock_covers_all_direct_edit_apis_and_allows_validated_effects()->Result<()> {
        let mut db=Storage::in_memory()?;
        let c=db.create_campaign("Vaisseau",CampaignMode::Free,"",1)?;
        let mut ch=db.characters(&c)?[0].clone();
        assert!(!db.setup_complete(&c)?);
        ch.name="Kael".into();db.save_character(&c,&ch)?;
        db.add_item(&c,&ch.id,"Lampe",1,"")?;
        db.grant_holding_by_user(&c,&ch.id,"crédits",20)?;
        db.register_npc(&c,"Mara")?;
        assert!(db.ai_snapshot(&c,&ch.id,"Je parle",4096).is_err());
        db.finalize_setup(&c)?;
        assert!(db.setup_complete(&c)?);
        assert!(db.finalize_setup(&c).is_err());
        ch.name="Triche".into();
        assert!(db.save_character(&c,&ch).is_err());
        assert!(db.add_item(&c,&ch.id,"Épée",1,"").is_err());
        let id=db.inventory(&ch.id)?[0].id.clone();
        assert!(db.delete_item(&c,&id).is_err());
        assert!(db.grant_holding_by_user(&c,&ch.id,"crédits",999).is_err());
        assert!(db.register_npc(&c,"Jax").is_err());
        let version=db.get_campaign(&c)?.unwrap().state_version;
        assert!(db.save_setup(&c,version,"Triche",CampaignMode::Free,"",&["X".into()]).is_err());
        let p=AiProposal{decision:Decision::AutoSuccess,ability:Ability::Charisma,
            proficient:false,dc:10,roll_mode:RollMode::Normal,reason:"Don vérifiable".into(),
            success_text:"Mara reçoit cinq crédits".into(),failure_text:"".into(),
            fact_proposal:None,stakes_public_success:None,stakes_public_failure:None,
            effect:Some(ProposedEffect::Transfer{asset:"crédits".into(),quantity:5,recipient:"Mara".into()})};
        let req=ActionRequest{key:"paid".into(),campaign_id:c.clone(),actor_id:ch.id.clone(),
            text:"Je donne cinq crédits à Mara".into(),decision:p.decision,ability:p.ability,
            proficient:p.proficient,dc:p.dc,roll_mode:p.roll_mode,target_id:None,hp_delta:0,
            success_text:p.success_text.clone(),failure_text:p.failure_text.clone()};
        db.resolve_ai_action(&req,&p,"mock")?;
        assert_eq!(db.holdings(&c,&ch.id)?[0].1,15);
        assert!(db.save_character(&c,&ch).is_err());
        Ok(())
    }
    #[test]
    fn initial_character_hp_has_clear_validation_and_accepts_twelve_of_twelve()->Result<()> {
        let mut db=Storage::in_memory()?;
        let camp=db.create_campaign("Fiche",CampaignMode::Free,"",1)?;
        let mut ch=db.characters(&camp)?[0].clone();
        ch.hp=12; // Maximum initial par défaut : 10
        let err=db.save_character(&camp,&ch).unwrap_err().to_string();
        assert!(err.contains("PV actuels 12 supérieurs aux PV maximum 10"),"{err}");
        ch.max_hp=12;
        db.save_character(&camp,&ch)?;
        assert_eq!(db.characters(&camp)?[0].hp,12);
        assert_eq!(db.characters(&camp)?[0].max_hp,12);
        Ok(())
    }
    #[test]
    fn setup_state_survives_reopening_fresh_database()->Result<()> {
        let dir=std::env::temp_dir().join(format!("mj-l232-fresh-lock-{}",Uuid::new_v4()));
        let path=dir.join("db.sqlite3");
        let (started,unstarted);
        { let mut db=Storage::open(&path)?;
          started=db.create_campaign("Déjà commencée",CampaignMode::Free,"",1)?;
          unstarted=db.create_campaign("Encore en préparation",CampaignMode::Free,"",1)?;
          db.finalize_setup(&started)?;
        }
        { let db=Storage::open(&path)?;
          assert_eq!(db.schema_version()?,27);
          assert!(db.setup_complete(&started)?);
          assert!(!db.setup_complete(&unstarted)?);
        }
        fs::remove_dir_all(dir)?;
        Ok(())
    }
}

#[cfg(test)]
mod l24_creation_tests {
    use super::*;
    use crate::llm::{CharacterSuggestion,ProposedStartingItem,ProposedStartingFund,ProposedEffect};
    fn sample(name:&str)->CharacterSuggestion {
        CharacterSuggestion {
            name:name.into(),role:"marin".into(),sex:"homme".into(),gender:"masculin".into(),
            age_years:28,height_cm:177,weight_kg:73,appearance:"cheveux bruns".into(),
            origin:"Vigo".into(),personality:"prudent".into(),background:"marin de commerce".into(),
            initial_location:"Port de Vigo".into(),languages:"galicien, castillan".into(),
            distinctive_features:"".into(),voice:"accent galicien".into(),stats:Stats{strength:15,dexterity:13,
                constitution:14,intelligence:8,wisdom:12,charisma:10},
            skills:vec!["Athletisme".into(),"Survie".into()],
            items:vec![ProposedStartingItem{name:"corde".into(),quantity:1,notes:String::new()}],
            starting_funds:vec![ProposedStartingFund{currency:"réal espagnol".into(),amount:12}],
        }
    }
    #[test] fn character_selection_is_atomic_and_personal() -> Result<()> {
        let mut db=Storage::in_memory()?;
        assert_eq!(db.schema_version()?,27);
        let campaign=db.create_campaign("Vigo 1800",CampaignMode::Free,"Vigo en 1800",2)?;
        let chars=db.characters(&campaign)?;
        let first=&chars[0].id;
        let second=&chars[1].id;
        db.apply_character_suggestion(&campaign,first,&sample("Mateo"))?;
        assert_eq!(db.currency_list(&campaign)?,vec!["réal espagnol".to_string()]);
        assert_eq!(db.actor_location(&campaign,first)?.as_deref(),Some("Port de Vigo"));
        assert!(db.actor_location(&campaign,second)?.is_none());
        assert_eq!(db.inventory(first)?.len(),1);
        assert!(db.inventory(second)?.is_empty());
        assert!(db.holdings(&campaign,first)?.iter().any(|(v,n)|v=="réal espagnol"&&*n==12));
        let mut isabel=sample("Isabel");
        isabel.starting_funds=vec![ProposedStartingFund{currency:"livre britannique".into(),amount:5}];
        db.apply_character_suggestion(&campaign,second,&isabel)?;
        assert_eq!(db.currency_list(&campaign)?.len(),2);
        assert!(db.holdings(&campaign,second)?.iter().any(|(v,n)|v=="livre britannique"&&*n==5));
        db.finalize_setup(&campaign)?;
        assert!(db.apply_character_suggestion(&campaign,first,&sample("Autre")).is_err());
        assert!(db.add_currency(&campaign,"or").is_err());
        let profile=db.character_profile(first)?.unwrap();
        assert!(db.save_character_profile(&campaign,first,&profile).is_err());
        Ok(())
    }
    #[test] fn proposed_l1_character_budget_and_inventory_limits() {
        let mut c=sample("Olivia");assert!(c.validate().is_ok());
        c.stats.strength=30; assert!(c.validate().is_err());
        c=sample("Olivia");c.items[0].quantity=0;assert!(c.validate().is_err());
        c=sample("Olivia");c.starting_funds[0].amount=100_000;assert!(c.validate().is_err());
    }
    #[test] fn a_campaign_can_start_without_currency_or_with_multiple_currencies() -> Result<()> {
        let mut db=Storage::in_memory()?;
        let campaign=db.create_campaign("Troc",CampaignMode::Free,"Economie de troc",2)?;
        let characters=db.characters(&campaign)?;
        let first=&characters[0].id;
        let second=&characters[1].id;
        let mut no_money=sample("Sans Argent");
        no_money.starting_funds.clear();
        db.apply_character_suggestion(&campaign,first,&no_money)?;
        assert!(db.currency_list(&campaign)?.is_empty());
        assert!(db.holdings(&campaign,first)?.is_empty());
        let mut money=sample("Marchand");
        money.starting_funds=vec![
            ProposedStartingFund{currency:"pièces d'or".into(),amount:10},
            ProposedStartingFund{currency:"pièces d'argent".into(),amount:20},
        ];
        db.apply_character_suggestion(&campaign,second,&money)?;
        assert_eq!(db.currency_list(&campaign)?.len(),2);
        assert!(db.holdings(&campaign,second)?.iter().any(|(v,n)|v=="pièces d'or"&&*n==10));
        assert!(db.holdings(&campaign,second)?.iter().any(|(v,n)|v=="pièces d'argent"&&*n==20));
        assert!(db.holdings(&campaign,first)?.is_empty());
        db.finalize_setup(&campaign)?;
        assert_eq!(db.currency_list(&campaign)?.len(),2);
        Ok(())
    }
    #[test] fn starting_currency_rejects_duplicate_and_fake_inventory() {
        let mut c=sample("Boursier");
        c.starting_funds=vec![
            ProposedStartingFund{currency:"or".into(),amount:3},
            ProposedStartingFund{currency:"OR".into(),amount:5},
        ];
        assert!(c.validate().is_err());
        c.starting_funds.pop();
        c.items[0].name="or".into();
        assert!(c.validate().is_err());
        c.items.clear();
        assert!(c.validate().is_ok());
    }
    #[test] fn npc_introduction_creates_a_unique_contact_only_after_validation() -> Result<()> {
        let mut db=Storage::in_memory()?;
        let campaign=db.create_campaign("Vigo",CampaignMode::Free,"Vigo en 1800",1)?;
        let actor=db.characters(&campaign)?[0].id.clone();
        db.finalize_setup(&campaign)?;
        let effect=Some(ProposedEffect::IntroduceNpc{name:"Robet".into(),role:"charcutier".into()});
        // Le joueur peut demander Robet, mais cette demande seule ne prouve pas sa présence.
        let (without_proof_request,without_proof_proposal)=super::l23_tests::auto(
            &campaign,&actor,"npc-without-proof","Je vais voir Robet le charcutier",effect.clone());
        assert!(db.resolve_ai_action(&without_proof_request,&without_proof_proposal,"mock").is_err());
        assert!(db.known_npcs(&campaign)?.is_empty());
        assert!(db.events(&campaign,100)?.iter().all(|e|e.kind!="npc_introduced"));

        // La narration du MJ mentionne explicitement Robet : la proposition est sourcée.
        let narration="Robet le charcutier te répond depuis son étal.";
        let (mut request,mut proposal)=super::l23_tests::auto(&campaign,&actor,"npc-one",
            "Je vais voir Robet le charcutier",effect.clone());
        request.success_text=narration.into();
        proposal.success_text=narration.into();
        db.resolve_ai_action(&request,&proposal,"mock")?;
        assert!(db.known_npcs(&campaign)?.iter().any(|(_,name)|name=="Robet"));
        let snapshot=db.ai_snapshot(&campaign,&actor,"Je parle a Robet",4096)?;
        assert!(snapshot.contacts.to_string().contains("charcutier"));
        // Un autre tour ne peut pas introduire deux fois le même PNJ, même avec provenance.
        let (mut request2,mut proposal2)=super::l23_tests::auto(&campaign,&actor,"npc-again",
            "Je vais voir Robet le charcutier",effect);
        request2.success_text=narration.into();
        proposal2.success_text=narration.into();
        let err=db.resolve_ai_action(&request2,&proposal2,"mock").unwrap_err();
        assert!(err.to_string().contains("existe déjà"),"{err:#}");
        assert_eq!(db.known_npcs(&campaign)?.iter().filter(|(_,name)|name=="Robet").count(),1);
        Ok(())
    }
    #[test] fn npc_creation_must_be_ordinary_and_intentional() -> Result<()> {
        let npc=ProposedEffect::IntroduceNpc{name:"Robet".into(),role:"charcutier".into()};
        npc.validate()?;
        assert!(ProposedEffect::IntroduceNpc{name:"Robet".into(),role:"gouverneur".into()}.validate().is_err());
        Ok(())
    }
}

#[cfg(test)]
mod l243_multiplayer_regressions {
    use super::*;
    use crate::dice::RollMode;
    use crate::llm::{AiProposal,NpcIntroduction,ProposedEffect};
    use crate::uare::{Ability,Decision};

    fn place_profile(place:&str)->CharacterProfile {
        CharacterProfile{sex:"non précisé".into(),gender:"non précisé".into(),
            age_years:28,height_cm:175,weight_kg:70,appearance:"manteau de voyage".into(),
            origin:"Sangvent".into(),personality:"attentif".into(),
            initial_location:place.into(),languages:"français".into(),
            distinctive_features:String::new(),voice:String::new()}
    }
    fn resolve(db:&mut Storage,c:&str,actor:&str,key:&str,action:&str,
               narration:&str,effect:Option<ProposedEffect>)->Result<()> {
        let p=AiProposal{decision:Decision::AutoSuccess,ability:Ability::Wisdom,
            proficient:false,dc:10,roll_mode:RollMode::Normal,reason:"Scène ordinaire".into(),
            success_text:narration.into(),failure_text:String::new(),fact_proposal:None,
            stakes_public_success:None,stakes_public_failure:None,effect};
        let req=ActionRequest{key:key.into(),campaign_id:c.into(),actor_id:actor.into(),
            text:action.into(),decision:p.decision,ability:p.ability,proficient:p.proficient,
            dc:p.dc,roll_mode:p.roll_mode,target_id:None,hp_delta:0,
            success_text:p.success_text.clone(),failure_text:p.failure_text.clone()};
        db.resolve_ai_action(&req,&p,"mock")?;
        Ok(())
    }
    #[test]
    fn two_players_share_scene_after_move_even_with_different_wording()->Result<()> {
        assert_eq!(scene_key("sur la place du vieux puits, à Pierre-Pluie"),
                   scene_key("place du vieux puits"));
        assert_ne!(scene_key("devant la porte ouest"),scene_key("porte ouest"));
        assert_eq!(scene_key("emplacement exact inconnu"),None);
        let mut db=Storage::in_memory()?;
        let c=db.create_campaign("Sanctuaire",CampaignMode::Free,"Pierre-Pluie",2)?;
        let ids=db.characters(&c)?;
        db.save_character_profile(&c,&ids[0].id,&place_profile("devant la porte ouest"))?;
        db.save_character_profile(&c,&ids[1].id,&place_profile("sur la place du vieux puits, à Pierre-Pluie"))?;
        db.finalize_setup(&c)?;
        let before=db.ai_snapshot(&c,&ids[0].id,"Qui est présent ?",4096)?;
        assert_eq!(before.action["other_players_same_known_place"].as_array().unwrap().len(),0);
        resolve(&mut db,&c,&ids[0].id,"arrive-place", "Je vais sur la place du vieux puits",
            "Je rejoins la place du vieux puits.",
            Some(ProposedEffect::Move{destination:"place du vieux puits".into()}))?;
        let after=db.ai_snapshot(&c,&ids[0].id,"Qui est présent ?",4096)?;
        assert_eq!(after.action["other_players_same_known_place"][0],ids[1].name);
        let other=db.ai_snapshot(&c,&ids[1].id,"Qui est autour de moi ?",4096)?;
        assert_eq!(other.action["other_players_same_known_place"][0],ids[0].name);
        assert_eq!(db.actor_location(&c,&ids[0].id)?.as_deref(),Some("place du vieux puits"));
        assert_eq!(db.actor_location(&c,&ids[1].id)?.as_deref(),
                   Some("sur la place du vieux puits, à Pierre-Pluie"));
        Ok(())
    }
    #[test]
    fn npc_pair_introduced_atomically_only_once()->Result<()> {
        let mut db=Storage::in_memory()?;
        let c=db.create_campaign("Sangvent",CampaignMode::Free,"Pierre-Pluie",1)?;
        let actor=db.characters(&c)?[0].id.clone();
        db.finalize_setup(&c)?;
        let effect=Some(ProposedEffect::IntroduceNpcs{npcs:vec![
            NpcIntroduction{name:"Brondan".into(),role:"habitant".into()},
            NpcIntroduction{name:"Jorik".into(),role:"habitant".into()},
        ]});
        resolve(&mut db,&c,&actor,"names","Je leur demande leurs noms",
            "Je m'appelle Brondan, dit le premier. Lui, c'est Jorik.",effect.clone())?;
        assert_eq!(db.known_npcs(&c)?.len(),2);
        // Le même message rejoué retourne son résultat, sans dupliquer les PNJ.
        resolve(&mut db,&c,&actor,"names","Je leur demande leurs noms",
            "Je m'appelle Brondan, dit le premier. Lui, c'est Jorik.",effect)?;
        assert_eq!(db.known_npcs(&c)?.len(),2);
        assert_eq!(db.events(&c,100)?.iter().filter(|e|e.kind=="npc_introduced").count(),2);
        Ok(())
    }
    #[test]
    fn untrusted_npc_identity_cannot_take_a_player_name()->Result<()> {
        let mut db=Storage::in_memory()?;
        let c=db.create_campaign("Sangvent",CampaignMode::Free,"Pierre-Pluie",2)?;
        let chars=db.characters(&c)?;
        db.finalize_setup(&c)?;
        let effect=Some(ProposedEffect::IntroduceNpc{
            name:chars[1].name.clone(),role:"marchand".into()});
        let msg=format!("Je vais voir {}",chars[1].name);
        let result=resolve(&mut db,&c,&chars[0].id,"duplicate-player",&msg,
            &format!("{} se présente.",chars[1].name),effect);
        assert!(result.is_err());
        assert!(db.known_npcs(&c)?.is_empty());
        Ok(())
    }
    #[test]
    fn storage_rejects_puppeteering_player_before_commit()->Result<()> {
        let mut db=Storage::in_memory()?;
        let c=db.create_campaign("Cyberpunk",CampaignMode::Free,"Néo-Kyoto",2)?;
        let chars=db.characters(&c)?;
        let akira=&chars[0].id;
        let rin=&chars[1].id;
        db.save_character_profile(&c,akira,&place_profile("marché souterrain d'Asagiri"))?;
        db.save_character_profile(&c,rin,&place_profile("marché souterrain d'Asagiri"))?;
        db.finalize_setup(&c)?;
        let events_before=db.events(&c,500)?.len();
        let wrong=resolve(&mut db,&c,akira,"puppeteer", &format!("Je dis bonjour à {}",chars[1].name),
            &format!("Tu salues {}. Elle lève les yeux et répond.",chars[1].name),None);
        assert!(wrong.is_err());
        assert_eq!(db.events(&c,500)?.len(),events_before);
        resolve(&mut db,&c,akira,"greeting",&format!("Je dis bonjour à {}",chars[1].name),
            &format!("Tu salues {} et attends sa réaction.",chars[1].name),None)?;
        assert_eq!(db.events(&c,500)?.iter().filter(|e|e.kind=="action_resolved").count(),1);
        Ok(())
    }
    #[test]
    fn shared_scene_survives_restart_without_corrupting_old_version()->Result<()> {
        let dir=std::env::temp_dir().join(format!("mj-l243-{}",Uuid::new_v4()));
        let path=dir.join("save.sqlite3");
        let (c,actor);
        {
            let mut db=Storage::open(&path)?;
            c=db.create_campaign("Vigo",CampaignMode::Free,"Vigo 1800",1)?;
            actor=db.characters(&c)?[0].id.clone();
            db.save_character_profile(&c,&actor,&place_profile("marché du port"))?;
            db.finalize_setup(&c)?;
            resolve(&mut db,&c,&actor,"walk", "Je vais sur la place du vieux puits",
                "Tu rejoins la place.",Some(ProposedEffect::Move{destination:"place du vieux puits".into()}))?;
        }
        {
            let db=Storage::open(&path)?;
            assert_eq!(db.schema_version()?,27);
            assert_eq!(db.actor_location(&c,&actor)?.as_deref(),Some("place du vieux puits"));
        }
        fs::remove_dir_all(dir)?;
        Ok(())
    }
}
