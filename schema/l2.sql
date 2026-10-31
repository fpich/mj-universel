-- Schéma L2.4. Base neuve, sans migration depuis les versions précédentes.
CREATE TABLE IF NOT EXISTS campaigns (
 id TEXT PRIMARY KEY, title TEXT NOT NULL CHECK(length(trim(title)) BETWEEN 1 AND 120),
 mode TEXT NOT NULL CHECK(mode IN ('free','partial','complete')),
 seed_context TEXT NOT NULL DEFAULT '' CHECK(length(seed_context)<=40000),
 player_count INTEGER NOT NULL CHECK(player_count BETWEEN 1 AND 6),
 status TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active','archived')),
 state_version INTEGER NOT NULL DEFAULT 0 CHECK(state_version>=0),
 created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS campaigns_updated ON campaigns(updated_at DESC,id);
CREATE TABLE IF NOT EXISTS participants (
 id TEXT PRIMARY KEY, campaign_id TEXT NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
 slot INTEGER NOT NULL CHECK(slot BETWEEN 1 AND 6),
 display_name TEXT NOT NULL CHECK(length(trim(display_name)) BETWEEN 1 AND 80),
 UNIQUE(campaign_id,slot)
);
CREATE TABLE IF NOT EXISTS app_settings (
 key TEXT PRIMARY KEY, value_json TEXT NOT NULL CHECK(json_valid(value_json)), updated_at INTEGER NOT NULL
);
INSERT OR IGNORE INTO app_settings(key,value_json,updated_at)
VALUES('general','{"llm_endpoint":"http://127.0.0.1:8080/v1","llm_model":"","context_target_tokens":4096,"compaction_threshold_pct":75,"llm_temperature":0.7,"llm_max_tokens":768,"llm_timeout_seconds":90,"last_campaign_id":null}',CAST(strftime('%s','now') AS INTEGER));
CREATE TABLE IF NOT EXISTS characters (
 id TEXT PRIMARY KEY, participant_id TEXT NOT NULL UNIQUE REFERENCES participants(id) ON DELETE CASCADE,
 name TEXT NOT NULL CHECK(length(trim(name)) BETWEEN 1 AND 80),
 class_name TEXT NOT NULL DEFAULT 'Aventurier',level INTEGER NOT NULL DEFAULT 1 CHECK(level BETWEEN 1 AND 20),
 strength INTEGER NOT NULL DEFAULT 10 CHECK(strength BETWEEN 1 AND 30),
 dexterity INTEGER NOT NULL DEFAULT 10 CHECK(dexterity BETWEEN 1 AND 30),
 constitution INTEGER NOT NULL DEFAULT 10 CHECK(constitution BETWEEN 1 AND 30),
 intelligence INTEGER NOT NULL DEFAULT 10 CHECK(intelligence BETWEEN 1 AND 30),
 wisdom INTEGER NOT NULL DEFAULT 10 CHECK(wisdom BETWEEN 1 AND 30),
 charisma INTEGER NOT NULL DEFAULT 10 CHECK(charisma BETWEEN 1 AND 30),
 hp INTEGER NOT NULL DEFAULT 10 CHECK(hp>=0), max_hp INTEGER NOT NULL DEFAULT 10 CHECK(max_hp BETWEEN 1 AND 9999),
 ac INTEGER NOT NULL DEFAULT 10 CHECK(ac BETWEEN 1 AND 50),
 skills_json TEXT NOT NULL DEFAULT '[]' CHECK(json_valid(skills_json)),
 notes TEXT NOT NULL DEFAULT '', updated_at INTEGER NOT NULL,
 CHECK(hp<=max_hp)
);
CREATE TABLE IF NOT EXISTS inventory (
 id TEXT PRIMARY KEY, character_id TEXT NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
 name TEXT NOT NULL CHECK(length(trim(name)) BETWEEN 1 AND 120),
 quantity INTEGER NOT NULL CHECK(quantity BETWEEN 1 AND 9999), notes TEXT NOT NULL DEFAULT ''
);
CREATE INDEX IF NOT EXISTS inventory_character ON inventory(character_id);
CREATE TABLE IF NOT EXISTS game_events (
 seq INTEGER PRIMARY KEY AUTOINCREMENT,
 id TEXT NOT NULL UNIQUE,
 campaign_id TEXT NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
 request_key TEXT NOT NULL,kind TEXT NOT NULL,
 request_json TEXT NOT NULL CHECK(json_valid(request_json)),
 payload_json TEXT NOT NULL CHECK(json_valid(payload_json)),
 created_at INTEGER NOT NULL,
 UNIQUE(campaign_id,request_key)
);
CREATE INDEX IF NOT EXISTS events_campaign_order ON game_events(campaign_id,seq);

-- Décisions MJ IA; transaction atomique avec action_resolved.
CREATE TABLE ai_decisions (
 campaign_id TEXT NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
 action_key TEXT NOT NULL,
 actor_id TEXT NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
 provider_model TEXT NOT NULL,
 proposal_json TEXT NOT NULL CHECK(json_valid(proposal_json)),
 created_at INTEGER NOT NULL,
 PRIMARY KEY (campaign_id, action_key)
);
-- Propositions narratives non canoniques jusqu'à validation humaine.
CREATE TABLE ai_fact_proposals (
 id TEXT PRIMARY KEY, campaign_id TEXT NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
 source_action_key TEXT NOT NULL, fact_text TEXT NOT NULL CHECK(length(fact_text) BETWEEN 1 AND 500),
 status TEXT NOT NULL CHECK(status IN ('pending','approved','rejected')) DEFAULT 'pending',
 created_at INTEGER NOT NULL, reviewed_at INTEGER,
 UNIQUE(campaign_id,source_action_key,fact_text)
);
CREATE INDEX ai_facts_campaign ON ai_fact_proposals(campaign_id,status,created_at);
-- Clés API chiffrées, jamais en clair dans SQLite.
CREATE TABLE api_credentials (
 id INTEGER PRIMARY KEY CHECK(id=1), salt BLOB NOT NULL, nonce BLOB NOT NULL, ciphertext BLOB NOT NULL,
 created_at INTEGER NOT NULL
);

-- Une seule résolution suspendue par campagne, restaurée à l'ouverture.
CREATE TABLE IF NOT EXISTS pending_rolls (
 campaign_id TEXT PRIMARY KEY REFERENCES campaigns(id) ON DELETE CASCADE,
 request_key TEXT NOT NULL UNIQUE,
 actor_id TEXT NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
 request_json TEXT NOT NULL CHECK(json_valid(request_json)),
 proposal_json TEXT NOT NULL CHECK(json_valid(proposal_json)),
 provider_model TEXT NOT NULL,
 created_at INTEGER NOT NULL
);
-- L2.3 : comptes de ressources nommées, PNJ connus et promesses déclarées.
CREATE TABLE IF NOT EXISTS known_npcs (
 id TEXT PRIMARY KEY, campaign_id TEXT NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
 name TEXT NOT NULL CHECK(length(trim(name)) BETWEEN 1 AND 100),
 role TEXT NOT NULL DEFAULT '',
 UNIQUE(campaign_id,name)
);
CREATE TABLE IF NOT EXISTS holdings (
 campaign_id TEXT NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
 owner_key TEXT NOT NULL, asset TEXT NOT NULL CHECK(length(trim(asset)) BETWEEN 1 AND 120),
 quantity INTEGER NOT NULL CHECK(quantity BETWEEN 0 AND 1000000000),
 PRIMARY KEY(campaign_id,owner_key,asset)
);
CREATE TABLE IF NOT EXISTS promises (
 id TEXT PRIMARY KEY, campaign_id TEXT NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
 actor_id TEXT NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
 recipient TEXT NOT NULL, description TEXT NOT NULL,
 source_action_key TEXT NOT NULL, created_at INTEGER NOT NULL,
 UNIQUE(campaign_id,source_action_key)
);
-- L2.3.2 : verrouillage de l'état initial après confirmation de la campagne.
CREATE TABLE IF NOT EXISTS campaign_setup (
 campaign_id TEXT PRIMARY KEY REFERENCES campaigns(id) ON DELETE CASCADE,
 started_at INTEGER
);
-- L2.4 : profil physique, depart canonique et registre monetaires par campagne.
CREATE TABLE IF NOT EXISTS character_profiles (
 character_id TEXT PRIMARY KEY REFERENCES characters(id) ON DELETE CASCADE,
 sex TEXT NOT NULL DEFAULT 'non précisé', gender TEXT NOT NULL DEFAULT 'non précisé',
 age_years INTEGER NOT NULL CHECK(age_years BETWEEN 1 AND 5000),
 height_cm INTEGER NOT NULL CHECK(height_cm BETWEEN 25 AND 400),
 weight_kg INTEGER NOT NULL CHECK(weight_kg BETWEEN 2 AND 500),
 appearance TEXT NOT NULL DEFAULT '', origin TEXT NOT NULL DEFAULT '',
 personality TEXT NOT NULL DEFAULT '', initial_location TEXT NOT NULL DEFAULT '',
 languages TEXT NOT NULL DEFAULT '', distinctive_features TEXT NOT NULL DEFAULT '',
 voice TEXT NOT NULL DEFAULT '', updated_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS campaign_currencies (
 id TEXT PRIMARY KEY, campaign_id TEXT NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
 name TEXT NOT NULL CHECK(length(trim(name)) BETWEEN 1 AND 80),
 UNIQUE(campaign_id,name COLLATE NOCASE)
);
-- L2.4.3 : lieu structuré propre à chaque PJ, cohérent malgré les variantes textuelles.
CREATE TABLE IF NOT EXISTS actor_scenes (
 actor_id TEXT PRIMARY KEY REFERENCES characters(id) ON DELETE CASCADE,
 campaign_id TEXT NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
 place_key TEXT NOT NULL DEFAULT '', label TEXT NOT NULL DEFAULT '',
 source_action_key TEXT NOT NULL DEFAULT ''
);
CREATE INDEX IF NOT EXISTS actor_scenes_campaign_place ON actor_scenes(campaign_id,place_key);
PRAGMA user_version=27;
