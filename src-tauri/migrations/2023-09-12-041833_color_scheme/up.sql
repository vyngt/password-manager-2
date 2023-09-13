CREATE TABLE color_scheme (
  id INTEGER NOT NULL PRIMARY KEY,
  `name` VARCHAR NOT NULL,
  `primary` VARCHAR NOT NULL,
  `secondary` VARCHAR NOT NULL,
  success VARCHAR NOT NULL,
  danger VARCHAR NOT NULL,
  warning VARCHAR NOT NULL,
  foreground VARCHAR NOT NULL,
  background VARCHAR NOT NULL,
  selection VARCHAR NOT NULL
);

ALTER TABLE theme ADD color_scheme_id INTEGER REFERENCES color_scheme(id);


INSERT INTO color_scheme (id, `name`, `primary`, `secondary`, success, danger, warning, foreground, background, selection)
VALUES (1, 'Default','blue', 'gray', 'green', 'yellow', 'red', 'white', 'black', 'white');

INSERT INTO theme (id, color_scheme_id) VALUES (1, 1);