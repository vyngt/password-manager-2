-- Your SQL goes here
CREATE TABLE item_tags (
  id INTEGER NOT NULL PRIMARY KEY,
  `name` VARCHAR NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_item_tag_pk ON items(id);
CREATE INDEX IF NOT EXISTS idx_item_tag_name ON items(name);