-- Add structured details column to feature_history
-- Migrates existing data from separate columns into JSON structure

ALTER TABLE feature_history ADD COLUMN details JSON;

-- Migrate existing data into the new details column
UPDATE feature_history
SET details = json_object(
    'summary', summary,
    'author', author,
    'files_changed', json(COALESCE(files_changed, '[]')),
    'commits', json('[]')
)
WHERE details IS NULL;
