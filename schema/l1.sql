-- Schéma complet L1. Aucun script de migration entre lots.
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
VALUES('general','{"llm_endpoint":"http://127.0.0.1:8080/v1","llm_model":"","context_target_tokens":4096,"compaction_threshold_pct":75,"last_campaign_id":null}',CAST(strftime('%s','now') AS INTEGER));
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
PRAGMA user_version=11;
