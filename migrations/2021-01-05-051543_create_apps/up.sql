-- Your SQL goes here
CREATE TABLE apps (
    id BIGSERIAL PRIMARY KEY,
    owner_id BIGINT NOT NULL,
    title VARCHAR(12) NOT NULL UNIQUE,
    description VARCHAR(256) NOT NULL,
    domain VARCHAR(253) NOT NULL,
    token CHAR(60) NOT NULL,
    connected BOOLEAN NOT NULL DEFAULT false,
    connected_error VARCHAR NOT NULL DEFAULT 'No connection attempted.'
);