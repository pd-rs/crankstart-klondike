extern crate alloc;

use alloc::{fmt, vec::Vec};
use anyhow::Error;
use core::mem;
use enum_iterator::IntoEnumIterator;
use rand::{seq::SliceRandom, SeedableRng};

#[derive(Clone, Copy, Debug, Eq, Hash, IntoEnumIterator, Ord, PartialEq, PartialOrd)]
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
    fn next_impl(&self, wrap: bool) -> Option<Self> {
        match self {
            StackId::Stock => Some(StackId::Waste),
            StackId::Waste => Some(StackId::Foundation1),
            StackId::Foundation1 => Some(StackId::Foundation2),
            StackId::Foundation2 => Some(StackId::Foundation3),
            StackId::Foundation3 => Some(StackId::Foundation4),
            StackId::Foundation4 => Some(StackId::Tableau1),
            StackId::Tableau1 => Some(StackId::Tableau2),
            StackId::Tableau2 => Some(StackId::Tableau3),
            StackId::Tableau3 => Some(StackId::Tableau4),
            StackId::Tableau4 => Some(StackId::Tableau5),
            StackId::Tableau5 => Some(StackId::Tableau6),
            StackId::Tableau6 => Some(StackId::Tableau7),
            StackId::Tableau7 => {
                if wrap {
                    Some(StackId::Stock)
                } else {
                    None
                }
            }
            StackId::Hand => Some(StackId::Hand),
        }
    }

    pub fn next_no_wrap(&self) -> Option<Self> {
        self.next_impl(false)
    }

    pub fn next(&self) -> Self {
        self.next_impl(true).expect("next")
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

    pub fn is_foundation(&self) -> bool {
        match self {
            StackId::Foundation1
            | StackId::Foundation2
            | StackId::Foundation3
            | StackId::Foundation4 => true,
            _ => false,
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

#[derive(Clone, Copy, Debug, Eq, IntoEnumIterator, Ord, PartialEq, PartialOrd, Hash)]
pub enum StackType {
    Stock,
    Waste,
    Foundation,
    Tableau,
    Hand,
}

#[derive(Debug, PartialEq, Hash)]
pub enum Color {
    Black,
    Red,
}

#[derive(Clone, Copy, Eq, Hash, IntoEnumIterator, Ord, PartialEq, PartialOrd)]
pub enum Suit {
    Diamond = 2,
    Club = 1,
    Heart = 3,
    Spade = 4,
}

impl fmt::Debug for Suit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        let s = match self {
            Suit::Diamond => "♦️",
            Suit::Club => "♣️",
            Suit::Heart => "♥️",
            Suit::Spade => "♠️",
        };
        f.write_str(s)
    }
}

impl Suit {
    fn color(&self) -> Color {
        match self {
            Suit::Diamond | Suit::Heart => Color::Red,
            Suit::Club | Suit::Spade => Color::Black,
        }
    }
}

//const SUITS: &[Suit] = &[Suit::Diamond, Suit::Club, Suit::Heart, Suit::Spade];

#[derive(Clone, Copy, Debug, Eq, Hash, IntoEnumIterator, Ord, PartialEq, PartialOrd)]
pub enum Rank {
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

#[derive(Clone, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct Card {
    pub suit: Suit,
    pub rank: Rank,
    pub face_up: bool,
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

impl fmt::Debug for Card {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        let face_up = if self.face_up { "" } else { "-" };
        let rank: &str = self.rank.into();
        f.write_fmt(format_args!("{}{}{:?}", face_up, rank, self.suit))
    }
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct Stack {
    pub stack_id: StackId,
    pub stack_type: StackType,
    pub cards: Vec<Card>,
}

impl Stack {
    pub fn find_card(&self, rank: Rank, suit: Suit) -> Option<usize> {
        self.cards
            .iter()
            .enumerate()
            .filter(|(_index, card)| card.rank == rank && card.suit == suit)
            .map(|(index, _card)| index)
            .nth(0)
    }

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

    pub fn previous_active_card(&self, start_index: Option<usize>) -> Option<usize> {
        if self.cards.is_empty() {
            return None;
        }
        let max_index = self.cards.len() - 1;
        let index = if let Some(start_index) = start_index {
            if start_index == 0 {
                return None;
            }
            start_index - 1
        } else {
            max_index
        };
        match self.stack_type {
            StackType::Stock | StackType::Foundation | StackType::Waste => {
                if start_index.is_none() {
                    Some(max_index)
                } else {
                    None
                }
            }
            _ => {
                for active_index in (0..=index).rev() {
                    if self.cards[active_index].face_up {
                        return Some(active_index);
                    }
                }
                None
            }
        }
    }

    pub fn next_active_card(&self, start_index: Option<usize>) -> Option<usize> {
        if self.cards.is_empty() || self.stack_type == StackType::Stock {
            return None;
        }
        let max_index = self.cards.len() - 1;
        let index = if let Some(start_index) = start_index {
            start_index + 1
        } else {
            0
        };
        if index <= max_index {
            match self.stack_type {
                StackType::Stock | StackType::Foundation | StackType::Waste => Some(max_index),
                _ => {
                    for active_index in index..=max_index {
                        if self.cards[active_index].face_up {
                            return Some(active_index);
                        }
                    }
                    None
                }
            }
        } else {
            None
        }
    }

    pub fn is_top_face_up_card(&self, index: usize) -> bool {
        if self.cards.is_empty() {
            return false;
        }
        let max_index = self.cards.len() - 1;
        for search_index in 0..=max_index {
            if self.cards[search_index].face_up {
                return search_index == index;
            }
        }
        return false;
    }

    pub fn foundation_can_accept_card(&self, card: &Card) -> bool {
        if self.cards.is_empty() {
            if card.rank == Rank::Ace {
                match self.stack_id {
                    StackId::Foundation1 => card.suit == Suit::Spade,
                    StackId::Foundation2 => card.suit == Suit::Club,
                    StackId::Foundation3 => card.suit == Suit::Heart,
                    StackId::Foundation4 => card.suit == Suit::Diamond,
                    _ => false,
                }
            } else {
                false
            }
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
    }

    pub fn foundation_can_accept_hand(&self, hand: &Stack) -> bool {
        if hand.cards.len() > 1 {
            false
        } else {
            if let Some(card) = &hand.top_card() {
                self.foundation_can_accept_card(card)
            } else {
                false
            }
        }
    }

    pub fn tableau_can_accept_card(&self, card: &Card) -> bool {
        if let Some(top_card) = self.top_card() {
            if !top_card.is_same_color(card) {
                card.is_one_below(top_card)
            } else {
                false
            }
        } else {
            card.rank == Rank::King
        }
    }

    pub fn tableau_can_accept_hand(&self, hand: &Stack) -> bool {
        if let Some(card) = &hand.bottom_card() {
            self.tableau_can_accept_card(card)
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

    pub fn can_play_card(&self, card: &Card, moving_cards_count: usize) -> bool {
        match self.stack_type {
            StackType::Foundation => {
                moving_cards_count == 1 && self.foundation_can_accept_card(card)
            }
            StackType::Tableau => self.tableau_can_accept_card(card),
            _ => false,
        }
    }

    pub fn flip_top_card(&mut self) {
        if !self.cards.is_empty() {
            let index = self.cards.len() - 1;
            let card = &mut self.cards[index];
            card.face_up = !card.face_up;
        }
    }
}

impl fmt::Debug for Stack {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        f.write_fmt(format_args!("{:?}: [", self.stack_id))?;
        if self.cards.len() > 0 {
            let last_index = self.cards.len() - 1;
            for (index, card) in self.cards.iter().enumerate() {
                f.write_fmt(format_args!("{:?}", card))?;
                if index < last_index {
                    f.write_str(", ")?;
                }
            }
        }
        f.write_str("]")?;
        Ok(())
    }
}

pub fn make_deck(seed: u64) -> Vec<Card> {
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

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct Source {
    pub stack: StackId,
    pub index: usize,
}

impl Source {
    pub fn new(stack: StackId, index: usize) -> Self {
        Self { stack, index }
    }

    pub fn stock() -> Self {
        Source {
            stack: StackId::Stock,
            index: 0,
        }
    }
}

impl fmt::Debug for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        f.write_fmt(format_args!("{:?}[{}]", self.stack, self.index))
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Table {
    pub stock: Stack,
    pub waste: Stack,
    pub in_hand: Stack,
    pub foundations: Vec<Stack>,
    pub tableaux: Vec<Stack>,
    pub source: Source,
    pub target: StackId,
}

impl Table {
    pub fn new(seed: u64) -> Self {
        let mut cards = make_deck(seed);

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
        let source_index = stock.next_active_card(None).unwrap_or(0);
        Self {
            stock,
            waste,
            foundations,
            tableaux,
            in_hand,
            source: Source {
                stack: StackId::Stock,
                index: source_index,
            },
            target: StackId::Stock,
        }
    }

    pub fn get_stack(&self, stack_type: StackId) -> &Stack {
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

    pub fn get_stack_mut(&mut self, stack_type: StackId) -> &mut Stack {
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

    pub fn find_card(&self, rank: Rank, suit: Suit) -> Option<Source> {
        for stack_id in StackId::into_enum_iter() {
            let stack = self.get_stack(stack_id);
            if let Some(index) = stack.find_card(rank, suit) {
                return Some(Source {
                    stack: stack_id,
                    index,
                });
            }
        }
        None
    }

    pub fn cards_in_hand(&self) -> bool {
        self.in_hand.cards.len() > 0
    }

    pub fn has_cards_in_stock(&self) -> bool {
        self.stock.cards.len() > 0
    }

    pub fn has_cards_in_waste(&self) -> bool {
        self.waste.cards.len() > 0
    }

    pub fn cards_in_foundation(&self) -> usize {
        self.foundations
            .iter()
            .map(|stack| stack.cards.len())
            .sum::<usize>()
    }

    pub fn winner(&self) -> bool {
        self.cards_in_foundation() == 52
    }

    pub fn next_active_card(&self) -> Option<Source> {
        let mut source = self.source;
        let mut start = Some(source.index);
        loop {
            let source_stack = self.get_stack(source.stack);
            let next_index = source_stack.next_active_card(start);
            if next_index.is_some() {
                return Some(Source {
                    stack: source.stack,
                    index: next_index.unwrap(),
                });
            } else {
                source.stack = source.stack.next();
                start = None;
            }
        }
    }

    pub fn previous_active_card(&self) -> Option<Source> {
        let mut source = self.source;
        let mut start = Some(source.index);
        loop {
            let source_stack = self.get_stack(source.stack);
            let previous_index = source_stack.previous_active_card(start);
            if previous_index.is_some() {
                return Some(Source {
                    stack: source.stack,
                    index: previous_index.unwrap(),
                });
            } else {
                source.stack = source.stack.previous();
                start = None;
            }
        }
    }

    pub fn next_play_location(&self) -> StackId {
        let orginal_stack = self.target;
        let mut target = orginal_stack.next();
        loop {
            let target_stack = self.get_stack(target);
            if target_stack.can_play(&self.in_hand) {
                break;
            } else {
                target = target.next();
            }
            if target == self.source.stack {
                break;
            }
        }
        target
    }

    pub fn previous_play_location(&self) -> StackId {
        let orginal_stack = self.target;
        let mut target = orginal_stack.previous();
        loop {
            let target_stack = self.get_stack(target);
            if target_stack.can_play(&self.in_hand) {
                break;
            } else {
                target = target.previous();
            }
            if target == self.source.stack {
                break;
            }
        }
        target
    }

    pub fn deal_from_stock(&mut self) {
        let amount_to_deal = 3.min(self.stock.cards.len());
        if amount_to_deal == 0 {
            mem::swap(&mut self.waste.cards, &mut self.stock.cards);
            for mut card in &mut self.stock.cards {
                card.face_up = false;
            }
            self.stock.cards.reverse();
        } else {
            for _ in 0..amount_to_deal {
                let mut dealt_card = self.stock.cards.pop().expect("card");
                dealt_card.face_up = true;
                self.waste.cards.push(dealt_card);
            }
        }
    }

    pub fn recycle_waste(&mut self) {
        self.deal_from_stock();
    }

    pub fn expose_top_card_of_stack(&mut self, stack_id: StackId) {
        let stack = self.get_stack_mut(stack_id);
        stack.expose_top_card();
    }

    pub fn take_top_card_from_stack(&mut self, stack_id: StackId) {
        let stack = self.get_stack_mut(stack_id);
        let count = stack.cards.len();
        if count > 0 {
            let last_index = count - 1;
            let mut card = stack.cards.remove(last_index);
            card.face_up = true;
            self.in_hand.cards.push(card);
        }
    }

    pub fn take_selected_cards_from_stack(&mut self, stack_id: StackId, index: usize) {
        let cards_for_hand = {
            let stack = self.get_stack_mut(stack_id);
            stack.cards.split_off(index)
        };
        let count = cards_for_hand.len();
        if count > 0 {
            self.in_hand.cards = cards_for_hand;
        }
    }

    pub fn put_hand_on_stack(&mut self, source: Source, stack_id: StackId) -> usize {
        let mut cards = Vec::new();
        mem::swap(&mut cards, &mut self.in_hand.cards);
        let target_stack = self.get_stack_mut(stack_id);
        let index = target_stack.cards.len();
        target_stack.cards.append(&mut cards);
        self.expose_top_card_of_stack(source.stack);
        index
    }

    pub fn put_hand_on_target(&mut self) {
        let target = self.target;
        let index = self.put_hand_on_stack(self.source, target);
        self.source = Source {
            stack: target,
            index: index,
        };
    }

    pub fn go_next(&mut self) -> Result<(), Error> {
        if self.cards_in_hand() {
            self.target = self.next_play_location();
        } else {
            self.source = self.next_active_card().unwrap_or_else(|| Source::stock())
        }
        Ok(())
    }

    pub fn go_previous(&mut self) -> Result<(), Error> {
        if self.cards_in_hand() {
            self.target = self.previous_play_location();
        } else {
            self.source = self
                .previous_active_card()
                .unwrap_or_else(|| Source::stock());
        }
        Ok(())
    }
}
