-- Your SQL goes here
CREATE TABLE color_scheme (
  id INTEGER NOT NULL PRIMARY KEY,
  `name` VARCHAR NOT NULL,
  `primary` VARCHAR NOT NULL,
  `secondary` VARCHAR NOT NULL,
  success VARCHAR NOT NULL,
  danger VARCHAR NOT NULL,
  warning VARCHAR NOT NULL,
  foreground VARCHAR NOT NULL,
  background VARCHAR NOT NULL
);

INSERT INTO color_scheme (`name`, `primary`, `secondary`, success, danger, warning, foreground, background)
VALUES ('Default','blue', 'gray', 'green', 'red', 'yellow', 'white', 'black');