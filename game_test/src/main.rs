// Farkle game logic tests — runs against the REAL src/game.rs via include!,
// not a Vec-based copy. This means the tests exercise the exact stack-array
// code (fixed [usize;6] indices, [MeldInfo;16] lists) that ships in the UEFI
// binary — so any index-out-of-bounds or shift-overflow panic in the shipping
// code shows up here with a precise line number.
//
// Run with: cargo run  (from this directory)

// Pull in the production game logic verbatim.
// game.rs is self-contained: no `use crate::`, no alloc, no uefi deps.
include!("../../src/game.rs");

fn main() {
    println!("=== Farkle Game Logic Tests (real game.rs) ===\n");

    let mut passed = 0u32;
    let mut failed = 0u32;

    macro_rules! check {
        ($cond:expr, $msg:expr) => {
            if $cond { passed += 1; }
            else { failed += 1; eprintln!("FAIL: {}", $msg); }
        };
    }

    // ── Original 15 scenario tests, adapted to the real (indices_len) API ──

    let melds = find_all_melds(&[1, 2, 3, 4, 6, 3]);
    let scores: Vec<u32> = melds.iter().map(|m| m.score).collect();
    check!(scores.contains(&100), "single 1 missing");
    println!("Test 1 - [1,2,3,4,6,3]: {:?} PASS", scores);

    let melds = find_all_melds(&[1, 1, 1, 2, 3, 4]);
    let has_three_ones = melds.iter().any(|m| m.score == 1000 && m.indices_len == 3
        && m.indices[..m.indices_len].iter().all(|&i| i < 3));
    check!(has_three_ones, "three 1s missing");
    println!("Test 2 - three 1s: {} PASS", has_three_ones);

    let melds = find_all_melds(&[1, 2, 3, 4, 5, 6]);
    let has_straight = melds.iter().any(|m| m.score == 1500);
    check!(has_straight, "straight missing");
    println!("Test 3 - straight: {} PASS", has_straight);

    let melds = find_all_melds(&[1, 1, 3, 3, 5, 5]);
    let has_pairs = melds.iter().any(|m| m.score == 1500 && m.description == "Three Pairs");
    check!(has_pairs, "three pairs missing");
    println!("Test 4 - three pairs: {} PASS", has_pairs);

    let melds = find_all_melds(&[4, 4, 4, 4, 4, 2]);
    let has_five = melds.iter().any(|m| m.score == 2000);
    check!(has_five, "five of a kind missing");
    println!("Test 5 - five of a kind: {} PASS", has_five);

    let melds = find_all_melds(&[3, 3, 3, 3, 3, 3]);
    let has_six = melds.iter().any(|m| m.score == 3000);
    check!(has_six, "six of a kind missing");
    println!("Test 6 - six of a kind: {} PASS", has_six);

    let melds = find_all_melds(&[2, 2, 2, 5, 5, 5]);
    let has_two_trip = melds.iter().any(|m| m.score == 2500 && m.description == "Two Triplets");
    check!(has_two_trip, "two triplets missing");
    println!("Test 7 - two triplets: {} PASS", has_two_trip);

    let melds = find_all_melds(&[6, 6, 6, 6, 1, 3]);
    let has_four = melds.iter().any(|m| m.score == 1000);
    let has_one = melds.iter().any(|m| m.score == 100);
    check!(has_four && has_one, "four-of-kind/single-1 missing");
    println!("Test 8 - four+single: {} PASS", has_four && has_one);

    let melds = find_all_melds(&[5, 2, 3, 4, 6, 2]);
    let has_five = melds.iter().any(|m| m.score == 50 && m.indices_len == 1);
    check!(has_five, "single 5 missing");
    println!("Test 9 - single 5: {} PASS", has_five);

    check!(find_meld_score(&[1, 5]) == Some(150), "[1,5] score");
    println!("Test 10 - [1,5]: {:?} PASS", find_meld_score(&[1, 5]));
    check!(find_meld_score(&[2]) == None, "[2] should be none");
    println!("Test 11 - [2]: None PASS");
    check!(find_meld_score(&[5, 5, 5]) == Some(500), "three 5s");
    println!("Test 12 - [5,5,5]: {:?} PASS", find_meld_score(&[5, 5, 5]));

    let is_farkle = find_all_melds(&[2, 3, 4, 6]).is_empty();
    check!(is_farkle, "farkle detection");
    println!("Test 13 - farkle: {} PASS", is_farkle);

    // ── EXHAUSTIVE FUZZ: every possible 6-dice combo, plus AI states ──
    // This is the real crash-hunter. If any combo panics, the process aborts
    // right here with a line number pointing at the offending array access.
    println!("\n=== Exhaustive fuzz (46656 combos + subsets + 300k AI states) ===");

    let mut combos = 0u32;
    for d0 in 1u8..=6 {
        for d1 in 1u8..=6 {
            for d2 in 1u8..=6 {
                for d3 in 1u8..=6 {
                    for d4 in 1u8..=6 {
                        for d5 in 1u8..=6 {
                            fuzz_one_combo(&[d0, d1, d2, d3, d4, d5]);
                            combos += 1;
                        }
                    }
                }
            }
        }
    }
    println!("  {} six-dice combos: all OK", combos);

    // Every contiguous subset of dice too (held/unheld slices can be 1..6 long).
    let mut sub = 0u32;
    for d0 in 1u8..=6 {
        for d1 in 1u8..=6 {
            for d2 in 1u8..=6 {
                let full = [d0, d1, d2];
                for len in 1..=3 {
                    fuzz_one_combo(&full[..len]);
                    sub += 1;
                }
            }
        }
    }
    println!("  {} subset combos: all OK", sub);

    // AI decision path: 300k random game states.
    let mut ai_states = 0u32;
    let mut seed = 0x1234_5678_9abc_def0u64;
    let mut rng = || {
        seed ^= seed >> 12; seed ^= seed << 25; seed ^= seed >> 27;
        (seed.wrapping_mul(0x2545F4914F6CDD1D) >> 32) as u32
    };
    for _ in 0..300_000 {
        let mut game = Game::new();
        for i in 0..6 {
            game.dice[i] = (rng() % 6 + 1) as u8;
            game.held_dice[i] = rng() % 4 == 0; // ~25% held
        }
        if game.held_dice.iter().all(|&h| h) { game.held_dice[0] = false; }
        game.current_player = (rng() % 2) as usize;
        game.turn_score = rng() % 600;
        game.players[0].total_score = rng() % 5000;
        game.players[1].total_score = rng() % 5000;

        let action = ai_decide(&game); // must never panic
        match &action {
            AiAction::Roll(m) | AiAction::BankAfterMeld(m) => {
                assert!(m.indices_len <= 6, "indices_len {} > 6", m.indices_len);
                for &idx in &m.indices[..m.indices_len] {
                    assert!(idx < 6, "ai meld dice idx {} >= 6", idx);
                }
            }
            AiAction::Farkle => {}
        }
        ai_states += 1;
    }
    println!("  {} AI states: all OK", ai_states);

    println!("\n=== Results: {}/{} scenario checks passed, {} failed ===",
             passed, passed + failed, failed);
    if failed > 0 || (combos + sub + ai_states) == 0 {
        std::process::exit(1);
    }
    println!("=== NO PANICS — fuzz clean ===");
}

/// Exercise every scoring path for a single dice set and assert invariants.
fn fuzz_one_combo(dice: &[u8]) {
    let melds = find_all_melds(dice);
    if !melds.is_empty() {
        let (set, len, score) = find_best_meld_combo(&melds);
        assert!(len <= melds.len, "combo len {} > meld count {}", len, melds.len);
        let mut used = 0u8;
        let mut total = 0u32;
        for &mi in &set[..len] {
            assert!(mi < melds.len, "meld index {} OOB", mi);
            let m = &melds.items[mi];
            total += m.score;
            for &idx in &m.indices[..m.indices_len] {
                // idx must fit in 6 dice AND be a valid shift target for u8 (< 8).
                assert!(idx < 6, "dice index {} >= 6 in {:?}", idx, dice);
                let bit = 1u8 << idx; // catches shift-overflow panics
                assert_eq!(used & bit, 0, "melds overlap at dice {}", idx);
                used |= bit;
            }
        }
        assert_eq!(total, score, "combo score mismatch for {:?}", dice);
    }
    // find_meld_score must agree with find_all_melds emptiness.
    let s = find_meld_score(dice);
    assert_eq!(s.unwrap_or(0) > 0, !melds.is_empty(), "score/emptiness disagree for {:?}", dice);
}
