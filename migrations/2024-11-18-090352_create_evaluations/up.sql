-- Your SQL goes here
CREATE TABLE evaluations
(
    sid                              INTEGER NOT NULL PRIMARY KEY,
    section_name                     TEXT    NOT NULL,
    enrollment                       INTEGER NOT NULL,
    responses                        INTEGER NOT NULL,
    class_helped_understanding       TEXT    NOT NULL,
    assignments_helped_understanding TEXT    NOT NULL,
    fair_exams                       TEXT    NOT NULL,
    timely_feedback                  TEXT    NOT NULL,
    developed_understanding          TEXT    NOT NULL,
    engaging                         TEXT    NOT NULL,
    communication                    TEXT    NOT NULL,
    help_opportunities               TEXT    NOT NULL,
    effective_methods                TEXT    NOT NULL,
    timeliness                       TEXT    NOT NULL,
    welcoming                        TEXT    NOT NULL,
    materials                        TEXT    NOT NULL,
    hours                            TEXT    NOT NULL,
    expected_grades                  TEXT    NOT NULL,
    actual_grades                    TEXT    NOT NULL,
    course_code                      TEXT    NOT NULL,
    term_id                          INTEGER NOT NULL,
    instructor_id                    INTEGER NOT NULL,
    FOREIGN KEY (course_code) REFERENCES courses (code),
    FOREIGN KEY (term_id) REFERENCES terms (id),
    FOREIGN KEY (instructor_id) REFERENCES instructors (id)
);

CREATE TABLE terms
(
    id   INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    name TEXT    NOT NULL UNIQUE
);

CREATE TABLE instructors
(
    id   INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    name TEXT    NOT NULL UNIQUE
);