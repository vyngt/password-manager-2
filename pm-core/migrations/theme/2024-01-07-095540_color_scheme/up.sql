CREATE TABLE theme (id INTEGER NOT NULL PRIMARY KEY);
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
ALTER TABLE theme
ADD color_scheme_id INTEGER REFERENCES color_scheme(id);
INSERT INTO color_scheme (
        id,
        `name`,
        `primary`,
        `secondary`,
        success,
        danger,
        warning,
        foreground,
        background,
        selection
    )
VALUES (
        1,
        'Peace',
        '244 31 198',
        '184 215 136',
        '30 200 135',
        '244 37 99',
        '224 193 36',
        '255 194 194',
        '97 25 74',
        '34 96 135'
    );
INSERT INTO color_scheme (
        id,
        `name`,
        `primary`,
        `secondary`,
        success,
        danger,
        warning,
        foreground,
        background,
        selection
    )
VALUES (
        2,
        'Crimson',
        '0 123 199',
        '214 136 215',
        '200 30 177',
        '244 37 99',
        '132 193 1',
        '255 194 194',
        '80 2 2',
        '135 34 34'
    );
INSERT INTO color_scheme (
        id,
        `name`,
        `primary`,
        `secondary`,
        success,
        danger,
        warning,
        foreground,
        background,
        selection
    )
VALUES (
        3,
        'Forest',
        '48 175 13',
        '83 198 127',
        '4 214 0',
        '253 103 23',
        '187 158 17',
        '146 176 150',
        '23 64 31',
        '34 135 82'
    );
INSERT INTO theme (id, color_scheme_id)
VALUES (1, 1);