-- Your SQL goes here
CREATE TABLE sids
(
    sid         INTEGER NOT NULL PRIMARY KEY,
    course_code TEXT    NOT NULL,
    FOREIGN KEY (course_code) REFERENCES courses (code)
);