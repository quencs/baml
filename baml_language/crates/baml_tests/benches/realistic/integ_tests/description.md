# Integration Tests Benchmark

This benchmark compiles the complete `integ-tests/baml_src` project, which contains
approximately 120 BAML files representing a realistic, production-like BAML project.

## Purpose

This benchmark measures:
- Full project compilation time for a large, realistic BAML project
- Performance characteristics with many interdependent files
- Real-world compilation scenarios

## Project Statistics

- ~120 BAML files
- Multiple client configurations
- Various function definitions with complex prompts
- Nested class hierarchies
- Enum definitions with aliases
- Dynamic types
- Media file references (images, audio, video, PDF)

## When to Use

Use this benchmark to validate compiler performance on realistic workloads
and detect regressions that might not appear in smaller synthetic tests.
