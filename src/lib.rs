//! K-O Palace — Open AI Runtime Registry
//!
//! A production-ready, runtime-agnostic AI package registry implementing the
//! K-O Palace manifest specification. This crate separates routes, state, storage,
//! authentication, validation, trust, search, and configuration.

#![deny(unsafe_code)]

pub mod app;
pub mod artifact;
pub mod auth;
pub mod config;
pub mod error;
pub mod identity;
pub mod models;
pub mod pagination;
pub mod rate_limit;
pub mod repository;
pub mod request_id;
pub mod resolve;
pub mod routes;
pub mod search;
pub mod security;
pub mod trust;
pub mod validation;
