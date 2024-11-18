// @generated automatically by Diesel CLI.

diesel::table! {
    courses (code) {
        code -> Text,
        name -> Text,
        unit_id -> Integer,
    }
}

diesel::table! {
    evaluations (sid) {
        sid -> Integer,
        section_name -> Text,
        enrollment -> Integer,
        responses -> Integer,
        class_helped_understanding -> Text,
        assignments_helped_understanding -> Text,
        fair_exams -> Text,
        timely_feedback -> Text,
        developed_understanding -> Text,
        engaging -> Text,
        communication -> Text,
        help_opportunities -> Text,
        effective_methods -> Text,
        timeliness -> Text,
        welcoming -> Text,
        materials -> Text,
        hours -> Text,
        expected_grades -> Text,
        actual_grades -> Text,
        course_code -> Text,
        term_id -> Integer,
        instructor_id -> Integer,
    }
}

diesel::table! {
    instructors (id) {
        id -> Integer,
        name -> Text,
    }
}

diesel::table! {
    sids (sid) {
        sid -> Integer,
        course_code -> Text,
    }
}

diesel::table! {
    terms (id) {
        id -> Integer,
        name -> Text,
    }
}

diesel::table! {
    units (id) {
        id -> Integer,
        name -> Text,
    }
}

diesel::joinable!(courses -> units (unit_id));
diesel::joinable!(evaluations -> courses (course_code));
diesel::joinable!(evaluations -> instructors (instructor_id));
diesel::joinable!(evaluations -> terms (term_id));
diesel::joinable!(sids -> courses (course_code));

diesel::allow_tables_to_appear_in_same_query!(
    courses,
    evaluations,
    instructors,
    sids,
    terms,
    units,
);
