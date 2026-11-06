use super::*;

pub(super) fn talent_tools(ctx: &ToolCtx<'_>, name: &str, args: &Value) -> String {
    use crate::core::talent;
    // What comes back holds names and profiles found on other people's pages.
    ctx.taint();
    match name {
        "candidates_check" => talent::check_tool(&ctx.data_dir, &ctx.settings, args),
        _ => talent::shortlist_tool(&ctx.data_dir, &ctx.settings, args, &chrono::Local::now().format("%Y-%m-%d").to_string()),
    }
}
