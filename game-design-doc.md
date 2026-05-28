# Farkle UEFI Game - Game Design Document

## Game Overview

**Farkle** (also known as Zonk, Hot Dice, Ten Thousand) is a press-your-luck dice game for 2+ players. The UEFI version is a single-player game against an AI opponent, rendered entirely as a graphical UEFI application with pixel-based UI.

- **Target Platform**: UEFI (EmulatorPkg / real firmware)
- **Players**: 1 Human vs 1 AI
- **Win Condition**: First to reach **5,000 points**
- **Input**: Keyboard (arrow keys, Space, Enter, Esc)

---

## Game Flow (State Machine)

```
┌──────────┐    Start     ┌──────────────┐
│  TITLE   │──────────────▶│ PLAYER_TURN  │◀──────────────────────┐
│  SCREEN  │               │              │                        │
└──────────┘               ├──────────────┤                        │
                           │ ROLLING      │ (dice animating)       │
                           ├──────────────┤                        │
                           │ SELECTING    │ (player picks melds)   │
                           ├──────────────┤                        │
                           │ BANKING      │ (end turn, score adds) │
                           ├──────────────┤                        │
                           │ FARKLE       │ (bust! no points)      │
                           └──────┬───────┘                        │
                                  │                                │
                  ┌───────┐       ▼                                │
                  │  AI   │  ┌───────────┐                         │
                  │ TURN  │──│  SWITCH   │─────────────────────────┘
                  └───────┘  │  PLAYER   │
                             └─────┬─────┘
                                   │ (someone reaches 5000+)
                                   ▼
                             ┌───────────┐
                             │ GAME OVER │
                             └───────────┘
```

### State Details

| State | Description | Transitions |
|-------|-------------|-------------|
| **TITLE** | Game title screen, "Press Enter to Start" | → PLAYER_TURN on Enter |
| **ROLLING** | Dice animate for ~1 second | → SELECTING after animation |
| **SELECTING** | Player uses arrow keys + Space to select scoring dice | → ROLLING (re-roll remaining) / → BANKING (end turn) / → FARKLE (no meld possible) |
| **BANKING** | Short display of earned points; score adds to total | → AI_TURN or → GAME_OVER |
| **FARKLE** | "Farkle!" display, no points earned | → AI_TURN |
| **AI_TURN** | AI plays automatically with simple strategy | → PLAYER_TURN or → GAME_OVER |
| **GAME_OVER** | Final scores, winner announcement | → TITLE on Enter |

---

## Scoring Rules (Melds)

Players set aside at least one scoring die/meld per roll. Remaining dice are re-rolled. If no meld is rolled, the player "Farkles" and scores 0 for the turn.

| Meld | Points |
|------|--------|
| Each 1 | 100 |
| Each 5 | 50 |
| Three 1s | 1,000 |
| Three 2s | 200 |
| Three 3s | 300 |
| Three 4s | 400 |
| Three 5s | 500 |
| Three 6s | 600 |
| Four of any number | 1,000 |
| 1-to-6 Straight | 1,500 |
| Three pairs | 1,500 |
| Five of any number | 2,000 |
| Two triplets | 2,500 |
| Six of any number | 3,000 |

### Gameplay Rules Implemented

1. **Must score**: At least one scoring die/meld must be set aside each roll
2. **Hot dice**: If all 6 dice score, player MUST re-roll all 6 (tracked automatically)
3. **Farkle**: Roll with zero scoring dice → lose all unbanked turn points
4. **Minimum bank**: No minimum to bank (can bank any turn score)
5. **Re-roll available dice**: Non-scoring dice are re-rolled; previously set-aside dice stay set aside
6. **Equal turns**: AI gets a final turn if player reaches 5000 first

---

## AI Strategy

A simple rule-based AI that makes decisions each roll:

1. **Detect all possible melds** in the current roll
2. **Prioritize high-value melds**: two triplets > straight > five/6 of a kind > four of a kind > three of a kind
3. **Bank decision**: Bank if:
   - Turn score ≥ 500 points, OR
   - Only 1-2 dice remain to re-roll (high Farkle risk), OR
   - Turn score + total ≥ 5000 (winning)
4. **Otherwise continue rolling**

---

## UI Design

### Screen Layout (Minimum 640×480)

```
┌─────────────────────────────────────────────────┐
│  F A R K L E                                    │  ← Title bar (40px)
├─────────────────────────────────────────────────┤
│                                                 │
│    Player: 2350         Computer: 1980          │  ← Scoreboard (60px)
│                                                 │
├─────────────────────────────────────────────────┤
│                                                 │
│    ┌───┐  ┌───┐  ┌───┐  ┌───┐  ┌───┐  ┌───┐   │
│    │ ● │  │   │  │● ●│  │   │  │● ●│  │   │   │  ← Dice area
│    │   │  │ ● │  │   │  │   │  │   │  │   │   │     (200px, 6 dice)
│    │   │  │   │  │● ●│  │   │  │● ●│  │   │   │
│    └───┘  └───┘  └───┘  └───┘  └───┘  └───┘   │
│      ★      2      3             5     [ ]      │  ← Selection indicator
│                                                 │
├─────────────────────────────────────────────────┤
│  Turn Score: 350  │  Remaining Dice: 3          │  ← Turn info (40px)
├─────────────────────────────────────────────────┤
│  [ Roll ]   [ Bank ]                            │  ← Action buttons (60px)
│                                                 │
│  ←→ Select Die   Space:Mark   Enter:Confirm     │  ← Help bar (30px)
└─────────────────────────────────────────────────┘
```

### Dice Face Rendering

Each die is a rounded rectangle with pips drawn as filled circles. Die values 1-6:

```
 ┌─────┐  ┌─────┐  ┌─────┐  ┌─────┐  ┌─────┐  ┌─────┐
 │     │  │ ●   │  │ ●   │  │ ● ● │  │ ● ● │  │ ● ● │
 │  ●  │  │     │  │  ●  │  │     │  │  ●  │  │ ● ● │
 │     │  │   ● │  │   ● │  │ ● ● │  │ ● ● │  │ ● ● │
 └─────┘  └─────┘  └─────┘  └─────┘  └─────┘  └─────┘
    1        2        3        4        5        6
```

- Selected dice highlighted with yellow border
- "Held" (already scored) dice shown dimmed or with lock icon
- During roll animation, dice cycle through random values rapidly

### Color Palette

| Element | Color (Hex) | Use |
|---------|------------|-----|
| Background | `#1a1a2e` | Dark blue background |
| Dice face | `#e6e6e6` | Off-white die body |
| Dice pip | `#2d2d2d` | Dark pip dots |
| Selection highlight | `#f0c040` | Gold border |
| Held die | `#888888` | Gray dimmed |
| Score text | `#ffffff` | White |
| Turn score | `#4ecca3` | Teal green |
| Farkle text | `#e74c3c` | Red |
| Button (roll) | `#2ecc71` | Green |
| Button (bank) | `#e67e22` | Orange |
| Title | `#f0c040` | Gold |

### Animations

1. **Dice roll**: Rapid value cycling (60ms per frame) for ~1 second, then settle
2. **Score increment**: Counter animation when points are added
3. **Farkle flash**: Red screen flash + "FARKLE!" text bounce
4. **Win celebration**: Gold border pulsing on winner's score

---

## Input Handling

### Controls

| Input | Context | Action |
|-------|---------|--------|
| Space | SELECTING | Toggle selection of the highlighted die |
| Arrow Left/Right | SELECTING | Move highlight between dice |
| Enter | SELECTING | Confirm selection → ROLLING (re-roll remaining) |
| Enter | Can bank? | End turn, bank points |
| R | SELECTING | Quick re-roll current selection |
| B | SELECTING | Bank turn score |
| Esc | any | Quit to title screen |
| Enter | TITLE / GAME_OVER | Start new game |

### Input Loop

```
loop {
    wait_for_key_event(timeout=16ms)  // ~60 FPS polling
    if key_pressed:
        dispatch(key)
    if state_has_animation:
        advance_animation_frame()
        render()
}
```

---

## Data Structures

### Game State

```rust
struct GameState {
    phase: GamePhase,
    players: [Player; 2],
    current_player: usize,        // 0 = Human, 1 = AI
    dice: [u8; 6],               // Current dice values (1-6)
    selected_dice: [bool; 6],    // Which dice player has selected this roll
    held_dice: [bool; 6],        // Which dice are already scored & set aside
    turn_score: u32,             // Unbanked score for current turn
    cursor: usize,               // Which die the selection cursor is on (0-5)
    roll_count_this_turn: u32,   // How many rolls this turn
    animation: Option<Animation>,
}
```

### Player

```rust
struct Player {
    name: &'static str,
    total_score: u32,
    is_human: bool,
}
```

### Animation State

```rust
enum Animation {
    Rolling {
        frames_remaining: u32,     // ~16 frames at 60fps
        current_frame: u32,
    },
    Farkle {
        frames_remaining: u32,
    },
    ScoreIncrement {
        from: u32,
        to: u32,
        progress: f32,
    },
    None,
}
```

---

## MelD Detection Algorithm

```rust
fn find_melds(dice: &[u8; 6]) -> Vec<Meld> {
    // 1. Count occurrences of each value 1-6
    // 2. Check for special combinations:
    //    - Six of a kind: count[val] == 6
    //    - Two triplets: two values with count == 3
    //    - Straight: one of each 1-6
    //    - Three pairs: three values with count == 2
    //    - Five of a kind: count[val] == 5
    //    - Four of a kind: count[val] == 4
    //    - Three of a kind: count[val] >= 3
    // 3. Remaining 1s and 5s are single melds
}
```

---

## Edge Cases

| Scenario | Handling |
|----------|----------|
| All 6 dice score (Hot Dice) | Player MUST re-roll all 6; held dice reset |
| Player selects invalid meld | Ignore selection; show brief "Invalid meld" text |
| AI reaches 5000 on same round | Player gets final turn (equal turns rule) |
| Zero remaining dice after selection | Auto-detect hot dice; all dice become available |
| Consecutive Farkles | Track; maybe show "Ouch!" after 2 in a row |
| Screen too small | Minimum 640×480; scale dice smaller if needed |
