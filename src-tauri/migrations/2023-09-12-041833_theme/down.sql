-- This file should undo anything in `up.sql`
DELETE FROM theme WHERE id > 0;
DROP TABLE theme;