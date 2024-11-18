-- Your SQL goes here
CREATE TABLE courses
(
    code    TEXT    NOT NULL PRIMARY KEY,
    name    TEXT    NOT NULL,
    unit_id INTEGER NOT NULL,
    FOREIGN KEY (unit_id) REFERENCES units (id)
);
