use anyhow::Error;

#[path = "../klondike.rs"]
#[allow(dead_code)]
mod klondike;

use crate::klondike::{
    ActiveCardIterator, Card, CardPlayIterator, Play, Rank, Source, Stack, StackId, Table,
};
use argh::FromArgs;
use core::iter::Iterator;
use std::{
    cmp::Ordering,
    collections::HashSet,
    io::{stdin, stdout, Write},
};

#[derive(Debug, Clone, Copy)]
struct WeightedPlay {
    play: Play,
    score: isize,
    priority: isize,
}

impl PartialEq for WeightedPlay {
    fn eq(&self, other: &Self) -> bool {
        self.score == other.score && self.priority == other.priority
    }
}

impl Eq for WeightedPlay {}

impl PartialOrd for WeightedPlay {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for WeightedPlay {
    fn cmp(&self, other: &Self) -> Ordering {
        let r = self.score.cmp(&other.score);
        if r == Ordering::Equal {
            self.priority.cmp(&other.priority)
        } else {
            r
        }
    }
}

impl WeightedPlay {
    pub fn new(play: Play, table: &Table) -> Self {
        let (score, priority) = match play {
            Play::MoveCards(source, target) => match target {
                StackId::Foundation1
                | StackId::Foundation2
                | StackId::Foundation3
                | StackId::Foundation4 => (5, 0),
                StackId::Tableau1
                | StackId::Tableau2
                | StackId::Tableau3
                | StackId::Tableau4
                | StackId::Tableau5
                | StackId::Tableau6
                | StackId::Tableau7 => match source.stack {
                    StackId::Waste => {
                        let stack = table.get_stack(source.stack);
                        let card = &stack.cards[source.index];
                        let score = 5;
                        let priority = if card.rank == Rank::King {
                            Self::waste_king_priority(source, target, stack, card, table)
                        } else {
                            1
                        };
                        (score, priority)
                    }
                    StackId::Tableau1
                    | StackId::Tableau2
                    | StackId::Tableau3
                    | StackId::Tableau4
                    | StackId::Tableau5
                    | StackId::Tableau6
                    | StackId::Tableau7 => Self::tableau_move(source, target, table),
                    StackId::Foundation1
                    | StackId::Foundation2
                    | StackId::Foundation3
                    | StackId::Foundation4 => (-10, 0),
                    _ => (0, 0),
                },
                _ => (0, 0),
            },
            _ => (0, 0),
        };
        Self {
            play,
            score,
            priority,
        }
    }

    fn tableau_move(source: Source, _target: StackId, table: &Table) -> (isize, isize) {
        let stack = table.get_stack(source.stack);
        let score = 0;
        if stack.is_top_face_up_card(source.index) {
            if stack.cards.len() > 0 {
                (score, source.index as isize + 1)
            } else {
                (score, 1)
            }
        } else {
            (score, 1)
        }
    }

    fn waste_king_priority(
        _source: Source,
        _target: StackId,
        _stack: &Stack,
        card: &Card,
        table: &Table,
    ) -> isize {
        if let Some(queen_card_location) = table.find_card(Rank::Queen, card.suit) {
            match queen_card_location.stack {
                StackId::Tableau1
                | StackId::Tableau2
                | StackId::Tableau3
                | StackId::Tableau4
                | StackId::Tableau5
                | StackId::Tableau6
                | StackId::Tableau7 => {
                    let stack = table.get_stack(queen_card_location.stack);
                    let card = &stack.cards[queen_card_location.index];
                    if card.face_up {
                        1
                    } else {
                        -1
                    }
                }
                _ => 1,
            }
        } else {
            99
        }
    }
}

enum PlayIteratorPhase<'a> {
    Start,
    Stock,
    ActiveCards(ActiveCardIterator<'a>, Option<CardPlayIterator<'a>>),
    Done,
}

struct PlayIterator<'a> {
    table: &'a Table,
    phase: PlayIteratorPhase<'a>,
}

impl<'a> PlayIterator<'a> {
    pub fn new(table: &'a Table) -> Self {
        Self {
            table,
            phase: PlayIteratorPhase::Start,
        }
    }
}

impl<'a> Iterator for PlayIterator<'a> {
    type Item = Play;

    fn next(&mut self) -> Option<Play> {
        loop {
            match &mut self.phase {
                PlayIteratorPhase::Start => {
                    self.phase = PlayIteratorPhase::Stock;
                }
                PlayIteratorPhase::Stock => {
                    self.phase =
                        PlayIteratorPhase::ActiveCards(ActiveCardIterator::new(self.table), None);
                    if self.table.has_cards_in_stock() {
                        return Some(Play::DrawFromStock);
                    }
                    if self.table.has_cards_in_waste() {
                        return Some(Play::RecycleWaste);
                    }
                }
                PlayIteratorPhase::ActiveCards(iterator, card_iterator) => {
                    if let Some(active_card_iterator) = card_iterator {
                        let play = active_card_iterator.next();
                        if play.is_none() {
                            *card_iterator = None;
                        } else {
                            return play;
                        }
                    } else {
                        let next_active_card = iterator.next();
                        if let Some(active_card) = next_active_card {
                            let stack = self.table.get_stack(active_card.stack);
                            let card = &stack.cards[active_card.index];
                            let card_play_iterator =
                                CardPlayIterator::new(self.table, card, active_card);
                            *card_iterator = Some(card_play_iterator);
                        } else {
                            self.phase = PlayIteratorPhase::Done;
                            return None;
                        }
                    }
                }
                PlayIteratorPhase::Done => {
                    return None;
                }
            }
        }
    }
}

struct SearchNode {
    parent: Option<usize>,
    index: usize,
    play: Play,
    table: Table,
    weighted_plays: Vec<WeightedPlay>,
}

impl SearchNode {
    fn new(parent: Option<usize>, index: usize, play: Play, table: Table) -> SearchNode {
        let mut weighted_plays: Vec<WeightedPlay> = PlayIterator::new(&table)
            .map(|play| WeightedPlay::new(play, &table))
            .collect();
        weighted_plays.sort();
        Self {
            parent,
            index,
            play,
            table,
            weighted_plays,
        }
    }

    fn filter_play(&self, play: &Play, previous_plays: &Vec<Play>) -> Option<Play> {
        match play {
            Play::RecycleWaste => {
                if previous_plays.len() > 0 {
                    let mut search_index = previous_plays.len() as isize - 1;
                    while search_index >= 0 {
                        match previous_plays[search_index as usize] {
                            Play::DrawFromStock => (),
                            Play::RecycleWaste => search_index = -1,
                            _ => break,
                        }
                        search_index -= 1;
                    }
                    if search_index < 0 {
                        return None;
                    }
                }
                Some(*play)
            }
            Play::MoveCards(source, target) => match target {
                StackId::Foundation1
                | StackId::Foundation2
                | StackId::Foundation3
                | StackId::Foundation4 => Some(*play),
                _ => match source.stack {
                    StackId::Foundation1
                    | StackId::Foundation2
                    | StackId::Foundation3
                    | StackId::Foundation4 => None,
                    StackId::Waste => Some(*play),
                    _ => {
                        let stack = self.table.get_stack(source.stack);
                        if source.index == 0 {
                            if stack.cards[0].rank == Rank::King {
                                return None;
                            } else {
                                Some(*play)
                            }
                        } else if stack.is_top_face_up_card(source.index) {
                            Some(*play)
                        } else {
                            None
                        }
                    }
                },
            },
            _ => Some(*play),
        }
    }

    fn search(
        &mut self,
        next_index: usize,
        previous_plays: &Vec<Play>,
        stepping: bool,
    ) -> Option<SearchNode> {
        while let Some(weighted_play) = self.weighted_plays.pop() {
            if stepping {
                println!("chose {:?}", weighted_play);
            }
            let table = self.table.clone();
            if let Some(play) = self.filter_play(&weighted_play.play, previous_plays) {
                let new_table = make_move(play, &table);
                return Some(Self::new(
                    Some(self.index),
                    next_index,
                    weighted_play.play,
                    new_table,
                ));
            }
        }
        None
    }
}

fn test_plays_iter(table: Table, opt: &Opt) {
    let mut stepping = true;
    let mut max_foundation = 0;
    let mut search_nodes = Vec::new();
    let mut tables: HashSet<Table> = HashSet::new();
    search_nodes.push(SearchNode::new(None, 0, Play::Setup, table));
    let mut iterations = 0;
    while search_nodes.len() > 0 {
        let len = search_nodes.len();
        let last_index = len - 1;
        let mut traverse = last_index;
        let mut parents = Vec::new();
        while let Some(parent) = search_nodes[traverse].parent {
            parents.push(parent);
            traverse = parent;
        }
        parents.reverse();
        let plays: Vec<Play> = parents
            .iter()
            .map(|parent| search_nodes[*parent].play)
            .collect();
        if stepping {
            let mut s = String::new();
            print!("Solver command: ");
            let _ = stdout().flush();
            stdin().read_line(&mut s).expect("Did not enter command");
            match s.trim() {
                "c" => stepping = false,
                "p" => {
                    println!("plays: {:?}", plays);
                    println!("table: {:#?}", search_nodes[last_index].table);
                }
                _ => (),
            }
        } else if iterations % 1_000_000 == 1 {
            if opt.verbose {
                println!("plays: {:?}", plays);
                println!("table: {:#?}", search_nodes[last_index].table);
            }
        }
        let cards_in_foundation = search_nodes[last_index].table.cards_in_foundation();
        if cards_in_foundation > max_foundation {
            max_foundation = cards_in_foundation;
            if opt.verbose {
                println!("new max foundation {}", max_foundation);
                println!("plays: {:?}", plays);
                println!("table: {:#?}", search_nodes[last_index].table);
            }
        }
        if let Some(node) = search_nodes[last_index].search(len, &plays, stepping) {
            if node.table.winner() {
                println!("Winner! {:#?}", node.table);
                println!("plays: {:?} final {:?}", plays, node.play);
                break;
            }
            if stepping {
                if opt.verbose {
                    println!("{:#?}", node.table);
                    println!("{:#?}", node.weighted_plays);
                }
            }
            tables.insert(node.table.clone());
            search_nodes.push(node);
        } else {
            search_nodes.pop();
            if stepping {
                let len = search_nodes.len();
                if len > 0 {
                    let last_index = len - 1;
                    if opt.verbose {
                        println!("returning to {}", search_nodes.len() - 1);
                        println!("table: {:#?}", search_nodes[last_index].table);
                        println!(
                            "weighted_plays: {:#?}",
                            search_nodes[last_index].weighted_plays
                        );
                    }
                }
            }
        }
        iterations += 1;
        if iterations > 5_000_000 {
            println!("Iteration limit met");
            println!("plays: {:?}", plays);
            let len = search_nodes.len();
            if len > 0 {
                let last_index = len - 1;
                println!("table: {:#?}", search_nodes[last_index].table);
            }
            break;
        }
    }
    if search_nodes.len() == 0 {
        println!("exhaustive search failed to find win");
    }
}

fn make_move(play: Play, table: &Table) -> Table {
    let mut new_table = table.clone();
    match play {
        Play::DrawFromStock => {
            assert!(new_table.has_cards_in_stock());
            new_table.deal_from_stock()
        }
        Play::RecycleWaste => {
            assert!(!new_table.has_cards_in_stock());
            new_table.recycle_waste();
        }
        Play::MoveCards(source, stack_id) => {
            new_table.take_selected_cards_from_stack(source.stack, source.index);
            new_table.put_hand_on_stack(source, stack_id);
        }
        Play::Setup => panic!("Unhandled play"),
    }
    new_table
}

/// Options
#[derive(FromArgs, Debug)]
struct Opt {
    /// verbose
    #[argh(switch)]
    verbose: bool,
    /// seed
    #[argh(option, default = "326")]
    seed: u64,
    /// recursive
    #[argh(switch, short = 'r')]
    recursive: bool,
}

fn main() -> Result<(), Error> {
    let opt: Opt = argh::from_env();
    let table = Table::new(opt.seed);

    println!("start table {} {:#?}", opt.seed, table);
    test_plays_iter(table, &opt);
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::klondike::Suit;

    const TEST_SEED: u64 = 324;

    #[test]
    fn test_recycle_waste() {
        let table = Table::new(TEST_SEED);
        let mut work_table = table.clone();

        assert_eq!(
            table.get_stack(StackId::Stock),
            work_table.get_stack(StackId::Stock)
        );

        while work_table.has_cards_in_stock() {
            work_table.deal_from_stock();
        }

        work_table.recycle_waste();

        assert_eq!(
            table.get_stack(StackId::Stock),
            work_table.get_stack(StackId::Stock)
        );
    }

    #[test]
    fn test_find_card() {
        let mut table = Table::new(TEST_SEED);
        table.deal_from_stock();
        println!("table = {:#?}", table);
        let queen_card_location = table.find_card(Rank::Queen, Suit::Diamond);
        assert_eq!(Some(Source::new(StackId::Tableau3, 0)), queen_card_location);

        let two_diamonds_card_location = table.find_card(Rank::Two, Suit::Diamond);
        assert_eq!(
            Some(Source::new(StackId::Stock, 6)),
            two_diamonds_card_location
        );

        let waste_card_location = table.find_card(Rank::Nine, Suit::Club);
        assert_eq!(Some(Source::new(StackId::Waste, 2)), waste_card_location);
    }
}
