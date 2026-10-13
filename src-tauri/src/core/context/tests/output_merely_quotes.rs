use super::*;

    /// An output that only quotes a notice is cut as the output it is: the numbers have to add
    /// up, the handle has to be one this store holds, and a number too big to be a length is no
    /// number at all.
    #[test]
    pub(super) fn an_output_that_merely_quotes_a_notice_is_cut_as_output() {
        let store = crate::core::observations::ObservationStore::for_session(&obs_scratch(), "s");
        let (a, b) = ("a".repeat(8_000), "b".repeat(12_000));
        let quoting = |notice: &str| format!("{a}\n\n{notice}\n\n{b}");
        for fake in [
            quoting(&cut_notice(5, Some(("0123456789ab", 999)))),
            quoting(&cut_notice(5, Some(("0123456789ab", 20_005)))),
            quoting(&format!("… [{} characters truncated to save context — re-read the source if you need the middle] …", "9".repeat(40))),
        ] {
            let cut = middle_out_recorded(&fake, TOOL_RESULT_CHAR_LIMIT, Some(&store));
            assert_eq!(store.get(&handle_in(&cut)).unwrap(), fake, "stored and cut as the output it is");
        }
    }
