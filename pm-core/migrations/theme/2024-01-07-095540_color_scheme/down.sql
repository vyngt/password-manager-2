ALTER TABLE theme DROP color_scheme_id;
DELETE FROM theme
WHERE id > 0;
DELETE FROM color_scheme
WHERE id > 0;
DROP TABLE color_scheme;
DROP TABLE theme;