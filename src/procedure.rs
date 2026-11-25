use std::fmt::{self, Display, Formatter};

use crate::{EXPECTED_PROBE, PROBE};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    Lower,
    Trim,
    Sort,
    Dedup,
    DropBlank,
}

const STEPS: [Step; 5] = [
    Step::Lower,
    Step::Trim,
    Step::Sort,
    Step::Dedup,
    Step::DropBlank,
];
const REPLAY: &[&str] = &["  BIRCH-2\t", " \tPINE  ", "birch-2", " \t ", " ELM "];
const EXPECTED_REPLAY: &[&str] = &["birch-2", "elm", "pine"];

impl Step {
    fn name(self) -> &'static str {
        match self {
            Self::Lower => "lower",
            Self::Trim => "trim",
            Self::Sort => "sort",
            Self::Dedup => "dedup",
            Self::DropBlank => "drop-blank",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Procedure(Vec<Step>);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Work {
    pub lowercase_bytes: usize,
    pub sort_items: usize,
}

impl Work {
    pub fn improves_on(self, baseline: Self) -> bool {
        self != baseline
            && self.lowercase_bytes <= baseline.lowercase_bytes
            && self.sort_items <= baseline.sort_items
    }
}

impl Display for Work {
    fn fmt(&self, output: &mut Formatter<'_>) -> fmt::Result {
        write!(
            output,
            "lowercase_bytes:{}, sort_items:{}",
            self.lowercase_bytes, self.sort_items
        )
    }
}

pub struct Execution {
    pub tags: Vec<String>,
    pub work: Work,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Search {
    All,
    Unique,
    ValidUnique,
}

pub struct SearchTable(Vec<(Procedure, Option<Procedure>)>);

impl SearchTable {
    pub fn lookup(&self, baseline: &Procedure) -> Option<&Option<Procedure>> {
        self.0
            .iter()
            .find(|(order, _)| order == baseline)
            .map(|(_, candidate)| candidate)
    }
}

#[cfg(unix)]
impl SearchTable {
    #[cfg(target_os = "macos")]
    pub fn orders(&self) -> impl Iterator<Item = &Procedure> {
        self.0.iter().map(|(order, _)| order)
    }
    pub fn measure(policy: Search) -> (Self, usize) {
        let mut entries = Vec::new();
        let mut probes = 0;
        let mut steps = STEPS;
        visit_orders(&mut steps, 0, &mut |baseline| {
            if baseline.valid_order() {
                let (candidate, counts) = baseline.search(policy, &mut crate::Progress::silent());
                probes += counts[1];
                entries.push((baseline, candidate));
            }
        });
        (Self(entries), probes)
    }

    pub fn encode(&self) -> String {
        self.0
            .iter()
            .map(|(baseline, candidate)| {
                let target = candidate
                    .as_ref()
                    .map_or_else(|| "-".into(), ToString::to_string);
                format!("{baseline}>{target}")
            })
            .collect::<Vec<_>>()
            .join(";")
    }

    pub fn decode(text: &str) -> Option<Self> {
        let entries: Option<Vec<_>> = text
            .split(';')
            .map(|row| {
                let (baseline, candidate) = row.split_once('>')?;
                let candidate = if candidate == "-" {
                    None
                } else {
                    Some(Procedure::parse(candidate)?)
                };
                Some((Procedure::parse(baseline)?, candidate))
            })
            .collect();
        entries.map(Self)
    }

    pub fn verify(&self) -> (bool, usize) {
        let mut passed = self.0.len() == 7;
        let mut probes = 0;
        let mut steps = STEPS;
        visit_orders(&mut steps, 0, &mut |baseline| {
            if baseline.valid_order() {
                let (reference, counts) =
                    baseline.search(Search::All, &mut crate::Progress::silent());
                probes += counts[1];
                passed &= self.lookup(&baseline) == Some(&reference);
                passed &= reference.as_ref().is_none_or(|order| {
                    order.valid_order() && order.probes().1 && baseline.is_relocation(order)
                });
            }
        });
        (passed, probes)
    }
}

impl Search {
    pub fn name(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Unique => "unique",
            Self::ValidUnique => "valid-unique",
        }
    }

    pub fn next(self) -> Option<Self> {
        match self {
            Self::All => Some(Self::Unique),
            Self::Unique => Some(Self::ValidUnique),
            Self::ValidUnique => None,
        }
    }
}

impl Procedure {
    pub fn original() -> Self {
        Self(STEPS.to_vec())
    }

    pub fn parse(text: &str) -> Option<Self> {
        let mut steps = match text {
            "lower-then-trim" => return Some(Self::original()),
            "trim-then-lower" => {
                return Some(Self(vec![
                    Step::Trim,
                    Step::Lower,
                    Step::Sort,
                    Step::Dedup,
                    Step::DropBlank,
                ]));
            }
            "trim-only" => {
                return Some(Self(vec![
                    Step::Trim,
                    Step::Sort,
                    Step::Dedup,
                    Step::DropBlank,
                ]));
            }
            _ => Vec::new(),
        };
        for name in text.split(',') {
            let step = STEPS.into_iter().find(|step| step.name() == name)?;
            if steps.contains(&step) {
                return None;
            }
            steps.push(step);
        }
        Some(Self(steps))
    }

    pub fn trim_first(&self) -> Option<Self> {
        (self == &Self::original()).then(|| Self::parse("trim-then-lower").unwrap())
    }

    pub fn without_lower(&self) -> Self {
        Self(
            self.0
                .iter()
                .copied()
                .filter(|step| *step != Step::Lower)
                .collect(),
        )
    }

    pub fn valid_order(&self) -> bool {
        let position = |step| self.0.iter().position(|item| *item == step);
        let before = |left, right| match (position(left), position(right)) {
            (Some(left), Some(right)) => left < right,
            _ => false,
        };
        // These dependencies preserve canonical output for any supported ASCII input.
        before(Step::Trim, Step::Sort)
            && before(Step::Lower, Step::Sort)
            && before(Step::Sort, Step::Dedup)
            && before(Step::Trim, Step::DropBlank)
    }

    pub fn apply(&self, input: &[&str]) -> Execution {
        let mut work = Work::default();
        let mut tags: Vec<_> = input.iter().map(|tag| (*tag).to_owned()).collect();
        for step in &self.0 {
            match step {
                Step::Lower => tags.iter_mut().for_each(|tag| {
                    work.lowercase_bytes += tag.len();
                    tag.make_ascii_lowercase();
                }),
                Step::Trim => tags.iter_mut().for_each(|tag| {
                    let trimmed = tag.trim();
                    if trimmed.len() != tag.len() {
                        *tag = trimmed.to_owned();
                    }
                }),
                Step::Sort => {
                    work.sort_items += tags.len();
                    tags.sort_unstable();
                }
                Step::Dedup => tags.dedup(),
                Step::DropBlank => tags.retain(|tag| !tag.is_empty()),
            }
        }
        Execution { tags, work }
    }

    pub fn probes(&self) -> (Execution, bool) {
        let probe = self.apply(PROBE);
        let replay = self.apply(REPLAY);
        let passed = probe.tags == EXPECTED_PROBE && replay.tags == EXPECTED_REPLAY;
        (probe, passed)
    }

    fn relocations(&self) -> impl Iterator<Item = Self> + '_ {
        (0..self.0.len()).flat_map(move |from| {
            (0..self.0.len())
                .filter(move |to| *to != from)
                .map(move |to| self.relocate(from, to))
        })
    }

    fn relocate(&self, from: usize, to: usize) -> Self {
        let mut steps = self.0.clone();
        let step = steps.remove(from);
        steps.insert(to, step);
        Self(steps)
    }

    pub fn is_relocation(&self, candidate: &Self) -> bool {
        self.relocations().any(|order| &order == candidate)
    }

    pub fn search(
        &self,
        policy: Search,
        progress: &mut crate::Progress,
    ) -> (Option<Self>, [usize; 2]) {
        let baseline = self.apply(PROBE).work;
        progress.event("probe", format_args!("{baseline}"));
        let mut counts = [0, 0];
        for from in 0..self.0.len() {
            for to in 0..self.0.len() {
                // Moving an adjacent step back yields the same order as the earlier forward move.
                if from == to || (policy != Search::All && from == to + 1) {
                    continue;
                }
                if policy == Search::ValidUnique && !self.valid_relocation(from, to) {
                    continue;
                }
                let candidate = self.relocate(from, to);
                counts[0] += 1;
                if !candidate.valid_order() {
                    progress.event("skip", format_args!("{candidate}"));
                    continue;
                }
                counts[1] += 1;
                let work = candidate.apply(PROBE).work;
                let improves = work.improves_on(baseline);
                progress.event(
                    if improves { "found" } else { "trial" },
                    format_args!("{candidate} · {work}"),
                );
                if improves {
                    return (Some(candidate), counts);
                }
            }
        }
        (None, counts)
    }

    fn valid_relocation(&self, from: usize, to: usize) -> bool {
        let mut moved = [Step::Lower; STEPS.len()];
        // Inspect the resulting order on the stack before creating a Procedure.
        let mut remaining = self
            .0
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != from);
        for (index, step) in moved.iter_mut().enumerate() {
            *step = if index == to {
                self.0[from]
            } else {
                *remaining.next().unwrap().1
            };
        }
        let before = |a, b| {
            let mut found = false;
            for step in moved {
                if step == b {
                    return found;
                }
                found |= step == a;
            }
            false
        };
        before(Step::Trim, Step::Sort)
            && before(Step::Lower, Step::Sort)
            && before(Step::Sort, Step::Dedup)
            && before(Step::Trim, Step::DropBlank)
    }
}

#[cfg(unix)]
pub fn search_gate(previous: Search, candidate: Search) -> (bool, [usize; 2], [usize; 2], usize) {
    let mut before = [0, 0];
    let mut after = [0, 0];
    let mut matches = true;
    let mut cases = 0;
    let mut steps = STEPS;
    visit_orders(&mut steps, 0, &mut |procedure| {
        if !procedure.valid_order() {
            return;
        }
        cases += 1;
        let mut quiet = crate::Progress::silent();
        let reference = procedure.search(Search::All, &mut quiet).0;
        let (original, old_cost) = procedure.search(previous, &mut quiet);
        let (proposed, new_cost) = procedure.search(candidate, &mut quiet);
        matches &= original == reference && proposed == reference;
        matches &= new_cost[0] <= old_cost[0] && new_cost[1] <= old_cost[1];
        matches &= proposed
            .as_ref()
            .is_none_or(|order| order.probes().1 && procedure.is_relocation(order));
        for i in 0..2 {
            before[i] += old_cost[i];
            after[i] += new_cost[i];
        }
    });
    (
        matches && cases == 7 && after != before,
        before,
        after,
        cases,
    )
}

#[cfg(any(test, unix))]
fn visit_orders(steps: &mut [Step], first: usize, visit: &mut impl FnMut(Procedure)) {
    if first == steps.len() {
        visit(Procedure(steps.to_vec()));
        return;
    }
    for other in first..steps.len() {
        steps.swap(first, other);
        visit_orders(steps, first + 1, visit);
        steps.swap(first, other);
    }
}

impl Display for Procedure {
    fn fmt(&self, output: &mut Formatter<'_>) -> fmt::Result {
        for (index, step) in self.0.iter().enumerate() {
            if index != 0 {
                output.write_str(",")?;
            }
            output.write_str(step.name())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn reference_rejects_a_changed_search_decision() {
        let (mut table, _) = SearchTable::measure(Search::ValidUnique);
        let saved = table.encode();
        assert!(SearchTable::decode(&saved).unwrap().verify().0);
        let (_, selected) = table
            .0
            .iter_mut()
            .find(|(_, candidate)| candidate.is_some())
            .unwrap();
        *selected = None;
        assert!(!table.verify().0);
    }

    #[test]
    fn every_admitted_order_agrees_with_the_independent_reference() {
        let examples = ["", " \t ", "ASH", "\t ash ", "elm-2", "Elm-2", "pine"];
        let mut visited = 0;
        let mut admitted = 0;
        let mut steps = STEPS;
        visit_orders(&mut steps, 0, &mut |procedure| {
            visited += 1;
            if procedure.valid_order() {
                admitted += 1;
                assert!(procedure.probes().1);
                for a in examples {
                    for b in examples {
                        for c in examples {
                            let input = [a, b, c];
                            assert_eq!(procedure.apply(&input).tags, crate::expected_tags(&input));
                        }
                    }
                }
            }
        });
        assert_eq!(visited, 120);
        assert_eq!(admitted, 7);
    }
}
