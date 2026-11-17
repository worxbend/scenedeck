//! Fitness tests for the module boundaries described in `CLAUDE.md`.
//!
//! Each boundary here was written down before it was checked, and at least
//! one of them had already been broken while the documentation still said
//! otherwise. Nothing failed when that happened, because a documented rule
//! that no check enforces is a suggestion. These tests turn the prose into
//! something that can be violated only on purpose.

#![cfg(test)]

mod rules;
mod scanner;
mod scanner_tests;
