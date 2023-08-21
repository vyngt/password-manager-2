-- This file should undo anything in `up.sql`
DROP INDEX IF EXISTS idx_item_pk;
DROP INDEX IF EXISTS idx_item_name;
DROP TABLE items;