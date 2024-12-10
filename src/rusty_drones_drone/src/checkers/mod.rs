#![cfg(test)]
use std::time::Duration;

mod flood;
mod integrations;
mod wgl_test;

const TIMEOUT: Duration = Duration::from_millis(50);
