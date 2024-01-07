-- Your SQL goes here
CREATE TABLE program (
    id INTEGER NOT NULL PRIMARY KEY,
    `version` CHAR NOT NULL
);
INSERT INTO program(id, `version`)
VALUES (1, "2.0.0");