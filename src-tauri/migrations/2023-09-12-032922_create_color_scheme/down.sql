-- This file should undo anything in `up.sql`
DELETE FROM color_scheme WHERE id > 0;
DROP TABLE color_scheme;