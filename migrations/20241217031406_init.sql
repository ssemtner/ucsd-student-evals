CREATE TABLE
    units (id SERIAL PRIMARY KEY, name VARCHAR(100) NOT NULL);

CREATE TABLE
    courses (
        code VARCHAR(100) PRIMARY KEY,
        name VARCHAR(100) NOT NULL,
        unit_id INTEGER NOT NULL REFERENCES units (id)
    );

CREATE TABLE
    sids (
        sid SERIAL PRIMARY KEY,
        course_code VARCHAR(100) NOT NULL REFERENCES courses (code)
    );

CREATE TABLE
    terms (
        id SERIAL PRIMARY KEY,
        name VARCHAR(100) NOT NULL UNIQUE
    );

CREATE TABLE
    instructors (id SERIAL PRIMARY KEY, name TEXT NOT NULL UNIQUE);

CREATE TABLE
    evaluations (
        sid SERIAL PRIMARY KEY,
        section_name VARCHAR(100) NOT NULL,
        enrollment INTEGER NOT NULL,
        responses INTEGER NOT NULL,
        class_helped_understanding INTEGER[] NOT NULL,
        assignments_helped_understanding INTEGER[] NOT NULL,
        fair_exams INTEGER[] NOT NULL,
        timely_feedback INTEGER[] NOT NULL,
        developed_understanding INTEGER[] NOT NULL,
        engaging INTEGER[] NOT NULL,
        communication INTEGER[] NOT NULL,
        help_opportunities INTEGER[] NOT NULL,
        effective_methods INTEGER[] NOT NULL,
        timeliness INTEGER[] NOT NULL,
        welcoming INTEGER[] NOT NULL,
        materials INTEGER[] NOT NULL,
        hours INTEGER[] NOT NULL,
        expected_grades INTEGER[] NOT NULL,
        actual_grades INTEGER[] NOT NULL,
        course_code VARCHAR(100) NOT NULL REFERENCES courses (code),
        term_id INTEGER NOT NULL REFERENCES terms (id),
        instructor_id INTEGER NOT NULL REFERENCES instructors (id)
    );
