-- Your SQL goes here
CREATE TABLE users
(
    id SERIAL PRIMARY KEY,
    password_hash VARCHAR(255) NULL
);

CREATE TABLE sessions
(
    id SERIAL PRIMARY KEY,
    user_id INT NOT NULL,
    token VARCHAR(255) NOT NULL,
    created TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP NOT NULL,
    last_used TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP NOT NULL,
    last_ip BYTEA NOT NULL,         -- Bincode Serialized IpAddr
    user_agent VARCHAR(512) NOT NULL,
    UNIQUE (user_id, token),
    FOREIGN KEY (user_id) REFERENCES users (id)
);

CREATE TABLE files
(
    id SERIAL PRIMARY KEY
);
