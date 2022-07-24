CREATE TABLE IF NOT EXISTS accounts (
    id          INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    uuid        TEXT NOT NULL UNIQUE,
    name        TEXT NOT NULL,
    created_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  DATETIME,
    deleted     BOOLEAN NOT NULL DEFAULT false,
    deleted_at  DATETIME
);

CREATE TABLE IF NOT EXISTS projects (
    id          INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    account_id  INTEGER NOT NULL REFERENCES accounts (id),
    uuid        TEXT NOT NULL UNIQUE,
    name        TEXT NOT NULL,
    created_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  DATETIME,
    deleted     BOOLEAN NOT NULL DEFAULT false,
    deleted_at  DATETIME
);

CREATE TABLE IF NOT EXISTS checks (
    id          INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    account_id  INTEGER NOT NULL REFERENCES accounts (id),
    project_id  INTEGER NOT NULL REFERENCES projects (id),
    uuid        TEXT NOT NULL UNIQUE,
    name        TEXT NOT NULL DEFAULT '',
    created_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  DATETIME,
    deleted     BOOLEAN NOT NULL DEFAULT false,
    deleted_at  DATETIME
);
