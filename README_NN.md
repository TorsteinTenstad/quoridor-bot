# Neural Network Features for Quoridor Bot

This document describes how to use the neural network functionality in the Quoridor bot.

## Managing Neural Networks

The `quoridor-bot-nn` binary provides tools to create, save, load, and test neural networks.

### Saving a Zero-Weight Network

Create and save a network with zero-initialized weights (useful for testing):

```bash
cargo run --bin quoridor-bot-nn -- --save model.mpk
```

This creates a network where all weights are 0, making it predictable for testing purposes.

### Loading and Testing a Network

Load a saved network and test it:

```bash
cargo run --bin quoridor-bot-nn -- --load model.mpk
```

This will:
- Load the network from the specified file
- Run a simple forward pass test
- Display sample outputs to verify the network is working

### Running Tests

Run the test suite to verify the neural network implementation:

```bash
cargo run --bin quoridor-bot-nn -- --test
```

This runs three tests:
1. **Forward Pass Test**: Verifies basic network operation and batch processing
2. **Zero-Weight Network Test**: Confirms computation correctness with controlled weights  
3. **Input Differentiation Test**: Ensures the network can distinguish different inputs

## Playing with Neural Networks

Use the GUI to play games with neural network players.

### Basic Usage

Play as human (White) against a neural network (Black):

```bash
cargo run --bin quoridor-bot-gui -- --player-a Human --player-b NeuralNet
```

### Loading Trained Networks

Load a specific network for a player:

```bash
# Load network for White player
cargo run --bin quoridor-bot-gui -- \\
    --player-a NeuralNet --network-a-path model_white.mpk \\
    --player-b Human

# Load networks for both players
cargo run --bin quoridor-bot-gui -- \\
    --player-a NeuralNet --network-a-path model1.mpk \\
    --player-b NeuralNet --network-b-path model2.mpk
```

### Network Selection Behavior

- If you specify `--player-a NeuralNet` or `--player-b NeuralNet` **without** a network path, the program will create a new randomly initialized network
- If you provide a path with `--network-a-path` or `--network-b-path`, it will load the network from that file
- Random networks are unlikely to play well - you should train or load a trained network for meaningful gameplay

### Adjusting Temperature

Control the randomness of neural network move selection:

```bash
cargo run --bin quoridor-bot-gui -- \\
    --player-a NeuralNet --network-a-path trained.mpk \\
    --temperature 0.5
```

- `temperature = 0.0`: Greedy selection (always pick highest probability move)
- `temperature = 1.0`: Proportional to probability (default)
- `temperature > 1.0`: More random exploration

## Network Architecture

The `QuoridorNet` implements an AlphaZero-style policy-value network:

```
Input: [batch, 8, 9, 9]  # 8 channels of board state
    ↓
Conv2D: 8 → 64 channels (3x3 kernel) + ReLU
    ↓
Conv2D: 64 → 64 channels (3x3 kernel) + ReLU
    ↓
Flatten: [batch, 1600]
    ↓
    ├─→ Policy Head: Linear(1600 → 138)
    │   Output: [batch, 138] logits for all possible moves
    │
    └─→ Value Head: Linear(1600 → 64) → ReLU → Linear(64 → 1) → Tanh
        Output: [batch, 1] value estimate in [-1, 1]
```

### Input Encoding (8 channels)

1. **Channel 0**: White player position (1.0 at position, 0.0 elsewhere)
2. **Channel 1**: Black player position (1.0 at position, 0.0 elsewhere)
3. **Channel 2**: Horizontal walls (1.0 where placed)
4. **Channel 3**: Vertical walls (1.0 where placed)
5. **Channel 4**: White walls remaining (normalized by 10)
6. **Channel 5**: Black walls remaining (normalized by 10)
7. **Channel 6**: Current player indicator (1.0 for White's turn, 0.0 for Black)
8. **Channel 7**: (Reserved for future use)

## File Format

Networks are saved in MessagePack format (`.mpk` extension) using Burn's `NamedMpkFileRecorder` with full precision settings. This format:
- Preserves all weight values with full floating-point precision
- Includes layer structure and parameter names
- Is portable across different machines
- Can be loaded efficiently

## Next Steps

### Training a Network

To train a network, you'll need to implement:
1. Self-play game generation
2. Training loop with optimizer (e.g., Adam)
3. Loss functions (cross-entropy for policy, MSE for value)

### Integration with MCTS

The network is designed to work with Monte Carlo Tree Search:
- Policy output provides prior probabilities for move selection
- Value output guides tree evaluation
- Temperature controls exploration vs exploitation

## Example Workflow

```bash
# 1. Create a zero-weight network for testing
cargo run --bin quoridor-bot-nn -- --save zero_model.mpk

# 2. Test that it loads correctly
cargo run --bin quoridor-bot-nn -- --load zero_model.mpk

# 3. Play a game to see it in action
cargo run --bin quoridor-bot-gui -- \\
    --player-a Human \\
    --player-b NeuralNet --network-b-path zero_model.mpk \\
    --temperature 1.0

# 4. Run all tests
cargo run --bin quoridor-bot-nn -- --test
```

## Troubleshooting

**Network fails to load**:
- Verify the file exists and path is correct
- Ensure the file was created with a compatible version
- Check file isn't corrupted

**Network plays poorly**:
- Random/untrained networks won't play well
- Zero-weight networks will output constant values
- Train the network or use a pre-trained model

**Out of memory errors**:
- Reduce batch size if processing multiple positions
- Use a smaller network architecture
- Ensure you're using the NdArray backend (CPU) not GPU accidentally
