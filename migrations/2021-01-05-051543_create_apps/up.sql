-- Your SQL goes here
CREATE TABLE apps (
    id BIGSERIAL PRIMARY KEY,
    owner_id BIGINT NOT NULL,
    title VARCHAR NOT NULL UNIQUE,
    description VARCHAR NOT NULL,
    domain VARCHAR(253) NOT NULL,
    token CHAR(60) NOT NULL,
    connected BOOLEAN NOT NULL DEFAULT false,
    connected_error VARCHAR NOT NULL DEFAULT 'No connection attempted.'
);