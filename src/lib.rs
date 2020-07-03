#![no_std]
#![allow(unused_imports)]

extern crate alloc;

#[allow(dead_code)]
mod klondike;

use crate::klondike::*;
use alloc::{boxed::Box, collections::BTreeMap, format, string::String, vec::Vec};
use anyhow::Error;
use core::{iter, mem};
use crankstart::{
    crankstart_game,
    graphics::{
        Bitmap, BitmapDrawMode, BitmapFlip, BitmapTable, Font, Graphics, LCDRect, SolidColor,
    },
    log_to_console, Game, Playdate,
};
use crankstart_sys::{
    PDButtons_kButtonA, PDButtons_kButtonB, PDButtons_kButtonLeft, PDButtons_kButtonRight,
    LCD_COLUMNS, LCD_ROWS,
};
use enum_iterator::IntoEnumIterator;
use euclid::{Point2D, Vector2D};
use hashbrown::HashMap;
use rand::{prelude::*, seq::SliceRandom, SeedableRng};

const SCREEN_CLIP: LCDRect = LCDRect {
    left: 0,
    right: LCD_COLUMNS as i32,
    top: 0,
    bottom: LCD_ROWS as i32,
};

const SCREEN_WIDTH: i32 = LCD_COLUMNS as i32;
//const SCREEN_HEIGHT: i32 = LCD_ROWS as i32;
const MARGIN: i32 = 10;
//const INDEX_MARGIN_X: i32 = 4;
//const INDEX_MARGIN_Y: i32 = 1;
const GUTTER: i32 = 5;
const CARD_WIDTH: i32 = 50;
const CARD_HEIGHT: i32 = 70;

const CRANK_THRESHHOLD: i32 = 10;

pub struct ScreenSpace;
pub type ScreenPoint = Point2D<i32, ScreenSpace>;
pub type ScreenVector = Vector2D<i32, ScreenSpace>;

#[derive(Debug)]
enum FanDirection {
    Down,
    Right,
}

#[derive(Debug)]
enum StackDrawMode {
    Squared,
    Fanned(FanDirection, usize),
}

#[derive(Debug)]
struct StackView {
    stack_id: StackId,
    position: ScreenPoint,
    mode: StackDrawMode,
}

impl StackView {
    pub fn get_card_position(&self, index: usize) -> ScreenPoint {
        let (vector, count) = match &self.mode {
            StackDrawMode::Squared => (ScreenVector::zero(), 0),
            StackDrawMode::Fanned(direction, visible) => match direction {
                FanDirection::Down => (ScreenVector::new(0, MARGIN), *visible),
                FanDirection::Right => (ScreenVector::new(MARGIN, 0), *visible),
            },
        };
        let number = index.min(count.saturating_sub(1));
        self.position + vector * number as i32
    }

    #[allow(unused)]
    pub fn get_top_card_position(&self, stack: &Stack) -> ScreenPoint {
        let index = if stack.cards.is_empty() {
            0
        } else {
            stack.cards.len() - 1
        };
        self.get_card_position(index)
    }

    fn draw_empty(&self, resources: &Resources) -> Result<(), Error> {
        resources.empty.draw(
            None,
            None,
            self.position.x,
            self.position.y,
            BitmapDrawMode::Copy,
            BitmapFlip::Unflipped,
            SCREEN_CLIP,
        )?;
        Ok(())
    }

    fn draw_card_at(
        card: &Card,
        postion: &ScreenPoint,
        resources: &Resources,
    ) -> Result<(), Error> {
        let bitmap = if card.face_up {
            if let Some(bitmap) = resources.card_bitmaps.get(&(card.suit, card.rank)) {
                &bitmap
            } else {
                &resources.empty
            }
        } else {
            &resources.back
        };
        bitmap.draw(
            None,
            None,
            postion.x,
            postion.y,
            BitmapDrawMode::Copy,
            BitmapFlip::Unflipped,
            SCREEN_CLIP,
        )?;
        Ok(())
    }

    fn draw_squared(&self, stack: &Stack, resources: &Resources) -> Result<(), Error> {
        let card = &stack.cards[stack.cards.len() - 1];
        let bitmap = if card.face_up {
            resources
                .card_bitmaps
                .get(&(card.suit, card.rank))
                .unwrap_or(&resources.empty)
        } else {
            &resources.back
        };
        bitmap.draw(
            None,
            None,
            self.position.x,
            self.position.y,
            BitmapDrawMode::Copy,
            BitmapFlip::Unflipped,
            SCREEN_CLIP,
        )?;
        Ok(())
    }

    fn draw_fanned(
        &self,
        stack: &Stack,
        resources: &Resources,
        source: &Source,
        direction: &FanDirection,
        visible: usize,
    ) -> Result<(), Error> {
        let cards_in_stack = stack.cards.len();
        let cards_to_draw = cards_in_stack.min(visible);
        let mut card_pos = self.position;

        let fan_vector = match direction {
            FanDirection::Down => ScreenVector::new(0, MARGIN),
            FanDirection::Right => ScreenVector::new(MARGIN, 0),
        };

        let start = cards_in_stack - cards_to_draw;
        let max_index = cards_in_stack - 1;
        for index in start..cards_in_stack {
            let card = &stack.cards[index];
            if card.face_up
                && index < max_index
                && index == source.index
                && stack.stack_id == source.stack
            {
                let peeked = card_pos - Vector2D::new(0, CARD_HEIGHT / 4);
                Self::draw_card_at(card, &peeked, resources)?;
            } else {
                Self::draw_card_at(card, &card_pos, resources)?;
            }
            card_pos += fan_vector;
        }

        Ok(())
    }

    fn draw(&self, source: &Source, stack: &Stack, resources: &Resources) -> Result<(), Error> {
        if stack.cards.len() == 0 {
            self.draw_empty(resources)?;
        } else {
            match &self.mode {
                StackDrawMode::Squared => self.draw_squared(stack, resources)?,
                StackDrawMode::Fanned(direction, visible) => {
                    self.draw_fanned(stack, resources, source, direction, *visible)?
                }
            }
        }
        Ok(())
    }
}

struct Resources {
    card_bitmaps: HashMap<(Suit, Rank), Bitmap>,
    back: Bitmap,
    empty: Bitmap,
    #[allow(unused)]
    graphics: Graphics,
    point: Bitmap,
}

struct KlondikeGame {
    table: Table,
    active_cards: Vec<Source>,
    source_index: usize,
    targets: Vec<StackId>,
    target_index: usize,
    views: HashMap<StackId, StackView>,
    #[allow(unused)]
    cards_table: BitmapTable,
    resources: Resources,
    crank_threshhold: i32,
}

impl KlondikeGame {
    pub fn load_resources(
        cards_table: &BitmapTable,
        graphics: Graphics,
    ) -> Result<Resources, Error> {
        let mut card_bitmaps = HashMap::new();
        for suit in Suit::into_enum_iter() {
            let row = match suit {
                Suit::Diamond => 2,
                Suit::Heart => 1,
                Suit::Spade => 3,
                Suit::Club => 4,
            };
            let mut col = 0;
            for rank in Rank::into_enum_iter() {
                let index = row * 13 + col;
                let bitmap = cards_table.get_bitmap(index)?;
                card_bitmaps.insert((suit, rank), bitmap);
                col += 1;
            }
        }
        let back = cards_table.get_bitmap(4)?;
        let empty = cards_table.get_bitmap(0)?;
        let point = graphics.load_bitmap("assets/point")?;
        Ok(Resources {
            card_bitmaps,
            back,
            empty,
            graphics,
            point,
        })
    }

    fn update_active_cards(&mut self) {
        self.active_cards = iter::once(Source::stock())
            .chain(ActiveCardIterator::new(&self.table))
            .collect();
    }

    fn update_targets(&mut self) {
        let source = self.table.source;

        self.targets = StackId::into_enum_iter()
            .filter(|stack_id| {
                *stack_id == source.stack || self.table.stack_can_accept_hand(*stack_id)
            })
            .collect();
        log_to_console!("self.targets = {:#?}", self.targets);
        self.target_index = self
            .targets
            .iter()
            .position(|stack_id| *stack_id == source.stack)
            .unwrap_or(0);
        log_to_console!("target_index = {:#?}", self.target_index);
    }

    fn go_previous(&mut self) {
        if self.table.cards_in_hand() {
            if self.target_index == 0 {
                self.target_index = self.targets.len().saturating_sub(1);
            } else {
                self.target_index -= 1;
            }
            self.table.target = self.targets[self.target_index];
        } else {
            if self.source_index == 0 {
                self.source_index = self.active_cards.len().saturating_sub(1);
            } else {
                self.source_index -= 1;
            }
            self.table.source = self.active_cards[self.source_index];
        }
    }

    fn go_next(&mut self) {
        if self.table.cards_in_hand() {
            let max_index = self.targets.len().saturating_sub(1);
            if self.target_index == max_index {
                self.target_index = 0;
            } else {
                self.target_index += 1;
            }
            self.table.target = self.targets[self.target_index];
        } else {
            let max_index = self.active_cards.len().saturating_sub(1);
            if self.source_index == max_index {
                self.source_index = 0;
            } else {
                self.source_index += 1;
            }
            self.table.source = self.active_cards[self.source_index];
        }
    }

    pub fn new(playdate: &Playdate) -> Result<Box<Self>, Error> {
        let table = Table::new(331);
        let graphics = playdate.graphics();
        let cards_table = graphics.load_bitmap_table("assets/cards")?;

        let foundation_gutter_count = (FOUNDATIONS.len() - 1) as i32;
        let mut position = ScreenPoint::new(
            SCREEN_WIDTH
                - FOUNDATIONS.len() as i32 * 50
                - foundation_gutter_count * GUTTER
                - MARGIN,
            MARGIN,
        );

        let foundations = FOUNDATIONS.iter().map(|foundation| {
            let stack = StackView {
                stack_id: *foundation,
                position,
                mode: StackDrawMode::Squared,
            };
            position.x += CARD_WIDTH + GUTTER;
            stack
        });

        let mut position = ScreenPoint::new(MARGIN, MARGIN + CARD_HEIGHT + GUTTER);
        let mut stack_count = 1;
        let tableaux = TABLEAUX.iter().map(|tableau| {
            let stack = StackView {
                stack_id: *tableau,
                position,
                mode: StackDrawMode::Fanned(FanDirection::Down, 52),
            };
            stack_count += 1;
            position.x += 55;
            stack
        });

        let stock = StackView {
            stack_id: StackId::Stock,
            position: ScreenPoint::new(MARGIN, MARGIN),
            mode: StackDrawMode::Squared,
        };
        let waste = StackView {
            stack_id: StackId::Waste,
            position: ScreenPoint::new(MARGIN + GUTTER + CARD_WIDTH, MARGIN),
            mode: StackDrawMode::Fanned(FanDirection::Right, 3),
        };
        let in_hand = StackView {
            stack_id: StackId::Hand,
            position: ScreenPoint::zero(),
            mode: StackDrawMode::Squared,
        };

        let views: HashMap<StackId, StackView> = foundations
            .chain(tableaux)
            .chain(iter::once(stock))
            .chain(iter::once(waste).chain(iter::once(in_hand)))
            .map(|stack_view| (stack_view.stack_id, stack_view))
            .collect();
        let resources = Self::load_resources(&cards_table, playdate.graphics())?;
        let active_cards = iter::once(Source::stock())
            .chain(ActiveCardIterator::new(&table))
            .collect();
        Ok(Box::new(Self {
            table,
            active_cards,
            source_index: 0,
            targets: Vec::new(),
            target_index: 0,
            views,
            cards_table,
            resources,
            crank_threshhold: 0,
        }))
    }

    fn check_crank(&mut self, playdate: &mut Playdate) -> Result<(), Error> {
        let change = playdate.system().get_crank_change()? as i32;
        self.crank_threshhold += change;

        if self.crank_threshhold > CRANK_THRESHHOLD {
            self.go_next();
            self.crank_threshhold = -CRANK_THRESHHOLD;
        } else if self.crank_threshhold < -CRANK_THRESHHOLD {
            self.go_previous();
            self.crank_threshhold = CRANK_THRESHHOLD;
        }
        Ok(())
    }

    fn check_buttons(&mut self, playdate: &mut Playdate) -> Result<(), Error> {
        let (_, pushed, _) = playdate.system().get_button_state()?;
        if (pushed & PDButtons_kButtonA) != 0 || (pushed & PDButtons_kButtonB) != 0 {
            if self.table.cards_in_hand() {
                self.table.put_hand_on_target();
                self.update_active_cards();
            } else {
                match self.table.source.stack {
                    StackId::Stock => {
                        self.table.deal_from_stock();
                        self.update_active_cards();
                    }
                    StackId::Waste
                    | StackId::Foundation1
                    | StackId::Foundation2
                    | StackId::Foundation3
                    | StackId::Foundation4 => {
                        self.table.take_top_card_from_stack(self.table.source.stack)
                    }
                    StackId::Tableau1
                    | StackId::Tableau2
                    | StackId::Tableau3
                    | StackId::Tableau4
                    | StackId::Tableau5
                    | StackId::Tableau6
                    | StackId::Tableau7 => self.table.take_selected_cards_from_stack(
                        self.table.source.stack,
                        self.table.source.index,
                    ),
                    StackId::Hand => (),
                }
                self.table.target = self.table.source.stack;
                self.update_targets();
            }
        } else if pushed & PDButtons_kButtonLeft != 0 {
            self.go_previous();
        } else if pushed & PDButtons_kButtonRight != 0 {
            self.go_next();
        }
        Ok(())
    }
}

impl Game for KlondikeGame {
    fn update(
        &mut self,
        playdate: &mut crankstart::Playdate,
    ) -> core::result::Result<(), anyhow::Error> {
        self.check_crank(playdate)?;
        self.check_buttons(playdate)?;

        let cards_in_hand = self.table.cards_in_hand();
        if cards_in_hand {
            let top_card_index = self.table.get_stack(self.table.target).top_card_index();
            let position = self
                .views
                .get(&self.table.target)
                .and_then(|view| {
                    Some(view.get_card_position(top_card_index) + Vector2D::new(10, 10))
                })
                .unwrap_or_else(|| ScreenPoint::zero());
            if let Some(in_hand) = self.views.get_mut(&StackId::Hand) {
                in_hand.position = position;
            }
        }

        playdate.graphics().clear(SolidColor::White)?;

        for (stack_id, view) in &self.views {
            if *stack_id != StackId::Hand || cards_in_hand {
                let stack = self.table.get_stack(*stack_id);
                view.draw(&self.table.source, stack, &self.resources)?;
            }
        }

        let position = if cards_in_hand {
            let target = self.table.get_stack(self.table.target);
            let target_view = self.views.get(&target.stack_id).expect("target_view");
            let position =
                target_view.get_card_position(target.top_card_index()) + Vector2D::new(10, 10);
            position
        } else {
            let source = self.table.get_stack(self.table.source.stack);
            let source_view = self.views.get(&source.stack_id).expect("source_view");
            source_view.get_card_position(self.table.source.index)
        };

        self.resources.point.draw(
            None,
            None,
            position.x + CARD_WIDTH / 2,
            position.y + CARD_HEIGHT / 2,
            BitmapDrawMode::Copy,
            BitmapFlip::Unflipped,
            SCREEN_CLIP,
        )?;

        Ok(())
    }
}

#[cfg(not(test))]
crankstart_game!(KlondikeGame);
