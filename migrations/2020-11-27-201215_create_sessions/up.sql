-- Your SQL goes here
CREATE TABLE sessions (
    id BIGSERIAL PRIMARY KEY,
    logged_in_user BIGINT
);