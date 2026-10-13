use super::super::*;

mod recall;
mod discipline;
mod shell;
mod handing_work_assistant;

pub static HELD_OUT: std::sync::LazyLock<Vec<Task>> =
    std::sync::LazyLock::new(|| [recall::RECALL, discipline::DISCIPLINE, shell::SHELL, handing_work_assistant::HANDING_WORK_ASSISTANT].concat());
