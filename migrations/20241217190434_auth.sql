CREATE TABLE
    tokens (
        id SERIAL PRIMARY KEY,
        token TEXT NOT NULL,
        name TEXT NOT NULL
    );

CREATE TABLE
    course_accesses (
        id SERIAL PRIMARY KEY,
        num INTEGER NOT NULL,
        token_id INTEGER NOT NULL REFERENCES tokens (id),
        course_code VARCHAR(100) NOT NULL REFERENCES courses (code) UNIQUE
    );

