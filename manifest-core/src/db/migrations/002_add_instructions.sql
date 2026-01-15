-- Add instructions fields for AI agent guidance
-- Project-level: coding guidelines, conventions
-- Directory-level: build commands, test commands

ALTER TABLE projects ADD COLUMN instructions TEXT;
ALTER TABLE project_directories ADD COLUMN instructions TEXT;
