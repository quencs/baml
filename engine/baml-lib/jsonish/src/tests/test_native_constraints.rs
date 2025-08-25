use super::*;

/// Test suite for native BAML constraint expressions (non-Jinja)
/// 
/// These tests verify that native BAML expressions work correctly for constraints
/// alongside the existing Jinja expression support.

// Basic native constraint expressions  
const CLASS_WITH_NATIVE_CONSTRAINTS: &str = r#"
class Person {
  age int
    @check(age_reasonable, this < 150)
    @check(age_adult, this >= 18)
    @assert(age_positive, this >= 0)
  name string
    @assert(name_not_empty, this.len() > 0)
    @check(name_length, this.len() <= 50)
  score float
    @check(score_range, this >= 0.0 && this <= 100.0)
}
"#;

// Test basic native constraint functionality
test_deserializer_with_expected_score!(
    test_native_constraints_all_pass,
    CLASS_WITH_NATIVE_CONSTRAINTS,
    r#"{"age": 25, "name": "John", "score": 85.5}"#,
    TypeIR::class("Person"),
    5
);

test_deserializer_with_expected_score!(
    test_native_constraints_some_checks_fail,
    CLASS_WITH_NATIVE_CONSTRAINTS,
    r#"{"age": 160, "name": "VeryLongNameThatExceedsFiftyCharactersLimitTest", "score": -10.0}"#,
    TypeIR::class("Person"),
    5
);

test_failing_deserializer!(
    test_native_constraints_assert_fails,
    CLASS_WITH_NATIVE_CONSTRAINTS,
    r#"{"age": -5, "name": "John", "score": 50.0}"#,
    TypeIR::class("Person")
);

test_failing_deserializer!(
    test_native_constraints_empty_name_fails,
    CLASS_WITH_NATIVE_CONSTRAINTS,
    r#"{"age": 25, "name": "", "score": 85.5}"#,
    TypeIR::class("Person")
);

// Boolean logic and comparison operations
const CLASS_WITH_COMPLEX_NATIVE_LOGIC: &str = r#"
class Product {
  price float
    @check(price_reasonable, this > 0.0 && this < 10000.0)
    @assert(price_positive, this >= 0.0)
  quantity int
    @check(bulk_order, this >= 100 || this <= 10)
    @assert(quantity_valid, this > 0)
  is_available bool
    @check(availability_check, this == true)
}
"#;

test_deserializer_with_expected_score!(
    test_complex_native_logic_pass,
    CLASS_WITH_COMPLEX_NATIVE_LOGIC,
    r#"{"price": 29.99, "quantity": 5, "is_available": true}"#,
    TypeIR::class("Product"),
    4
);

test_deserializer_with_expected_score!(
    test_complex_native_logic_some_fail,
    CLASS_WITH_COMPLEX_NATIVE_LOGIC,
    r#"{"price": 29.99, "quantity": 50, "is_available": false}"#,
    TypeIR::class("Product"),
    4
);

// Arithmetic operations
const CLASS_WITH_ARITHMETIC_CONSTRAINTS: &str = r#"
class Circle {
  radius float
    @check(radius_small, this * 2 < 20.0)
    @assert(radius_positive, this > 0.0)
  area float
    @check(area_formula, this == 3.14159 * this * this)
}
"#;

test_deserializer_with_expected_score!(
    test_arithmetic_constraints,
    CLASS_WITH_ARITHMETIC_CONSTRAINTS,
    r#"{"radius": 5.0, "area": 78.53975}"#,
    TypeIR::class("Circle"),
    3
);

// String operations and method calls
const CLASS_WITH_STRING_OPERATIONS: &str = r#"
class User {
  username string
    @check(username_length, this.len() >= 3 && this.len() <= 20)
    @assert(username_not_empty, this.len() > 0)
  email string  
    @check(email_format, this.len() > 5)
    @assert(email_not_empty, this.len() > 0)
}
"#;

test_deserializer_with_expected_score!(
    test_string_operations_pass,
    CLASS_WITH_STRING_OPERATIONS,
    r#"{"username": "john_doe", "email": "john@example.com"}"#,
    TypeIR::class("User"),
    4
);

test_deserializer_with_expected_score!(
    test_string_operations_length_fail,
    CLASS_WITH_STRING_OPERATIONS,
    r#"{"username": "jo", "email": "a@b.c"}"#,
    TypeIR::class("User"),
    4
);

// Array/list operations
const CLASS_WITH_ARRAY_CONSTRAINTS: &str = r#"
class Team {
  members string[]
    @check(team_size, this.len() >= 2 && this.len() <= 10)
    @assert(has_members, this.len() > 0)
  scores int[]
    @check(scores_count, this.len() == 5)
}
"#;

test_deserializer_with_expected_score!(
    test_array_constraints_pass,
    CLASS_WITH_ARRAY_CONSTRAINTS,
    r#"{"members": ["Alice", "Bob", "Charlie"], "scores": [85, 90, 78, 92, 88]}"#,
    TypeIR::class("Team"),
    3
);

test_deserializer_with_expected_score!(
    test_array_constraints_fail,
    CLASS_WITH_ARRAY_CONSTRAINTS,
    r#"{"members": ["Alice"], "scores": [85, 90]}"#,
    TypeIR::class("Team"),
    3
);

test_failing_deserializer!(
    test_array_constraints_empty_members,
    CLASS_WITH_ARRAY_CONSTRAINTS,
    r#"{"members": [], "scores": [85, 90, 78, 92, 88]}"#,
    TypeIR::class("Team")
);

// Map operations
const CLASS_WITH_MAP_CONSTRAINTS: &str = r#"
class Configuration {
  settings map<string, int>
    @check(has_settings, this.len() > 0)
    @check(max_timeout_setting, this["timeout"] != null && this["timeout"] <= 3000)
}
"#;

test_deserializer_with_expected_score!(
    test_map_constraints_pass,
    CLASS_WITH_MAP_CONSTRAINTS,
    r#"{"settings": {"timeout": 1000, "retries": 3, "buffer_size": 1024}}"#,
    TypeIR::class("Configuration"),
    2
);

test_deserializer_with_expected_score!(
    test_map_constraints_timeout_fail,
    CLASS_WITH_MAP_CONSTRAINTS,
    r#"{"settings": {"timeout": 5000, "retries": 3}}"#,
    TypeIR::class("Configuration"),
    2
);

// Nested class constraints
const NESTED_NATIVE_CONSTRAINTS: &str = r#"
class Address {
  zip_code string
    @check(zip_length, this.len() == 5)
    @assert(zip_not_empty, this.len() > 0)
}

class Person {
  name string
    @assert(name_not_empty, this.len() > 0)
  age int
    @check(age_adult, this >= 18)
  address Address
}
"#;

test_deserializer_with_expected_score!(
    test_nested_native_constraints_pass,
    NESTED_NATIVE_CONSTRAINTS,
    r#"{"name": "Jane", "age": 30, "address": {"zip_code": "12345"}}"#,
    TypeIR::class("Person"),
    3
);

test_deserializer_with_expected_score!(
    test_nested_native_constraints_zip_fail,
    NESTED_NATIVE_CONSTRAINTS,
    r#"{"name": "Jane", "age": 16, "address": {"zip_code": "123"}}"#,
    TypeIR::class("Person"),
    3
);

// Union types with native constraints
const UNION_WITH_NATIVE_CONSTRAINTS: &str = r#"
class Student {
  grade int
    @check(student_grade, this >= 0 && this <= 100)
}

class Teacher {
  years_experience int
    @check(teacher_experience, this >= 0)
}

class School {
  person Student | Teacher
  count int
    @check(reasonable_count, this >= 1 && this <= 1000)
}
"#;

test_deserializer_with_expected_score!(
    test_union_native_constraints_student,
    UNION_WITH_NATIVE_CONSTRAINTS,
    r#"{"person": {"grade": 85}, "count": 25}"#,
    TypeIR::class("School"),
    3
);

test_deserializer_with_expected_score!(
    test_union_native_constraints_teacher,
    UNION_WITH_NATIVE_CONSTRAINTS,
    r#"{"person": {"years_experience": 10}, "count": 100}"#,
    TypeIR::class("School"),
    3
);

// Block-level (class-level) native constraints
const CLASS_LEVEL_NATIVE_CONSTRAINTS: &str = r#"
class Rectangle {
  width float
  height float
  @@assert(positive_dimensions, this.width > 0.0 && this.height > 0.0)
  @@check(reasonable_ratio, this.width / this.height >= 0.1 && this.width / this.height <= 10.0)
}
"#;

test_deserializer_with_expected_score!(
    test_class_level_native_constraints_pass,
    CLASS_LEVEL_NATIVE_CONSTRAINTS,
    r#"{"width": 10.0, "height": 5.0}"#,
    TypeIR::class("Rectangle"),
    2
);

test_deserializer_with_expected_score!(
    test_class_level_native_constraints_ratio_fail,
    CLASS_LEVEL_NATIVE_CONSTRAINTS,
    r#"{"width": 1.0, "height": 50.0}"#,
    TypeIR::class("Rectangle"),
    2
);

test_failing_deserializer!(
    test_class_level_native_constraints_negative_dimension,
    CLASS_LEVEL_NATIVE_CONSTRAINTS,
    r#"{"width": -5.0, "height": 10.0}"#,
    TypeIR::class("Rectangle")
);

// Mixed Jinja and native expressions in the same class
const MIXED_CONSTRAINT_TYPES: &str = r#"
class MixedExample {
  value int
    @check(jinja_check, {{ this > 10 }})
    @check(native_check, this < 100)
    @assert(jinja_assert, {{ this >= 0 }})
    @assert(native_assert, this != 50)
  name string
    @check(jinja_name_check, {{ this|length > 0 }})
    @check(native_name_check, this.len() <= 20)
}
"#;

test_deserializer_with_expected_score!(
    test_mixed_constraint_types_all_pass,
    MIXED_CONSTRAINT_TYPES,
    r#"{"value": 25, "name": "test"}"#,
    TypeIR::class("MixedExample"),
    4
);

test_deserializer_with_expected_score!(
    test_mixed_constraint_types_some_fail,
    MIXED_CONSTRAINT_TYPES,
    r#"{"value": 5, "name": "very_long_name_that_exceeds_limit"}"#,
    TypeIR::class("MixedExample"),
    4
);

test_failing_deserializer!(
    test_mixed_constraint_types_assert_fail,
    MIXED_CONSTRAINT_TYPES,
    r#"{"value": 50, "name": "test"}"#,
    TypeIR::class("MixedExample")
);

// Enum with native constraints
const ENUM_WITH_NATIVE_CONSTRAINTS: &str = r#"
enum Priority {
  LOW
  MEDIUM
  HIGH
  URGENT
  @@check(valid_priority, this == "HIGH" || this == "MEDIUM")
}

class Task {
  priority Priority
  urgency_level int
    @check(urgency_matches_priority, this >= 1 && this <= 5)
}
"#;

test_deserializer_with_expected_score!(
    test_enum_native_constraints_pass,
    ENUM_WITH_NATIVE_CONSTRAINTS,
    r#"{"priority": "HIGH", "urgency_level": 4}"#,
    TypeIR::class("Task"),
    2
);

test_deserializer_with_expected_score!(
    test_enum_native_constraints_priority_fail,
    ENUM_WITH_NATIVE_CONSTRAINTS,
    r#"{"priority": "URGENT", "urgency_level": 5}"#,
    TypeIR::class("Task"),
    2
);

// Complex nested expressions with method chaining
const COMPLEX_NATIVE_EXPRESSIONS: &str = r#"
class DataSet {
  values float[]
    @check(reasonable_count, this.len() >= 3 && this.len() <= 1000)
    @assert(has_values, this.len() > 0)
  metadata map<string, string>
    @check(has_title, this["title"] != null)
    @check(metadata_size, this.len() <= 10)
}
"#;

test_deserializer_with_expected_score!(
    test_complex_native_expressions_pass,
    COMPLEX_NATIVE_EXPRESSIONS,
    r#"{"values": [1.5, 2.3, 4.7, 3.1], "metadata": {"title": "Test Dataset", "version": "1.0"}}"#,
    TypeIR::class("DataSet"),
    4
);

test_deserializer_with_expected_score!(
    test_complex_native_expressions_fail,
    COMPLEX_NATIVE_EXPRESSIONS,
    r#"{"values": [1.5, 2.3], "metadata": {"version": "1.0"}}"#,
    TypeIR::class("DataSet"),
    4
);

// Edge cases and error conditions
const EDGE_CASE_CONSTRAINTS: &str = r#"
class EdgeCases {
  nullable_field int?
    @check(null_or_positive, this == null || this > 0)
  zero_field int
    @check(exactly_zero, this == 0)
    @assert(not_negative, this >= 0)
  boolean_field bool
    @check(is_true, this == true)
}
"#;

test_deserializer_with_expected_score!(
    test_edge_cases_with_null,
    EDGE_CASE_CONSTRAINTS,
    r#"{"nullable_field": null, "zero_field": 0, "boolean_field": true}"#,
    TypeIR::class("EdgeCases"),
    4
);

test_deserializer_with_expected_score!(
    test_edge_cases_without_null,
    EDGE_CASE_CONSTRAINTS,
    r#"{"nullable_field": 42, "zero_field": 0, "boolean_field": false}"#,
    TypeIR::class("EdgeCases"),
    4
);

test_failing_deserializer!(
    test_edge_cases_negative_zero_field,
    EDGE_CASE_CONSTRAINTS,
    r#"{"nullable_field": null, "zero_field": -1, "boolean_field": true}"#,
    TypeIR::class("EdgeCases")
);