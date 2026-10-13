use super::super::*;

mod brain;
mod instruction_following;
mod recall;
mod writing;
mod discipline;
mod standing_instructions;
mod browser;

pub static TASKS: std::sync::LazyLock<Vec<Task>> =
    std::sync::LazyLock::new(|| [brain::BRAIN, instruction_following::INSTRUCTION_FOLLOWING, recall::RECALL, writing::WRITING, discipline::DISCIPLINE, standing_instructions::STANDING_INSTRUCTIONS, browser::BROWSER].concat());
