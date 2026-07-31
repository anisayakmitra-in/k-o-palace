//! K-O Palace — Open AI Runtime Registry
//!
//! A production-ready, runtime-agnostic AI package registry implementing the
//! KUBER manifest specification. This crate separates routes, state, storage,
//! authentication, validation, trust, search, and configuration.

#![deny(unsafe_code)]

pub mod app;
pub mod artifact;
pub mod auth;
pub mod config;
pub mod error;
pub mod models;
pub mod pagination;
pub mod repository;
pub mod routes;
pub mod search;
pub mod security;
pub mod trust;
pub mod validation;
