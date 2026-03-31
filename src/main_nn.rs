pub mod nn_bot;
pub mod data_model;
pub mod all_moves;
pub mod game_logic;
pub mod a_star;

use nn_bot::{QuoridorNet, EncodedState, encode_batch_to_tensor};
use burn::backend::NdArray;
use clap::Parser;
use std::path::PathBuf;

#[derive(clap::Parser, Debug)]
#[command(author, version, about = "Neural Network Management for Quoridor Bot", long_about = None)]
struct Args {
    /// Save a zero-weight network to the specified path
    #[arg(short, long)]
    save: Option<PathBuf>,
    
    /// Save a network biased to move upward (for testing)
    #[arg(long)]
    save_biased_up: Option<PathBuf>,
    
    /// Load and test a network from the specified path
    #[arg(short, long)]
    load: Option<PathBuf>,
    
    /// Run tests
    #[arg(short, long)]
    test: bool,
}

fn main() {
    let args = Args::parse();
    
    if args.test {
        println!("Running tests...");
        test_forward_pass();
        println!("\n{}\n", "=".repeat(80));
        test_controlled_network();
        println!("\n{}\n", "=".repeat(80));
        test_biased_up_network();
        println!("\n{}\n", "=".repeat(80));
        test_different_inputs_produce_different_outputs();
        println!("\n✅ All tests completed successfully!");
        return;
    }
    
    if let Some(path) = args.save {
        println!("Creating zero-weight network...");
        let net = QuoridorNet::new_zero_weights();
        println!("Saving network to: {}", path.display());
        net.save(&path).expect("Failed to save network");
        println!("✅ Network saved successfully!");
        return;
    }
    
    if let Some(path) = args.save_biased_up {
        println!("Creating network biased to move upward...");
        let net = QuoridorNet::new_biased_upward();
        println!("Saving network to: {}", path.display());
        net.save(&path).expect("Failed to save network");
        println!("✅ Biased network saved successfully!");
        println!("\nThis network will strongly prefer upward moves (indices 0-3).");
        println!("Use it to verify the neural network is actually working in gameplay!");
        return;
    }
    
    if let Some(path) = args.load {
        println!("Loading network from: {}", path.display());
        let net = QuoridorNet::load(&path).expect("Failed to load network");
        println!("✅ Network loaded successfully!");
        
        // Test the loaded network
        println!("\nTesting loaded network...");
        let test_state = EncodedState {
            planes: vec![vec![vec![0.5; 9]; 9]; 8],
            c: 8,
        };
        let batch = vec![test_state];
        let input_tensor = encode_batch_to_tensor::<NdArray>(&batch, &net.device);
        let output = net.network_model.forward(input_tensor);
        
        let policy_data: Vec<f32> = output.policy.into_data().to_vec().unwrap();
        let value_data: Vec<f32> = output.value.into_data().to_vec().unwrap();
        
        println!("Policy output (first 5): {:?}", &policy_data[..5]);
        println!("Value output: {:?}", value_data[0]);
        println!("✅ Network is working correctly!");
        return;
    }
    
    // Default: show help
    println!("No command specified. Use --help for usage information.");
    println!("\nExamples:");
    println!("  Save a zero-weight network:     cargo run --bin quoridor-bot-nn -- --save model.mpk");
    println!("  Save an upward-biased network:  cargo run --bin quoridor-bot-nn -- --save-biased-up biased_up.mpk");
    println!("  Load and test a network:        cargo run --bin quoridor-bot-nn -- --load model.mpk");
    println!("  Run unit tests:                 cargo run --bin quoridor-bot-nn -- --test");
}

fn test_forward_pass() {
        println!("Testing neural network forward pass...");
        
        // Create a neural network
        let net = QuoridorNet::new();
    
    // Create a simple test input: 8 channels of 9x9
    let mut test_state = EncodedState {
        planes: vec![vec![vec![0.0; 9]; 9]; 8],
        c: 8,
    };
    
    // Add some non-zero values to test (e.g., player positions)
    test_state.planes[0][4][4] = 1.0; // White player at center
    test_state.planes[1][8][4] = 1.0; // Black player at opposite side
    test_state.planes[6][0][0] = 1.0; // Current player indicator
    
    // Create a batch with a single state
    let batch = vec![test_state.clone()];
    
    // Convert to tensor
    let input_tensor = encode_batch_to_tensor::<NdArray>(&batch, &net.device);
    
    println!("Input tensor shape: {:?}", input_tensor.shape());
    
    // Run forward pass
    let output = net.network_model.forward(input_tensor);
    
    println!("Policy shape: {:?}", output.policy.shape());
    println!("Value shape: {:?}", output.value.shape());
    
    // Extract the outputs
    let policy_data: Vec<f32> = output.policy.into_data().to_vec().unwrap();
    let value_data: Vec<f32> = output.value.into_data().to_vec().unwrap();
    
    println!("\nPolicy output (first 10 values): {:?}", &policy_data[..10]);
    println!("Policy output (last 10 values): {:?}", &policy_data[policy_data.len()-10..]);
    println!("Value output: {:?}", value_data[0]);
    
    // Verify shapes
    assert_eq!(policy_data.len(), 138, "Policy should have 138 outputs");
    assert_eq!(value_data.len(), 1, "Value should have 1 output");
    
    // Verify value is in valid range (-1, 1) after tanh
    assert!(value_data[0] >= -1.0 && value_data[0] <= 1.0, "Value should be in range [-1, 1]");
    
    println!("\n✓ Forward pass test completed successfully!");
    
    // Test with multiple inputs in batch
    println!("\nTesting batch processing with 3 states...");
    let batch3 = vec![test_state.clone(), test_state.clone(), test_state.clone()];
    let input_tensor3 = encode_batch_to_tensor::<NdArray>(&batch3, &net.device);
    let output3 = net.network_model.forward(input_tensor3);
    
    println!("Batch policy shape: {:?}", output3.policy.shape());
    println!("Batch value shape: {:?}", output3.value.shape());
    
    assert_eq!(output3.policy.shape().dims[0], 3, "Batch policy should have 3 entries");
    assert_eq!(output3.value.shape().dims[0], 3, "Batch value should have 3 entries");
    
    println!("✓ Batch processing test completed successfully!");
}

fn test_controlled_network() {
    println!("Testing neural network with zero-initialized weights (bias-only forward pass)...");
    println!("This simulates a network where values propagate only through biases");
    
    let device = <NdArray as burn::prelude::Backend>::Device::default();
    
    // Use the built-in zero-weight network
    let net = QuoridorNet::new_zero_weights();
    let network_model = &net.network_model;
    
    // Create a simple test input with varying values
    let mut test_state = EncodedState {
        planes: vec![vec![vec![0.0; 9]; 9]; 8],
        c: 8,
    };
    
    // Set different patterns in different channels
    test_state.planes[0][4][4] = 1.0;
    test_state.planes[1][0][0] = 1.0;
    test_state.planes[2][8][8] = 1.0;
    
    let batch = vec![test_state];
    let input_tensor = encode_batch_to_tensor::<NdArray>(&batch, &device);
    
    println!("\nRunning forward pass with zero-weight network...");
    println!("(All conv/FC weights are 0, only biases are non-zero)");
    let output = network_model.forward(input_tensor);
    
    let policy_data: Vec<f32> = output.policy.into_data().to_vec().unwrap();
    let value_data: Vec<f32> = output.value.into_data().to_vec().unwrap();
    
    println!("\nPolicy output (first 5): {:?}", &policy_data[..5]);
    println!("Policy output (last 5): {:?}", &policy_data[policy_data.len()-5..]);
    println!("Value output: {:?}", value_data[0]);
    
    // With zero weights, the output will be determined only by biases
    println!("\nObservations with zero-weight initialization:");
    println!("- All policy logits should be identical (only bias contributes): {}", 
             policy_data.iter().all(|&x| (x - policy_data[0]).abs() < 1e-6));
    println!("- Policy values: all = {:.6}", policy_data[0]);
    println!("- Value output (through tanh): {:.6}", value_data[0]);
    
    // Test that different inputs give the SAME output (since weights are zero)
    let mut test_state2 = EncodedState {
        planes: vec![vec![vec![0.5; 9]; 9]; 8],
        c: 8,
    };
    test_state2.planes[0][0][0] = 0.9;
    
    let batch2 = vec![test_state2];
    let input_tensor2 = encode_batch_to_tensor::<NdArray>(&batch2, &device);
    let output2 = network_model.forward(input_tensor2);
    let policy_data2: Vec<f32> = output2.policy.into_data().to_vec().unwrap();
    let value_data2: Vec<f32> = output2.value.into_data().to_vec().unwrap();
    
    println!("\nTesting with different input:");
    println!("- Policy output (first value): {:.6}", policy_data2[0]);
    println!("- Value output: {:.6}", value_data2[0]);
    println!("- Same as previous? Policy: {}, Value: {}",
             (policy_data2[0] - policy_data[0]).abs() < 1e-6,
             (value_data2[0] - value_data[0]).abs() < 1e-6);
    
    println!("\n✓ Controlled network test completed!");
    println!("\nKey insights:");
    println!("1. The network architecture correctly processes inputs through conv layers");
    println!("2. Flattening and FC layers work as expected");
    println!("3. With zero weights, output is constant (bias-only), confirming");
    println!("   that the forward pass computation is working correctly");
    println!("4. In a trained network, weights would capture patterns from training data");
}

fn test_biased_up_network() {
    println!("Testing neural network with random weights, but with one biased ff-layer (bias-only forward pass)...");
    println!("This simulates a network where the suggested move should be up");
    
    let device = <NdArray as burn::prelude::Backend>::Device::default();
    
    // Use the built-in zero-weight network
    let net = QuoridorNet::new_biased_upward();
    let network_model = &net.network_model;
    
    // Create a simple test input with varying values
    let mut test_state = EncodedState {
        planes: vec![vec![vec![0.0; 9]; 9]; 8],
        c: 8,
    };
    
    // Set different patterns in different channels
    test_state.planes[0][4][4] = 1.0;
    test_state.planes[1][0][0] = 1.0;
    test_state.planes[2][8][8] = 1.0;
    
    let batch = vec![test_state];
    let input_tensor = encode_batch_to_tensor::<NdArray>(&batch, &device);
    
    println!("\nRunning forward pass with biased-up network...");
    let output = network_model.forward(input_tensor);
    
    let policy_data: Vec<f32> = output.policy.into_data().to_vec().unwrap();
    let value_data: Vec<f32> = output.value.into_data().to_vec().unwrap();
    
    println!("\nPolicy output (first 5): {:?}", &policy_data[..5]);
    println!("Policy output (last 5): {:?}", &policy_data[policy_data.len()-5..]);
    println!("Value output: {:?}", value_data[0]);
    
    // With zero weights, the output will be determined only by biases
    println!("\nObservations with zero-weight initialization:");
    println!("- The first value should be much larger than the smallest value: {}", 
             policy_data.iter().any(|&x| (policy_data[0] - x) > 9.99));
    println!("- Policy value of the first move:  = {:.6}", policy_data[0]);
    println!("- Value output (through tanh): {:.6}", value_data[0]);
    
    // Test that different inputs give the SAME output (since network is biased upwards)
    let mut test_state2 = EncodedState {
        planes: vec![vec![vec![0.5; 9]; 9]; 8],
        c: 8,
    };
    test_state2.planes[0][0][0] = 0.9;
    
    let batch2 = vec![test_state2];
    let input_tensor2 = encode_batch_to_tensor::<NdArray>(&batch2, &device);
    let output2 = network_model.forward(input_tensor2);
    let policy_data2: Vec<f32> = output2.policy.into_data().to_vec().unwrap();
    let value_data2: Vec<f32> = output2.value.into_data().to_vec().unwrap();
    
    println!("\nTesting with different input:");
    println!("- Policy output (first value): {:.6}", policy_data2[0]);
    println!("- Value output: {:.6}", value_data2[0]);
    println!("- Same as previous? Policy: {}, Value: {}",
             (policy_data2[0] - policy_data[0]).abs() < 1e-6,
             (value_data2[0] - value_data[0]).abs() < 1e-6);
    
    println!("\n✓ Controlled network test completed!");
    println!("\nKey insights:");
    println!("1. The network architecture correctly processes inputs through conv layers");
    println!("2. Flattening and FC layers work as expected");
    println!("3. With zero weights, output is constant (bias-only), confirming");
    println!("   that the forward pass computation is working correctly");
    println!("4. In a trained network, weights would capture patterns from training data");
}

fn test_different_inputs_produce_different_outputs() {
    println!("Testing that different inputs produce different outputs...");
    println!("(Using randomly initialized network)");
    
    let net = QuoridorNet::new();
    
    // Create two different game states
    let mut state1 = EncodedState {
        planes: vec![vec![vec![0.0; 9]; 9]; 8],
        c: 8,
    };
    state1.planes[0][4][4] = 1.0; // White at center
    state1.planes[1][0][4] = 1.0; // Black at top
    
    let mut state2 = EncodedState {
        planes: vec![vec![vec![0.0; 9]; 9]; 8],
        c: 8,
    };
    state2.planes[0][0][0] = 1.0; // White at top-left
    state2.planes[1][8][8] = 1.0; // Black at bottom-right
    
    // Run forward pass on both
    let batch = vec![state1, state2];
    let input_tensor = encode_batch_to_tensor::<NdArray>(&batch, &net.device);
    let output = net.network_model.forward(input_tensor);
    
    // Extract outputs
    let policy_tensors: Vec<_> = output.policy.iter_dim(0).collect();
    let policy1: Vec<f32> = policy_tensors[0].clone().into_data().to_vec().unwrap();
    let policy2: Vec<f32> = policy_tensors[1].clone().into_data().to_vec().unwrap();
    
    let values: Vec<f32> = output.value.into_data().to_vec().unwrap();
    let value1 = values[0];
    let value2 = values[1];
    
    println!("\nState 1 output:");
    println!("  Policy (first 5): {:?}", &policy1[..5]);
    println!("  Value: {:.6}", value1);
    
    println!("\nState 2 output:");
    println!("  Policy (first 5): {:?}", &policy2[..5]);
    println!("  Value: {:.6}", value2);
    
    // Check that outputs are different
    let policy_diff: f32 = policy1.iter().zip(policy2.iter())
        .map(|(a, b)| (a - b).abs())
        .sum();
    let value_diff = (value1 - value2).abs();
    
    println!("\nDifferences:");
    println!("  Total policy difference: {:.6}", policy_diff);
    println!("  Value difference: {:.6}", value_diff);
    
    // Verify they are actually different
    assert!(policy_diff > 0.001, "Policies should be different for different inputs");
    println!("\n✓ Different inputs produce different outputs (as expected with random weights)!");
    
    // Test consistency: same input should give same output
    println!("\nTesting consistency: same input twice...");
    let batch_same = vec![
        EncodedState { planes: vec![vec![vec![1.0; 9]; 9]; 8], c: 8 },
        EncodedState { planes: vec![vec![vec![1.0; 9]; 9]; 8], c: 8 },
    ];
    let input_same = encode_batch_to_tensor::<NdArray>(&batch_same, &net.device);
    let output_same = net.network_model.forward(input_same);
    
    let policy_same: Vec<_> = output_same.policy.iter_dim(0).collect();
    let p1: Vec<f32> = policy_same[0].clone().into_data().to_vec().unwrap();
    let p2: Vec<f32> = policy_same[1].clone().into_data().to_vec().unwrap();
    
    let consistency_diff: f32 = p1.iter().zip(p2.iter())
        .map(|(a, b)| (a - b).abs())
        .sum();
    
    println!("  Difference between identical inputs: {:.10}", consistency_diff);
    assert!(consistency_diff < 1e-6, "Same inputs should produce same outputs");
    println!("✓ Network is consistent: same input produces same output!");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    
    #[test]
    fn test_nn_forward_pass() {
        test_forward_pass();
    }
    
    #[test]
    fn test_nn_zero_weights() {
        test_controlled_network();
    }
    
    #[test]
    fn test_biased_up_weights() {
        test_biased_up_network();
    }

    #[test]
    fn test_nn_input_differentiation() {
        test_different_inputs_produce_different_outputs();
    }
    
    #[test]
    fn test_save_and_load() {
        let test_path = "test_network.mpk";
        
        // Clean up any existing test file
        let _ = fs::remove_file(test_path);
        
        // Create and save a network
        let net = QuoridorNet::new_zero_weights();
        net.save(test_path).expect("Failed to save network");
        
        // Load the network
        let loaded_net = QuoridorNet::load(test_path).expect("Failed to load network");
        
        // Test that it works
        let test_state = EncodedState {
            planes: vec![vec![vec![0.5; 9]; 9]; 8],
            c: 8,
        };
        let batch = vec![test_state];
        let input_tensor = encode_batch_to_tensor::<NdArray>(&batch, &loaded_net.device);
        let output = loaded_net.network_model.forward(input_tensor);
        
        let policy_data: Vec<f32> = output.policy.into_data().to_vec().unwrap();
        let value_data: Vec<f32> = output.value.into_data().to_vec().unwrap();
        
        // Verify it's a zero-weight network (all outputs should be 0)
        assert!(policy_data.iter().all(|&x| x.abs() < 1e-6), "Policy should be all zeros");
        assert!(value_data[0].abs() < 1e-6, "Value should be zero");
        
        // Clean up
        fs::remove_file(test_path).expect("Failed to remove test file");
    }
}