ALTER TABLE tuning_presets ADD COLUMN upgrades TEXT;          -- JSON: UpgradeCategory[] | null
ALTER TABLE tuning_presets ADD COLUMN baseline_id INTEGER REFERENCES tuning_presets(id);
