//! Placing timed events side by side in a day column, the way Google Calendar does.

/// Column of an event and how many columns its group of overlapping events uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Slot {
    pub column: usize,
    pub columns: usize,
}

/// Lays out events given as `(start, end)` minutes, in the order given (sort by start first).
/// Events shorter than `min_minutes` count as that long, since they are drawn that tall.
pub fn columns(events: &[(u32, u32)], min_minutes: u32) -> Vec<Slot> {
    let mut slots = vec![
        Slot {
            column: 0,
            columns: 1
        };
        events.len()
    ];
    // Events that overlap, directly or through each other, share one column count.
    let mut group: Vec<usize> = Vec::new();
    // End minute of the last event placed in each column of the current group.
    let mut column_ends: Vec<u32> = Vec::new();
    let mut group_end = 0;

    for (index, &(start, end)) in events.iter().enumerate() {
        let end = end.max(start + min_minutes);
        if !group.is_empty() && start >= group_end {
            close_group(&mut slots, &group, column_ends.len());
            group.clear();
            column_ends.clear();
        }
        let column = match column_ends.iter().position(|&column_end| column_end <= start) {
            Some(free) => {
                column_ends[free] = end;
                free
            }
            None => {
                column_ends.push(end);
                column_ends.len() - 1
            }
        };
        slots[index].column = column;
        group.push(index);
        group_end = if group.len() == 1 { end } else { group_end.max(end) };
    }
    close_group(&mut slots, &group, column_ends.len());
    slots
}

fn close_group(slots: &mut [Slot], group: &[usize], columns: usize) {
    for &index in group {
        slots[index].columns = columns.max(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slot(column: usize, columns: usize) -> Slot {
        Slot { column, columns }
    }

    #[test]
    fn separate_events_take_the_full_width() {
        let slots = columns(&[(540, 600), (600, 660)], 15);
        assert_eq!(slots, vec![slot(0, 1), slot(0, 1)]);
    }

    #[test]
    fn overlapping_events_share_the_width() {
        // 9:00-10:00, 9:30-10:30, 10:00-11:00: the third reuses the first column.
        let slots = columns(&[(540, 600), (570, 630), (600, 660)], 15);
        assert_eq!(slots, vec![slot(0, 2), slot(1, 2), slot(0, 2)]);
    }

    #[test]
    fn short_events_overlap_by_their_drawn_height() {
        // Two 5-minute events five minutes apart are drawn 15 minutes tall, so they collide.
        let slots = columns(&[(540, 545), (545, 550)], 15);
        assert_eq!(slots, vec![slot(0, 2), slot(1, 2)]);
    }

    #[test]
    fn a_long_event_groups_everything_it_spans() {
        let slots = columns(&[(480, 720), (540, 570), (600, 630), (780, 810)], 15);
        assert_eq!(slots, vec![slot(0, 2), slot(1, 2), slot(1, 2), slot(0, 1)]);
    }
}
