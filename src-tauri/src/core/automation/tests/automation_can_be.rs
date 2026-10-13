use super::*;

    #[test]
    pub(super) fn the_newest_unread_run_decides_the_tint_not_the_worst_one() {
        // A failure followed by a success is a task that recovered. Showing it as
        // failing would send you to a run that is no longer the story.
        let dir = scratch();
        let mut store = AutomationStore::load(&dir);
        let id = store
            .create(ScheduledTask::new("Brief".into(), "x".into(), daily("0 7 * * *"), "/tmp".into(), None, None))
            .id;
        let bad = store.start_run(&id, "schedule").unwrap();
        store.finish_run(&id, &bad.run_id, "error", None, vec![], Some("boom".into())).unwrap();
        let good = store.start_run(&id, "schedule").unwrap();
        store.finish_run(&id, &good.run_id, "ok", Some("fine".into()), vec![], None).unwrap();
        assert_eq!(store.unseen(&id), (2, false));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    pub(super) fn marking_an_unknown_task_seen_says_so() {
        let dir = scratch();
        let mut store = AutomationStore::load(&dir);
        assert!(store.mark_seen("task-nope").is_err());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    pub(super) fn an_enabled_task_with_no_next_run_is_healed_rather_than_left_dead() {
        // `due()` compares `next_run <= now`, and None is not <= anything — so such a
        // task sits in the list looking armed and never fires, silently. Only
        // `create` and `set_enabled` computed one, so any other route to a null was
        // permanent.
        let dir = scratch();
        let mut store = AutomationStore::load(&dir);
        let id = store
            .create(ScheduledTask::new("Brief".into(), "x".into(), daily("0 7 * * *"), "/tmp".into(), None, None))
            .id;
        // Knock it out the way a hand-edited file would.
        store.get_mut(&id).unwrap().next_run = None;
        store.save();

        let healed = AutomationStore::load(&dir);
        assert!(healed.get(&id).unwrap().next_run.is_some(), "an enabled task must always know when it next fires");

        // A DISABLED task keeps its null: it is off, and inventing a fire time for it
        // would make "off" mean "armed but ignored".
        let mut off = AutomationStore::load(&dir);
        off.set_enabled(&id, false).unwrap();
        off.get_mut(&id).unwrap().next_run = None;
        off.save();
        assert!(AutomationStore::load(&dir).get(&id).unwrap().next_run.is_none());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    pub(super) fn store_crud_and_runs_persist() {
        let dir = scratch();
        let mut store = AutomationStore::load(&dir);
        let task = store.create(ScheduledTask::new("Brief".into(), "write a brief".into(), daily("0 7 * * *"), "/tmp".into(), Some("assistant".into()), None));
        let id = task.id.clone();

        let run = store.start_run(&id, "manual").unwrap();
        store.finish_run(&id, &run.run_id, "ok", Some("done".into()), vec!["brief.md".into()], None).unwrap();

        // Reload from disk to prove persistence.
        let store2 = AutomationStore::load(&dir);
        assert_eq!(store2.list().len(), 1);
        assert_eq!(store2.get(&id).unwrap().run_count, 1);
        assert_eq!(store2.get(&id).unwrap().last_status.as_deref(), Some("ok"));
        let runs = store2.list_runs(&id);
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].artifacts, vec!["brief.md"]);

        let mut store3 = AutomationStore::load(&dir);
        assert!(store3.delete(&id));
        assert!(AutomationStore::load(&dir).list().is_empty());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    pub(super) fn an_automation_can_be_retitled_reworded_and_rescheduled() {
        // The detail view offered all three and none of them worked — the route took
        // only `enabled` and `workspace`, so every save answered "nothing to update".
        let dir = scratch();
        let mut store = AutomationStore::load(&dir);
        let t = store.create(ScheduledTask::new("Old".into(), "do the old thing".into(), daily("0 7 * * *"), "/tmp".into(), None, None));
        let id = t.id.clone();
        let first_next = t.next_run;

        store.edit(&id, Some("New"), Some("do the new thing"), Some("0 20 * * *")).unwrap();
        let after = AutomationStore::load(&dir);
        let t2 = after.get(&id).unwrap();
        assert_eq!(t2.title, "New");
        assert_eq!(t2.instructions, "do the new thing");
        assert_eq!(t2.schedule.cron.as_deref(), Some("0 20 * * *"));
        // A changed schedule with an unchanged next_run is a task that fires at the
        // old time while claiming the new one.
        assert_ne!(t2.next_run, first_next, "rescheduling must recompute the next run");

        // One field at a time leaves the others alone.
        store.edit(&id, Some("Newer"), None, None).unwrap();
        let t3 = AutomationStore::load(&dir);
        let t3 = t3.get(&id).unwrap();
        assert_eq!(t3.title, "Newer");
        assert_eq!(t3.instructions, "do the new thing");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    pub(super) fn a_schedule_that_never_fires_is_refused_before_anything_is_written() {
        // A half-applied edit is worse than a refused one: the title would move and
        // the schedule would not, and the caller can only report one outcome.
        let dir = scratch();
        let mut store = AutomationStore::load(&dir);
        let t = store.create(ScheduledTask::new("Keep".into(), "keep".into(), daily("0 7 * * *"), "/tmp".into(), None, None));
        let id = t.id.clone();

        assert!(store.edit(&id, Some("Changed"), None, Some("not a cron")).is_err());
        let after = AutomationStore::load(&dir);
        assert_eq!(after.get(&id).unwrap().title, "Keep", "nothing may be written when the edit is refused");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    pub(super) fn each_automation_keeps_its_own_model() {
        // The point of the feature: a nightly digest on something small and a weekly
        // review on something capable, in the same app, without touching a setting
        // between them.
        let dir = scratch();
        let mut store = AutomationStore::load(&dir);
        let a = store.create(ScheduledTask::new("Digest".into(), "summarise".into(), daily("0 7 * * *"), "/tmp".into(), None, Some("gemma4:12b".into())));
        let b = store.create(ScheduledTask::new("Review".into(), "review".into(), daily("0 8 * * *"), "/tmp".into(), None, Some("claude-opus-5".into())));

        let reloaded = AutomationStore::load(&dir);
        assert_eq!(reloaded.get(&a.id).unwrap().model.as_deref(), Some("gemma4:12b"));
        assert_eq!(reloaded.get(&b.id).unwrap().model.as_deref(), Some("claude-opus-5"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    pub(super) fn a_model_can_be_changed_and_put_back_to_the_default() {
        // `None` is a real choice — "run on whatever the app would pick" — and has to
        // be reachable, or pinning a model once would be permanent. An empty string is
        // the same request, because that is what a picker's "Default" option sends.
        let dir = scratch();
        let mut store = AutomationStore::load(&dir);
        let t = store.create(ScheduledTask::new("Brief".into(), "write".into(), daily("0 7 * * *"), "/tmp".into(), None, None));
        let id = t.id.clone();
        assert_eq!(t.model, None, "a new automation follows the app by default");

        assert_eq!(store.set_model(&id, Some("gemma4:12b")).unwrap().as_deref(), Some("gemma4:12b"));
        assert_eq!(AutomationStore::load(&dir).get(&id).unwrap().model.as_deref(), Some("gemma4:12b"));

        assert_eq!(store.set_model(&id, Some("  ")).unwrap(), None, "blank means default");
        assert_eq!(AutomationStore::load(&dir).get(&id).unwrap().model, None);

        store.set_model(&id, Some("claude-opus-5")).unwrap();
        assert_eq!(store.set_model(&id, None).unwrap(), None, "and so does null");
        assert_eq!(AutomationStore::load(&dir).get(&id).unwrap().model, None);

        assert!(store.set_model("no-such-task", Some("x")).is_err());

        let _ = fs::remove_dir_all(&dir);
    }
