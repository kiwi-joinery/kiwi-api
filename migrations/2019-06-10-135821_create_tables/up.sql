-- Your SQL goes here
CREATE TABLE users
(
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    email VARCHAR(255) NOT NULL UNIQUE,
    password_hash VARCHAR(255) NULL,
    is_admin BOOLEAN DEFAULT FALSE NOT NULL,
    password_reset_token VARCHAR(255) NULL
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

CREATE TABLE lectures
(
    id SERIAL PRIMARY KEY,
    user_id INT NOT NULL,
    title VARCHAR(255) NOT NULL,
    file_id INT NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users (id),
    FOREIGN KEY (file_id) REFERENCES files (id)
);

INSERT INTO users (name, email, is_admin, password_hash) VALUES ('admin', 'admin', 't', '$2a$10$W0QseTznvCwlOHUd6g3ZieURwl26V5HVZbwk8dVa6HsdRpkHG4.d2');
