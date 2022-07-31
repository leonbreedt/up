CREATE TABLE IF NOT EXISTS accounts (
    id          BIGSERIAL PRIMARY KEY,
    uuid        UUID NOT NULL DEFAULT gen_random_uuid(),
    shortid     TEXT NOT NULL,
    name        TEXT NOT NULL,
    created_at  TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc'),
    updated_at  TIMESTAMP WITHOUT TIME ZONE,
    deleted     BOOLEAN NOT NULL DEFAULT false,
    deleted_at  TIMESTAMP WITHOUT TIME ZONE,
    CONSTRAINT  accounts_unique_uuid UNIQUE (uuid),
    CONSTRAINT  accounts_unique_shortid UNIQUE (shortid)
);

CREATE TABLE IF NOT EXISTS users (
    id          BIGSERIAL PRIMARY KEY,
    uuid        UUID NOT NULL DEFAULT gen_random_uuid(),
    shortid     TEXT NOT NULL,
    email       TEXT NOT NULL,
    created_at  TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc'),
    updated_at  TIMESTAMP WITHOUT TIME ZONE,
    deleted     BOOLEAN NOT NULL DEFAULT false,
    deleted_at  TIMESTAMP WITHOUT TIME ZONE,
    CONSTRAINT  users_unique_uuid UNIQUE (uuid),
    CONSTRAINT  users_unique_shortid UNIQUE (shortid)
);

CREATE TABLE IF NOT EXISTS user_accounts (
    user_id    BIGINT NOT NULL REFERENCES users (id),
    account_id BIGINT NOT NULL REFERENCES accounts (id),
    PRIMARY KEY (user_id, account_id)
);

CREATE UNIQUE INDEX IF NOT EXISTS users_unique_email
ON users (email)
WHERE deleted_at IS NULL;

CREATE TABLE IF NOT EXISTS projects (
    id          BIGSERIAL PRIMARY KEY,
    account_id  BIGINT NOT NULL REFERENCES accounts (id),
    uuid        UUID NOT NULL DEFAULT gen_random_uuid(),
    shortid     TEXT NOT NULL,
    name        TEXT NOT NULL,
    created_at  TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc'),
    updated_at  TIMESTAMP WITHOUT TIME ZONE,
    deleted     BOOLEAN NOT NULL DEFAULT false,
    deleted_at  TIMESTAMP WITHOUT TIME ZONE,
    CONSTRAINT  projects_unique_uuid UNIQUE (uuid),
    CONSTRAINT  projects_unique_shortid UNIQUE (shortid)
);

CREATE TABLE IF NOT EXISTS user_projects (
    user_id    BIGINT NOT NULL REFERENCES users (id),
    project_id BIGINT NOT NULL REFERENCES projects (id),
    PRIMARY KEY (user_id, project_id)
);

CREATE TABLE IF NOT EXISTS checks (
    id          BIGSERIAL PRIMARY KEY,
    account_id  BIGINT NOT NULL REFERENCES accounts (id),
    project_id  BIGINT NOT NULL REFERENCES projects (id),
    uuid        UUID NOT NULL DEFAULT gen_random_uuid(),
    shortid     TEXT NOT NULL,
    name        TEXT NOT NULL DEFAULT '',
    created_at  TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc'),
    updated_at  TIMESTAMP WITHOUT TIME ZONE,
    deleted     BOOLEAN NOT NULL DEFAULT false,
    deleted_at  TIMESTAMP WITHOUT TIME ZONE,
    CONSTRAINT  checks_unique_uuid UNIQUE (uuid),
    CONSTRAINT  checks_unique_shortid UNIQUE (shortid)
);

CREATE TABLE IF NOT EXISTS tags (
    id          BIGSERIAL PRIMARY KEY,
    account_id  BIGINT NOT NULL REFERENCES accounts (id),
    name        TEXT NOT NULL,
    created_at  TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc'),
    updated_at  TIMESTAMP WITHOUT TIME ZONE,
    CONSTRAINT  tags_unique_uuid UNIQUE (name)
);

CREATE TABLE IF NOT EXISTS check_tags (
    check_id BIGINT NOT NULL REFERENCES checks (id),
    tag_id   BIGINT NOT NULL REFERENCES tags (id),
    PRIMARY KEY (tag_id, check_id)
);

INSERT INTO accounts (uuid, shortid, name) VALUES ('09e37c58-8a7d-4ecd-8cec-db623400aa4c', 'lNyYYnbbxw5ZVtpJZqKJk3x6K', 'leon@sector42.io');
INSERT INTO users (uuid, shortid, email) VALUES ('3f5292c6-e1f4-4c57-8dc4-ee70b59c4748', 'wWO8BkekbbX4oIY0y6Wgpp0aJ8', 'leon@sector42.io');
INSERT INTO projects (account_id, uuid, shortid, name) VALUES (1, '7923c465-2115-4dac-ae34-36f35af6e004', '2a4OzNYlKExNjIOqagqgozjXm9', 'leon@sector42.io');
INSERT INTO user_projects (user_id, project_id) VALUES (1, 1);
INSERT INTO user_accounts (user_id, account_id) VALUES (1, 1);
INSERT INTO checks (account_id, project_id, uuid, shortid, name) VALUES (1, 1, '9f5cc288-4273-4219-b426-1a53fdaa645b', '3Q2g25YDlba4GcyryWgaAqEewW', 'Tarsnap Backups: Galactica');
INSERT INTO checks (account_id, project_id, uuid, shortid, name) VALUES (1, 1, '4f6f63c9-ac7d-42aa-a267-c93972c340a1', 'dl3ZykVQONNYPhG9j6xaEVmbyb', 'Tarsnap Backups: Starbuck');
INSERT INTO checks (account_id, project_id, uuid, shortid, name) VALUES (1, 1, '5baed0b7-70c6-4228-85de-4e3ac2b8f4db', 'lNdogJQDrkgZ2IKg98lZnbGG5B', 'SSL Certificates: leonbreedt');
INSERT INTO checks (account_id, project_id, uuid, shortid, name) VALUES (1, 1, '45c544f4-36ac-4428-8f9e-187037a6c87b', 'Xw8QKAZdNA3nnFKxopJ88kweGE', 'SSL Certificates: sector42');
INSERT INTO tags (account_id, name) VALUES (1, 'backups');
INSERT INTO tags (account_id, name) VALUES (1, 'certs');
INSERT INTO check_tags (check_id, tag_id) VALUES (1, 1);
INSERT INTO check_tags (check_id, tag_id) VALUES (2, 1);
INSERT INTO check_tags (check_id, tag_id) VALUES (3, 2);
INSERT INTO check_tags (check_id, tag_id) VALUES (4, 2);
