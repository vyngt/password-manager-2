-- This file should undo anything in `up.sql`
DROP INDEX IF EXISTS idx_item_tag_pk;
DROP INDEX IF EXISTS idx_item_tag_name;

DROP TABLE item_tags;