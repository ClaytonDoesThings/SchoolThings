-- Your SQL goes here
CREATE TABLE repos (
    id BIGSERIAL PRIMARY KEY,
    owner_id BIGINT NOT NULL,
    title VARCHAR NOT NULL,
    description VARCHAR NOT NULL,
    apps BIGINT[] NOT NULL DEFAULT '{}'
);

CREATE UNIQUE INDEX repos_title_unique_idx on repos (LOWER(title));