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
VALUES (1, 'Peace', '#f41fc6', '#b8d788', '#1ec887', '#f42563', '#e0c124', '#ffc2c2', '#61194a', '#226087');
INSERT INTO color_scheme (id, `name`, `primary`, `secondary`, success, danger, warning, foreground, background, selection)
VALUES (2,'Crimson','#007bc7','#d688d7','#c81eb1','#f42563','#84c101','#ffc2c2','#500202','#872222');
INSERT INTO color_scheme (id, `name`, `primary`, `secondary`, success, danger, warning, foreground, background, selection)
VALUES (3,'Forest','#30af0d','#53c67f','#04d600','#fd6717','#bb9e11','#92b096','#17401f','#228752');


INSERT INTO theme (id, color_scheme_id) VALUES (1, 1);