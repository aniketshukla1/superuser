//! the assistants' browsers

mod persona_remote_test;
mod agui_run;
mod handle_computer_socket;
mod expenses_status;
mod perf_check;
mod settings_api;
mod settings_set_scratch;
mod settings_set_approvals;
mod providers_list;
mod provider;
mod automation_api;
mod inbox_resolve_api;
mod feedback_post;
mod workspace_trust_set;

pub(super) use persona_remote_test::*;
pub(super) use agui_run::*;
pub(super) use handle_computer_socket::*;
pub(super) use expenses_status::*;
pub(super) use perf_check::*;
pub(super) use settings_api::*;
pub(super) use settings_set_scratch::*;
pub(super) use settings_set_approvals::*;
pub(super) use providers_list::*;
pub(super) use provider::*;
pub(super) use automation_api::*;
pub(super) use inbox_resolve_api::*;
pub(super) use feedback_post::*;
pub(super) use workspace_trust_set::*;
