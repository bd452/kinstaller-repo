//! SignalKit for Kindle — a Rust port of the SignalKit reactive UI framework,
//! targeting e-ink e-readers.
//!
//! The reactive model is faithful to the original: [`Signal`]s hold state,
//! [`Component`]s build a widget tree once via [`Component::build`], and signal
//! changes mutate widget properties directly (no re-render, no diffing).
//! [`structural::slot`] and [`structural::for_each`] are the only pieces that
//! change the tree after mount.
//!
//! What the port adds — because there is no UIKit underneath — is a retained
//! [`widget`] tree, a [`layout`] solver, a [`render::Renderer`] abstraction
//! (with an FBInk framebuffer backend behind the `fbink` feature and a mock
//! backend for tests), touch [`input`], and an [`app::App`] event loop.
//!
//! A C ABI (behind the `capi` feature) makes the library usable from other
//! compiled languages.

pub mod app;
pub mod component;
pub mod disposable;
pub mod geometry;
pub mod input;
pub mod layout;
mod lifecycle;
pub mod node;
pub mod render;
pub mod signal;
pub mod structural;
pub mod widget;

#[cfg(feature = "capi")]
pub mod ffi;

// --- Flat re-exports of the primary API ---

pub use app::{App, ExitHandle};
pub use component::{BuildCtx, Component};
pub use disposable::Disposable;
pub use geometry::{Point, Rect, Size};
pub use node::{group, hstack, vstack, IntoNode, Node};
pub use render::{Color, DrawCmd, RefreshMode, Renderer};
pub use signal::Signal;
pub use structural::{for_each, slot};
pub use widget::{Align, AnyWidget, Axis, Button, Label, Spacer, Stack};

/// Library version string (from `Cargo.toml`).
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
