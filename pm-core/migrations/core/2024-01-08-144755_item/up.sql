CREATE TABLE item (
    id INTEGER NOT NULL PRIMARY KEY,
    `name` VARCHAR NOT NULL,
    `url` VARCHAR NOT NULL,
    `username` VARCHAR NOT NULL,
    `password` VARCHAR NOT NULL
);
CREATE INDEX idx_item_pk ON item(id);
CREATE INDEX idx_item_name ON item(name);