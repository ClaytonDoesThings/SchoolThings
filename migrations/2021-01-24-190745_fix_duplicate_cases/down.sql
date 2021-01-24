-- This file should undo anything in `up.sql`
DROP INDEX users_username_unique_idx;
ALTER TABLE users ADD UNIQUE (username);
DROP INDEX users_email_unique_idx;
ALTER TABLE users ADD UNIQUE (email);

DROP INDEX apps_title_unique_idx;
ALTER TABLE apps ADD UNIQUE (title);