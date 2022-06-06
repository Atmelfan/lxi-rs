CREATE TABLE IF NOT EXISTS articles (
    id          INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    text        TEXT NOT NULL,
    title       TEXT NOT NULL,
    created     NUMERIC NOT NULL,
    updated     NUMERIC NOT NULL
);

CREATE TABLE IF NOT EXISTS users (
    id          INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    name        TEXT NOT NULL,
    role        TEXT NOT NULL,
    password    BLOB,
    salt        BLOB NOT NULL,
    created     NUMERIC NOT NULL,
    updated     NUMERIC NOT NULL
);
