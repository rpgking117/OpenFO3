//! The "You're SPECIAL!" book's rules (`world::chargen::special_book`, from
//! Fallout3.exe 1.7.0.3's `SPECIALBookMenu`), on a plugin with nothing in it:
//! the player's attributes are set in the state.

use esm::{ActivePlugins, LoadOrder};
use testdata::{record, sub};
use world::chargen::special_book::{self as book, Message};
use world::chargen::SPECIAL;
use world::dialogue::PLAYER_REF;
use world::scripting::GameState;

fn order(name: &str) -> (testdata::TempData, LoadOrder) {
    let mut hedr = 0.94f32.to_le_bytes().to_vec();
    hedr.extend([0; 8]);
    let plugin = record(b"TES4", 0, &sub(b"HEDR", &hedr));
    let data = testdata::quests(name);
    data.write("Fallout3.esm", &plugin);
    let order = LoadOrder::from_data_dir(data.path(), &ActivePlugins::OfficialOnly).unwrap();
    (data, order)
}

fn fives() -> GameState {
    let mut state = GameState::default();
    for av in SPECIAL {
        state.actor_values.insert((PLAYER_REF, av), 5.0);
    }
    state
}

/// The nursery's 40 points over seven 5s leave 5 to share; plus stops when
/// they're gone, and at 10; minus stops at 1.
#[test]
fn points_are_shared_within_one_and_ten() {
    let (_data, order) = order("book-points");
    let mut state = fives();
    assert_eq!(book::total(&order, &state), 35);
    assert_eq!(book::remaining(&order, &state, 40), 5);
    for _ in 0..5 {
        assert!(book::increase(&order, &mut state, 0, 40));
    }
    assert_eq!(book::value(&order, &state, 0), 10);
    assert_eq!(book::remaining(&order, &state, 40), 0);
    // None left: plus does nothing, and doesn't show.
    assert!(!book::increase(&order, &mut state, 1, 40));
    assert!(!book::plus_shown(&order, &state, 1, 40));
    // Back down, then up to 10 and no further.
    assert!(book::decrease(&order, &mut state, 0));
    assert!(book::plus_shown(&order, &state, 0, 40));
    assert!(book::increase(&order, &mut state, 0, 40));
    assert!(!book::increase(&order, &mut state, 0, 41));
    for _ in 0..10 {
        book::decrease(&order, &mut state, 6);
    }
    assert_eq!(book::value(&order, &state, 6), 1);
    assert!(!book::minus_shown(&order, &state, 6));
    assert!(book::minus_shown(&order, &state, 0));
}

/// The message and the points-left digits as the book shows them.
#[test]
fn the_points_left_show_as_the_game_shows_them() {
    assert_eq!(book::message(5), Message::MoreLeft);
    assert_eq!(book::message(1), Message::OneLeft);
    assert_eq!(book::message(0), Message::NoneLeft);
    assert_eq!(book::remaining_digits(0), (None, None));
    assert_eq!(book::remaining_digits(5), (None, Some(5)));
    assert_eq!(book::remaining_digits(12), (Some(1), Some(2)));
}

/// Turning pages (`00675a00`): the cover to the back cover and back, each
/// turn with the sequence for the page it reaches.
#[test]
fn pages_turn_with_the_games_sequences() {
    let mut b = book::Book::new(40);
    assert_eq!(b.sequence(), None);
    assert_eq!(b.turn(false), None);
    assert_eq!(b.page, 0);
    assert_eq!(b.turn(true), Some("Forward"));
    assert_eq!(b.attribute(), Some(0));
    for _ in 0..7 {
        b.turn(true);
    }
    assert_eq!(b.page, book::INDEX_PAGE);
    assert_eq!(b.sequence(), Some("FastRight"));
    assert_eq!(b.attribute(), None);
    assert_eq!(b.turn(true), Some("TurnLeft"));
    assert_eq!(b.turn(true), None);
    assert_eq!(b.page, book::LAST_PAGE);
    assert_eq!(b.turn(false), Some("TurnRight"));
    assert_eq!(b.page, 8);
    for _ in 0..8 {
        b.turn(false);
    }
    assert_eq!(b.page, 0);
    assert_eq!(b.sequence(), Some("Attack2"));
    // A page waits for the turn, except on the cover.
    b.update_ready(false);
    assert!(b.ready);
    b.turn(true);
    b.update_ready(false);
    assert!(!b.ready);
    b.update_ready(true);
    assert!(b.ready);
}

/// The index's focus stays between rows 1 and 7, and only on the index.
#[test]
fn the_index_focus_stays_on_the_index() {
    let mut b = book::Book::new(40);
    assert!(!b.move_focus(false));
    b.page = book::INDEX_PAGE;
    assert!(!b.move_focus(true));
    for _ in 0..6 {
        assert!(b.move_focus(false));
    }
    assert!(!b.move_focus(false));
    assert_eq!(b.focus, 7);
}
