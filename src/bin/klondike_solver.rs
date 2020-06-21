#![allow(unused, dead_code)]
use anyhow::Error;

#[path = "../klondike.rs"]
mod klondike;

use crate::klondike::{Card, Source, StackId, Table};
use core::iter::Iterator;

#[derive(Debug, Clone, Copy)]
enum Play {
    DrawFromStock,
    RecycleWaste,
    MoveCards(Source, StackId),
}

struct ActiveCardIterator<'a> {
    table: &'a Table,
    source: Option<Source>,
}

impl<'a> ActiveCardIterator<'a> {
    pub fn new(table: &'a Table) -> Self {
        let stock = table.get_stack(StackId::Stock);
        let source_index = stock.next_active_card(None).unwrap_or(0);
        let source = Some(Source {
            stack: StackId::Stock,
            index: source_index,
        });
        Self { table, source }
    }
}

impl<'a> Iterator for ActiveCardIterator<'a> {
    type Item = Source;

    fn next(&mut self) -> Option<Source> {
        let next = self.source;
        if let Some(mut source) = next {
            let mut start = Some(source.index);
            loop {
                let source_stack = self.table.get_stack(source.stack);
                let next_index = source_stack.next_active_card(start);
                if next_index.is_some() {
                    let source = Source {
                        stack: source.stack,
                        index: next_index.unwrap(),
                    };
                    self.source = Some(source);
                    break;
                } else {
                    source.stack = source.stack.next();
                    if source.stack == StackId::Stock {
                        self.source = None;
                        break;
                    }
                    start = None;
                }
            }
        }
        next
    }
}

struct PlayIterator<'a> {
    table: &'a Table,
    card: &'a Card,
    source: Source,
    play: Option<Play>,
}

impl<'a> PlayIterator<'a> {
    pub fn new(table: &'a Table, card: &'a Card, source: Source) -> Self {
        let play = if table.has_cards_in_stock() {
            Some(Play::DrawFromStock)
        } else if (table.has_cards_in_waste()) {
            Some(Play::RecycleWaste)
        } else {
            Self::next_legal_play(table, card, source, StackId::Waste)
        };
        Self {
            table,
            card,
            source,
            play,
        }
    }

    pub fn next_legal_play(
        table: &'a Table,
        card: &'a Card,
        source: Source,
        start: StackId,
    ) -> Option<Play> {
        let mut target = Some(start);
        loop {
            if let Some(current_target) = target {
                let stack = table.get_stack(current_target);
                if stack.can_play_card(card) {
                    return Some(Play::MoveCards(source, current_target));
                }
                target = current_target.next_no_wrap();
            } else {
                break;
            }
        }
        None
    }
}

impl<'a> Iterator for PlayIterator<'a> {
    type Item = Play;

    fn next(&mut self) -> Option<Play> {
        let next_play = self.play;
        if let Some(mut play) = next_play {
            loop {
                match play {
                    Play::DrawFromStock => {
                        play = Play::RecycleWaste;
                        if self.table.has_cards_in_waste() && !self.table.has_cards_in_stock() {
                            self.play = Some(play);
                            break;
                        }
                    }
                    Play::RecycleWaste => {
                        self.play = Self::next_legal_play(
                            self.table,
                            self.card,
                            self.source,
                            StackId::Foundation1,
                        );
                        break;
                    }
                    _ => {
                        if let Some(legal) = next_play {
                            match legal {
                                Play::MoveCards(_, target) => {
                                    let next_target = target.next_no_wrap();
                                    if let Some(next_target) = next_target {
                                        self.play = Self::next_legal_play(
                                            self.table,
                                            self.card,
                                            self.source,
                                            next_target,
                                        );
                                    } else {
                                        self.play = None;
                                    }
                                }
                                _ => {
                                    self.play = None;
                                }
                            }
                        }
                        break;
                    }
                }
            }
        }
        next_play
    }
}

fn main() -> Result<(), Error> {
    let mut table = Table::new(321);
    table.deal_from_stock();
    println!("table = {:#?}", table);
    let active_card_iter = ActiveCardIterator::new(&table);
    for card_location in active_card_iter {
        println!("card_location = {:?}", card_location);
        let stack = table.get_stack(card_location.stack);
        let card = &stack.cards[card_location.index];
        println!("card = {:?}", card);
        let play_iter = PlayIterator::new(&table, card, card_location);
        for play in play_iter {
            println!("play = {:?}", play);
        }
        println!("#####");
    }
    Ok(())
}
