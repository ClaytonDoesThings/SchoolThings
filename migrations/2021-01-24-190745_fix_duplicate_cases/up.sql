-- Your SQL goes here
ALTER TABLE users DROP CONSTRAINT users_username_key;
CREATE UNIQUE INDEX users_username_unique_idx on users (LOWER(username));
ALTER TABLE users DROP CONSTRAINT users_email_key;
CREATE UNIQUE INDEX users_email_unique_idx on users (LOWER(email));

ALTER TABLE apps DROP CONSTRAINT apps_title_key;
CREATE UNIQUE INDEX apps_title_unique_idx on apps (LOWER(title));