-- Your SQL goes here
CREATE TABLE users (
    id BIGSERIAL PRIMARY KEY,
    username VARCHAR NOT NULL UNIQUE,
    email VARCHAR(254) NOT NULL UNIQUE,
    password_hash CHAR(60) NOT NULL
);