-- Your SQL goes here
CREATE TABLE theme (
  id INTEGER NOT NULL PRIMARY KEY,
  color_scheme_id INTEGER NOT NULL,
  FOREIGN KEY(color_scheme_id) REFERENCES color_scheme(id)
);

INSERT INTO theme (id, color_scheme_id) VALUES (1, 1);