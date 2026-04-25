# Quoridor Bot Improvement Roadmap

This document outlines how to verify learning and build a strong Quoridor AI, from quick testing to expert-level play.

## Quick Verification (10-30 minutes)

Test that the training system is working correctly:

```bash
cargo run --release --bin quoridor-bot-nn -- \
  --train \
  --iterations 5 \
  --games-per-iter 5 \
  --sims-per-move 50 \
  --output quick_test.mpk
```

### What to Look For

**1. Loss Values Decreasing**
- Policy loss should trend downward (may fluctuate)
- Value loss should also decrease
- After 5 iterations, losses should be noticeably lower than iteration 1

**2. Game Length Increasing**
- Random play: typically 20-40 moves
- Improving play: 50-100+ moves
- Games ending in draws indicate learning board control

**3. Self-Play Patterns**
- Early: chaotic, quick games
- Later: more strategic, longer games
- Look for repeated structures (learning patterns)

**4. Model Comparison**
```bash
# Save initial random model
cargo run --release --bin quoridor-bot-nn -- --save random.mpk

# Train briefly  
cargo run --release --bin quoridor-bot-nn -- --train --iterations 10 --output trained.mpk

# Compare: play them against each other in GUI
# (Requires implementing model selection in GUI)
```

## Current Training Diagnostics

### Why Loss May Not Decrease

If you observe flat loss values during training (e.g., running 50 iterations with 5 games per iteration), this is likely due to:

**1. Overfitting on Limited Data**

With default settings:
- Training steps per iteration: 100
- Batch size: 128
- Total samples per iteration: 100 × 128 = 12,800 samples
- Unique positions generated: ~200 (from 5 games × ~40 positions each)

**Result:** The network sees each position ~64 times, memorizing rather than learning patterns.

**Solution:** Adjust the data/training ratio:

```bash
# Option A: Generate more data (recommended)
cargo run --release --bin quoridor-bot-nn -- \
  --train \
  --iterations 50 \
  --games-per-iter 20 \    # Increased from 5 → 800 positions/iter
  --sims-per-move 50 \
  --output better_ratio.mpk

# Option B: Reduce training steps (less preferred)
# Modify TrainCfg in main_nn.rs: steps_per_iter = 20
# This gives 20 × 128 = 2,560 samples, seeing each position ~13 times
```

**2. Stale Data in Replay Buffer**

Early training data comes from a random network making poor moves. This stale data poisons the buffer for hundreds of iterations.

**Solution:** Implement periodic buffer clearing:

```rust
// In train_loop(), add every 50 iterations:
if iter % 50 == 0 && iter > 0 {
    println!("Clearing replay buffer at iteration {}", iter);
    replay_buf = ReplayBuffer::new(10_000);
}
```

**3. Learning Rate Too High or Too Low**

Current: Adam optimizer with lr = 0.001 (default)

**Symptoms:**
- Loss oscillates wildly → learning rate too high
- Loss doesn't move → learning rate too low

**Solution:** Add learning rate decay (see Critical Improvements #3 below)

**4. Limited Network Capacity**

Current network architecture is very small compared to problem complexity.

### Network Architecture Details

**Current Structure:**

```
Input: 8 channels × 9×9 board
  ↓
Conv1: 8 → 64 channels, 3×3 kernel, padding=1
  → Output: 64 × 9×9 = 5,184 features
  → Parameters: (3×3×8 + 1 bias) × 64 = 4,672
  ↓
ReLU activation
  ↓
Conv2: 64 → 64 channels, 3×3 kernel, padding=1
  → Output: 64 × 9×9 = 5,184 features
  → Parameters: (3×3×64 + 1 bias) × 64 = 36,928
  ↓
ReLU activation
  ↓
Flatten: 64 × 9 × 9 = 5,184 → 1,600 features
  ↓
┌─────────────────┬──────────────────┐
│  Policy Head    │   Value Head     │
│  Linear(1600→178)│   Linear(1600→64)│
│  Parameters:    │   Parameters:    │
│  284,978        │   102,464        │
│                 │   ↓              │
│                 │   ReLU           │
│                 │   ↓              │
│                 │   Linear(64→1)   │
│                 │   Parameters: 65 │
└─────────────────┴──────────────────┘

Total Parameters: 429,107
```

**Comparison to AlphaZero:**
- AlphaZero (Chess/Go): 12-20 million parameters
- Current network: 429K parameters (~35x smaller)
- Quoridor complexity: Between Chess and Go in state space

**Impact:** Small network may struggle to learn complex positional patterns.

**Solution:** See Critical Improvements #5 (Larger Architecture) below.

### Recommended First Steps

1. **Run with better data ratio:**
   ```bash
   cargo run --release --bin quoridor-bot-nn -- \
     --train --iterations 50 \
     --games-per-iter 20 \  # 4x more data
     --sims-per-move 100 \  # Better move quality
     --output improved.mpk
   ```

2. **Monitor these metrics:**
   - Value loss should decrease (indicates learning board evaluation)
   - Average game length should increase (indicates better play)
   - Policy loss may stay high initially (that's okay)

3. **Add diagnostic logging** (modify `train_loop()` in nn_bot.rs):
   ```rust
   println!("Iter {}: Policy Loss={:.4}, Value Loss={:.4}, Avg Game Length={:.1}",
            iter, policy_loss, value_loss, avg_game_len);
   ```

## Known Limitations

### Weight Transfer Issue (HIGH PRIORITY)

**Problem:** Training currently starts from random weights each iteration.

**Impact:**
- Cannot resume from checkpoints
- Cannot build on previous training runs
- Each training session is independent

**Current Workaround:** Run long continuous sessions (don't stop training)

**Proper Fix Required:**
The issue is in `train_loop()` - needs to convert `NetworkModelRecord<NdArray>` to `NetworkModelRecord<Autodiff<NdArray>>`.

**Approaches to fix:**
1. Manual parameter copying layer by layer
2. Serialize to bytes, deserialize on new backend
3. Use same backend for training and inference (may require different burn features)

**Priority:** Fix this before serious training runs

## Building a Strong Bot

### Training Scale Requirements

| Bot Level | Iterations | Games/Iter | Sims/Move | Time (CPU) | Time (GPU) |
|-----------|-----------|------------|-----------|------------|------------|
| **Baseline** | 10-20 | 5-10 | 50 | 1-2 hours | 5-10 min |
| **Decent** | 50-100 | 20-50 | 200 | 12-24 hours | 1-2 hours |
| **Strong** | 500-1000 | 50-100 | 400-800 | 3-7 days | 6-24 hours |
| **Expert** | 1000+ | 100+ | 800+ | Weeks | Days |

### Critical Improvements

#### 1. GPU Acceleration (HIGHEST IMPACT)

**Current:** CPU-only (NdArray backend) - very slow

**Change to GPU backend:**

```toml
# Cargo.toml
burn = {version = "0.16.0", features = ["wgpu", "train", "std"]}
# Or for NVIDIA GPUs:
burn = {version = "0.16.0", features = ["tch", "train", "std"]}
```

**Expected speedup:** 50-100x faster for neural network operations

**Effort:** Low (just change dependencies and rebuild)
**Impact:** Game-changing for training speed

#### 2. Network Evaluation

**Current:** No quality assessment between iterations

**Add competitive evaluation:**

```rust
pub fn evaluate_networks(
    new_net: &QuoridorNet,
    old_net: &QuoridorNet,
    games: usize,
    sims_per_move: usize,
) -> f32 {
    let mut new_wins = 0;
    
    for game_idx in 0..games {
        // Play new_net as white vs old_net as black
        let white_net = if game_idx % 2 == 0 { new_net } else { old_net };
        let black_net = if game_idx % 2 == 0 { old_net } else { new_net };
        
        let winner = play_competitive_game(white_net, black_net, sims_per_move);
        
        if (game_idx % 2 == 0 && winner == Player::White) ||
           (game_idx % 2 == 1 && winner == Player::Black) {
            new_wins += 1;
        }
    }
    
    new_wins as f32 / games as f32
}

// In train_loop:
if iter % 10 == 0 && iter > 0 {
    let win_rate = evaluate_networks(&current_net, &best_net, 40, 200);
    if win_rate > 0.55 {
        println!("New network wins {:.1}% - promoting!", win_rate * 100.0);
        best_net = current_net.clone();
    }
}
```

**Effort:** Medium (2-3 hours)
**Impact:** High - ensures training actually improves

#### 3. Learning Rate Schedule

**Current:** Fixed learning rate (0.001)

**Add decay schedule:**

```rust
pub struct TrainCfg {
    pub initial_lr: f32,
    pub lr_decay_steps: Vec<usize>,  // [100, 200, 300]
    pub lr_decay_factor: f32,        // 0.1
    // ... existing fields
}

// In train_loop:
let lr = if iter < tcfg.lr_decay_steps[0] {
    tcfg.initial_lr
} else if iter < tcfg.lr_decay_steps[1] {
    tcfg.initial_lr * tcfg.lr_decay_factor
} else {
    tcfg.initial_lr * tcfg.lr_decay_factor * tcfg.lr_decay_factor
};

// Update optimizer learning rate
optim.set_lr(lr);
```

**Schedule:**
- 0-100 iterations: lr = 0.001
- 100-200: lr = 0.0001
- 200+: lr = 0.00001

**Effort:** Low (30 min)
**Impact:** Medium - improves convergence

#### 4. Data Augmentation

**Current:** Each position used only once

**Add board symmetry:**

```rust
pub fn augment_position(state: &EncodedState) -> EncodedState {
    // Quoridor has left-right symmetry
    let mut mirrored = state.clone();
    
    for channel in 0..state.c {
        for y in 0..9 {
            for x in 0..9 {
                // Mirror horizontally
                mirrored.planes[channel][y][x] = state.planes[channel][y][8 - x];
            }
        }
    }
    
    mirrored
}

// In ReplayBuffer::push_game:
pub fn push_game(&mut self, traj: &Trajectory) {
    for i in 0..traj.encodings.len() {
        let z = traj.result * traj.players[i] as f32;
        
        // Original position
        self.push(traj.encodings[i].clone(), traj.policies[i], z);
        
        // Mirrored position with mirrored policy
        let mirrored_state = augment_position(&traj.encodings[i]);
        let mirrored_policy = mirror_policy(&traj.policies[i]);
        self.push(mirrored_state, mirrored_policy, z);
    }
}
```

**Effort:** Medium (1-2 hours)
**Impact:** Medium - doubles effective training data

#### 5. Larger Network Architecture

**Current:** Small network (64 filters, 2 conv layers)

**AlphaZero-style residual network:**

```rust
// Increase filters
let conv_cfg = Conv2dConfig::new([8, 128], [3, 3]);  // 64 -> 128

// Add residual blocks
pub struct ResidualBlock<B: Backend> {
    conv1: Conv2d<B>,
    conv2: Conv2d<B>,
}

impl<B: Backend> ResidualBlock<B> {
    pub fn forward(&self, x: Tensor<B, 4>) -> Tensor<B, 4> {
        let residual = x.clone();
        let x = self.conv1.forward(x).relu();
        let x = self.conv2.forward(x);
        (x + residual).relu()
    }
}

// Stack 5-10 residual blocks
```

**Effort:** High (4-6 hours)
**Impact:** High - much stronger pattern recognition

#### 6. Advanced MCTS Improvements

**Virtual Loss (for parallel MCTS):**
```rust
// Prevent multiple simulations from exploring the same node
e.n += VIRTUAL_LOSS;  // Before selection
e.n -= VIRTUAL_LOSS;  // After backup
```

**Progressive Widening:**
```rust
// Limit action exploration early in search
let max_children = (sum_n.log2() * 2.0) as usize;
```

**PUCT-V (value-based exploration):**
```rust
// Use value variance for exploration bonus
let u = e.q + c_puct * e.p * sqrt(sum_n) / (1 + e.n) * (1 + e.variance);
```

**Effort:** Medium-High (3-5 hours)
**Impact:** Medium - better search quality

## Training Pipeline

### Phase 1: Bootstrap (First 100 iterations)

**Goal:** Build basic understanding

```bash
cargo run --release --bin quoridor-bot-nn -- \
  --train \
  --iterations 100 \
  --games-per-iter 20 \
  --sims-per-move 200 \
  --output bootstrap.mpk
```

**Configuration:**
- High exploration (temperature = 1.0 for 30 moves)
- Replay buffer: 10,000 positions
- Learning rate: 0.001
- Batch size: 128

**Expected:**
- Games get longer
- Value accuracy improves
- Policy becomes more focused

### Phase 2: Refinement (Iterations 100-300)

**Goal:** Develop strategic play

```bash
cargo run --release --bin quoridor-bot-nn -- \
  --load bootstrap.mpk \
  --train \
  --iterations 200 \
  --games-per-iter 50 \
  --sims-per-move 400 \
  --output refined.mpk
```

**Configuration:**
- Deeper search (400 sims)
- Lower learning rate: 0.0001
- Network evaluation every 10 iterations
- Larger replay buffer: 50,000

**Expected:**
- Win/loss balance around 50/50
- Sophisticated wall placement
- Long-term planning visible

### Phase 3: Expert Polish (Iterations 300+)

**Goal:** Fine-tune to expert level

```bash
cargo run --release --bin quoridor-bot-nn -- \
  --load refined.mpk \
  --train \
  --iterations 500 \
  --games-per-iter 100 \
  --sims-per-move 800 \
  --output expert.mpk
```

**Configuration:**
- Maximum search depth (800 sims)
- Very low learning rate: 0.00001
- Compete against best checkpoints
- Tournament selection

**Expected:**
- Near-optimal play
- Rare mistakes
- Consistent strategy

## Monitoring & Debugging

### Key Metrics to Track

**Loss Metrics:**
```rust
println!("Policy Loss: {:.4}, Value Loss: {:.4}, Total: {:.4}", 
         policy_loss, value_loss, policy_loss + value_loss);
```

**Game Statistics:**
```rust
println!("Avg game length: {:.1}, Max: {}, Min: {}", 
         avg_len, max_len, min_len);
println!("Win rate - White: {:.1}%, Black: {:.1}%, Draw: {:.1}%",
         white_pct, black_pct, draw_pct);
```

**Policy Metrics:**
```rust
let entropy = -pi.iter().map(|&p| if p > 0.0 { p * p.ln() } else { 0.0 }).sum::<f32>();
println!("Policy entropy: {:.2} (lower = more decisive)", entropy);
```

**Value Accuracy:**
```rust
let value_error = (predicted_value - actual_outcome).abs();
println!("Value prediction error: {:.3}", value_error);
```

### Common Issues

**Problem:** Loss stops decreasing
- **Cause:** Learning rate too high or low
- **Fix:** Adjust learning rate, check for overfitting

**Problem:** Games don't get longer
- **Cause:** Not learning strategic play
- **Fix:** Increase MCTS sims, check network capacity

**Problem:** Always same winner
- **Cause:** Imbalanced data or network bias
- **Fix:** Ensure equal white/black games, check initialization

**Problem:** Policy all zeros
- **Cause:** Gradient vanishing or explosion
- **Fix:** Check learning rate, network architecture, gradient clipping

## Estimated Timelines

### With Current CPU Implementation

| Goal | Training | Testing | Total |
|------|----------|---------|-------|
| Verify learning | 30 min | 10 min | 40 min |
| Beat random | 2-4 hours | 30 min | 3-5 hours |
| Beat greedy | 12-24 hours | 1 hour | 15-25 hours |
| Strong play | 3-7 days | 4 hours | ~1 week |
| Expert level | 2-4 weeks | 1 day | ~1 month |

### With GPU Implementation (50x faster)

| Goal | Training | Testing | Total |
|------|----------|---------|-------|
| Verify learning | 1 min | 5 min | 6 min |
| Beat random | 3-5 min | 10 min | 15 min |
| Beat greedy | 15-30 min | 15 min | 45 min |
| Strong play | 2-4 hours | 30 min | 3-5 hours |
| Expert level | 1-3 days | 2 hours | 2-4 days |

## Recommended Next Steps

### Immediate (This Week)

1. ✅ **Fix infinite loop** (DONE)
2. ⚠️ **Fix weight transfer** (IN PROGRESS - documented as limitation)
3. 🔄 **Quick verification run** (10 min)
4. 📊 **Add loss logging** (30 min)

### Short Term (This Month)

1. 🚀 **Switch to GPU backend** (1 hour setup, massive speedup)
2. 🎯 **Implement network evaluation** (2-3 hours)
3. 📈 **Add learning rate schedule** (30 min)
4. 🔬 **Train baseline model** (overnight)

### Medium Term (Next 3 Months)

1. 🏗️ **Larger network architecture** (4-6 hours)
2. 🎲 **Data augmentation** (1-2 hours)
3. 🎮 **GUI integration** for model comparison
4. 📦 **Tournament system** for checkpoint selection

### Long Term (6+ Months)

1. 🧠 **Residual network architecture**
2. ⚡ **Parallel MCTS** (tree parallelism)
3. 🌐 **Distributed training** across machines
4. 🏆 **Public benchmark** against other bots

## Quick Reference Commands

```bash
# Quick test (5 min)
cargo run --release --bin quoridor-bot-nn -- --train \
  --iterations 3 --games-per-iter 3 --sims-per-move 20

# Overnight training (8-12 hours)
cargo run --release --bin quoridor-bot-nn -- --train \
  --iterations 100 --games-per-iter 20 --sims-per-move 200

# Weekend training (48 hours)
cargo run --release --bin quoridor-bot-nn -- --train \
  --iterations 500 --games-per-iter 50 --sims-per-move 400

# Save and resume
cargo run --release --bin quoridor-bot-nn -- --train \
  --iterations 50 --output checkpoint.mpk
# Note: Resuming not yet supported due to weight transfer issue

# Compare models (future feature)
cargo run --release --bin quoridor-bot-nn -- \
  --evaluate model1.mpk model2.mpk --games 100
```

## Success Criteria

### Milestone 1: Learning Verified ✓
- [ ] Loss decreases over 10 iterations
- [ ] Games get longer (40+ moves)
- [ ] No crashes or infinite loops

### Milestone 2: Beats Random
- [ ] >80% win rate vs random play
- [ ] Average game length >50 moves
- [ ] Uses walls strategically

### Milestone 3: Competent Play
- [ ] >70% win rate vs greedy algorithm
- [ ] Demonstrates blocking strategies
- [ ] Plans 5+ moves ahead

### Milestone 4: Strong Play
- [ ] Competitive with intermediate human players
- [ ] Sophisticated wall placement
- [ ] Recognizes won/lost positions

### Milestone 5: Expert Level
- [ ] Competitive with strong human players
- [ ] Minimal mistakes in standard positions
- [ ] Creative in unusual positions

---

**Last Updated:** Based on current implementation (MCTS + AlphaZero training)
**Status:** Core training working, weight transfer needs fix, GPU strongly recommended
