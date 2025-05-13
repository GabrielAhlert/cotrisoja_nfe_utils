// src/modules/mod.rs
pub mod cli;
pub mod watcher;
pub mod nfe;

// reexporta o que o main mais usa  ──► facilita o `use` depois
pub use cli::{Args, Ambiente};
pub use watcher::watch;
pub use nfe::get_nfe;
