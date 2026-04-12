use crate::generic_heuristic::GenericHeuristicWeights;

pub type Genes = GenericHeuristicWeights;

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Population(pub Vec<Genes>);

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EvaluatedGenes {
    pub genes: Genes,
    pub win_rate: f32,
}

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EvaluatedPopulation(pub Vec<EvaluatedGenes>);
