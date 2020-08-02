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
    geometry::{ScreenPoint, ScreenVector},
    graphics::{
        Bitmap, BitmapTable, Font, Graphics, LCDBitmapDrawMode, LCDBitmapFlip, LCDColor, LCDRect,
        LCDSolidColor, LCD_COLUMNS, LCD_ROWS,
    },
    log_to_console,
    system::{PDButtons, System},
    Game, Playdate,
};
use enum_iterator::IntoEnumIterator;
use euclid::{vec2, Point2D, Vector2D};
use hashbrown::HashMap;
use rand::{prelude::*, seq::SliceRandom, SeedableRng};

const WINABLE_SEEDS: &[u64] = &[
    322, 331, 341, 1004, 1006, 1013, 1016, 1018, 1021, 1023, 1026, 1032, 1038, 1040, 1041, 1042,
    1044, 1055, 1056, 1058, 1061, 1064, 1079, 1082, 1088, 1093, 1095, 1104, 1113, 1118, 1119, 1120,
    1125, 1132, 1138, 1145, 1146, 1165, 1172, 1176, 1177, 1178, 1180, 1181, 1191, 1193, 1195, 1203,
    1207, 1208, 1211, 1215, 1219, 1222, 1225, 1227, 1229, 1231, 1239, 1240, 1244, 1245, 1247, 1248,
    1249, 1252, 1256, 1265, 1272, 1273, 1274, 1275, 1277, 1278, 1291, 1293, 1295, 1306, 1307, 1308,
    1312, 1318, 1320, 1329, 1330, 1336, 1341, 1354, 1357, 1360, 1362, 1366, 1367, 1369, 1373, 1378,
    1379, 1380, 1382, 1385, 1386, 1397, 1409, 1415, 1418, 1428, 1434, 1435, 1441, 1447, 1448, 1451,
    1455, 1458, 1460, 1463, 1466, 1476, 1477, 1478, 1481, 1497, 1499, 1512, 1515, 1518, 1520, 1527,
    1532, 1536, 1541, 1542, 1545, 1556, 1557, 1561, 1562, 1573, 1581, 1585, 1592, 1599, 1600, 1602,
    1616, 1621, 1622, 1623, 1624, 1625, 1627, 1628, 1631, 1632, 1639, 1642, 1653, 1657, 1659, 1660,
    1668, 1678, 1679, 1682, 1683, 1684, 1694, 1712, 1714, 1731, 1748, 1750, 1753, 1754, 1758, 1762,
    1764, 1777, 1778, 1791, 1808, 1812, 1813, 1816, 1825, 1846, 1851, 1860, 1864, 1866, 1867, 1869,
    1872, 1876, 1882, 1884, 1886, 1889, 1891, 1893, 1896, 1901, 1902, 1904, 1906, 1916, 1920, 1921,
    1922, 1927, 1929, 1934, 1935, 1943, 1944, 1946, 1954, 1955, 1956, 1959, 1968, 1972, 1978, 1987,
    1990, 1993,
];

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
        let index = if stack.is_empty() { 0 } else { stack.len() - 1 };
        self.get_card_position(index)
    }

    fn draw_empty(&self, resources: &Resources) -> Result<(), Error> {
        resources.empty.draw(
            None,
            None,
            self.position,
            LCDBitmapDrawMode::kDrawModeCopy,
            LCDBitmapFlip::kBitmapUnflipped,
            SCREEN_CLIP,
        )?;
        Ok(())
    }

    fn draw_card_at(
        card: &Card,
        position: &ScreenPoint,
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
            *position,
            LCDBitmapDrawMode::kDrawModeCopy,
            LCDBitmapFlip::kBitmapUnflipped,
            SCREEN_CLIP,
        )?;
        Ok(())
    }

    fn draw_squared(&self, stack: &Stack, resources: &Resources) -> Result<(), Error> {
        if let Some(card) = stack.get_top_card() {
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
                self.position,
                LCDBitmapDrawMode::kDrawModeCopy,
                LCDBitmapFlip::kBitmapUnflipped,
                SCREEN_CLIP,
            )?;
        }
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
        let cards_in_stack = stack.len();
        let cards_to_draw = cards_in_stack.min(visible);
        let mut card_pos = self.position;

        let fan_vector = match direction {
            FanDirection::Down => ScreenVector::new(0, MARGIN),
            FanDirection::Right => ScreenVector::new(MARGIN, 0),
        };

        let start = cards_in_stack - cards_to_draw;
        let max_index = cards_in_stack - 1;
        for index in start..cards_in_stack {
            if let Some(card) = stack.get_card(index) {
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
            }
            card_pos += fan_vector;
        }

        Ok(())
    }

    fn draw(&self, source: &Source, stack: &Stack, resources: &Resources) -> Result<(), Error> {
        if stack.is_empty() {
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
        self.target_index = self
            .targets
            .iter()
            .position(|stack_id| *stack_id == source.stack)
            .unwrap_or(0);
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
            if self.active_cards.len() > 0 {
                let max_index = self.active_cards.len().saturating_sub(1);
                if self.source_index >= max_index {
                    self.source_index = 0;
                } else {
                    self.source_index += 1;
                }
                self.table.source = self.active_cards[self.source_index];
            }
        }
    }

    pub fn new(_playdate: &Playdate) -> Result<Box<Self>, Error> {
        let (secs, _) = System::get().get_seconds_since_epoch()?;
        let mut rng = rand_pcg::Pcg32::seed_from_u64(secs as u64);
        let seed = WINABLE_SEEDS.choose(&mut rng).expect("seed");
        let table = Table::new(*seed);
        let graphics = Graphics::get();
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
        let resources = Self::load_resources(&cards_table, Graphics::get())?;
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

    fn check_crank(&mut self, _playdate: &mut Playdate) -> Result<(), Error> {
        let change = System::get().get_crank_change()? as i32;
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

    fn check_buttons(&mut self, _playdate: &mut Playdate) -> Result<(), Error> {
        let (_, pushed, _) = System::get().get_button_state()?;
        if (pushed & PDButtons::kButtonA) == PDButtons::kButtonA
            || (pushed & PDButtons::kButtonB) == PDButtons::kButtonB
        {
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
        } else if pushed & PDButtons::kButtonLeft == PDButtons::kButtonLeft {
            self.go_previous();
        } else if pushed & PDButtons::kButtonRight == PDButtons::kButtonRight {
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

        Graphics::get().clear(LCDColor::Solid(LCDSolidColor::kColorWhite))?;

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
            position + vec2(CARD_WIDTH, CARD_HEIGHT) / 2,
            LCDBitmapDrawMode::kDrawModeCopy,
            LCDBitmapFlip::kBitmapUnflipped,
            SCREEN_CLIP,
        )?;

        Ok(())
    }
}

#[cfg(not(test))]
crankstart_game!(KlondikeGame);
