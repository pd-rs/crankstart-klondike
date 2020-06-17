extern crate alloc;

use alloc::vec::Vec;
use anyhow::Error;
use core::mem;
use enum_iterator::IntoEnumIterator;
use rand::{seq::SliceRandom, SeedableRng};

#[derive(Clone, Copy, Debug, Eq, IntoEnumIterator, Ord, PartialEq, PartialOrd)]
pub enum StackId {
    Stock,
    Waste,
    Foundation1,
    Foundation2,
    Foundation3,
    Foundation4,
    Tableau1,
    Tableau2,
    Tableau3,
    Tableau4,
    Tableau5,
    Tableau6,
    Tableau7,
    Hand,
}

impl StackId {
    pub fn next(&self) -> Self {
        match self {
            StackId::Stock => StackId::Waste,
            StackId::Waste => StackId::Foundation1,
            StackId::Foundation1 => StackId::Foundation2,
            StackId::Foundation2 => StackId::Foundation3,
            StackId::Foundation3 => StackId::Foundation4,
            StackId::Foundation4 => StackId::Tableau1,
            StackId::Tableau1 => StackId::Tableau2,
            StackId::Tableau2 => StackId::Tableau3,
            StackId::Tableau3 => StackId::Tableau4,
            StackId::Tableau4 => StackId::Tableau5,
            StackId::Tableau5 => StackId::Tableau6,
            StackId::Tableau6 => StackId::Tableau7,
            StackId::Tableau7 => StackId::Stock,
            StackId::Hand => StackId::Hand,
        }
    }

    pub fn previous(&self) -> Self {
        match self {
            StackId::Stock => StackId::Tableau7,
            StackId::Waste => StackId::Stock,
            StackId::Foundation1 => StackId::Waste,
            StackId::Foundation2 => StackId::Foundation1,
            StackId::Foundation3 => StackId::Foundation2,
            StackId::Foundation4 => StackId::Foundation3,
            StackId::Tableau1 => StackId::Foundation4,
            StackId::Tableau2 => StackId::Tableau1,
            StackId::Tableau3 => StackId::Tableau2,
            StackId::Tableau4 => StackId::Tableau3,
            StackId::Tableau5 => StackId::Tableau4,
            StackId::Tableau6 => StackId::Tableau5,
            StackId::Tableau7 => StackId::Tableau6,
            StackId::Hand => StackId::Hand,
        }
    }
}

pub const FOUNDATIONS: &[StackId] = &[
    StackId::Foundation1,
    StackId::Foundation2,
    StackId::Foundation3,
    StackId::Foundation4,
];

pub const TABLEAUX: &[StackId] = &[
    StackId::Tableau1,
    StackId::Tableau2,
    StackId::Tableau3,
    StackId::Tableau4,
    StackId::Tableau5,
    StackId::Tableau6,
    StackId::Tableau7,
];

#[derive(Clone, Copy, Debug, Eq, IntoEnumIterator, Ord, PartialEq, PartialOrd)]
pub enum StackType {
    Stock,
    Waste,
    Foundation,
    Tableau,
    Hand,
}

#[derive(Clone, Copy, Debug, Eq, Hash, IntoEnumIterator, Ord, PartialEq, PartialOrd)]
pub enum Suit {
    Diamond = 2,
    Club = 1,
    Heart = 3,
    Spade = 4,
}

#[derive(Debug, PartialEq)]
pub enum Color {
    Black,
    Red,
}

impl Suit {
    pub fn color(&self) -> Color {
        match self {
            Suit::Diamond | Suit::Heart => Color::Red,
            Suit::Club | Suit::Spade => Color::Black,
        }
    }
}

//const SUITS: &[Suit] = &[Suit::Diamond, Suit::Club, Suit::Heart, Suit::Spade];

#[derive(Clone, Copy, Debug, Eq, Hash, IntoEnumIterator, Ord, PartialEq, PartialOrd)]
enum Rank {
    Ace = 1,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
    Jack,
    Queen,
    King,
}

impl From<Rank> for &'static str {
    fn from(rank: Rank) -> Self {
        let label = match rank {
            Rank::Ace => "A",
            Rank::Two => "2",
            Rank::Three => "3",
            Rank::Four => "4",
            Rank::Five => "5",
            Rank::Six => "6",
            Rank::Seven => "7",
            Rank::Eight => "8",
            Rank::Nine => "9",
            Rank::Ten => "T",
            Rank::Jack => "J",
            Rank::Queen => "Q",
            Rank::King => "K",
        };
        label
    }
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Card {
    suit: Suit,
    rank: Rank,
    face_up: bool,
}

impl Card {
    pub fn is_same_color(&self, other: &Card) -> bool {
        self.suit.color() == other.suit.color()
    }

    pub fn is_one_below(&self, other: &Card) -> bool {
        let delta = other.rank as i32 - self.rank as i32;
        delta == 1
    }
}

#[derive(Debug)]
pub struct Stack {
    stack_id: StackId,
    stack_type: StackType,
    cards: Vec<Card>,
}

impl Stack {
    pub fn top_card_index(&self) -> usize {
        if self.cards.is_empty() {
            0
        } else {
            self.cards.len() - 1
        }
    }

    pub fn bottom_card(&self) -> Option<&Card> {
        if self.cards.is_empty() {
            None
        } else {
            Some(&self.cards[0])
        }
    }

    pub fn top_card(&self) -> Option<&Card> {
        if self.cards.is_empty() {
            None
        } else {
            Some(&self.cards[self.cards.len() - 1])
        }
    }

    pub fn expose_top_card(&mut self) {
        if !self.cards.is_empty() {
            let last_index = self.cards.len() - 1;
            self.cards[last_index].face_up = true;
        }
    }

    fn foundation_can_accept_hand(&self, hand: &Stack) -> bool {
        if hand.cards.len() > 1 {
            false
        } else {
            if let Some(card) = &hand.top_card() {
                if self.cards.is_empty() {
                    card.rank == Rank::Ace
                } else {
                    if let Some(top_card) = self.top_card() {
                        if card.suit == top_card.suit {
                            top_card.is_one_below(card)
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                }
            } else {
                false
            }
        }
    }

    fn tableau_can_accept_hand(&self, hand: &Stack) -> bool {
        if let Some(card) = &hand.bottom_card() {
            if let Some(top_card) = self.top_card() {
                if !top_card.is_same_color(card) {
                    card.is_one_below(top_card)
                } else {
                    false
                }
            } else {
                card.rank == Rank::King
            }
        } else {
            false
        }
    }

    pub fn can_play(&self, hand: &Stack) -> bool {
        match self.stack_type {
            StackType::Foundation => self.foundation_can_accept_hand(hand),
            StackType::Tableau => self.tableau_can_accept_hand(hand),
            _ => false,
        }
    }

    fn flip_top_card(&mut self) {
        if !self.cards.is_empty() {
            let index = self.cards.len() - 1;
            let card = &mut self.cards[index];
            card.face_up = !card.face_up;
        }
    }
}

fn make_deck(seed: u64) -> Vec<Card> {
    let mut rng = rand_pcg::Pcg32::seed_from_u64(seed);

    let mut cards: Vec<Card> = Suit::into_enum_iter()
        .map(move |suit| {
            Rank::into_enum_iter().map(move |rank| Card {
                suit,
                rank,
                face_up: false,
            })
        })
        .flatten()
        .collect();
    cards.shuffle(&mut rng);
    cards
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Source {
    stack: StackId,
    index: usize,
}

impl Source {
    pub fn stock() -> Self {
        Source {
            stack: StackId::Stock,
            index: 0,
        }
    }
}

pub struct Table {
    stock: Stack,
    waste: Stack,
    foundations: Vec<Stack>,
    tableaux: Vec<Stack>,
    in_hand: Stack,
}

impl Table {
    pub fn new() -> Result<Self, Error> {
        let mut cards = make_deck(321);

        let foundations: Vec<Stack> = FOUNDATIONS
            .iter()
            .map(|foundation| Stack {
                stack_id: *foundation,
                stack_type: StackType::Foundation,
                cards: Vec::new(),
            })
            .collect();

        let mut stack_count = 1;
        let tableaux: Vec<Stack> = TABLEAUX
            .iter()
            .map(|tableau| {
                let start = cards.len() - stack_count;
                let mut stack = Stack {
                    stack_id: *tableau,
                    stack_type: StackType::Tableau,
                    cards: cards.split_off(start),
                };
                stack.flip_top_card();
                stack_count += 1;
                stack
            })
            .collect();

        let stock = Stack {
            stack_id: StackId::Stock,
            stack_type: StackType::Stock,
            cards: cards,
        };
        let waste = Stack {
            stack_id: StackId::Waste,
            stack_type: StackType::Waste,
            cards: Vec::new(),
        };
        let in_hand = Stack {
            stack_id: StackId::Hand,
            stack_type: StackType::Hand,
            cards: Vec::new(),
        };
        Ok(Self {
            stock,
            waste,
            foundations,
            tableaux,
            in_hand,
        })
    }

    fn get_stack(&self, stack_type: StackId) -> &Stack {
        match stack_type {
            StackId::Stock => &self.stock,
            StackId::Waste => &self.waste,
            StackId::Foundation1 => &self.foundations[0],
            StackId::Foundation2 => &self.foundations[1],
            StackId::Foundation3 => &self.foundations[2],
            StackId::Foundation4 => &self.foundations[3],
            StackId::Tableau1 => &self.tableaux[0],
            StackId::Tableau2 => &self.tableaux[1],
            StackId::Tableau3 => &self.tableaux[2],
            StackId::Tableau4 => &self.tableaux[3],
            StackId::Tableau5 => &self.tableaux[4],
            StackId::Tableau6 => &self.tableaux[5],
            StackId::Tableau7 => &self.tableaux[6],
            StackId::Hand => &self.in_hand,
        }
    }

    fn get_stack_mut(&mut self, stack_type: StackId) -> &mut Stack {
        match stack_type {
            StackId::Stock => &mut self.stock,
            StackId::Waste => &mut self.waste,
            StackId::Foundation1 => &mut self.foundations[0],
            StackId::Foundation2 => &mut self.foundations[1],
            StackId::Foundation3 => &mut self.foundations[2],
            StackId::Foundation4 => &mut self.foundations[3],
            StackId::Tableau1 => &mut self.tableaux[0],
            StackId::Tableau2 => &mut self.tableaux[1],
            StackId::Tableau3 => &mut self.tableaux[2],
            StackId::Tableau4 => &mut self.tableaux[3],
            StackId::Tableau5 => &mut self.tableaux[4],
            StackId::Tableau6 => &mut self.tableaux[5],
            StackId::Tableau7 => &mut self.tableaux[6],
            StackId::Hand => &mut self.in_hand,
        }
    }

    fn cards_in_hand(&self) -> bool {
        self.in_hand.cards.len() > 0
    }

    fn deal_from_stock(&mut self) {
        let amount_to_deal = 3.min(self.stock.cards.len());
        if amount_to_deal == 0 {
            mem::swap(&mut self.waste.cards, &mut self.stock.cards);
            for mut card in &mut self.stock.cards {
                card.face_up = false;
            }
            self.stock.cards.reverse();
        } else {
            let start = self.stock.cards.len() - amount_to_deal;
            let mut dealt_cards = self.stock.cards.split_off(start);
            for mut card in &mut dealt_cards {
                card.face_up = true;
            }
            self.waste.cards.append(&mut dealt_cards);
        }
    }

    fn expose_top_card_of_stack(&mut self, stack_id: StackId) {
        let stack = self.get_stack_mut(stack_id);
        stack.expose_top_card();
    }

    fn take_top_card_from_stack(&mut self, stack_id: StackId) {
        let stack = self.get_stack_mut(stack_id);
        let count = stack.cards.len();
        if count > 0 {
            let last_index = count - 1;
            let mut card = stack.cards.remove(last_index);
            card.face_up = true;
            self.in_hand.cards.push(card);
        }
    }

    fn take_selected_cards_from_stack(&mut self, stack_id: StackId, index: usize) {
        let cards_for_hand = {
            let stack = self.get_stack_mut(stack_id);
            stack.cards.split_off(index)
        };
        let count = cards_for_hand.len();
        if count > 0 {
            self.in_hand.cards = cards_for_hand;
        }
    }

    fn put_hand_on_target(&mut self, source: &Source, target: &StackId) -> Source {
        let mut cards = Vec::new();
        mem::swap(&mut cards, &mut self.in_hand.cards);
        let target_stack = self.get_stack_mut(*target);
        let index = target_stack.cards.len();
        target_stack.cards.append(&mut cards);
        self.expose_top_card_of_stack(source.stack);
        Source {
            stack: *target,
            index: index,
        }
    }
}
