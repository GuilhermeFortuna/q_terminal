#[derive(Debug, Clone, PartialEq)]
pub struct BarColumns {
    pub time: Vec<i64>,
    pub open: Vec<f64>,
    pub high: Vec<f64>,
    pub low: Vec<f64>,
    pub close: Vec<f64>,
    pub volume: Vec<f64>,
}

impl BarColumns {
    pub fn new(
        time: Vec<i64>,
        open: Vec<f64>,
        high: Vec<f64>,
        low: Vec<f64>,
        close: Vec<f64>,
        volume: Vec<f64>,
    ) -> Self {
        Self {
            time,
            open,
            high,
            low,
            close,
            volume,
        }
    }

    pub fn single(time: i64, open: f64, high: f64, low: f64, close: f64, volume: f64) -> Self {
        Self {
            time: vec![time],
            open: vec![open],
            high: vec![high],
            low: vec![low],
            close: vec![close],
            volume: vec![volume],
        }
    }

    pub fn len(&self) -> usize {
        self.time.len()
    }

    pub fn is_empty(&self) -> bool {
        self.time.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Phase {
    Subscribing,
    Buffering,
    Live,
    AwaitingHistory { from_seq: i64, until_seq: i64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DropReason {
    WrongSymbol,
    WrongTimeframe,
    DuplicateSequence,
    BelowWatermark,
    UnsubscribedTopic,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Action<P = BarColumns> {
    Apply(P),
    FetchHistory { from_seq: i64 },
    ReSnapshot,
    Drop(DropReason),
}

#[derive(Debug, Clone, PartialEq)]
pub struct PendingEntry<P = BarColumns> {
    pub seq: i64,
    pub epoch: String,
    pub symbol: String,
    pub timeframe: String,
    pub payload: P,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TopicState<P = BarColumns> {
    pub topic: String,
    pub is_coalescing: bool,
    pub filter: TopicFilter,
    pub phase: Phase,
    pub epoch: Option<String>,
    pub last_applied_seq: Option<i64>,
    pub buffered: Vec<PendingEntry<P>>,
}

/// Which entries of a topic the client keeps. Execution topics carry no routing filter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TopicFilter {
    Bars { symbol: String, timeframe: String },
    None,
}

impl<P> TopicState<P> {
    /// A state for a topic whose entries are all relevant (execution topics).
    pub fn unfiltered(topic: &str) -> Self {
        Self {
            topic: topic.to_string(),
            is_coalescing: false,
            filter: TopicFilter::None,
            phase: Phase::Subscribing,
            epoch: None,
            last_applied_seq: None,
            buffered: Vec::new(),
        }
    }
}

impl TopicState<BarColumns> {
    pub fn new(
        topic: &str,
        is_coalescing: bool,
        target_symbol: &str,
        target_timeframe: &str,
    ) -> Self {
        Self {
            topic: topic.to_string(),
            is_coalescing,
            filter: TopicFilter::Bars {
                symbol: target_symbol.to_string(),
                timeframe: target_timeframe.to_string(),
            },
            phase: Phase::Subscribing,
            epoch: None,
            last_applied_seq: None,
            buffered: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Event<P = BarColumns> {
    Subscribed {
        epoch: String,
        last_seq: i64,
    },
    Snapshot {
        epoch: String,
        seq: i64,
        payload: P,
    },
    Entry {
        epoch: String,
        seq: i64,
        symbol: String,
        timeframe: String,
        payload: P,
    },
    HistoryPage {
        epoch: String,
        entries: Vec<(i64, P)>,
        next_seq: Option<i64>,
    },
    HistoryExpired,
    Lagging {
        from_seq: i64,
    },
    CursorExpired,
    EpochChanged {
        new_epoch: String,
    },
}

pub fn on_event<P>(state: &mut TopicState<P>, event: Event<P>) -> Vec<Action<P>> {
    let mut actions = Vec::new();

    match event {
        Event::Subscribed { epoch, last_seq: _ } => {
            state.epoch = Some(epoch);
            state.phase = Phase::Buffering;
        }

        Event::Snapshot {
            epoch,
            seq,
            payload,
        } => {
            state.epoch = Some(epoch);
            state.last_applied_seq = Some(seq);
            state.phase = Phase::Live;
            actions.push(Action::Apply(payload));

            let mut buffered = std::mem::take(&mut state.buffered);
            buffered.sort_by_key(|e| e.seq);

            let mut remaining = Vec::new();
            let mut gap_found = false;

            for entry in buffered {
                if gap_found {
                    remaining.push(entry);
                    continue;
                }

                let last_seq = state.last_applied_seq.unwrap_or(0);
                if entry.seq <= last_seq {
                    actions.push(Action::Drop(DropReason::BelowWatermark));
                } else if entry.seq == last_seq + 1 {
                    state.last_applied_seq = Some(entry.seq);
                    actions.push(Action::Apply(entry.payload));
                } else {
                    gap_found = true;
                    if state.is_coalescing {
                        actions.push(Action::ReSnapshot);
                        state.phase = Phase::Buffering;
                        remaining.push(entry);
                        break;
                    } else {
                        let from_seq = last_seq + 1;
                        let until_seq = entry.seq - 1;
                        state.phase = Phase::AwaitingHistory {
                            from_seq,
                            until_seq,
                        };
                        actions.push(Action::FetchHistory { from_seq });
                        remaining.push(entry);
                    }
                }
            }
            state.buffered = remaining;
        }

        Event::Entry {
            epoch,
            seq,
            symbol,
            timeframe,
            payload,
        } => {
            if let TopicFilter::Bars {
                symbol: target_symbol,
                timeframe: target_timeframe,
            } = &state.filter
            {
                if symbol != *target_symbol {
                    return vec![Action::Drop(DropReason::WrongSymbol)];
                }
                if timeframe != *target_timeframe {
                    return vec![Action::Drop(DropReason::WrongTimeframe)];
                }
            }

            match &state.phase {
                Phase::Subscribing | Phase::Buffering => {
                    state.buffered.push(PendingEntry {
                        seq,
                        epoch,
                        symbol,
                        timeframe,
                        payload,
                    });
                }
                Phase::AwaitingHistory { .. } => {
                    state.buffered.push(PendingEntry {
                        seq,
                        epoch,
                        symbol,
                        timeframe,
                        payload,
                    });
                }
                Phase::Live => {
                    let last_seq = state.last_applied_seq.unwrap_or(0);
                    if seq <= last_seq {
                        actions.push(Action::Drop(DropReason::DuplicateSequence));
                    } else if seq == last_seq + 1 {
                        state.last_applied_seq = Some(seq);
                        actions.push(Action::Apply(payload));
                    } else {
                        if state.is_coalescing {
                            actions.push(Action::ReSnapshot);
                            state.phase = Phase::Buffering;
                            state.buffered.push(PendingEntry {
                                seq,
                                epoch,
                                symbol,
                                timeframe,
                                payload,
                            });
                        } else {
                            let from_seq = last_seq + 1;
                            let until_seq = seq - 1;
                            state.phase = Phase::AwaitingHistory {
                                from_seq,
                                until_seq,
                            };
                            state.buffered.push(PendingEntry {
                                seq,
                                epoch,
                                symbol,
                                timeframe,
                                payload,
                            });
                            actions.push(Action::FetchHistory { from_seq });
                        }
                    }
                }
            }
        }

        Event::HistoryPage {
            epoch: _,
            entries,
            next_seq: _,
        } => {
            for (seq, payload) in entries {
                let last_seq = state.last_applied_seq.unwrap_or(0);
                if seq == last_seq + 1 {
                    state.last_applied_seq = Some(seq);
                    actions.push(Action::Apply(payload));
                } else if seq <= last_seq {
                    actions.push(Action::Drop(DropReason::DuplicateSequence));
                }
            }

            state.buffered.sort_by_key(|e| e.seq);
            let mut remaining = Vec::new();
            let mut gap_found = false;

            let buffered = std::mem::take(&mut state.buffered);
            for entry in buffered {
                if gap_found {
                    remaining.push(entry);
                    continue;
                }

                let last_seq = state.last_applied_seq.unwrap_or(0);
                if entry.seq <= last_seq {
                    actions.push(Action::Drop(DropReason::DuplicateSequence));
                } else if entry.seq == last_seq + 1 {
                    state.last_applied_seq = Some(entry.seq);
                    actions.push(Action::Apply(entry.payload));
                } else {
                    gap_found = true;
                    remaining.push(entry);
                }
            }

            state.buffered = remaining;
            if !gap_found {
                state.phase = Phase::Live;
            }
        }

        Event::HistoryExpired => {
            state.buffered.clear();
            state.phase = Phase::Buffering;
            actions.push(Action::ReSnapshot);
        }

        Event::Lagging { from_seq: _ } | Event::CursorExpired => {
            state.buffered.clear();
            state.phase = Phase::Buffering;
            actions.push(Action::ReSnapshot);
        }

        Event::EpochChanged { new_epoch } => {
            state.epoch = Some(new_epoch);
            state.last_applied_seq = None;
            state.buffered.clear();
            state.phase = Phase::Buffering;
            actions.push(Action::ReSnapshot);
        }
    }

    actions
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_bars(t: i64) -> BarColumns {
        BarColumns::single(t, 10.0, 11.0, 9.0, 10.5, 100.0)
    }

    #[test]
    fn test_buffer_before_snapshot() {
        let mut state = TopicState::new("bars.completed", false, "PETR4", "1m");
        on_event(
            &mut state,
            Event::Subscribed {
                epoch: "ep1".into(),
                last_seq: 10,
            },
        );

        let actions = on_event(
            &mut state,
            Event::Entry {
                epoch: "ep1".into(),
                seq: 11,
                symbol: "PETR4".into(),
                timeframe: "1m".into(),
                payload: dummy_bars(100),
            },
        );

        assert!(
            actions.is_empty(),
            "should buffer before snapshot without applying"
        );
        assert_eq!(state.buffered.len(), 1);
        assert_eq!(state.buffered[0].seq, 11);
    }

    #[test]
    fn test_discard_at_or_below_watermark() {
        let mut state = TopicState::new("bars.completed", false, "PETR4", "1m");
        on_event(
            &mut state,
            Event::Subscribed {
                epoch: "ep1".into(),
                last_seq: 10,
            },
        );

        // Buffer seq 10 and 11
        on_event(
            &mut state,
            Event::Entry {
                epoch: "ep1".into(),
                seq: 10,
                symbol: "PETR4".into(),
                timeframe: "1m".into(),
                payload: dummy_bars(100),
            },
        );
        on_event(
            &mut state,
            Event::Entry {
                epoch: "ep1".into(),
                seq: 11,
                symbol: "PETR4".into(),
                timeframe: "1m".into(),
                payload: dummy_bars(101),
            },
        );

        // Snapshot arrives with seq 11
        let actions = on_event(
            &mut state,
            Event::Snapshot {
                epoch: "ep1".into(),
                seq: 11,
                payload: dummy_bars(101),
            },
        );

        // Snapshot applied, buffered entries at or below seq 11 dropped
        assert_eq!(actions[0], Action::Apply(dummy_bars(101)));
        assert_eq!(actions[1], Action::Drop(DropReason::BelowWatermark));
        assert_eq!(actions[2], Action::Drop(DropReason::BelowWatermark));
        assert!(state.buffered.is_empty());
        assert_eq!(state.last_applied_seq, Some(11));
    }

    #[test]
    fn test_apply_above_watermark_in_order() {
        let mut state = TopicState::new("bars.completed", false, "PETR4", "1m");
        on_event(
            &mut state,
            Event::Subscribed {
                epoch: "ep1".into(),
                last_seq: 10,
            },
        );

        // Buffer seq 10, 12, 13
        on_event(
            &mut state,
            Event::Entry {
                epoch: "ep1".into(),
                seq: 10,
                symbol: "PETR4".into(),
                timeframe: "1m".into(),
                payload: dummy_bars(100),
            },
        );
        on_event(
            &mut state,
            Event::Entry {
                epoch: "ep1".into(),
                seq: 13,
                symbol: "PETR4".into(),
                timeframe: "1m".into(),
                payload: dummy_bars(103),
            },
        );
        on_event(
            &mut state,
            Event::Entry {
                epoch: "ep1".into(),
                seq: 12,
                symbol: "PETR4".into(),
                timeframe: "1m".into(),
                payload: dummy_bars(102),
            },
        );

        // Snapshot arrives with seq 11
        let actions = on_event(
            &mut state,
            Event::Snapshot {
                epoch: "ep1".into(),
                seq: 11,
                payload: dummy_bars(101),
            },
        );

        // Apply snapshot (11), drop 10, apply 12, apply 13 in order
        assert_eq!(actions[0], Action::Apply(dummy_bars(101)));
        assert_eq!(actions[1], Action::Drop(DropReason::BelowWatermark));
        assert_eq!(actions[2], Action::Apply(dummy_bars(102)));
        assert_eq!(actions[3], Action::Apply(dummy_bars(103)));
        assert_eq!(state.last_applied_seq, Some(13));
        assert!(state.buffered.is_empty());
    }

    #[test]
    fn test_duplicate_sequence_applied_once() {
        let mut state = TopicState::new("bars.completed", false, "PETR4", "1m");
        on_event(
            &mut state,
            Event::Subscribed {
                epoch: "ep1".into(),
                last_seq: 10,
            },
        );
        on_event(
            &mut state,
            Event::Snapshot {
                epoch: "ep1".into(),
                seq: 10,
                payload: dummy_bars(100),
            },
        );

        // First time seq 11 arrives -> Apply
        let a1 = on_event(
            &mut state,
            Event::Entry {
                epoch: "ep1".into(),
                seq: 11,
                symbol: "PETR4".into(),
                timeframe: "1m".into(),
                payload: dummy_bars(101),
            },
        );
        assert_eq!(a1, vec![Action::Apply(dummy_bars(101))]);

        // Second time seq 11 arrives -> Drop(DuplicateSequence)
        let a2 = on_event(
            &mut state,
            Event::Entry {
                epoch: "ep1".into(),
                seq: 11,
                symbol: "PETR4".into(),
                timeframe: "1m".into(),
                payload: dummy_bars(101),
            },
        );
        assert_eq!(a2, vec![Action::Drop(DropReason::DuplicateSequence)]);
    }

    #[test]
    fn test_gap_emitting_fetch_history_and_holding_later_entries() {
        let mut state = TopicState::new("bars.completed", false, "PETR4", "1m");
        on_event(
            &mut state,
            Event::Subscribed {
                epoch: "ep1".into(),
                last_seq: 10,
            },
        );
        on_event(
            &mut state,
            Event::Snapshot {
                epoch: "ep1".into(),
                seq: 10,
                payload: dummy_bars(100),
            },
        );

        // Live seq 13 arrives when expected is 11 -> Gap!
        let actions = on_event(
            &mut state,
            Event::Entry {
                epoch: "ep1".into(),
                seq: 13,
                symbol: "PETR4".into(),
                timeframe: "1m".into(),
                payload: dummy_bars(103),
            },
        );

        assert_eq!(actions, vec![Action::FetchHistory { from_seq: 11 }]);
        assert_eq!(state.buffered.len(), 1);
        assert_eq!(state.buffered[0].seq, 13);
    }

    #[test]
    fn test_history_closing_gap_and_releasing_buffer() {
        let mut state = TopicState::new("bars.completed", false, "PETR4", "1m");
        on_event(
            &mut state,
            Event::Subscribed {
                epoch: "ep1".into(),
                last_seq: 10,
            },
        );
        on_event(
            &mut state,
            Event::Snapshot {
                epoch: "ep1".into(),
                seq: 10,
                payload: dummy_bars(100),
            },
        );
        on_event(
            &mut state,
            Event::Entry {
                epoch: "ep1".into(),
                seq: 13,
                symbol: "PETR4".into(),
                timeframe: "1m".into(),
                payload: dummy_bars(103),
            },
        );

        // History arrives with 11 and 12
        let actions = on_event(
            &mut state,
            Event::HistoryPage {
                epoch: "ep1".into(),
                entries: vec![(11, dummy_bars(101)), (12, dummy_bars(102))],
                next_seq: Some(13),
            },
        );

        // Applies 11, 12 from history, then releases buffered 13
        assert_eq!(
            actions,
            vec![
                Action::Apply(dummy_bars(101)),
                Action::Apply(dummy_bars(102)),
                Action::Apply(dummy_bars(103)),
            ]
        );
        assert_eq!(state.last_applied_seq, Some(13));
        assert!(state.buffered.is_empty());
        assert_eq!(state.phase, Phase::Live);
    }

    #[test]
    fn test_expired_history_emitting_resnapshot() {
        let mut state = TopicState::new("bars.completed", false, "PETR4", "1m");
        on_event(
            &mut state,
            Event::Subscribed {
                epoch: "ep1".into(),
                last_seq: 10,
            },
        );
        on_event(
            &mut state,
            Event::Snapshot {
                epoch: "ep1".into(),
                seq: 10,
                payload: dummy_bars(100),
            },
        );
        on_event(
            &mut state,
            Event::Entry {
                epoch: "ep1".into(),
                seq: 15,
                symbol: "PETR4".into(),
                timeframe: "1m".into(),
                payload: dummy_bars(105),
            },
        );

        let actions = on_event(&mut state, Event::HistoryExpired);
        assert_eq!(actions, vec![Action::ReSnapshot]);
        assert!(state.buffered.is_empty());
    }

    #[test]
    fn test_epoch_changed_resetting_one_topic_only() {
        let mut state1 = TopicState::new("bars.completed", false, "PETR4", "1m");
        let mut state2 = TopicState::new("bars.forming", true, "PETR4", "1m");

        on_event(
            &mut state1,
            Event::Subscribed {
                epoch: "ep1".into(),
                last_seq: 10,
            },
        );
        on_event(
            &mut state1,
            Event::Snapshot {
                epoch: "ep1".into(),
                seq: 10,
                payload: dummy_bars(100),
            },
        );

        on_event(
            &mut state2,
            Event::Subscribed {
                epoch: "ep1".into(),
                last_seq: 20,
            },
        );
        on_event(
            &mut state2,
            Event::Snapshot {
                epoch: "ep1".into(),
                seq: 20,
                payload: dummy_bars(200),
            },
        );

        let actions1 = on_event(
            &mut state1,
            Event::EpochChanged {
                new_epoch: "ep2".into(),
            },
        );
        assert_eq!(actions1, vec![Action::ReSnapshot]);
        assert_eq!(state1.epoch, Some("ep2".into()));
        assert_eq!(state1.last_applied_seq, None);

        // state2 is untouched
        assert_eq!(state2.epoch, Some("ep1".into()));
        assert_eq!(state2.last_applied_seq, Some(20));
    }

    #[test]
    fn test_lagging_and_cursor_expired_emitting_resnapshot() {
        let mut state = TopicState::new("bars.completed", false, "PETR4", "1m");
        let a1 = on_event(&mut state, Event::Lagging { from_seq: 50 });
        assert_eq!(a1, vec![Action::ReSnapshot]);

        let a2 = on_event(&mut state, Event::CursorExpired);
        assert_eq!(a2, vec![Action::ReSnapshot]);
    }

    #[test]
    fn test_forming_topic_gap_emitting_resnapshot_rather_than_fetch_history() {
        // bars.forming has is_coalescing = true
        let mut state = TopicState::new("bars.forming", true, "PETR4", "1m");
        on_event(
            &mut state,
            Event::Subscribed {
                epoch: "ep1".into(),
                last_seq: 10,
            },
        );
        on_event(
            &mut state,
            Event::Snapshot {
                epoch: "ep1".into(),
                seq: 10,
                payload: dummy_bars(100),
            },
        );

        // Gap on coalescing topic emits ReSnapshot, NOT FetchHistory
        let actions = on_event(
            &mut state,
            Event::Entry {
                epoch: "ep1".into(),
                seq: 15,
                symbol: "PETR4".into(),
                timeframe: "1m".into(),
                payload: dummy_bars(105),
            },
        );
        assert_eq!(actions, vec![Action::ReSnapshot]);
    }

    #[test]
    fn test_entry_for_another_symbol_emitting_drop() {
        let mut state = TopicState::new("bars.completed", false, "PETR4", "1m");
        on_event(
            &mut state,
            Event::Subscribed {
                epoch: "ep1".into(),
                last_seq: 10,
            },
        );
        on_event(
            &mut state,
            Event::Snapshot {
                epoch: "ep1".into(),
                seq: 10,
                payload: dummy_bars(100),
            },
        );

        let actions = on_event(
            &mut state,
            Event::Entry {
                epoch: "ep1".into(),
                seq: 11,
                symbol: "VALE3".into(),
                timeframe: "1m".into(),
                payload: dummy_bars(101),
            },
        );
        assert_eq!(actions, vec![Action::Drop(DropReason::WrongSymbol)]);
    }
}
