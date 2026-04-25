# Quoridor Bot Training Guide

## Overview

This guide explains how to use the AlphaZero-style training functionality for the Quoridor neural network bot.

## What's Implemented

The training infrastructure includes:

1. **MCTS (Monte Carlo Tree Search)** - Explores game tree using neural network guidance
2. **Self-Play** - Generates training data by playing games against itself
3. **Replay Buffer** - Stores game trajectories for training
4. **Training Loop** - Orchestrates self-play and training iterations
5. **Backpropagation** - Updates network weights using Adam optimizer
6. **Loss Computation** - Policy (cross-entropy) + Value (MSE) losses

### Current Status

✅ **Fully Implemented:**
- MCTS with UCT selection
- Dirichlet noise for exploration
- Self-play game generation
- Replay buffer management
- Game state encoding
- Win/loss detection
- Action space (138 moves)
- **Adam optimizer integration**
- **Cross-entropy policy loss**
- **MSE value loss**
- **Backpropagation and weight updates**
- **Checkpoint saving**

⚠️ **Note:**
- Training starts with random weights (weight transfer from initial network TODO)
- Model improves through self-play and gradient descent

## Quick Start

### 1. Run a Small Training Test

Test the self-play and MCTS functionality with a short training run:

```bash
cargo run --bin quoridor-bot-nn -- --train --iterations 2 --games-per-iter 3 --sims-per-move 100
```

This will:
- Run 2 training iterations
- Play 3 games per iteration
- Use 100 MCTS simulations per move
- Save the model to `trained_model.mpk`

### 2. Full Training Run (When Ready)

For actual training (once optimizer is implemented):

```bash
cargo run --bin quoridor-bot-nn --release -- \
  --train \
  --iterations 100 \
  --games-per-iter 50 \
  --sims-per-move 400 \
  --output my_trained_model.mpk
```

## Command Line Options

```
--train              Enable training mode
--iterations <N>     Number of training iterations (default: 100)
--games-per-iter <N> Self-play games per iteration (default: 10)
--sims-per-move <N>  MCTS simulations per move (default: 400)
--output <PATH>      Where to save trained model (default: trained_model.mpk)
```

## Training Configuration

Training parameters can be adjusted in the code:

### MCTS Configuration
```rust
MctsConfig {
    c_puct: 1.5,              // Exploration constant
    dirichlet_alpha: 0.3,     // Dirichlet noise alpha
    dirichlet_eps: 0.25,      // Dirichlet noise epsilon
    simulations: 400,         // MCTS simulations per move
    root_noise: true,         // Add noise at root for exploration
    temperature: 1.0,         // Move selection temperature
}
```

### Self-Play Configuration
```rust
SelfPlayCfg {
    sims_per_move: 400,       // MCTS sims per move
    temperature_moves: 10,    // Use temp=1 for first N moves
}
```

### Training Configuration
```rust
TrainCfg {
    batch_size: 128,          // Training batch size
    steps_per_iter: 100,      // Training steps per iteration
    games_per_iter: 10,       // Self-play games per iteration
    replay_size: 10_000,      // Replay buffer capacity
    iterations: 100,          // Total training iterations
}
```

## Architecture

### Game State Encoding

The network receives 8 input channels (9x9 each):
- Channel 0: White player position
- Channel 1: Black player position
- Channel 2: Horizontal walls
- Channel 3: Vertical walls
- Channel 4: White walls remaining (normalized)
- Channel 5: Black walls remaining (normalized)
- Channel 6: Current player indicator
- Channel 7: Reserved

### Network Output

- **Policy Head**: 178 outputs (one per possible move)
- **Value Head**: Single output in range [-1, 1]

### Action Space

178 total actions:
- 16 pawn moves (4 directions × 4 collision directions)
- 162 wall placements (81 positions × 2 orientations)

## How Training Works

1. **Self-Play Phase**
   - Network plays against itself using MCTS
   - Each move: run N MCTS simulations
   - Record (state, policy, outcome) tuples
   - Continue until game ends

2. **Training Phase**
   - Sample batches from replay buffer
   - Forward pass through network
   - Compute policy loss (cross-entropy with MCTS policy)
   - Compute value loss (MSE with game outcome)
   - Backpropagate gradients
   - Update network weights via Adam optimizer

3. **Iteration**
   - Repeat self-play → training cycle
   - Save checkpoints every 10 iterations
   - Monitor loss values
   - Network gradually improves

## Loss Functions

**Policy Loss (Cross-Entropy):**
```
L_policy = -Σ(target_policy * log(predicted_policy))
```
Encourages the network to match the MCTS-improved policy

**Value Loss (Mean Squared Error):**
```
L_value = (predicted_value - game_outcome)²
```
Trains the network to predict game outcomes accurately

**Total Loss:**
```
L_total = L_policy + L_value
```

## Next Steps

Optional improvements:

1. **Weight Transfer** - Transfer initial network weights to autodiff backend
2. **Learning Rate Scheduling** - Decay learning rate over time
3. **Network Evaluation** - Compare new vs old network in matches
4. **Data Augmentation** - Board symmetries (rotations/reflections)
5. **Resign Threshold** - End hopeless games early
6. **Temperature Annealing** - Gradually reduce exploration

## Testing

Run the built-in tests:

```bash
cargo run --bin quoridor-bot-nn -- --test
```

This tests:
- Forward pass correctness
- Zero-weight network behavior
- Biased network behavior
- Input differentiation

## Helper Functions

Key functions available in `nn_bot.rs`:

- `is_game_over(game)` - Check if game is finished
- `terminal_value(game)` - Get win/loss value
- `legal_action_ids(game)` - Get legal moves
- `apply_action(game, action_id)` - Apply move
- `game_to_key(game)` - Hash game state
- `encode(game)` - Encode for neural net

## Performance Tips

- Use `--release` mode for faster training
- Reduce `sims_per_move` for faster games (but worse play)
- Increase `games_per_iter` for more data per iteration
- Monitor replay buffer size vs training frequency

## Example Output

```
Starting training loop...
Config: 2 iterations, 3 games/iter, 100 steps/iter

=== Iteration 1/2 ===
Generating 3 self-play games...
  Game 1: 45 moves, result: 1
  Game 2: 52 moves, result: -1
  Game 3: 38 moves, result: 1
Replay buffer size: 405
Training for 100 steps...
  Step 0/100
  Step 20/100
  ...
```

## References

- AlphaZero Paper: https://arxiv.org/abs/1712.01815
- Burn Framework: https://burn.dev
- MCTS: https://en.wikipedia.org/wiki/Monte_Carlo_tree_search
